//! Suppressing a band of a capture, over the whole of it or a time span, on a thread of its own.
//!
//! The edited capture is read through an `EditedSource`, first a few segments
//! to measure the noise beside the band, then the whole region, which
//! `argand_dsp::suppress` fills with that noise in place of the band. The samples
//! it changed are written as unit-scale `f32` into a scratch file, which the
//! edit then reads in place of the span.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use argand_core::{Adjustment, FrequencyBand, SampleSource, SampleSpan, SignalMeta};
use argand_dsp::{NoiseFill, NoiseFloor, SuppressPlan, Suppressor};
use argand_edit::Capture;
use argand_io::write::{FloatWave, FloatWaveRequest, SourceFile, SourceStamp, WriteError};

use crate::saving::{Ended, Job, Update};
use crate::scratch::Scratch;

/// Input samples read at a time, which bounds what one block holds.
const BLOCK_SAMPLES: usize = 1 << 18;

/// A band to suppress in a version of a capture.
#[derive(Debug, Clone)]
pub struct SuppressRequest {
    pub capture: Capture,
    /// The files it reads, by source id, with none for those it does not read.
    pub sources: Vec<Option<SourceFile>>,
    /// The capture as the window describes it, with its rate, reference frequency and kind.
    pub meta: SignalMeta,
    /// The band in physical hertz.
    pub band: FrequencyBand,
    /// The time the band is suppressed over, none for the whole capture.
    pub time: Option<SampleSpan>,
    /// Where the computed samples go.
    pub target: PathBuf,
}

/// The computed samples of a suppression and the span they replace.
#[derive(Debug)]
pub struct Suppressed {
    pub span: SampleSpan,
    /// Where in the scratch file the span's samples begin, after a sample that aligns their parity.
    pub offset: u64,
    pub scratch: Arc<Scratch>,
    /// The scratch file as it reads, its length the span's.
    pub meta: SignalMeta,
    pub stamp: Option<SourceStamp>,
}

impl SuppressRequest {
    /// The band's edges relative to the capture's baseband 0 Hz, held to the capture's band.
    fn edges(&self) -> (f64, f64) {
        let nyquist = self.meta.sample_rate / 2.0;
        let floor = if self.meta.is_iq() { -nyquist } else { 0.0 };
        let reference = self.meta.center_freq;
        (
            (self.band.low() - reference).max(floor),
            (self.band.high() - reference).min(nyquist),
        )
    }

    /// Whether the band can be suppressed, quickly enough for the window's thread.
    pub fn check(&self) -> Result<(), String> {
        let (low, high) = self.edges();
        SuppressPlan::check(self.meta.sample_rate, low, high, !self.meta.is_iq())
            .map_err(|error| error.to_string())
    }

    fn plan(&self) -> Result<SuppressPlan, WriteError> {
        let (low, high) = self.edges();
        SuppressPlan::new(self.meta.sample_rate, low, high, !self.meta.is_iq()).map_err(|error| {
            WriteError::Unsupported {
                path: self.meta.source.clone(),
                reason: error.to_string(),
            }
        })
    }

    /// Samples put before the computed ones so each shares the parity of the sample it replaces.
    ///
    /// A real inversion signs a sample by its index in its own file, so a scratch
    /// sample read at another parity than the one it replaces would turn over
    /// once the inversion is undone.
    fn parity_offset(&self, start: u64) -> u64 {
        if self.meta.is_iq() {
            return 0;
        }
        let (index, offset) = self.capture.locate(start);
        self.capture
            .pieces()
            .get(index)
            .map_or(0, |piece| (piece.start + offset) % 2)
    }

    /// The span the samples change in, with the taper's rise and fall within it.
    ///
    /// A time span is widened by the taper on each side, so the whole of it is
    /// suppressed, and held to the capture, where no taper is needed.
    fn region(&self, taper: u64) -> Option<(SampleSpan, u64, u64)> {
        let len = self.capture.len();
        let Some(time) = self.time else {
            return SampleSpan::between(0, len).map(|span| (span, 0, 0));
        };
        let time = time.within(len)?;
        let start = time.start().saturating_sub(taper);
        let end = time.end().saturating_add(taper).min(len);
        let span = SampleSpan::between(start, end)?;
        Some((span, time.start() - start, end - time.end()))
    }
}

