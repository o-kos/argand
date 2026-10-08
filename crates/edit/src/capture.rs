//! The piece table itself.

use std::sync::Arc;

use argand_core::SampleSpan;

/// Which source a piece reads from, as an index into a table the caller keeps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SourceId(pub u32);

/// A run of `len` samples of `source`, starting at sample `start` of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Piece {
    pub source: SourceId,
    pub start: u64,
    pub len: u64,
}

impl Piece {
    const fn end(self) -> u64 {
        self.start + self.len
    }

    /// The part of this piece from `from` to `to`, counted within the piece.
    const fn slice(self, from: u64, to: u64) -> Self {
        Self {
            source: self.source,
            start: self.start + from,
            len: to - from,
        }
    }
}

/// Samples taken out of a capture, as references to their sources.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Clip {
    pieces: Vec<Piece>,
}

impl Clip {
    pub fn new(pieces: Vec<Piece>) -> Self {
        Self {
            pieces: merged(pieces),
        }
    }

    pub fn pieces(&self) -> &[Piece] {
        &self.pieces
    }

    pub fn len(&self) -> u64 {
        self.pieces.iter().map(|piece| piece.len).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.pieces.is_empty()
    }

    /// The same samples with every source renamed, for pasting into a capture whose table differs.
    #[must_use]
    pub fn remap(&self, mut rename: impl FnMut(SourceId) -> SourceId) -> Self {
        Self::new(
            self.pieces
                .iter()
                .map(|piece| Piece {
                    source: rename(piece.source),
                    ..*piece
                })
                .collect(),
        )
    }
}

/// A capture as an immutable list of pieces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Capture {
    pieces: Arc<[Piece]>,
    /// `ends[i]` is the capture sample just past piece `i`.
    ends: Arc<[u64]>,
}

impl Capture {
    /// A capture that is the whole of one source.
    pub fn whole(source: SourceId, len: u64) -> Self {
        Self::from_pieces(vec![Piece {
            source,
            start: 0,
            len,
        }])
    }

    fn from_pieces(pieces: Vec<Piece>) -> Self {
        let pieces = merged(pieces);
        let mut end = 0;
        let ends = pieces
            .iter()
            .map(|piece| {
                end += piece.len;
                end
            })
            .collect();
        Self {
            pieces: pieces.into(),
            ends,
        }
    }

    pub fn len(&self) -> u64 {
        self.ends.last().copied().unwrap_or(0)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn pieces(&self) -> &[Piece] {
        &self.pieces
    }

    /// Every source the capture reads from, each once.
    pub fn sources(&self) -> Vec<SourceId> {
        let mut sources: Vec<SourceId> = self.pieces.iter().map(|piece| piece.source).collect();
        sources.sort_unstable();
        sources.dedup();
        sources
    }

    /// The piece holding capture sample `at` and the offset of `at` within it.
    ///
    /// `at` equal to the length answers one past the last piece, where an insertion appends.
    pub fn locate(&self, at: u64) -> (usize, u64) {
        let index = self.ends.partition_point(|&end| end <= at);
        let start = if index == 0 { 0 } else { self.ends[index - 1] };
        (index, at - start)
    }

    /// The capture sample where piece `index` begins.
    pub fn piece_start(&self, index: usize) -> u64 {
        if index == 0 { 0 } else { self.ends[index - 1] }
    }

    /// The source runs that `span` of the capture reads, in order.
    pub fn segments(&self, span: SampleSpan) -> Vec<Piece> {
        let Some(span) = span.within(self.len()) else {
            return Vec::new();
        };
        let (first, offset) = self.locate(span.start());
        let mut remaining = span.count();
        let mut out = Vec::new();
        let mut from = offset;
        for piece in &self.pieces[first..] {
            if remaining == 0 {
                break;
            }
            let take = (piece.len - from).min(remaining);
            out.push(piece.slice(from, from + take));
            remaining -= take;
            from = 0;
        }
        out
    }

    /// The samples of `span`, to paste elsewhere.
    pub fn copy(&self, span: SampleSpan) -> Clip {
        Clip::new(self.segments(span))
    }

    /// The capture without `span`.
    #[must_use]
    pub fn delete(&self, span: SampleSpan) -> Self {
        self.replace(span, &Clip::default())
    }

    /// The capture with `clip` inserted before sample `at`, clamped to the end.
    #[must_use]
    pub fn insert(&self, at: u64, clip: &Clip) -> Self {
        let at = at.min(self.len());
        let mut pieces = self.before(at);
        pieces.extend_from_slice(clip.pieces());
        pieces.extend(self.after(at));
        Self::from_pieces(pieces)
    }

    /// The capture with `span` replaced by `clip`.
    #[must_use]
    pub fn replace(&self, span: SampleSpan, clip: &Clip) -> Self {
        let Some(span) = span.within(self.len()) else {
            return self.insert(span.start(), clip);
        };
        let mut pieces = self.before(span.start());
        pieces.extend_from_slice(clip.pieces());
        pieces.extend(self.after(span.end()));
        Self::from_pieces(pieces)
    }

    /// The pieces covering capture samples before `at`.
    fn before(&self, at: u64) -> Vec<Piece> {
        let (index, offset) = self.locate(at);
        let mut out = self.pieces[..index.min(self.pieces.len())].to_vec();
        if offset > 0 && index < self.pieces.len() {
            out.push(self.pieces[index].slice(0, offset));
        }
        out
    }

    /// The pieces covering capture samples from `at` on.
    fn after(&self, at: u64) -> Vec<Piece> {
        let (index, offset) = self.locate(at);
        let Some(first) = self.pieces.get(index) else {
            return Vec::new();
        };
        let mut out = vec![first.slice(offset, first.len)];
        out.extend_from_slice(&self.pieces[index + 1..]);
        out
    }
}

/// Drop empty pieces and join neighbours that continue the same source.
fn merged(pieces: Vec<Piece>) -> Vec<Piece> {
    let mut out: Vec<Piece> = Vec::with_capacity(pieces.len());
    for piece in pieces.into_iter().filter(|piece| piece.len > 0) {
        match out.last_mut() {
            Some(last) if last.source == piece.source && last.end() == piece.start => {
                last.len += piece.len;
            }
            _ => out.push(piece),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    include!("capture_tests.rs");
}
