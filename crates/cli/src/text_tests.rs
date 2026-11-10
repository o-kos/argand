use super::*;
use argand_core::testutil::DejaVuSans;

fn blank(w: u32, h: u32) -> RgbImage {
    RgbImage::from_pixel(w, h, Rgb([0, 0, 0]))
}

/// White at the size the tests were written against.
const WHITE: TextStyle = TextStyle {
    size: 14.0,
    color: Rgb([255, 255, 255]),
};

#[test]
fn alignment_moves_the_run_around_the_anchor() {
    let t = TextRenderer::new();
    let text = "-110 dB";
    let anchor = 100.0;

    let bounds = |at: Anchor| {
        let mut canvas = blank(200, 40);
        t.draw(&mut canvas, text, at, WHITE);
        let xs: Vec<u32> = canvas
            .enumerate_pixels()
            .filter(|(_, _, p)| p.0 != [0, 0, 0])
            .map(|(x, _, _)| x)
            .collect();
        (*xs.iter().min().unwrap(), *xs.iter().max().unwrap())
    };

    let (left_min, _) = bounds(Anchor::left(anchor, 26.0));
    let (center_min, center_max) = bounds(Anchor::center(anchor, 26.0));
    let (_, right_max) = bounds(Anchor::right(anchor, 26.0));

    assert!(left_min >= anchor as u32 - 1, "left starts at the anchor");
    assert!(right_max <= anchor as u32 + 1, "right ends at the anchor");
    let center = (center_min + center_max) / 2;
    assert!(center.abs_diff(anchor as u32) <= 2, "centre straddles it");
}

#[test]
fn drawing_off_canvas_does_not_panic() {
    let t = TextRenderer::new();
    let mut canvas = blank(40, 20);
    for (x, y) in [(-500.0, 10.0), (500.0, 10.0), (10.0, -500.0), (10.0, 500.0)] {
        t.draw(&mut canvas, "edge", Anchor::left(x, y), WHITE);
    }
}

/// Every character an axis label can be built from.
const LABEL_ALPHABET: [char; 13] = [
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', '.', ':', '-',
];

#[test]
fn the_fixture_font_measures_what_the_real_one_does() {
    // `argand-core` lays its axes out against `testutil::DejaVuSans`, a table
    // of this font's advances rather than the font itself, because the core
    // may not open one. That table is only worth anything while it agrees
    // with the face this binary embeds, and it is stated at the size the plot
    // labels with, so that is where they are held against each other.
    let real = TextRenderer::new();
    let fixture = DejaVuSans;
    const SIZE: f32 = 13.0;

    let same = |label: &str| {
        let (want, got) = (real.width(label, SIZE), fixture.width(label, SIZE));
        assert_eq!(
            want, got,
            "{label:?} measures {want} in the font and {got} in the fixture"
        );
    };

    // Every glyph, then every ordered pair of them: the table records an
    // advance each and no kerning, so a pair is where a kerning class the
    // table does not know about would show up.
    for a in LABEL_ALPHABET {
        same(&a.to_string());
        for b in LABEL_ALPHABET {
            same(&format!("{a}{b}"));
        }
    }
    // And whole labels, including the pair the tick spacing is derived from.
    for label in ["00", "-300", "-10000", "12.579887", "3.07", "-1.30", "1:02:09", "60.00"] {
        same(label);
    }
    // The captions are held as whole strings rather than added to the alphabet
    // above: their letters kern against digits and separators they never stand
    // beside, and a pair that cannot occur is not worth a table entry.
    for caption in ["Hz", "kHz", "MHz", "GHz", "dB", "12.345 s", "#12345", "hms", "1,234.50"] {
        same(caption);
    }
    assert_eq!(real.digit_height(SIZE), fixture.digit_height(SIZE));
}
