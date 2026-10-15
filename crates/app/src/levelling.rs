//! Measuring the peak of a capture, over the whole of it, on a thread of its own.
//!
//! The peak is read from the sources' own samples, before the capture's
//! adjustment, so the gain a normalization sets replaces any gain before it.

use std::sync::atomic::{AtomicBool, Ordering};

use argand_core::{Adjustment, SampleSource, SignalMeta};
use argand_edit::Capture;
use argand_io::write::{SourceFile, WriteError};

use crate::saving::{Ended, Job, Update};

/// Samples read at a time.
const BLOCK_SAMPLES: usize = 1 << 18;

/// The lowest target level a normalization accepts, in dBFS.
pub const LOWEST_TARGET: f64 = -60.0;
/// The target level a normalization proposes first, in dBFS.
pub const DEFAULT_TARGET: f64 = -1.0;

/// A version of a capture whose peak is wanted.
#[derive(Debug, Clone)]
pub struct LevelRequest {
    pub capture: Capture,
    /// The files it reads, by source id, with none for those it does not read.
    pub sources: Vec<Option<SourceFile>>,
    pub meta: SignalMeta,
}

/// The largest `|I + jQ|` of a complex capture or `|x|` of a real one, in read values.
pub fn run(
    request: &LevelRequest,
    progress: &mut dyn FnMut(u64, u64),
    cancel: &AtomicBool,
) -> Result<f64, WriteError> {
    let unadjusted = request.capture.with_adjustment(Adjustment::IDENTITY);
    let mut source = crate::extraction::open(&unadjusted, &request.sources, &request.meta)?;
    let total = unadjusted.len();
    let channels = request.meta.channels();
    let path = &request.meta.source;
    let read_error = |error| WriteError::Source {
        path: path.clone(),
        source: error,
    };
    progress(0, total);
    source.seek(0).map_err(read_error)?;
    let mut buffer = vec![0.0f32; BLOCK_SAMPLES * channels];
    let mut peak = 0.0f64;
    let mut done = 0;
    while done < total {
        if cancel.load(Ordering::Relaxed) {
            return Err(WriteError::Cancelled);
        }
        let want = ((total - done) as usize).min(BLOCK_SAMPLES) * channels;
        let read = source.read(&mut buffer[..want]).map_err(read_error)?;
        if read == 0 {
            return Err(WriteError::SourceChanged { path: path.clone() });
        }
        peak = buffer[..read]
            .chunks_exact(channels)
            .map(|sample| match sample {
                [i, q] => f64::from(*i).hypot(f64::from(*q)),
                _ => f64::from(sample[0]).abs(),
            })
            .fold(peak, f64::max);
        done += (read / channels) as u64;
        progress(done, total);
    }
    crate::extraction::unchanged(&request.sources)?;
    Ok(peak)
}

/// The gain that puts `peak` at `target` dBFS, where `full_scale` is the read value of full scale.
///
/// None for a silent capture or a target outside what a normalization accepts.
pub fn gain(peak: f64, full_scale: f64, target: f64) -> Option<f64> {
    if !(LOWEST_TARGET..=0.0).contains(&target) || peak <= 0.0 || !peak.is_finite() {
        return None;
    }
    let gain = 10f64.powf(target / 20.0) * full_scale / peak;
    (gain.is_finite() && gain > 0.0).then_some(gain)
}

/// Start measuring a peak, with progress as a save reports it.
pub fn start(request: LevelRequest) -> (Job, async_channel::Receiver<Update<Ended<f64>>>) {
    crate::saving::spawn(
        "argand-level",
        move |progress, cancel| crate::saving::ended(run(&request, progress, cancel), "measuring"),
        Ended::Failed,
    )
}

#[cfg(test)]
mod tests {
    include!("levelling_tests.rs");
}
