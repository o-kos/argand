//! Time-ruler presentation; navigation itself remains in integer samples.

use crate::navigation::View;
use argand_core::axis::{self, AxisKind};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Origin {
    #[default]
    Relative,
    Absolute,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FrequencyMode {
    Baseband,
    #[default]
    Physical,
}

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
    /// decimals one pixel of `seconds_per_pixel` tells apart, capped at milliseconds,
    /// and fall back to samples where floating-point seconds cannot tell
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
        if self.writes_samples(rate, whole.end()) {
            let index = |sample: u64| format!("#{}", crate::numbers::number(sample));
            return [
                index(span.start()),
                index(span.end() - 1),
                crate::numbers::number(span.count()),
            ];
        }
        let precision = crate::navigation::time_precision(seconds_per_pixel).min(3);
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

impl Mode {
    /// Whether a selection's values in a capture of `total` samples are written as sample indexes.
    pub fn writes_samples(self, rate: f64, total: u64) -> bool {
        argand_core::SampleSpan::between(0, total.max(1))
            .and_then(|whole| self.exact_precision(whole, rate))
            .is_none()
    }

    /// The decimals that tell every sample of `whole` apart in this mode, none where only indexes do.
    fn exact_precision(self, whole: argand_core::SampleSpan, rate: f64) -> Option<usize> {
        (self != Self::Samples)
            .then(|| exact_time_precision(whole, rate))
            .flatten()
    }
}

/// The decimals that tell every sample of `span` apart in seconds, or nothing where `f64` cannot.
///
/// A boundary is exact in `f64` only up to 2^53, and the seconds it prints need
/// their integer digits plus one decimal per tenfold of the sample rate, which
/// together must stay within the 15 significant digits a double carries.
pub(super) fn exact_time_precision(span: argand_core::SampleSpan, rate: f64) -> Option<usize> {
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
    pub start: Option<argand_core::RecordingTime>,
    pub view: View,
    pub total: u64,
}

impl Ruler {
    pub fn kind(self) -> AxisKind {
        self.start
            .map_or_else(|| self.mode.kind(), AxisKind::UtcTime)
    }

    pub fn bounds(self, seconds: (f64, f64)) -> (f64, f64) {
        if self.start.is_some() {
            return seconds;
        }
        match self.mode {
            Mode::Samples => (
                self.view.start as f64,
                self.view.start.saturating_add(self.view.len) as f64,
            ),
            Mode::Clock | Mode::Seconds => seconds,
        }
    }

    pub fn readout(self, time: f64, span: f64, pixels: f64, fraction: f64) -> String {
        let precision = crate::navigation::time_precision(span / pixels).min(3);
        if let Some(start) = self.start {
            let rate = self.view.len as f64 / span;
            let exact = argand_core::SampleSpan::between(0, self.total.max(1))
                .and_then(|whole| exact_time_precision(whole, rate));
            if exact.is_none() {
                return Self {
                    start: None,
                    mode: Mode::Samples,
                    ..self
                }
                .readout(time, span, pixels, fraction);
            }
            return start.shifted_seconds(time).map_or_else(
                || "Out of range".into(),
                |time| crate::numbers::current().utc(time, precision),
            );
        }
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
#[path = "time_ruler_test.rs"]
mod tests;

#[cfg(test)]
include!("time_ruler_support_test.rs");
