//! Saving a capture, or a time selection of it, on a thread of its own.
//!
//! The writing itself is `argand_io::write`; this module runs it away from the
//! window, carries its progress back and decides what the dialog proposes.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use argand_core::{SampleSpan, SignalMeta};
use argand_io::write::{SaveRequest, Saved, Staged, WriteError, save, stage};

/// How often progress crosses to the window.
const PROGRESS_INTERVAL: Duration = Duration::from_millis(100);
/// How long closing the window waits for a cancelled save to remove its temporary file.
const SHUTDOWN_WAIT: Duration = Duration::from_secs(1);

/// What a save thread reports, ending with its outcome.
#[derive(Debug)]
pub enum Update<T = Outcome> {
    Progress { done: u64, total: u64 },
    Finished(T),
}

/// How a save ended.
#[derive(Debug)]
pub enum Outcome {
    Saved(Saved),
    Encoded {
        saved: Saved,
        clipped: u64,
    },
    /// Written to a temporary file that still has to replace its target.
    Staged(Box<Staged>),
    Cancelled,
    Failed(String),
}

/// A save, or other work on a thread of its own, in progress, cancelled when dropped.
pub struct Job {
    cancel: Arc<AtomicBool>,
    finished: mpsc::Receiver<()>,
}

impl Job {
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

impl Drop for Job {
    fn drop(&mut self) {
        self.cancel();
        let _ = self.finished.recv_timeout(SHUTDOWN_WAIT);
    }
}

/// Start writing `request` and return the job with its stream of updates.
///
/// With `staged` the temporary file is handed back rather than moved into place.
pub fn start(request: SaveRequest, staged: bool) -> (Job, async_channel::Receiver<Update>) {
    spawn_save(move |progress, cancel| {
        if staged {
            stage(&request, progress, cancel).map(|staged| Outcome::Staged(Box::new(staged)))
        } else {
            save(&request, progress, cancel).map(Outcome::Saved)
        }
    })
}

/// Start computing and writing a band of a capture, with the same updates as a save.
pub fn start_extraction(
    request: crate::processing::extraction::ExtractRequest,
    container: Option<argand_io::write::Container>,
) -> (Job, async_channel::Receiver<Update>) {
    spawn_save(move |progress, cancel| {
        if container == Some(argand_io::write::Container::Flac) {
            return extract_flac(request, progress, cancel)
                .map(|(saved, clipped)| Outcome::Encoded { saved, clipped });
        }
        crate::processing::extraction::run(&request, progress, cancel).map(Outcome::Saved)
    })
}

fn extract_flac(
    mut request: crate::processing::extraction::ExtractRequest,
    progress: &mut dyn FnMut(u64, u64),
    cancel: &AtomicBool,
) -> Result<(Saved, u64), WriteError> {
    use argand_io::write::{Container, Output, Protected, SourceFile};
    let folder = crate::scratch::folder().ok_or(WriteError::OutOfRange)?;
    let target = request.target.clone();
    let sample_type = request.meta.sample_type;
    let mut output = request.output_meta()?;
    output.sample_type = sample_type;
    Container::Flac.check(&output, output.len_samples)?;
    let original_sources = request.sources.clone();
    let mut protected = request.protected.clone();
    protected.extend(request.sources.iter().flatten().map(|source| Protected {
        path: source.meta.source.clone(),
        stamp: source.stamp,
    }));
    let float_path = folder.new_path("wav");
    let _float = crate::scratch::Scratch::new(float_path.clone());
    let encoded_path = folder.new_path("wav");
    let _encoded = crate::scratch::Scratch::new(encoded_path.clone());
    request.target = float_path.clone();
    crate::processing::extraction::run(
        &request,
        &mut |done, total| phase_progress(progress, 0, done, total),
        cancel,
    )?;
    let hints = argand_io::OpenHints {
        normalize: Some(argand_io::Normalize::None),
        ..Default::default()
    };
    let (source, stamp) =
        argand_io::open_stamped(&float_path, &hints).map_err(|error| WriteError::Unsupported {
            path: float_path.clone(),
            reason: error.to_string(),
        })?;
    let input = source.meta().clone();
    drop(source);
    let mut output = input.clone();
    output.sample_type = sample_type;
    let request = crate::processing::conversion::Request {
        capture: argand_edit::Capture::whole(argand_edit::SourceId(0), input.len_samples),
        sources: vec![Some(SourceFile {
            meta: input.clone(),
            hints: hints.clone(),
            stamp,
            encoding: None,
        })],
        input,
        output,
        carrier: None,
        container: Container::Flac,
        target: encoded_path.clone(),
        protected: protected.clone(),
        level: crate::processing::conversion::Level::Preserve,
        full_scale: 1.0,
        copy: None,
    };
    let converted = crate::processing::conversion::run(
        &request,
        &mut |done, total| phase_progress(progress, 1, done, total),
        cancel,
    )?;
    let (source, stamp) = argand_io::open_stamped(&encoded_path, &hints).map_err(|error| {
        WriteError::Unsupported {
            path: encoded_path.clone(),
            reason: error.to_string(),
        }
    })?;
    let source = SourceFile {
        meta: source.meta().clone(),
        hints,
        stamp,
        encoding: None,
    };
    let mut save_request = SaveRequest::span(source, None, target);
    save_request.output = Output::Container(Container::Flac);
    save_request.protected = protected;
    let staged = stage(
        &save_request,
        &mut |done, total| phase_progress(progress, 2, done, total),
        cancel,
    )?;
    if cancel.load(Ordering::Relaxed) {
        return Err(WriteError::Cancelled);
    }
    crate::processing::extraction::unchanged(&original_sources)?;
    let saved = staged.commit().map_err(|refused| refused.error)?;
    Ok((saved, converted.clipped))
}

pub fn target_output(
    output: argand_io::write::Output,
    current: &str,
    target: &Path,
) -> argand_io::write::Output {
    use argand_io::write::{Container, Output};
    let container = match target.extension().and_then(|extension| extension.to_str()) {
        Some(extension) if extension.eq_ignore_ascii_case("wav") => Container::Wave,
        Some(extension) if extension.eq_ignore_ascii_case("flac") => Container::Flac,
        _ => return output,
    };
    if output == Output::Native && container.extension() == current {
        return output;
    }
    Output::Container(container)
}

fn phase_progress(progress: &mut dyn FnMut(u64, u64), phase: u64, done: u64, total: u64) {
    let within = (u128::from(done) * 1_000_000 / u128::from(total.max(1))) as u64;
    progress(phase * 1_000_000 + within, 3_000_000);
}

/// How a write that ended in `result` is reported, a cancellation apart from a failure.
pub fn ended<T>(result: Result<T, WriteError>, what: &str) -> Ended<T> {
    match result {
        Ok(value) => Ended::Done(value),
        Err(WriteError::Cancelled) => Ended::Cancelled,
        Err(error) => {
            let text = message(&error);
            tracing::warn!(error = %text, "{what} failed");
            Ended::Failed(text)
        }
    }
}

/// How work other than a save ended.
#[derive(Debug)]
pub enum Ended<T> {
    Done(T),
    Cancelled,
    Failed(String),
}

fn spawn_save(
    work: impl FnOnce(&mut dyn FnMut(u64, u64), &AtomicBool) -> Result<Outcome, WriteError>
    + Send
    + 'static,
) -> (Job, async_channel::Receiver<Update>) {
    spawn(
        "argand-save",
        move |progress, cancel| match ended(work(progress, cancel), "save") {
            Ended::Done(outcome) => outcome,
            Ended::Cancelled => Outcome::Cancelled,
            Ended::Failed(text) => Outcome::Failed(text),
        },
        Outcome::Failed,
    )
}

/// Run `work` on a thread named `name`, carrying its progress and outcome back, or `failed` when no thread starts.
pub fn spawn<T: Send + 'static>(
    name: &str,
    work: impl FnOnce(&mut dyn FnMut(u64, u64), &AtomicBool) -> T + Send + 'static,
    failed: impl FnOnce(String) -> T,
) -> (Job, async_channel::Receiver<Update<T>>) {
    let cancel = Arc::new(AtomicBool::new(false));
    let (finished_tx, finished) = mpsc::channel();
    let (sender, receiver) = async_channel::bounded(2);
    let job = Job {
        cancel: cancel.clone(),
        finished,
    };
    let outgoing = sender.clone();
    let spawned = std::thread::Builder::new()
        .name(name.into())
        .spawn(move || {
            let mut last = Instant::now();
            let mut progress = |done: u64, total: u64| {
                if done == total || last.elapsed() >= PROGRESS_INTERVAL {
                    last = Instant::now();
                    let _ = sender.try_send(Update::Progress { done, total });
                }
            };
            let outcome = work(&mut progress, &cancel);
            // The temporary file is gone, or handed over, once the write returns.
            let _ = finished_tx.send(());
            let _ = sender.send_blocking(Update::Finished(outcome));
        });
    if let Err(error) = spawned {
        let _ = outgoing.try_send(Update::Finished(failed(error.to_string())));
    }
    (job, receiver)
}

