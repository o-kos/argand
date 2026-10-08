//! Reading an edited capture as if it were one signal.

use argand_core::{AccessPattern, SampleRange, SampleSource, SampleSpan, SignalMeta, SourceError};

use crate::{Capture, SourceId};

/// A [`SampleSource`] over a capture, reading each piece from its own source.
///
/// Every consumer builds its own, with its own opened sources, so no reader
/// shares a cursor with another.
pub struct EditedSource {
    capture: Capture,
    /// Opened sources, indexed by [`SourceId`].
    sources: Vec<Option<Box<dyn SampleSource>>>,
    meta: SignalMeta,
    pos: u64,
    /// The piece and the offset in it where that piece's source cursor stands, when known.
    positioned: Option<(usize, u64)>,
}

impl EditedSource {
    /// Read `capture` through `sources`, describing it with `meta` but for its length.
    ///
    /// Every source the capture reads must be present and hold samples of the same width.
    pub fn new(
        capture: Capture,
        sources: Vec<Option<Box<dyn SampleSource>>>,
        meta: SignalMeta,
    ) -> Result<Self, SourceError> {
        let mut source = Self {
            capture: Capture::whole(SourceId(0), 0),
            sources,
            meta,
            pos: 0,
            positioned: None,
        };
        source.set_capture(capture)?;
        Ok(source)
    }

    pub fn capture(&self) -> &Capture {
        &self.capture
    }

    /// Switch to another version of the capture, keeping the opened sources.
    pub fn set_capture(&mut self, capture: Capture) -> Result<(), SourceError> {
        let channels = self.meta.channels();
        for id in capture.sources() {
            let present = self
                .sources
                .get(id.0 as usize)
                .and_then(Option::as_ref)
                .ok_or_else(|| SourceError::Decode(format!("source {} is not open", id.0)))?;
            if present.meta().channels() != channels {
                return Err(SourceError::Decode(format!(
                    "source {} does not hold {channels} channel samples",
                    id.0
                )));
            }
        }
        self.meta.len_samples = capture.len();
        self.capture = capture;
        self.pos = self.pos.min(self.meta.len_samples);
        self.positioned = None;
        Ok(())
    }

    /// Add or replace the opened source for `id`.
    pub fn insert_source(&mut self, id: SourceId, source: Box<dyn SampleSource>) {
        let index = id.0 as usize;
        if self.sources.len() <= index {
            self.sources.resize_with(index + 1, || None);
        }
        self.sources[index] = Some(source);
        self.positioned = None;
    }

    pub fn has_source(&self, id: SourceId) -> bool {
        self.sources.get(id.0 as usize).is_some_and(Option::is_some)
    }

    fn source(&mut self, id: SourceId) -> Result<&mut Box<dyn SampleSource>, SourceError> {
        self.sources
            .get_mut(id.0 as usize)
            .and_then(Option::as_mut)
            .ok_or_else(|| SourceError::Decode(format!("source {} is not open", id.0)))
    }
}

impl SampleSource for EditedSource {
    fn access_pattern(&mut self, pattern: AccessPattern) {
        for source in self.sources.iter_mut().flatten() {
            source.access_pattern(pattern);
        }
    }

    fn prefetch(&mut self, range: SampleRange) {
        let Some(span) = SampleSpan::between(range.start, range.end()) else {
            return;
        };
        for piece in self.capture.segments(span) {
            if let Ok(source) = self.source(piece.source) {
                source.prefetch(SampleRange::new(piece.start, piece.len));
            }
        }
    }

    fn original_sample_units(&self) -> Option<(f64, f64)> {
        self.sources
            .iter()
            .flatten()
            .next()?
            .original_sample_units()
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
        Ok(())
    }

    fn read(&mut self, buf: &mut [f32]) -> Result<usize, SourceError> {
        let channels = self.meta.channels();
        let usable = buf.len() / channels;
        let mut written = 0;
        while written < usable && self.pos < self.meta.len_samples {
            let (index, offset) = self.capture.locate(self.pos);
            let piece = self.capture.pieces()[index];
            let positioned = self.positioned == Some((index, offset));
            let source = self.source(piece.source)?;
            if !positioned {
                source.seek(piece.start + offset)?;
            }
            let want = (piece.len - offset).min((usable - written) as u64) as usize;
            let read = source.read(&mut buf[written * channels..(written + want) * channels])?;
            let got = read / channels;
            if got == 0 {
                return Err(SourceError::Decode(format!(
                    "source {} ended inside the capture",
                    piece.source.0
                )));
            }
            written += got;
            self.pos += got as u64;
            self.positioned = Some((index, offset + got as u64));
        }
        Ok(written * channels)
    }
}

#[cfg(test)]
mod tests {
    include!("source_tests.rs");
}