/// Compute the suppressed samples, reporting `(input samples done, total)` and stopping when `cancel` is set.
pub fn run(
    request: &SuppressRequest,
    progress: &mut dyn FnMut(u64, u64),
    cancel: &AtomicBool,
) -> Result<Suppressed, WriteError> {
    let plan = request.plan()?;
    let (span, rise, fall) = request.region(plan.taper()).ok_or(WriteError::OutOfRange)?;
    let mut source = crate::extraction::open(&request.capture, &request.sources, &request.meta)?;
    // The samples are computed as shown and stored so that the adjustment reads them back so.
    let stored = request.capture.adjustment().inverse();
    let len = request.capture.len();
    let margin = plan.margin();
    let from = span.start().saturating_sub(margin);
    let to = span.end().saturating_add(margin).min(len);
    let floor = NoiseFloor::new(&plan, span.count());
    let measured = floor.segments().len() as u64 * floor.segment_len() as u64;
    let total = measured + (to - from);
    progress(0, total);
    let fill = measure(
        &mut source,
        floor,
        span,
        &request.meta,
        &mut |done| progress(done, total),
        cancel,
    )?;
    let suppressor = Suppressor::new(&plan, span.count(), rise, fall).map_err(|error| {
        WriteError::Unsupported {
            path: request.meta.source.clone(),
            reason: error.to_string(),
        }
    })?;
    let mut suppressor = match fill {
        Some(fill) => suppressor.filled_with(fill),
        None => suppressor,
    };
    let offset = request.parity_offset(span.start());
    let mut file = FloatWave::create(FloatWaveRequest {
        meta: request.meta.clone(),
        samples: offset + span.count(),
        target: request.target.clone(),
        sources: request.sources.iter().flatten().cloned().collect(),
        protected: Vec::new(),
    })?;
    file.write(&vec![[0.0; 2]; offset as usize])?;
    let mut written = offset;
    let mut out = Vec::new();
    suppressor.push_zeros(margin - (span.start() - from), &mut out);
    let channels = request.meta.channels();
    let path = &request.meta.source;
    let read_error = |error| WriteError::Source {
        path: path.clone(),
        source: error,
    };
    source.seek(from).map_err(read_error)?;
    let mut buffer = vec![0.0f32; BLOCK_SAMPLES * channels];
    let mut done = 0;
    while done < to - from {
        if cancel.load(Ordering::Relaxed) {
            return Err(WriteError::Cancelled);
        }
        let want = ((to - from - done) as usize).min(BLOCK_SAMPLES) * channels;
        let read = source.read(&mut buffer[..want]).map_err(read_error)?;
        if read == 0 {
            return Err(WriteError::SourceChanged { path: path.clone() });
        }
        suppressor.push(&buffer[..read], channels, &mut out);
        store(stored, &mut out, channels, written);
        written += out.len() as u64;
        file.write(&out)?;
        out.clear();
        done += (read / channels) as u64;
        progress(measured + done, total);
    }
    suppressor.push_zeros(suppressor.wanted(), &mut out);
    store(stored, &mut out, channels, written);
    file.write(&out)?;
    if cancel.load(Ordering::Relaxed) {
        return Err(WriteError::Cancelled);
    }
    crate::extraction::unchanged(&request.sources)?;
    let saved = file.finish(cancel)?;
    let scratch = Arc::new(Scratch::new(saved.path.clone()));
    let (opened, stamp) = argand_io::open_stamped(&saved.path, &argand_io::OpenHints::default())
        .map_err(|error| WriteError::Unsupported {
            path: saved.path.clone(),
            reason: error.to_string(),
        })?;
    let meta = opened.meta().clone();
    if meta.len_samples != offset + span.count() || meta.channels() != channels {
        return Err(WriteError::SourceChanged { path: saved.path });
    }
    Ok(Suppressed {
        span,
        offset,
        scratch,
        meta,
        stamp,
    })
}

/// Turn computed samples into what `stored` reads back as them, the first being sample `first` of the scratch file.
fn store(stored: Adjustment, samples: &mut [[f32; 2]], channels: usize, first: u64) {
    for (offset, sample) in samples.iter_mut().enumerate() {
        stored.apply(&mut sample[..channels], channels, first + offset as u64);
    }
}

/// Measure the noise beside the band over `span` of `source`, reporting samples read.
fn measure(
    source: &mut dyn SampleSource,
    mut floor: NoiseFloor,
    span: SampleSpan,
    meta: &SignalMeta,
    progress: &mut dyn FnMut(u64),
    cancel: &AtomicBool,
) -> Result<Option<NoiseFill>, WriteError> {
    let channels = meta.channels();
    let size = floor.segment_len();
    let mut segment = vec![0.0f32; size * channels];
    let read_error = |error| WriteError::Source {
        path: meta.source.clone(),
        source: error,
    };
    for (index, start) in floor.segments().into_iter().enumerate() {
        if cancel.load(Ordering::Relaxed) {
            return Err(WriteError::Cancelled);
        }
        source.seek(span.start() + start).map_err(read_error)?;
        let mut filled = 0;
        while filled < segment.len() {
            let read = source.read(&mut segment[filled..]).map_err(read_error)?;
            if read == 0 {
                return Err(WriteError::SourceChanged {
                    path: meta.source.clone(),
                });
            }
            filled += read;
        }
        floor.push(start, &segment, channels);
        progress((index as u64 + 1) * size as u64);
    }
    Ok(floor.finish())
}

/// Start computing a suppression, with progress as a save reports it.
pub fn start(
    request: SuppressRequest,
) -> (Job, async_channel::Receiver<Update<Ended<Suppressed>>>) {
    crate::saving::spawn(
        "argand-suppress",
        move |progress, cancel| {
            crate::saving::ended(run(&request, progress, cancel), "suppression")
        },
        Ended::Failed,
    )
}

#[cfg(test)]
mod tests {
    include!("suppression_tests.rs");
}
