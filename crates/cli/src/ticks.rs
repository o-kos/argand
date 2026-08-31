//! Where an axis puts its marks, and what they read as.
//!
//! A tick count picked in advance cannot know how long the axis is or how wide
//! its labels are, so it is wrong at both ends: eight marks across two thousand
//! pixels leave the plot empty, and the same eight across four hundred put
//! `12.579887 MHz` on top of `12.581887 MHz`.
//!
//! This module decides the other way round. It walks a ladder of round steps
//! from dense to sparse, formats what each one would print, measures it with
//! the font the plot draws with, and stops at the first step whose labels still
//! clear each other and the edge of the canvas. Only the accepted values come
//! back, so a caller cannot draw a grid line that no label agreed to.

use argand_core::format_hz;

use crate::text::TextRenderer;

/// A chosen tick: the value, the label measured for it, and where it lands.
#[derive(Debug, Clone, PartialEq)]
pub struct Tick {
    pub value: f64,
    pub label: String,
    /// Pixels from the low-value end of the axis, in `0..length`.
    pub offset: i64,
}

/// One axis to lay out.
///
/// `min` sits at offset 0 and `max` at offset `length - 1`, in that order, so a
/// frequency axis drawn upwards passes the room *below* the plot as `lead`.
/// Both give the pixels a label's ink may borrow past the end of the axis
/// before the canvas, or a panel sharing the same row, would clip it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Axis {
    pub length: i64,
    pub min: f64,
    pub max: f64,
    pub lead: i64,
    pub trail: i64,
}

/// What an axis prints, which fixes both its ladder and its labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AxisKind {
    /// Seconds, on a clock ladder no finer than one second.
    Time,
    /// Hertz.
    Frequency,
    /// Whole decibels, for the spectrum panel.
    Decibels,
    /// Whole decibels with the unit, for the colour bar.
    DecibelsWithUnit,
}

/// How labels sit on the axis, which decides what has to clear what.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabelRun {
    /// Side by side along the axis: the measured width of each label decides
    /// how close two ticks may be.
    Across,
    /// Stacked across it: a digit's ink height decides, whatever they say.
    Down,
}

/// The font the labels will be drawn with, and the spacing measured from it.
pub struct LabelMetrics<'a> {
    text: &'a TextRenderer,
    size: f32,
    run: LabelRun,
}

impl<'a> LabelMetrics<'a> {
    pub fn new(text: &'a TextRenderer, size: f32, run: LabelRun) -> Self {
        Self { text, size, run }
    }

    /// Ink the label occupies along the axis, centred on its tick.
    fn extent(&self, label: &str) -> f64 {
        match self.run {
            LabelRun::Across => f64::from(self.text.width(label, self.size)),
            LabelRun::Down => f64::from(self.text.digit_height(self.size)),
        }
    }

    /// Clear space two neighbouring labels have to keep.
    ///
    /// Two digits at the label size. Below that, `12` and `14` a few pixels
    /// apart read as one four-digit number; deriving it from the font rather
    /// than writing a pixel count down means it still holds if the label size
    /// changes.
    fn gap(&self) -> f64 {
        f64::from(self.text.width("00", self.size))
    }
}

/// Steps to try, densest first, until one fits.
///
/// The ladders are long enough to reach from the densest step an axis could
/// hold to a step wider than any span, so the search always terminates on a
/// candidate rather than on running out of them.
const CANDIDATES: usize = 40;

/// Whole-second steps a clock actually uses.
///
/// Decimal steps put marks at 250-second intervals, which read as `4.10` and
/// tell nobody anything.
const CLOCK_STEPS: [f64; 18] = [
    1.0, 2.0, 5.0, 10.0, 15.0, 30.0, // seconds
    60.0, 120.0, 300.0, 600.0, 900.0, 1800.0, // minutes
    3600.0, 7200.0, 10800.0, 21600.0, 43200.0, // hours
    86400.0, // a day
];

