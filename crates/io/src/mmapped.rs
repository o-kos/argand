//! Memory-mapped reader for fixed-width interleaved samples.
//!
//! Serves both WAVE and headerless files: once the header has told us where
//! the samples start and how wide they are, the two cases are identical. The
//! mapping means a multi-hour capture never lands in the process heap. The
//! samples may lie in several runs, one per `data` chunk, read as one array.

use std::fs::File;
use std::path::Path;

use argand_core::{AccessPattern, SampleRange, SampleSource, SampleType, SignalMeta, SourceError};
use memmap2::Mmap;

use crate::convert::convert;
use crate::normalize::{Normalize, gain_factor, resolve_divisor_over};
use crate::riff::{DataRun, WavLayout};

pub struct MmapSource {
    map: Mmap,
    runs: Vec<Run>,
    meta: SignalMeta,
    /// Combined normalization and gain multiplier.
    scale: f32,
    /// Divisor the level scan settled on, kept for the report.
    divisor: f32,
    pos: u64,
    /// Byte offset up to which pages have already been released.
    released: usize,
    access: AccessPattern,
    /// The mapped file as it was opened.
    stamp: Option<crate::write::SourceStamp>,
}

/// How far behind the read head pages are released, and how often.
///
/// A mapped page stays resident until the kernel needs the memory, so a
/// straight pass over a multi-gigabyte capture would report the whole file as
/// resident. Dropping what has already been consumed keeps the footprint flat
/// without giving up the mapping.
const RELEASE_LAG: usize = 2 << 20;
const RELEASE_STEP: usize = 8 << 20;

