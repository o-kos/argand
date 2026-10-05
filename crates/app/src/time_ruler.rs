//! Time-ruler presentation; navigation itself remains in integer samples.

use crate::navigation::View;
use argand_core::axis::{self, AxisKind};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Mode {
    #[default]
    Clock,
    Seconds,
    Samples,
}

impl Mode {
    pub fn caption(self) -> &'static str {
        match self {
            Self::Clock => "hms",
            Self::Seconds => "s",
            Self::Samples => "#",
        }
    }

    pub fn kind(self) -> AxisKind {
        match self {
            Self::Clock => AxisKind::PreciseTime,
            Self::Seconds => AxisKind::Seconds,
            Self::Samples => AxisKind::Samples,
        }
    }

    pub fn sample_step(self, step: f64, sample_rate: f64) -> f64 {
        match self {
            Self::Samples => step,
            Self::Clock | Self::Seconds => step * sample_rate,
        }
    }
}

impl Mode {
    /// A selection as its start, end and length, exact to the sample in this mode's units.
    ///
    /// Samples name the first and the last sample selected, the way the ruler
    /// numbers them. Clock and seconds give the boundaries in time, with as many
    /// decimals as one sample needs, and fall back to samples where seconds
    /// cannot tell neighbouring samples apart.
    pub fn selection(self, span: argand_core::SampleSpan, rate: f64) -> String {
        let Some(precision) = exact_time_precision(span, rate).filter(|_| self != Self::Samples)
        else {
            return format!(
                "#{} – #{} ({})",
                crate::numbers::number(span.start()),
                crate::numbers::number(span.end() - 1),
                crate::numbers::number(span.count())
            );
        };
        let end = span.end() as f64 / rate;
        let time = |seconds: f64| match self {
            Self::Clock => crate::numbers::current().axis_label(
                &axis::format_time(seconds, end, 10_f64.powi(-(precision as i32))),
                AxisKind::PreciseTime,
            ),
            Self::Seconds | Self::Samples => {
                crate::numbers::text(&format!("{seconds:.precision$} s"))
            }
        };
        format!(
            "{} – {} ({})",
            time(span.start() as f64 / rate),
            time(end),
            crate::numbers::text(&format!("{:.precision$} s", span.count() as f64 / rate))
        )
    }
}

/// The decimals that tell every sample of `span` apart in seconds, or nothing where `f64` cannot.
///
/// A boundary is exact in `f64` only up to 2^53, and the seconds it prints need
/// their integer digits plus one decimal per tenfold of the sample rate, which
/// together must stay within the 15 significant digits a double carries.
fn exact_time_precision(span: argand_core::SampleSpan, rate: f64) -> Option<usize> {
    if !(rate.is_finite() && rate > 0.) || span.end() > 1 << 53 {
        return None;
    }
    let precision = (rate.log10().ceil().max(0.) as usize).max(3);
    let whole = (span.end() as f64 / rate).max(1.).log10().floor() as usize + 1;
    (whole + precision <= 15).then_some(precision)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ruler {
    pub mode: Mode,
    pub view: View,
    pub total: u64,
}

impl Ruler {
    #[cfg(test)]
    pub const CLOCK: Self = Self {
        mode: Mode::Clock,
        view: View { start: 0, len: 1 },
        total: 1,
    };

    pub fn bounds(self, seconds: (f64, f64)) -> (f64, f64) {
        match self.mode {
            Mode::Samples => (
                self.view.start as f64,
                self.view.start.saturating_add(self.view.len) as f64,
            ),
            Mode::Clock | Mode::Seconds => seconds,
        }
    }

    pub fn readout(self, time: f64, span: f64, pixels: f64, fraction: f64) -> String {
        let precision = crate::navigation::time_precision(span / pixels);
        match self.mode {
            Mode::Clock => crate::numbers::current().axis_label(
                &axis::format_time(time, span, 10_f64.powi(-(precision as i32))),
                AxisKind::PreciseTime,
            ),
            Mode::Seconds => crate::numbers::text(&format!("{time:.precision$} s")),
            Mode::Samples => {
                let offset = (fraction.clamp(0., 1.) * self.view.len as f64).floor() as u64;
                let index = self
                    .view
                    .start
                    .saturating_add(offset)
                    .min(self.total.saturating_sub(1));
                format!("#{}", crate::numbers::number(index))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modes_preserve_capture_origin_and_use_sample_units_once() {
        let view = View {
            start: 4_000_000,
            len: 2_000,
        };
        let mut ruler = Ruler {
            mode: Mode::Samples,
            view,
            total: 8_000_000,
        };
        let seconds = view.seconds(2e6);
        assert_eq!(ruler.bounds(seconds), (4_000_000., 4_002_000.));
        assert_eq!(ruler.readout(2.0005, 0.001, 1000., 0.5), "#4,001,000");
        assert_eq!(Mode::Samples.sample_step(100., 2e6), 100.);
        ruler.mode = Mode::Seconds;
        assert_eq!(ruler.bounds(seconds), seconds);
        assert_eq!(ruler.readout(2.0005, 0.001, 1000., 0.5), "2.000500 s");
        assert_eq!(Mode::Seconds.sample_step(0.001, 2e6), 2000.);
        ruler.mode = Mode::Clock;
        assert_eq!(ruler.readout(2.0005, 0.001, 1000., 0.5), "0:02.000500");
        assert_eq!(ruler.view, view);
    }

    #[test]
    fn sample_badges_clamp_to_actual_capture_indices_without_large_origin_rounding() {
        let ruler = Ruler {
            mode: Mode::Samples,
            view: View {
                start: u64::MAX - 4096,
                len: 4096,
            },
            total: u64::MAX,
        };
        assert_eq!(
            ruler.readout(0., 1., 1000., 0.),
            format!("#{}", crate::numbers::number(u64::MAX - 4096))
        );
        assert_eq!(
            ruler.readout(0., 1., 1000., 0.5),
            format!("#{}", crate::numbers::number(u64::MAX - 2048))
        );
        assert_eq!(
            ruler.readout(0., 1., 1000., 1.),
            format!("#{}", crate::numbers::number(u64::MAX - 1))
        );
    }

    #[test]
    fn a_selection_reads_sample_exact_in_every_mode() {
        let span = argand_core::SampleSpan::between(4_000_000, 4_001_000).expect("a span");
        assert_eq!(
            Mode::Samples.selection(span, 2e6),
            "#4,000,000 – #4,000,999 (1,000)"
        );
        assert_eq!(
            Mode::Seconds.selection(span, 2e6),
            "2.0000000 s – 2.0005000 s (0.0005000 s)"
        );
        assert_eq!(
            Mode::Clock.selection(span, 2e6),
            "0:02.0000000 – 0:02.0005000 (0.0005000 s)"
        );
    }

    #[test]
    fn a_selection_seconds_cannot_tell_apart_is_given_in_samples() {
        let span = |a, b| argand_core::SampleSpan::between(a, b).expect("a span");
        let fast = Mode::Seconds.selection(span(100, 101), 4e9);
        assert_eq!(fast, "0.0000000250 s – 0.0000000253 s (0.0000000003 s)");
        assert_eq!(
            Mode::Clock.selection(span(u64::MAX - 1, u64::MAX), 24e6),
            Mode::Samples.selection(span(u64::MAX - 1, u64::MAX), 24e6)
        );
        assert_eq!(
            Mode::Seconds.selection(span(0, 10), 0.),
            "#0 – #9 (10)",
            "no rate, no seconds"
        );
    }
}