/// The ticks this axis accepts: the densest round step whose labels still read.
pub fn ticks(kind: AxisKind, axis: Axis, labels: &LabelMetrics<'_>) -> Vec<Tick> {
    let span = axis.max - axis.min;
    if axis.length < 2 || !axis.min.is_finite() || !span.is_finite() || span <= 0.0 {
        return Vec::new();
    }

    // A first bound on how dense the axis could possibly be: even a label of no
    // width needs the gap beside it, so no step below this one can ever fit.
    let length = axis.length as f64;
    let most = (length / labels.gap()).clamp(1.0, length);
    for step in ladder(kind, span / most) {
        let placed = place(kind, axis, step, labels);
        if !placed.is_empty() && readable(&placed, labels.gap()) {
            return placed.into_iter().map(|p| p.tick).collect();
        }
    }
    Vec::new()
}

/// The widest label an axis of this kind could print over `min..max`.
///
/// A gutter has to be reserved before any tick is chosen, so this bounds the
/// label rather than predicting it. Digits are the same width in this font, so
/// a bound built from zeros measures exactly like the value it stands in for.
pub fn widest_label(kind: AxisKind, min: f64, max: f64) -> String {
    let sign = if min < 0.0 { "-" } else { "" };
    let peak = min.abs().max(max.abs());
    match kind {
        AxisKind::Time => match clock_of(min, max) {
            Clock::HoursMinutesSeconds => format!("{sign}{}:00:00", (peak / 3600.0) as u64),
            Clock::MinutesSeconds => format!("{sign}{}.00", (peak / 60.0) as u64),
        },
        AxisKind::Frequency => widest_hz(sign, peak),
        AxisKind::Decibels | AxisKind::DecibelsWithUnit => {
            let ends = [
                format_label(kind, min, min, max),
                format_label(kind, max, min, max),
            ];
            ends.into_iter().max_by_key(String::len).unwrap_or_default()
        }
    }
}

/// A tick with the ink its label puts on the axis.
struct Placed {
    tick: Tick,
    extent: f64,
}

/// Every round multiple of `step` inside the axis, minus the outermost labels
/// the canvas cannot hold whole.
///
/// Values are `k * step` rather than a running sum: a tick at zero then comes
/// out exactly zero, and the last one has not drifted by an accumulated epsilon.
fn place(kind: AxisKind, axis: Axis, step: f64, labels: &LabelMetrics<'_>) -> Vec<Placed> {
    let span = axis.max - axis.min;
    let scale = (axis.length - 1) as f64 / span;
    // A hair of tolerance, so a range that lands on a multiple keeps that tick
    // instead of losing it to the last bit of the division. It is scaled to the
    // values, not to the step: a step-sized tolerance grows without bound as the
    // ladder climbs and ends up reaching right past a short range.
    let tolerance = axis.min.abs().max(axis.max.abs()) * 1e-12;
    let low = ((axis.min - tolerance) / step).ceil();
    let high = ((axis.max + tolerance) / step).floor();
    // Counted before either end is cast, so a step small enough to overflow an
    // index is refused rather than saturated into a tick at some arbitrary
    // value. More marks than pixels is refused for the same reason.
    if !(0.0..axis.length as f64).contains(&(high - low)) {
        return Vec::new();
    }
    let (first, last) = (low as i64, high as i64);

    let mut placed = Vec::new();
    for k in first..=last {
        let value = k as f64 * step;
        let label = format_label(kind, value, axis.min, axis.max);
        let extent = labels.extent(&label);
        let offset = ((value - axis.min) * scale)
            .round()
            .clamp(0.0, (axis.length - 1) as f64);
        placed.push(Placed {
            tick: Tick {
                value,
                label,
                offset: offset as i64,
            },
            extent,
        });
    }

    let lo = -(axis.lead as f64);
    let hi = (axis.length - 1 + axis.trail) as f64;
    placed.retain(|p| {
        let centre = p.tick.offset as f64;
        centre - p.extent / 2.0 >= lo && centre + p.extent / 2.0 <= hi
    });
    placed
}

/// Whether neighbouring labels stay apart and stay distinct.
///
/// The equality test is one rule where a resolution floor per axis would be
/// three: it stops a half-decibel step printing `-60` twice and a sub-hertz
/// step printing the same megahertz value twice, and it keeps holding if a
/// formatter changes.
fn readable(placed: &[Placed], gap: f64) -> bool {
    placed.windows(2).all(|pair| {
        let (a, b) = (&pair[0], &pair[1]);
        a.tick.label != b.tick.label
            && (b.tick.offset - a.tick.offset) as f64 >= (a.extent + b.extent) / 2.0 + gap
    })
}

