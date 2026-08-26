use std::path::PathBuf;

use serde::Serialize;

use crate::sample::SampleType;

/// What a reader could work out about a file before any samples are read.
#[derive(Debug, Clone, Serialize)]
pub struct SignalMeta {
    /// True capture rate in Hz. Not an audio rate: it may be megahertz.
    pub sample_rate: f64,
    /// Centre of the frequency axis in Hz. 0 means baseband.
    pub center_freq: f64,
    pub sample_type: SampleType,
    /// Number of samples, not of scalar values: an I/Q sample counts once.
    pub len_samples: u64,
    /// Container the reader recognised, for the report ("wav", "flac", "raw").
    pub container: &'static str,
    /// What raw sample values were divided by to reach the unit scale.
    ///
    /// Usually the format's full scale; a normalization pass replaces it with
    /// the measured peak. Multiplying a unit-scale value by this recovers the
    /// number actually stored in the file.
    pub divisor: f32,
    pub source: PathBuf,
}

impl SignalMeta {
    /// Scalar values per sample.
    pub fn channels(&self) -> usize {
        self.sample_type.channels()
    }

    pub fn is_iq(&self) -> bool {
        self.sample_type.is_iq()
    }

    pub fn duration_seconds(&self) -> f64 {
        if self.sample_rate > 0.0 {
            self.len_samples as f64 / self.sample_rate
        } else {
            0.0
        }
    }

    /// Lowest and highest frequency the spectrum covers, in Hz.
    ///
    /// Complex signals are two-sided around `center_freq`; real signals occupy
    /// `center_freq .. center_freq + Fs/2`.
    pub fn frequency_span(&self) -> (f64, f64) {
        let nyquist = self.sample_rate / 2.0;
        if self.is_iq() {
            (self.center_freq - nyquist, self.center_freq + nyquist)
        } else {
            (self.center_freq, self.center_freq + nyquist)
        }
    }
}

/// Half-open span of samples to analyse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SampleRange {
    pub start: u64,
    pub len: u64,
}

impl SampleRange {
    pub const fn new(start: u64, len: u64) -> Self {
        Self { start, len }
    }

    pub const fn end(&self) -> u64 {
        self.start + self.len
    }

    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Clamp to what the signal actually holds.
    pub fn clamped_to(&self, total: u64) -> Self {
        let start = self.start.min(total);
        Self {
            start,
            len: self.len.min(total - start),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SourceError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("seek past end of signal: sample {requested} of {total}")]
    SeekOutOfRange { requested: u64, total: u64 },
    #[error("decode failed: {0}")]
    Decode(String),
}

/// A lazily-read signal.
///
/// Implemented in `argand-io`, consumed by `argand-dsp`. Keeping it here is
/// what stops DSP from knowing about file formats and IO from knowing about
/// transforms.
pub trait SampleSource: Send {
    fn meta(&self) -> &SignalMeta;

    /// Position the next `read` at `sample` (an I/Q pair counts as one).
    fn seek(&mut self, sample: u64) -> Result<(), SourceError>;

    /// Fill `buf` from the current position, advancing it.
    ///
    /// Values are scaled to [-1, 1] using the format's full scale, with any
    /// normalization and gain already applied. I/Q arrives interleaved as
    /// `I, Q, I, Q, ...`, so callers should size `buf` in whole samples.
    ///
    /// Returns the number of scalar values written; a short result means the
    /// signal ended.
    fn read(&mut self, buf: &mut [f32]) -> Result<usize, SourceError>;
}

#[cfg(test)]
mod tests {
    include!("signal_tests.rs");
}
