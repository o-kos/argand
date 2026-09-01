//! Fixture helpers: a label measure that needs no font.
//!
//! Enabled for this crate's own tests and, via the `testutil` feature, for the
//! cli test suite. Tick layout is decided from measured labels, so testing it
//! at all needs metrics; this crate may not open a font, and a measure that
//! made every character one pixel wide would exercise a policy no front end
//! ever runs.

use crate::axis::LabelMeasure;

/// DejaVu Sans, the face `aspec` embeds, for the glyphs an axis label is made
/// of.
///
/// Advances are the font's own, over the 2384 units between its ascender and
/// its descender, which is the height ab_glyph scales a `PxScale` against. The
/// digit ink height is not a ratio the font states: it is the outline's
/// bounding box rounded outward to whole pixels, so it is taken here as the
/// value measured at the size the plot labels with.
///
/// `crates/cli/src/text_tests.rs` holds both against the real renderer, so a
/// change to the font asset fails there rather than quietly moving every tick
/// this fixture places.
pub struct DejaVuSans;

/// Units between the ascender and the descender.
const HEIGHT: f32 = 2384.0;

/// The size axis labels are drawn at, which is where the ink height was taken.
const MEASURED_AT: f32 = 13.0;

/// Ink a digit puts on the canvas at [`MEASURED_AT`].
const DIGIT_INK: f32 = 10.0;

impl DejaVuSans {
    /// Advance of one label glyph, in font units.
    ///
    /// Every character an axis label can hold is here. A new one has to be
    /// added deliberately rather than guessed at, so an unknown glyph is a
    /// failure and not a plausible width.
    fn advance(c: char) -> f32 {
        match c {
            '0'..='9' => 1303.0,
            '.' => 651.0,
            ':' => 690.0,
            '-' => 739.0,
            other => panic!("no advance recorded for {other:?}"),
        }
    }
}

impl LabelMeasure for DejaVuSans {
    fn width(&self, text: &str, size: f32) -> f32 {
        // Digits, separators and the minus sign kern against nothing in this
        // face, so the advances simply add up.
        text.chars().map(Self::advance).sum::<f32>() * size / HEIGHT
    }

    fn digit_height(&self, size: f32) -> f32 {
        size * DIGIT_INK / MEASURED_AT
    }
}