/// The error with its causes, as one line for the status bar.
pub fn message(error: &WriteError) -> String {
    let mut text = error.to_string();
    let mut cause = std::error::Error::source(error);
    while let Some(next) = cause {
        text.push_str(": ");
        text.push_str(&next.to_string());
        cause = next.source();
    }
    text
}

/// Whether two paths name the same file, comparing canonical forms where they exist.
pub fn same_path(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    }
}

/// The suggested source folder; resolve its availability away from the UI thread.
pub fn directory(source: &Path) -> PathBuf {
    source
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf()
}

pub fn existing_directory(path: &Path) -> PathBuf {
    path.ancestors().find(|parent| parent.is_dir()).map_or_else(
        || std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        Path::to_path_buf,
    )
}

/// The name the dialog proposes, with the selection's bounds in seconds.
///
/// A capture saved as WAVE whatever its own container gets that extension.
pub fn suggested_name(meta: &SignalMeta, as_wave: bool, span: Option<SampleSpan>) -> String {
    let source = &meta.source;
    let stem = source
        .file_stem()
        .map_or_else(|| "capture".into(), |stem| stem.to_string_lossy());
    let extension = match source.extension() {
        Some(extension) if !as_wave => extension.to_string_lossy().into_owned(),
        _ => "wav".into(),
    };
    match span {
        Some(span) => {
            let seconds = |sample: u64| sample as f64 / meta.sample_rate;
            format!(
                "{stem}_{:.3}-{:.3}s.{extension}",
                seconds(span.start()),
                seconds(span.end())
            )
        }
        None => format!("{stem}.{extension}"),
    }
}