/// Whole samples of one run of the mapping.
#[derive(Debug, Clone, Copy)]
struct Run {
    offset: usize,
    len: usize,
    first: u64,
}

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
        meta: SignalMeta,
        data_offset: usize,
        data_len: usize,
        normalize: Normalize,
        gain_db: f32,
        scan_bytes: Option<usize>,
    ) -> Result<Self, SourceError> {
        let levels = Levels {
            normalize,
            gain_db,
            scan_bytes,
        };
        Self::map(path, meta, levels, |map| {
            let available = map.len().saturating_sub(data_offset);
            Ok(vec![DataRun {
                offset: data_offset as u64,
                len: data_len.min(available) as u64,
            }])
        })
    }

    /// Map a WAVE file and read every `data` chunk of `layout` as one array.
    pub fn from_wave(
        path: &Path,
        meta: SignalMeta,
        layout: &WavLayout,
        normalize: Normalize,
        gain_db: f32,
        scan_bytes: Option<usize>,
    ) -> Result<Self, SourceError> {
        let levels = Levels {
            normalize,
            gain_db,
            scan_bytes,
        };
        Self::map(path, meta, levels, |map| {
            Ok(layout.runs(std::io::Cursor::new(&map[..]), map.len() as u64)?)
        })
    }

    fn map(
        path: &Path,
        mut meta: SignalMeta,
        levels: Levels,
        runs: impl FnOnce(&Mmap) -> Result<Vec<DataRun>, SourceError>,
    ) -> Result<Self, SourceError> {
        let file = File::open(path)?;
        let stamp = crate::write::SourceStamp::of_file(&file).ok();
        // Safety: the file is opened read-only and the mapping is never
        // handed out as a mutable slice. A concurrent truncation is the one
        // hazard, and it is the same one every mmap-based reader accepts.
        let map = unsafe { Mmap::map(&file)? };
        #[cfg(unix)]
        let _ = map.advise(memmap2::Advice::Sequential);

        let bytes_per_sample = meta.sample_type.bytes_per_sample();
        let mut first = 0u64;
        let runs: Vec<Run> = runs(&map)?
            .into_iter()
            .filter_map(|run| {
                let offset = usize::try_from(run.offset).ok()?.min(map.len());
                let len = usize::try_from(run.len).ok()?.min(map.len() - offset);
                let len = len - len % bytes_per_sample;
                let run = Run { offset, len, first };
                first += (len / bytes_per_sample) as u64;
                (len > 0).then_some(run)
            })
            .collect();
        meta.len_samples = first;

        let format = meta.sample_type.format;
        let slices: Vec<&[u8]> = runs
            .iter()
            .map(|run| &map[run.offset..run.offset + run.len])
            .collect();
        let divisor = resolve_divisor_over(levels.normalize, format, &slices, levels.scan_bytes);
        meta.divisor = divisor;
        let released = runs.first().map_or(0, |run| run.offset);

        Ok(Self {
            map,
            runs,
            meta,
            scale: gain_factor(levels.gain_db) / divisor,
            divisor,
            pos: 0,
            released,
            access: AccessPattern::Sequential,
            stamp,
        })
    }

    /// The mapped file as it was when it was opened, read from the mapping's own handle.
    pub fn stamp(&self) -> Option<crate::write::SourceStamp> {
        self.stamp
    }

    /// Divisor applied to raw values, in the file's own units.
    pub fn divisor(&self) -> f32 {
        self.divisor
    }

    fn sample_type(&self) -> SampleType {
        self.meta.sample_type
    }

    /// The run holding `sample`, or none past the last sample.
    fn run_of(&self, sample: u64) -> Option<&Run> {
        let stride = self.meta.sample_type.bytes_per_sample() as u64;
        let index = self
            .runs
            .partition_point(|run| run.first + run.len as u64 / stride <= sample);
        self.runs.get(index)
    }

    /// Byte offset of `sample` in the mapping, the end of the last run past the last sample.
    fn byte_of(&self, sample: u64) -> usize {
        let stride = self.meta.sample_type.bytes_per_sample();
        match self.run_of(sample) {
            Some(run) => run.offset + (sample - run.first) as usize * stride,
            None => self.runs.last().map_or(0, |run| run.offset + run.len),
        }
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
    fn original_sample_units(&self) -> Option<(f64, f64)> {
        let factor = 1.0 / f64::from(self.scale);
        let offset = if self.meta.sample_type.format == argand_core::SampleFormat::U8 {
            128.0
        } else {
            0.0
        };
        (factor.is_finite() && factor > 0.0).then_some((factor, offset))
    }

    fn prefetch(&mut self, range: SampleRange) {
        #[cfg(unix)]
        {
            let range = range.clamped_to(self.meta.len_samples);
            if range.len == 0 {
                return;
            }
            let start = self.byte_of(range.start);
            let end = self.byte_of(range.start + range.len - 1)
                + self.meta.sample_type.bytes_per_sample();
            let _ = self
                .map
                .advise_range(memmap2::Advice::WillNeed, start, end - start);
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
        self.released = self.released.min(self.byte_of(sample));
        Ok(())
    }

    fn read(&mut self, buf: &mut [f32]) -> Result<usize, SourceError> {
        let sample_type = self.sample_type();
        let channels = sample_type.channels();
        let stride = sample_type.bytes_per_sample();

        // Never split an I/Q pair across two calls.
        let usable = buf.len() - buf.len() % channels;
        let mut written = 0;
        while written < usable {
            let Some(&run) = self.run_of(self.pos) else {
                break;
            };
            let start = run.offset + (self.pos - run.first) as usize * stride;
            let end = run.offset + run.len;
            let take = (end - start).min((usable - written) / channels * stride);
            let count = convert(
                sample_type.format,
                &self.map[start..start + take],
                &mut buf[written..usable],
                self.scale,
            );
            written += count;
            self.pos += (count / channels) as u64;
            if self.access == AccessPattern::Sequential {
                self.release_behind(start + take);
            }
        }
        Ok(written)
    }
}

/// How a source's values are brought onto the unit scale.
struct Levels {
    normalize: Normalize,
    gain_db: f32,
    scan_bytes: Option<usize>,
}

#[cfg(test)]
mod tests {
    include!("mmapped_tests.rs");
}
