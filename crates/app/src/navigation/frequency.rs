//! A bounded frequency viewport, independent of time navigation and GUI state.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct View {
    pub start: f64,
    pub span: f64,
}

impl Default for View {
    fn default() -> Self {
        Self {
            start: 0.,
            span: 1.,
        }
    }
}

impl View {
    pub fn hertz(self, full: (f64, f64)) -> (f64, f64) {
        let width = full.1 - full.0;
        let end = self.start + self.span;
        (
            (full.0 + self.start * width).clamp(full.0, full.1),
            if end >= 1. {
                full.1
            } else {
                (full.0 + end * width).clamp(full.0, full.1)
            },
        )
    }

    pub fn zoom(self, factor: f64, anchor: f64, cells: usize) -> Self {
        if !factor.is_finite() || factor <= 0. || !anchor.is_finite() {
            return self;
        }
        let span = (self.span * factor).clamp(1. / cells.max(1) as f64, 1.);
        let start = (self.start + anchor.clamp(0., 1.) * (self.span - span)).clamp(0., 1. - span);
        Self { start, span }
    }

    pub fn pan(self, fraction: f64) -> Self {
        if !fraction.is_finite() {
            return self;
        }
        Self {
            start: (self.start + fraction * self.span).clamp(0., 1. - self.span),
            ..self
        }
    }
}

/// Reserve enough physical precision to keep both endpoints distinguishable.
pub fn cells(full: (f64, f64), bins: usize) -> usize {
    let magnitude = full.0.abs().max(full.1.abs());
    let representable = ((full.1 - full.0) / (magnitude * f64::EPSILON * 2.)) as usize;
    bins.min(2048).min(representable).max(1)
}

#[cfg(test)]
#[path = "frequency_test.rs"]
mod tests;
