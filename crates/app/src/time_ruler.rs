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
}
