//! Memory-mapped reader for fixed-width interleaved samples.
//!
//! Serves both WAVE and headerless files: once the header has told us where
//! the samples start and how wide they are, the two cases are identical. The
//! mapping means a multi-hour capture never lands in the process heap.

use std::fs::File;
use std::path::Path;

use argand_core::{AccessPattern, SampleRange, SampleSource, SampleType, SignalMeta, SourceError};
use memmap2::Mmap;

use crate::convert::convert;
use crate::normalize::{Normalize, gain_factor, resolve_divisor_with_budget};

pub struct MmapSource {
    map: Mmap,
    data_offset: usize,
    data_len: usize,
    meta: SignalMeta,
    /// Combined normalization and gain multiplier.
    scale: f32,
    /// Divisor the level scan settled on, kept for the report.
    divisor: f32,
    pos: u64,
    /// Byte offset up to which pages have already been released.
    released: usize,
    access: AccessPattern,
}

/// How far behind the read head pages are released, and how often.
///
/// A mapped page stays resident until the kernel needs the memory, so a
/// straight pass over a multi-gigabyte capture would report the whole file as
/// resident. Dropping what has already been consumed keeps the footprint flat
/// without giving up the mapping.
const RELEASE_LAG: usize = 2 << 20;
const RELEASE_STEP: usize = 8 << 20;

impl MmapSource {
    /// Map `path` and interpret `data_offset .. data_offset + data_len` as
    /// interleaved samples of `meta.sample_type`.
    pub fn new(
        path: &Path,
        meta: SignalMeta,
        data_offset: usize,
        data_len: usize,
        normalize: Normalize,
        gain_db: f32,
    ) -> Result<Self, SourceError> {
        Self::with_scan_budget(path, meta, data_offset, data_len, normalize, gain_db, None)
    }

    /// Map a source with a caller-specific automatic normalization scan budget.
    pub fn with_scan_budget(
        path: &Path,
        mut meta: SignalMeta,
        data_offset: usize,
        data_len: usize,
        normalize: Normalize,
        gain_db: f32,
        scan_bytes: Option<usize>,
    ) -> Result<Self, SourceError> {
        let file = File::open(path)?;
        // Safety: the file is opened read-only and the mapping is never
        // handed out as a mutable slice. A concurrent truncation is the one
        // hazard, and it is the same one every mmap-based reader accepts.
        let map = unsafe { Mmap::map(&file)? };
        #[cfg(unix)]
        let _ = map.advise(memmap2::Advice::Sequential);

        let available = map.len().saturating_sub(data_offset);
        let bytes_per_sample = meta.sample_type.bytes_per_sample();
        let data_len = data_len.min(available);
        let data_len = data_len - data_len % bytes_per_sample;

        meta.len_samples = (data_len / bytes_per_sample) as u64;

        let format = meta.sample_type.format;
        let divisor = resolve_divisor_with_budget(
            normalize,
            format,
            &map[data_offset..data_offset + data_len],
            scan_bytes,
        );
        meta.divisor = divisor;

        Ok(Self {
            map,
            data_offset,
            data_len,
            meta,
            scale: gain_factor(gain_db) / divisor,
            divisor,
            pos: 0,
            released: data_offset,
            access: AccessPattern::Sequential,
        })
    }

    /// Divisor applied to raw values, in the file's own units.
    pub fn divisor(&self) -> f32 {
        self.divisor
    }

    fn sample_type(&self) -> SampleType {
        self.meta.sample_type
    }

    /// Release pages the reader has moved well past.
    ///
    /// Only a hint: if the platform does not support it, or the call fails,
    /// the mapping is still correct and the data still readable.
    fn release_behind(&mut self, upto: usize) {
        let target = upto.saturating_sub(RELEASE_LAG);
        if target < self.released + RELEASE_STEP {
            return;
        }
        let len = target - self.released;
        #[cfg(unix)]
        // Safety: a read-only private file mapping has nothing to discard --
        // the pages are clean, and re-reading them faults them back in.
        unsafe {
            let _ = self.map.unchecked_advise_range(
                memmap2::UncheckedAdvice::DontNeed,
                self.released,
                len,
            );
        }
        let _ = len;
        self.released = target;
    }
}

impl SampleSource for MmapSource {
    fn prefetch(&mut self, range: SampleRange) {
        #[cfg(unix)]
        {
            let range = range.clamped_to(self.meta.len_samples);
            if range.len == 0 {
                return;
            }
            let stride = self.meta.sample_type.bytes_per_sample();
            let _ = self.map.advise_range(
                memmap2::Advice::WillNeed,
                self.data_offset + range.start as usize * stride,
                range.len as usize * stride,
            );
        }
        #[cfg(not(unix))]
        let _ = range;
    }

    fn access_pattern(&mut self, pattern: AccessPattern) {
        self.access = pattern;
        #[cfg(unix)]
        let _ = self.map.advise(match pattern {
            AccessPattern::Sequential => memmap2::Advice::Sequential,
            AccessPattern::Sparse => memmap2::Advice::Random,
        });
    }

    fn meta(&self) -> &SignalMeta {
        &self.meta
    }

    fn seek(&mut self, sample: u64) -> Result<(), SourceError> {
        if sample > self.meta.len_samples {
            return Err(SourceError::SeekOutOfRange {
                requested: sample,
                total: self.meta.len_samples,
            });
        }
        self.pos = sample;
        // Going backwards means those pages are wanted again.
        self.released = self
            .released
            .min(self.data_offset + sample as usize * self.meta.sample_type.bytes_per_sample());
        Ok(())
    }

    fn read(&mut self, buf: &mut [f32]) -> Result<usize, SourceError> {
        let sample_type = self.sample_type();
        let channels = sample_type.channels();
        let stride = sample_type.bytes_per_sample();

        // Never split an I/Q pair across two calls.
        let usable = buf.len() - buf.len() % channels;
        if usable == 0 {
            return Ok(0);
        }

        let start = self.data_offset + self.pos as usize * stride;
        let end = self.data_offset + self.data_len;
        if start >= end {
            return Ok(0);
        }

        let take = (end - start).min(usable / channels * stride);
        let written = convert(
            sample_type.format,
            &self.map[start..start + take],
            &mut buf[..usable],
            self.scale,
        );
        self.pos += (written / channels) as u64;
        if self.access == AccessPattern::Sequential {
            self.release_behind(start + take);
        }
        Ok(written)
    }
}

#[cfg(test)]
mod tests {
    include!("mmapped_tests.rs");
}
