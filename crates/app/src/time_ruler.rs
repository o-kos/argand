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
    /// A selection's start, end and length in this mode's units.
    ///
    /// Samples name the first and the last sample selected, the way the ruler
    /// numbers them. Clock and seconds give the boundaries in time with the
    /// decimals one pixel of `seconds_per_pixel` tells apart, never more than one
    /// sample needs, and fall back to samples where seconds cannot tell
    /// neighbouring samples apart. The clock keeps the fields the capture's
    /// `total` samples need, so its width stays while an edge is dragged.
    pub fn counters(
        self,
        span: argand_core::SampleSpan,
        rate: f64,
        total: u64,
        seconds_per_pixel: f64,
    ) -> [String; 3] {
        let whole = argand_core::SampleSpan::between(0, total.max(span.end())).unwrap_or(span);
        let precision = exact_time_precision(whole, rate)
            .filter(|&precision| match self {
                Self::Samples => false,
                Self::Clock => precision <= CLOCK_DECIMALS,
                Self::Seconds => true,
            })
            .map(|exact| crate::navigation::time_precision(seconds_per_pixel).min(exact));
        let Some(precision) = precision else {
            let index = |sample: u64| format!("#{}", crate::numbers::number(sample));
            return [
                index(span.start()),
                index(span.end() - 1),
                crate::numbers::number(span.count()),
            ];
        };
        let duration = whole.end() as f64 / rate;
        let time = |seconds: f64| match self {
            Self::Clock => crate::numbers::current().axis_label(
                &axis::format_time(seconds, duration, 10_f64.powi(-(precision as i32))),
                AxisKind::PreciseTime,
            ),
            Self::Seconds | Self::Samples => {
                crate::numbers::text(&format!("{seconds:.precision$} s"))
            }
        };
        [
            time(span.start() as f64 / rate),
            time(span.end() as f64 / rate),
            time(span.count() as f64 / rate),
        ]
    }
}

/// The most decimals the clock format prints, which `argand_core::axis::format_time` caps.
const CLOCK_DECIMALS: usize = 9;

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

    fn span(a: u64, b: u64) -> argand_core::SampleSpan {
        argand_core::SampleSpan::between(a, b).expect("a span")
    }

    fn texts(counters: [String; 3]) -> [String; 3] {
        counters
    }

    #[test]
    fn a_selection_reads_sample_exact_in_every_mode() {
        let span = span(4_000_000, 4_001_000);
        assert_eq!(
            texts(Mode::Samples.counters(span, 2e6, 4_100_000, 1e-9)),
            ["#4,000,000", "#4,000,999", "1,000"]
        );
        assert_eq!(
            texts(Mode::Seconds.counters(span, 2e6, 4_100_000, 1e-9)),
            ["2.0000000 s", "2.0005000 s", "0.0005000 s"]
        );
        assert_eq!(
            texts(Mode::Clock.counters(span, 2e6, 4_100_000, 1e-9)),
            ["0:02.0000000", "0:02.0005000", "0:00.0005000"]
        );
    }

    #[test]
    fn only_the_clock_keeps_the_capture_s_fields() {
        assert_eq!(
            Mode::Samples.counters(span(12, 345), 1e3, 1_000_000, 1e-9),
            ["#12", "#344", "333"]
        );
        assert_eq!(
            Mode::Seconds.counters(span(0, 10), 1e3, 100_000, 1e-9)[0],
            "0.000 s"
        );
        assert_eq!(
            Mode::Clock.counters(span(0, 3_600_000), 1e3, 7_200_000, 1e-9)[1],
            "1:00:00.000"
        );
    }

    #[test]
    fn decimals_follow_the_zoom_but_never_pass_one_sample() {
        let span = span(4_000_000, 4_001_000);
        assert_eq!(
            texts(Mode::Seconds.counters(span, 2e6, 4_100_000, 0.004)),
            ["2.000 s", "2.001 s", "0.001 s"],
            "milliseconds where a pixel holds four of them"
        );
        assert_eq!(
            texts(Mode::Seconds.counters(span, 2e6, 4_100_000, 2e-6)),
            ["2.000000 s", "2.000500 s", "0.000500 s"]
        );
    }

    #[test]
    fn a_selection_seconds_cannot_tell_apart_is_given_in_samples() {
        let fast = Mode::Seconds.counters(span(100, 101), 4e9, 200, 1e-12);
        assert_eq!(texts(fast)[0], "0.000000025 s");
        assert_eq!(
            texts(Mode::Clock.counters(span(100, 101), 4e9, 200, 1e-12)),
            ["#100", "#100", "1"],
            "the clock stops at nanoseconds"
        );
        assert_eq!(
            texts(Mode::Clock.counters(span(u64::MAX - 1, u64::MAX), 24e6, u64::MAX, 1e-9)),
            texts(Mode::Samples.counters(span(u64::MAX - 1, u64::MAX), 24e6, u64::MAX, 1e-9))
        );
        assert_eq!(
            texts(Mode::Seconds.counters(span(0, 10), 0., 10, 1e-9)),
            ["#0", "#9", "10"],
            "no rate, no seconds"
        );
    }
}
