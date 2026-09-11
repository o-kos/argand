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
mod tests {
    use super::*;

    #[test]
    fn physical_bounds_and_anchor_hold_for_real_iq_and_offset_captures() {
        for full in [(0., 12000.), (-12000., 12000.), (14000000., 14024000.)] {
            for anchor in [0., 0.25, 0.5, 1.] {
                let view = View::default().zoom(0.25, anchor, 1024);
                let hz = view.hertz(full);
                assert!(
                    (hz.0 + anchor * (hz.1 - hz.0) - (full.0 + anchor * (full.1 - full.0))).abs()
                        < 1e-8
                );
                assert_eq!(view.pan(-1e20).hertz(full).0, full.0);
                assert_eq!(view.pan(1e20).hertz(full).1, full.1);
                assert_eq!(view.zoom(1e20, anchor, 1024), View::default());
            }
        }
    }

    #[test]
    fn minimum_span_tracks_retained_resolution_and_rebounds_after_fft_changes() {
        let view = View::default().zoom(1e-20, 1., 2048);
        assert_eq!(view.span, 1. / 2048.);
        assert_eq!(view.zoom(1., 0.5, 128).span, 1. / 128.);
        assert_eq!(view.zoom(f64::NAN, 0.5, 1), view);
    }
    #[test]
    fn physical_rounding_never_expands_the_full_band_or_collapses_a_narrow_view() {
        let full = (-2040503.3724825433, -329272.3859428355);
        assert_eq!(View::default().hertz(full), full);
        let full = (1e12, 1e12 + 0.001);
        let view = View::default().zoom(1e-20, 0.5, cells(full, 2048));
        let band = view.hertz(full);
        assert!(band.0 < band.1 && band.0 >= full.0 && band.1 <= full.1);
    }
}
