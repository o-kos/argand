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

/// What a label costs on the canvas, asked of whatever will draw it.
///
/// The layout below has to measure a candidate label before it can know
/// whether the step that produced it fits, and only the front end owns a font.
/// Two questions are enough for that, and neither of them names a glyph, an
/// image or a toolkit, so the policy stays here and the font stays there.
pub trait LabelMeasure {
    /// Width of `text` in pixels at `size`.
    ///
    /// Every digit must come back the same width, which is what a face with
    /// tabular figures gives. [`widest_labels`] reserves a gutter before any
    /// tick is chosen, and it can only do that by standing a row of zeros in
    /// for a number nobody has picked yet; under proportional digits that
    /// bound is not a bound, and a label runs into the plot beside it.
    fn width(&self, text: &str, size: f32) -> f32;

    /// Height of the ink a numeric label puts on the canvas at `size`.
    fn digit_height(&self, size: f32) -> f32;
}

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

/// What an axis prints, which fixes its ladder, its labels and its unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AxisKind {
    /// Seconds, on a clock ladder no finer than one second.
    Time,
    /// Hertz, scaled to one unit chosen for the whole axis.
    Frequency,
    /// Whole decibels.
    Decibels,
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
    measure: &'a dyn LabelMeasure,
    size: f32,
    run: LabelRun,
}

impl<'a> LabelMetrics<'a> {
    pub fn new(measure: &'a dyn LabelMeasure, size: f32, run: LabelRun) -> Self {
        Self { measure, size, run }
    }

    /// Ink the label occupies along the axis, centred on its tick.
    fn extent(&self, label: &str) -> f64 {
        match self.run {
            LabelRun::Across => f64::from(self.measure.width(label, self.size)),
            LabelRun::Down => f64::from(self.measure.digit_height(self.size)),
        }
    }

    /// Clear space two neighbouring labels have to keep.
    ///
    /// Two digits at the label size. Below that, `12` and `14` a few pixels
    /// apart read as one four-digit number; deriving it from the font rather
    /// than writing a pixel count down means it still holds if the label size
    /// changes.
    fn gap(&self) -> f64 {
        f64::from(self.measure.width("00", self.size))
    }
}

/// Steps to try, densest first, until one fits.
///
/// Both ladders start at the densest step the axis could possibly hold and
/// climb from there, so the search ends on a candidate rather than on running
/// out of them.
const CANDIDATES: usize = 40;

/// A day, where the clock stops naming steps and starts doubling.
const DAY: f64 = 86_400.0;

/// Whole-second steps a clock actually uses.
///
/// Decimal steps put marks at 250-second intervals, which read as `4.10` and
/// tell nobody anything.
const CLOCK_STEPS: [f64; 18] = [
    1.0, 2.0, 5.0, 10.0, 15.0, 30.0, // seconds
    60.0, 120.0, 300.0, 600.0, 900.0, 1800.0, // minutes
    3600.0, 7200.0, 10800.0, 21600.0, 43200.0, // hours
    DAY,
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

/// The unit this axis prints in, named once instead of on every tick.
///
/// `None` where the labels carry their own meaning: a clock reads as a clock.
pub fn caption(kind: AxisKind, min: f64, max: f64) -> Option<&'static str> {
    match kind {
        AxisKind::Time => None,
        AxisKind::Frequency => Some(hertz_unit(min, max).name),
        AxisKind::Decibels => Some("dB"),
    }
}