/// Round steps for this kind of axis, densest first, starting at `from`.
fn ladder(kind: AxisKind, from: f64) -> Vec<f64> {
    if kind != AxisKind::Time {
        return decimal_ladder(from);
    }
    let all = clock_ladder();
    let start = all
        .iter()
        .position(|step| *step >= from)
        .unwrap_or(all.len() - 1);
    all[start..].to_vec()
}

/// `1`, `2` or `5` times a power of ten, from the first one at least `from`.
fn decimal_ladder(from: f64) -> Vec<f64> {
    let from = if from.is_finite() && from > 0.0 {
        from
    } else {
        f64::MIN_POSITIVE
    };
    let mut decade = 10f64.powi(from.log10().floor() as i32);
    let mut steps = Vec::with_capacity(CANDIDATES);
    while steps.len() < CANDIDATES {
        for mantissa in [1.0, 2.0, 5.0] {
            let step = mantissa * decade;
            if step >= from * (1.0 - 1e-9) {
                steps.push(step);
            }
        }
        decade *= 10.0;
    }
    steps
}

/// The clock ladder, extended past a day by doubling: nothing beyond that is
/// rounder than another day.
fn clock_ladder() -> Vec<f64> {
    let mut steps = CLOCK_STEPS.to_vec();
    while steps.len() < CANDIDATES {
        steps.push(steps[steps.len() - 1] * 2.0);
    }
    steps
}

/// Which clock format an axis prints.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Clock {
    HoursMinutesSeconds,
    MinutesSeconds,
}

/// The format follows the largest time the axis prints, not the length of the
/// span it covers.
///
/// A whole-file render makes the two the same thing. A selection does not: one
/// minute taken an hour in has a one-minute span, and printing it as `60.00`
/// would put a minute count past sixty on a clock.
fn clock_of(min: f64, max: f64) -> Clock {
    if min.abs().max(max.abs()) >= 3600.0 {
        Clock::HoursMinutesSeconds
    } else {
        Clock::MinutesSeconds
    }
}

fn format_label(kind: AxisKind, value: f64, min: f64, max: f64) -> String {
    match kind {
        AxisKind::Time => format_clock(value, clock_of(min, max)),
        AxisKind::Frequency => format_hz(value),
        AxisKind::Decibels => format!("{value:.0}"),
        AxisKind::DecibelsWithUnit => format!("{value:.0} dB"),
    }
}

/// A time-axis label: `1:02:09` once the axis reaches an hour, `3.07` below it.
///
/// Seconds are whole because the ladder never steps finer than one, so no
/// label needs a fraction and none carries one.
fn format_clock(seconds: f64, clock: Clock) -> String {
    let sign = if seconds < 0.0 { "-" } else { "" };
    let total = seconds.abs().round() as u64;
    let (hours, minutes, secs) = (total / 3600, (total % 3600) / 60, total % 60);
    match clock {
        Clock::HoursMinutesSeconds => format!("{sign}{hours}:{minutes:02}:{secs:02}"),
        Clock::MinutesSeconds => format!("{sign}{}.{secs:02}", total / 60),
    }
}

/// The widest frequency label an axis reaching `peak` can print.
///
/// `format_hz` picks its unit per value and trims trailing zeros, so the widest
/// label is the one at the largest magnitude with every decimal that unit
/// resolves to still in use.
fn widest_hz(sign: &str, peak: f64) -> String {
    let (scale, unit, decimals) = if peak >= 1e9 {
        (1e9, "GHz", 9)
    } else if peak >= 1e6 {
        (1e6, "MHz", 6)
    } else if peak >= 1e3 {
        (1e3, "kHz", 3)
    } else {
        (1.0, "Hz", 3)
    };
    format!(
        "{sign}{}.{} {unit}",
        (peak / scale) as u64,
        "0".repeat(decimals)
    )
}

#[cfg(test)]
mod tests {
    include!("ticks_tests.rs");
}
