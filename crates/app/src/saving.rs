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

/// What the save thread reports.
#[derive(Debug)]
pub enum Update {
    Progress { done: u64, total: u64 },
    Finished(Outcome),
}

/// How a save ended.
#[derive(Debug)]
pub enum Outcome {
    Saved(Saved),
    /// Written to a temporary file that still has to replace its target.
    Staged(Box<Staged>),
    Cancelled,
    Failed(String),
}

/// A save in progress, cancelled when dropped.
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
    spawn(move |progress, cancel| {
        if staged {
            stage(&request, progress, cancel).map(|staged| Outcome::Staged(Box::new(staged)))
        } else {
            save(&request, progress, cancel).map(Outcome::Saved)
        }
    })
}

/// Start computing and writing a band of a capture, with the same updates as a save.
pub fn start_extraction(
    request: crate::extraction::ExtractRequest,
) -> (Job, async_channel::Receiver<Update>) {
    spawn(move |progress, cancel| {
        crate::extraction::run(&request, progress, cancel).map(Outcome::Saved)
    })
}

/// Run `work` on the save thread, carrying its progress and outcome back.
fn spawn(
    work: impl FnOnce(&mut dyn FnMut(u64, u64), &AtomicBool) -> Result<Outcome, WriteError>
    + Send
    + 'static,
) -> (Job, async_channel::Receiver<Update>) {
    let cancel = Arc::new(AtomicBool::new(false));
    let (finished_tx, finished) = mpsc::channel();
    let (sender, receiver) = async_channel::bounded(2);
    let job = Job {
        cancel: cancel.clone(),
        finished,
    };
    let outgoing = sender.clone();
    let spawned = std::thread::Builder::new()
        .name("argand-save".into())
        .spawn(move || {
            let mut last = Instant::now();
            let mut progress = |done: u64, total: u64| {
                if done == total || last.elapsed() >= PROGRESS_INTERVAL {
                    last = Instant::now();
                    let _ = sender.try_send(Update::Progress { done, total });
                }
            };
            let outcome = match work(&mut progress, &cancel) {
                Ok(outcome) => outcome,
                Err(WriteError::Cancelled) => Outcome::Cancelled,
                Err(error) => {
                    let text = message(&error);
                    tracing::warn!(error = %text, "save failed");
                    Outcome::Failed(text)
                }
            };
            // The temporary file is gone, or handed over, once the write returns.
            let _ = finished_tx.send(());
            let _ = sender.send_blocking(Update::Finished(outcome));
        });
    if let Err(error) = spawned {
        let _ = outgoing.try_send(Update::Finished(Outcome::Failed(error.to_string())));
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

/// The folder the dialog opens in, which is the source's own.
pub fn directory(source: &Path) -> PathBuf {
    source
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf)
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
    let (unit, divisor) = [("GHz", 1e9), ("MHz", 1e6), ("kHz", 1e3)]
        .into_iter()
        .find(|(_, divisor)| band.high().abs().max(band.low().abs()) >= *divisor)
        .unwrap_or(("Hz", 1.));
    let frequencies = format!(
        "{:.3}-{:.3}{unit}",
        band.low() / divisor,
        band.high() / divisor
    );
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

#[cfg(test)]
mod tests {
    include!("saving_tests.rs");
}