/// The widest label an axis of this kind could print over `min..max`.
///
/// A gutter has to be reserved before any tick is chosen, so this bounds the
/// label rather than predicting it. The bound is built from zeros, which stand
/// in exactly for the value they replace as long as the measure keeps its side
/// of [`LabelMeasure::width`] and gives every digit one width.
///
/// More than one candidate comes back where more than one could be the widest,
/// because the caller has the font and this does not: which of two strings takes
/// more room is a question about glyphs, not about characters.
pub fn widest_labels(kind: AxisKind, min: f64, max: f64) -> Vec<String> {
    let sign = if min < 0.0 { "-" } else { "" };
    let peak = min.abs().max(max.abs());
    match kind {
        AxisKind::Time => match clock_of(min, max) {
            Clock::HoursMinutesSeconds => vec![format!("{sign}{}:00:00", (peak / 3600.0) as u64)],
            Clock::MinutesSeconds => vec![format!("{sign}{}.00", (peak / 60.0) as u64)],
        },
        // One unit for the whole axis, so the widest label is simply the
        // largest magnitude with every decimal that unit resolves to in use.
        AxisKind::Frequency => {
            let unit = hertz_unit(min, max);
            vec![format!(
                "{sign}{}.{}",
                (peak / unit.scale) as u64,
                "0".repeat(unit.decimals)
            )]
        }
        AxisKind::Decibels => vec![
            format_label(kind, min, min, max),
            format_label(kind, max, min, max),
        ],
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
    // The slack absorbs the last bit of the division, so a range whose end sits
    // on a multiple keeps that tick even when the quotient comes back a hair
    // short of the whole number it should be. It is measured in multiples
    // rather than in seconds or hertz -- a slack in values wide enough to cover
    // the division at one end of a long axis reaches right past a narrow range
    // at the other -- and capped, so that at a large quotient it cannot grow
    // into a whole multiple and push the index count over the guard below.
    //
    // The cap is not itself a promise about where a tick lands: past about
    // `2^40 / step` the division's own error is already wider than it. What a
    // tick may not do is checked on the finished value, further down.
    let slack = |quotient: f64| (quotient.abs().max(1.0) * 8.0 * f64::EPSILON).min(1.0 / 1024.0);
    let (lower, upper) = (axis.min / step, axis.max / step);
    let low = (lower - slack(lower)).ceil();
    let high = (upper + slack(upper)).floor();
    // Both ends are checked before either is cast. Past `2^53` an index no
    // longer round-trips through `f64`, and past `i64` the cast saturates into
    // a tick at some arbitrary value; more marks than pixels is refused for the
    // same reason.
    const INDEX_LIMIT: f64 = 9_007_199_254_740_992.0;
    if low.abs() > INDEX_LIMIT || high.abs() > INDEX_LIMIT {
        return Vec::new();
    }
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

    // Whatever the division, the addition and the multiplication did between
    // them, the finished value has to land where the axis can show it. Inside a
    // pixel of an end is a rounding artefact of that end, and keeping it is
    // what stops a range stopping at 0.3 with a step of 0.1 losing its last
    // tick to arithmetic nobody can see. Further out than a pixel is not an
    // artefact: it is a coordinate the caller never asked for.
    //
    // The distance is measured, not the widened end. `max + pixel` is itself a
    // sum that rounds, and where the pixel is small against the value it is
    // added to, that rounding swallows the very allowance being made.
    let pixel = span / (axis.length - 1) as f64;
    placed.retain(|p| escape(p.tick.value, axis) <= pixel);

    let lo = -(axis.lead as f64);
    let hi = (axis.length - 1 + axis.trail) as f64;
    placed.retain(|p| {
        let centre = p.tick.offset as f64;
        centre - p.extent / 2.0 >= lo && centre + p.extent / 2.0 <= hi
    });
    placed
}

/// How far a value lies outside the axis, and zero when it lies inside.
///
/// Subtracting two nearby values is exact where adding a small allowance to a
/// large end is not, which is why the comparison is made this way round.
fn escape(value: f64, axis: Axis) -> f64 {
    if value < axis.min {
        axis.min - value
    } else if value > axis.max {
        value - axis.max
    } else {
        0.0
    }
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
    match kind {
        AxisKind::Time => clock_ladder(from),
        _ => decimal_ladder(from),
    }
}

/// `1`, `2` or `5` times a power of ten, from the first one at least `from`.
fn decimal_ladder(from: f64) -> Vec<f64> {
    // The decade is held inside the normal range. Below it, `powi` underflows
    // to zero, and a decade of zero neither produces a step nor grows when it
    // is multiplied, so the search would spin for ever on a span too small to
    // label at all.
    let exponent = if from.is_finite() && from > 0.0 {
        from.log10().floor().clamp(-300.0, 300.0)
    } else {
        -300.0
    };
    let mut decade = 10f64.powi(exponent as i32);
    let mut steps = Vec::with_capacity(CANDIDATES);
    while steps.len() < CANDIDATES && decade.is_finite() {
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
/// rounder than another day, and it keeps doubling until it has passed `from`,
/// so a span of any length ends on a candidate.
fn clock_ladder(from: f64) -> Vec<f64> {
    let mut steps: Vec<f64> = CLOCK_STEPS.iter().copied().filter(|s| *s >= from).collect();
    let mut step = DAY;
    while steps.len() < CANDIDATES && step.is_finite() {
        step *= 2.0;
        if step >= from {
            steps.push(step);
        }
    }
    steps
}

/// Which clock format an axis prints.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Clock {
    HoursMinutesSeconds,
    MinutesSeconds,
}

/// The format follows the span the axis covers, not where on the recording it
/// starts.
///
/// Going by the largest time printed was tried first, to keep the minutes field
/// under sixty. It makes the format depend on where the window sits: panning a
/// one-minute selection across the hour mark changes every label without
/// changing the zoom. A minutes field that counts past sixty is the smaller
/// oddity, and tying the format to the zoom alone is what a reader can predict.
fn clock_of(min: f64, max: f64) -> Clock {
    if max - min >= 3600.0 {
        Clock::HoursMinutesSeconds
    } else {
        Clock::MinutesSeconds
    }
}

fn format_label(kind: AxisKind, value: f64, min: f64, max: f64) -> String {
    match kind {
        AxisKind::Time => format_clock(value, clock_of(min, max)),
        AxisKind::Frequency => hertz_unit(min, max).format(value),
        AxisKind::Decibels => format!("{value:.0}"),
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

/// The unit a frequency axis prints in.
///
/// One unit is chosen for the whole axis and named once beside it, rather than
/// repeated on every tick. Repeating it costs a third of each label -- ` MHz`
/// measures 27 pixels against the 60 the digits need -- to say the same thing
/// a dozen times, and picking the unit per value the way `format_hz` does would
/// put `999.999 Hz` and `1.000 kHz` on one axis, two spellings of neighbouring
/// values.
#[derive(Debug, Clone, Copy, PartialEq)]
struct HertzUnit {
    name: &'static str,
    scale: f64,
    /// Decimals that resolve to one hertz in this unit.
    decimals: usize,
}

impl HertzUnit {
    /// The value in this unit, with trailing zeros trimmed.
    fn format(self, hz: f64) -> String {
        let text = format!("{:.*}", self.decimals, hz / self.scale);
        let trimmed = text.trim_end_matches('0').trim_end_matches('.');
        // A value too small for this unit to resolve is zero, and zero has no
        // sign: without this an axis just below zero labels a tick `-0`.
        let bare = trimmed.trim_start_matches('-');
        if bare.is_empty() || bare.chars().all(|c| c == '0') {
            "0".to_string()
        } else {
            trimmed.to_string()
        }
    }
}

/// The unit that suits the whole of `min..max`.
///
/// It follows the largest magnitude on the axis, which is the one whose digits
/// have to fit: a span of a few kilohertz around 12.5 MHz is a megahertz axis,
/// not a kilohertz one.
fn hertz_unit(min: f64, max: f64) -> HertzUnit {
    let peak = min.abs().max(max.abs());
    let (name, scale, decimals) = if peak >= 1e9 {
        ("GHz", 1e9, 9)
    } else if peak >= 1e6 {
        ("MHz", 1e6, 6)
    } else if peak >= 1e3 {
        ("kHz", 1e3, 3)
    } else {
        ("Hz", 1.0, 3)
    };
    HertzUnit {
        name,
        scale,
        decimals,
    }
}

#[cfg(test)]
mod tests {
    include!("axis_tests.rs");
}