/// The name the dialog proposes for a band, with the span's bounds in seconds when it has one.
pub fn suggested_band_name(
    meta: &SignalMeta,
    span: Option<SampleSpan>,
    band: argand_core::FrequencyBand,
) -> String {
    let stem = meta
        .source
        .file_stem()
        .map_or_else(|| "capture".into(), |stem| stem.to_string_lossy());
    let frequencies = band_label(band);
    match span {
        Some(span) => {
            let seconds = |sample: u64| sample as f64 / meta.sample_rate;
            format!(
                "{stem}_{:.3}-{:.3}s_{frequencies}.wav",
                seconds(span.start()),
                seconds(span.end())
            )
        }
        None => format!("{stem}_{frequencies}.wav"),
    }
}

/// A band's edges in the largest unit that keeps them apart in six decimals, three at least.
fn band_label(band: argand_core::FrequencyBand) -> String {
    let magnitude = band.high().abs().max(band.low().abs());
    let width = band.high() - band.low();
    let (unit, divisor, decimals) = [("GHz", 1e9), ("MHz", 1e6), ("kHz", 1e3), ("Hz", 1.)]
        .into_iter()
        .filter(|&(unit, divisor)| unit == "Hz" || magnitude >= divisor)
        .map(|(unit, divisor)| {
            (
                unit,
                divisor,
                (-(width / divisor).log10()).ceil().max(3.0) as usize,
            )
        })
        .find(|&(unit, _, decimals)| unit == "Hz" || decimals <= 6)
        .unwrap_or(("Hz", 1., 3));
    format!(
        "{:.decimals$}-{:.decimals$}{unit}",
        band.low() / divisor,
        band.high() / divisor
    )
}

#[cfg(test)]
#[path = "saving_test.rs"]
mod tests;
