//! What a person has selected in a capture, in the capture's own units.
//!
//! A selection is kept in samples and hertz, never in screen positions, so
//! zooming, panning and turning the plot leave it where it was. Time is
//! counted in whole samples, and a complex sample counts once, which is what
//! keeps I and Q together whatever is later done with the span.

/// A run of whole samples, `start` included and `end` excluded, never empty.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SampleSpan {
    start: u64,
    end: u64,
}

impl SampleSpan {
    /// The samples between two boundaries given in either order.
    ///
    /// Boundaries are the gaps between samples, so equal ones hold nothing and
    /// answer `None`.
    pub fn between(a: u64, b: u64) -> Option<Self> {
        let (start, end) = if a <= b { (a, b) } else { (b, a) };
        (start < end).then_some(Self { start, end })
    }

    pub const fn start(self) -> u64 {
        self.start
    }

    pub const fn end(self) -> u64 {
        self.end
    }

    /// How many samples the span holds, never zero.
    pub const fn count(self) -> u64 {
        self.end - self.start
    }

    /// The part of this span inside a capture of `total` samples, if any.
    pub fn within(self, total: u64) -> Option<Self> {
        Self::between(self.start.min(total), self.end.min(total))
    }
}

/// A band of frequencies in hertz, `low` below `high`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrequencyBand {
    low: f64,
    high: f64,
}

impl FrequencyBand {
    /// The band between two finite frequencies given in either order.
    pub fn between(a: f64, b: f64) -> Option<Self> {
        if !a.is_finite() || !b.is_finite() {
            return None;
        }
        let (low, high) = if a <= b { (a, b) } else { (b, a) };
        (low < high).then_some(Self { low, high })
    }

    pub const fn low(self) -> f64 {
        self.low
    }

    pub const fn high(self) -> f64 {
        self.high
    }
}

/// A time span, a frequency band, or both, which makes a rectangle.
///
/// An absent axis means all of it: a time span alone covers every frequency
/// and a band alone covers the whole capture.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Selection {
    pub time: Option<SampleSpan>,
    pub band: Option<FrequencyBand>,
}

impl Selection {
    pub const fn time(span: SampleSpan) -> Self {
        Self {
            time: Some(span),
            band: None,
        }
    }

    pub const fn is_empty(self) -> bool {
        self.time.is_none() && self.band.is_none()
    }
}

#[cfg(test)]
mod tests {
    include!("selection_tests.rs");
}
