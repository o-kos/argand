use super::*;
use argand_core::{SampleRange, SampleType};
use argand_dsp::{AnalysisRequest, DbReference, Reduce, StftConfig, Window, analyze};
use argand_io::OpenHints;
use argand_io::testutil::{TempDir, iq_tone, real_tone, write_wav};

const RATE: f64 = 24_000.0;
const FFT: usize = 256;
const TONE_BIN: usize = 40;
const TONE_HZ: f64 = TONE_BIN as f64 * RATE / FFT as f64;

const WAVEFORM: Panels = Panels {
    waveform: true,
    psd: false,
    db: false,
};
const PSD: Panels = Panels {
    waveform: false,
    psd: true,
    db: false,
};
const DB: Panels = Panels {
    waveform: false,
    psd: false,
    db: true,
};
const EVERYTHING: Panels = Panels {
    waveform: true,
    psd: true,
    db: true,
};

/// The decibel window `--dynamic-range 60` opens, which is what these tests
/// render with.
const DECIBELS: (f64, f64) = (-60.0, 0.0);

/// The extents the test captures cover: 8192 samples at 24 kHz, at baseband.
const TIME: (f64, f64) = (0.0, 8192.0 / RATE);
const BASEBAND: (f64, f64) = (-RATE / 2.0, RATE / 2.0);

/// The gutters the test captures need.
fn gutters() -> Gutters {
    Gutters::measure(TIME, BASEBAND, DECIBELS)
}

fn laid_out(width: u32, height: u32, panels: Panels, orientation: Orientation) -> Layout {
    Layout::compute(width, height, panels, orientation, gutters())
}

fn analysis_of(dir: &TempDir, name: &str, values: &[f32], iq: bool, layout: &Layout) -> Analysis {
    let st: SampleType = if iq { "iq_f32" } else { "rl_f32" }.parse().unwrap();
    let path = write_wav(&dir.join(name), st, RATE as u32, values, 1.0);
    let mut src = argand_io::open(&path, &OpenHints::default()).unwrap();
    let range = SampleRange::new(0, src.meta().len_samples);
    let (width, height) = layout.transform_size();
    analyze(
        src.as_mut(),
        &AnalysisRequest {
            cfg: StftConfig::new(FFT, Window::Hann),
            range,
            width,
            height,
            reduce: Reduce::Max,
            colormap: Colormap::Grayscale,
            dynamic_range_db: 60.0,
            reference: DbReference::Peak,
            waveform_columns: layout.waveform_columns(),
        },
        &mut |_, _| {},
    )
    .unwrap()
}

fn analysis(dir: &TempDir, name: &str, iq: bool, layout: &Layout) -> Analysis {
    let values = if iq {
        iq_tone(8192, RATE, TONE_HZ, 0.8)
    } else {
        real_tone(8192, RATE, TONE_HZ, 0.8)
    };
    analysis_of(dir, name, &values, iq, layout)
}

fn plot(layout: &Layout, analysis: &Analysis) -> RgbImage {
    render(
        layout,
        &PlotInput {
            analysis,
            title: "title",
            footer: "footer",
            colormap: Colormap::Grayscale,
            waveform_full_scale: waveform_full_scale(analysis.time_peak, DbReference::Peak),
        },
    )
}

#[test]
fn the_default_render_is_a_waveform_and_a_spectrogram_only() {
    for orientation in [Orientation::Horizontal, Orientation::Vertical] {
        let layout = laid_out(900, 500, WAVEFORM, orientation);
        assert!(layout.spectrogram.is_some(), "{orientation}");
        assert!(layout.waveform.is_some(), "{orientation}");
        assert!(layout.psd.is_none(), "{orientation}: psd is opt-in");
        assert!(layout.colorbar.is_none(), "{orientation}: db is opt-in");
    }
}

#[test]
fn the_spectrogram_is_drawn_whatever_the_panels_say() {
    for panels in [Panels::NONE, WAVEFORM, EVERYTHING] {
        let layout = laid_out(900, 500, panels, Orientation::Horizontal);
        assert!(layout.spectrogram.is_some(), "{panels}");
    }
}

#[test]
fn a_horizontal_strip_sits_above_the_spectrogram_on_the_same_time_axis() {
    let layout = laid_out(1600, 500, EVERYTHING, Orientation::Horizontal);
    let spec = layout.spectrogram.expect("spectrogram");
    let strip = layout.waveform.expect("waveform");

    assert_eq!(strip.x, spec.x, "the time axis starts in the same column");
    assert_eq!(strip.w, spec.w, "and covers the same width");
    assert!(strip.bottom() < spec.y, "the strip sits above");
    assert_eq!(strip.h, WAVEFORM_SPAN, "a mini-map, not a panel");
    assert_eq!(layout.waveform_columns(), Some(spec.w as usize));
}

#[test]
fn a_vertical_strip_sits_beside_the_spectrogram_on_the_same_time_axis() {
    let layout = laid_out(700, 900, EVERYTHING, Orientation::Vertical);
    let spec = layout.spectrogram.expect("spectrogram");
    let strip = layout.waveform.expect("waveform");

    assert_eq!(strip.y, spec.y, "the time axis starts in the same row");
    assert_eq!(strip.h, spec.h, "and covers the same height");
    assert!(spec.right() < strip.x, "the strip sits to the right");
    assert_eq!(strip.w, WAVEFORM_SPAN);
    assert_eq!(layout.waveform_columns(), Some(spec.h as usize));
}

#[test]
fn the_spectrum_stays_on_the_spectrogram_frequency_axis() {
    // Horizontal: a column beside the waterfall, row for row with it.
    let layout = laid_out(1600, 500, EVERYTHING, Orientation::Horizontal);
    let spec = layout.spectrogram.unwrap();
    let psd = layout.psd.expect("psd");
    let bar = layout.colorbar.expect("colorbar");
    assert!(spec.right() < psd.x);
    assert!(psd.right() <= bar.x, "the colour bar is furthest right");
    assert_eq!((psd.y, psd.h), (spec.y, spec.h));
    assert!(spec.w > psd.w * 2, "the waterfall keeps most of the width");

    // Vertical: a row above the waterfall, column for column with it.
    let down = laid_out(700, 900, EVERYTHING, Orientation::Vertical);
    let spec = down.spectrogram.unwrap();
    let psd = down.psd.expect("psd");
    assert!(psd.bottom() < spec.y);
    assert_eq!((psd.x, psd.w), (spec.x, spec.w));
    assert!(spec.h > psd.h);
}

#[test]
fn dropping_a_panel_gives_its_room_to_the_spectrogram() {
    let all = laid_out(1600, 500, EVERYTHING, Orientation::Horizontal);
    let bare = laid_out(1600, 500, Panels::NONE, Orientation::Horizontal);

    let (all_spec, bare_spec) = (all.spectrogram.unwrap(), bare.spectrogram.unwrap());
    assert!(bare_spec.w > all_spec.w, "the spectrum column came back");
    assert!(bare_spec.h > all_spec.h, "so did the strip's height");
    assert!(bare.waveform.is_none() && bare.psd.is_none() && bare.colorbar.is_none());
}

#[test]
fn the_transform_is_asked_for_the_pixels_it_will_fill() {
    let horizontal = laid_out(1600, 500, EVERYTHING, Orientation::Horizontal);
    let rect = horizontal.spectrogram.unwrap();
    assert_eq!(horizontal.transform_size(), (rect.w as usize, rect.h as usize));

    // Turning the waterfall on its side swaps which axis is time.
    let vertical = laid_out(700, 900, EVERYTHING, Orientation::Vertical);
    let rect = vertical.spectrogram.unwrap();
    assert_eq!(vertical.transform_size(), (rect.h as usize, rect.w as usize));
}

#[test]
fn an_image_too_small_for_a_plot_yields_no_panels() {
    for (w, h) in [(1, 1), (40, 40), (200, 20)] {
        let layout = laid_out(w, h, EVERYTHING, Orientation::Horizontal);
        assert!(
            layout.spectrogram.is_none() || layout.spectrogram.unwrap().is_valid(),
            "{w}x{h} produced a degenerate rect"
        );
        if layout.spectrogram.is_none() {
            assert_eq!(layout.transform_size(), (0, 0), "{w}x{h}");
            assert_eq!(layout.waveform_columns(), None, "{w}x{h}");
        }
    }
}

#[test]
fn rendering_fills_the_requested_canvas() {
    let dir = TempDir::new("render-size");
    let layout = laid_out(900, 320, EVERYTHING, Orientation::Horizontal);
    let a = analysis(&dir, "a.wav", true, &layout);
    let canvas = plot(&layout, &a);

    assert_eq!((canvas.width(), canvas.height()), (900, 320));
    let distinct: std::collections::HashSet<_> = canvas.pixels().map(|p| p.0).collect();
    assert!(distinct.len() > 8, "expected a drawn plot, got {distinct:?}");
}

#[test]
fn the_waterfall_lands_inside_its_panel_and_nowhere_else() {
    let dir = TempDir::new("render-bounds");
    let layout = laid_out(900, 320, Panels::NONE, Orientation::Horizontal);
    let a = analysis(&dir, "b.wav", true, &layout);
    let canvas = plot(&layout, &a);
    let rect = layout.spectrogram.unwrap();

    // The tone's row inside the panel, counted from the top of the image.
    let img = &a.spectrogram;
    let brightest_row = (0..img.height).max_by_key(|&y| img.get(4, y)[0]).unwrap();
    let y = rect.y as u32 + brightest_row as u32;
    let inside = canvas.get_pixel(rect.x as u32 + 4, y);
    assert!(inside.0[0] > 100, "tone should be bright: {inside:?}");

    // Just outside the frame is background, not spillover.
    let outside = canvas.get_pixel(rect.x as u32 - 4, y);
    assert!(outside.0[0] < 60, "waterfall leaked outside its rect");
}

#[test]
fn a_vertical_waterfall_puts_low_frequency_on_the_left() {
    let dir = TempDir::new("render-vertical");
    let layout = laid_out(400, 700, Panels::NONE, Orientation::Vertical);
    let a = analysis(&dir, "c.wav", false, &layout);
    let canvas = plot(&layout, &a);
    let rect = layout.spectrogram.unwrap();

    // A real tone at bin 40 of 129 sits at about a third of the way up the
    // one-sided spectrum, which is a third of the way across the panel.
    let mid_y = (rect.y + rect.h / 2) as u32;
    let brightest = (0..rect.w)
        .max_by_key(|&x| canvas.get_pixel(rect.x as u32 + x as u32, mid_y).0[0])
        .unwrap();
    let fraction = brightest as f64 / rect.w as f64;
    let expected = TONE_BIN as f64 / (FFT / 2 + 1) as f64;
    assert!(
        (fraction - expected).abs() < 0.08,
        "tone at {fraction:.2} of the width, expected {expected:.2}"
    );
}

/// Counts pixels in `rect` that lean warm (Q) and cool (I).
fn trace_colors(canvas: &RgbImage, rect: Rect) -> (usize, usize) {
    let mut warm = 0;
    let mut cool = 0;
    for y in rect.y..rect.bottom() {
        for x in rect.x..rect.right() {
            let [r, _, b] = canvas.get_pixel(x as u32, y as u32).0;
            // In i32: the trace is full blue, so u8 arithmetic overflows here.
            let (r, b) = (r as i32, b as i32);
            if r > b + 30 {
                warm += 1;
            } else if b > r + 60 {
                cool += 1;
            }
        }
    }
    (warm, cool)
}

#[test]
fn the_strip_merges_i_and_q_into_one_span() {
    // Q swings far wider than I. A single track has to follow the wider of
    // the two, and must not reach for a second colour to say so.
    let mut values = Vec::new();
    for n in 0..8192 {
        let phase = std::f64::consts::TAU * TONE_HZ * n as f64 / RATE;
        values.push(0.25 * phase.cos() as f32);
        values.push(0.9 * phase.sin() as f32);
    }

    let dir = TempDir::new("render-iq");
    let layout = laid_out(900, 320, WAVEFORM, Orientation::Horizontal);
    let a = analysis_of(&dir, "iq.wav", &values, true, &layout);
    assert_eq!(a.waveform.as_ref().unwrap().channels, 2, "both are kept");
    let canvas = plot(&layout, &a);

    let strip = layout.waveform.unwrap();
    let (warm, cool) = trace_colors(&canvas, strip);
    assert!(cool > 200, "the trace is missing: {cool} cool pixels");
    assert_eq!(warm, 0, "one track means one colour");

    // Following I alone would reach a quarter of the height, not all of it.
    let column = strip.x + strip.w / 2;
    let lit: Vec<i64> = (strip.y..strip.bottom())
        .filter(|y| canvas.get_pixel(column as u32, *y as u32).0[2] > 100)
        .collect();
    let span = lit.last().unwrap() - lit.first().unwrap();
    assert!(
        span >= strip.h - 4,
        "the span follows I ({span} of {}), not Q",
        strip.h
    );
}

#[test]
fn nothing_is_drawn_outside_the_strip() {
    let dir = TempDir::new("render-real");
    let layout = laid_out(900, 320, WAVEFORM, Orientation::Horizontal);
    let a = analysis(&dir, "real.wav", false, &layout);
    assert_eq!(a.waveform.as_ref().unwrap().channels, 1);

    let canvas = plot(&layout, &a);
    let strip = layout.waveform.unwrap();
    assert!(trace_colors(&canvas, strip).1 > 200, "the trace is missing");

    // The gutter beside the strip carries axis labels and nothing else.
    let gutter = Rect {
        x: PAD,
        w: strip.x - PAD - 2,
        ..strip
    };
    assert_eq!(trace_colors(&canvas, gutter), (0, 0), "the trace leaked out");
}

#[test]
fn the_strip_reaches_its_edge_at_the_reference_level() {
    let dir = TempDir::new("render-scale");
    let layout = laid_out(900, 320, WAVEFORM, Orientation::Horizontal);
    // A quiet signal: under --ref peak the strip still uses its full height.
    let values = real_tone(8192, RATE, TONE_HZ, 0.02);
    let a = analysis_of(&dir, "quiet.wav", &values, false, &layout);
    let canvas = plot(&layout, &a);

    let strip = layout.waveform.unwrap();
    let column = strip.x + strip.w / 2;
    let lit: Vec<i64> = (strip.y..strip.bottom())
        .filter(|y| canvas.get_pixel(column as u32, *y as u32).0[2] > 100)
        .collect();
    let span = lit.last().unwrap() - lit.first().unwrap();
    assert!(
        span >= strip.h - 4,
        "the trace spans {span} of {} pixels",
        strip.h
    );
}

#[test]
fn the_reference_level_follows_the_ref_flag() {
    assert_eq!(waveform_full_scale(0.5, DbReference::FullScale), 1.0);
    assert_eq!(waveform_full_scale(0.5, DbReference::Peak), 0.5);
    // Silence must not become a division by zero.
    assert!(waveform_full_scale(0.0, DbReference::Peak) > 0.0);
}

#[test]
fn the_spectrum_panel_scales_to_its_own_trace() {
    // The averaged spectrum sits far below the per-frame maxima; the panel
    // must fit the trace, not the colour bar's range.
    let (lo, hi) = psd_range(&[-121.0, -119.0, -99.0]);
    assert!(hi > -99.0 && hi < -95.0, "top was {hi}");
    assert!(lo < -121.0 && lo > -130.0, "bottom was {lo}");

    // A flat trace still gets a usable span rather than a zero-height one.
    let (lo, hi) = psd_range(&[-60.0, -60.0]);
    assert!(hi - lo >= 10.0, "{lo}..{hi}");

    // Nothing finite to work with: fall back rather than produce NaN bounds.
    let (lo, hi) = psd_range(&[f32::NEG_INFINITY]);
    assert!(lo.is_finite() && hi.is_finite() && hi > lo);
}

#[test]
fn panel_lists_round_trip_through_their_names() {
    use std::str::FromStr;
    for (text, want) in [
        ("waveform", WAVEFORM),
        ("none", Panels::NONE),
        ("waveform,psd,db", EVERYTHING),
    ] {
        let panels = Panels::from_str(text).unwrap();
        assert_eq!(panels, want, "{text}");
        assert_eq!(panels.to_string(), text, "{text} did not round-trip");
    }

    // Order, case and spacing are the user's business, not the parser's.
    assert_eq!(Panels::from_str(" DB , Waveform ").unwrap().to_string(), "waveform,db");
    assert_eq!(Panels::from_str("psd,psd").unwrap().to_string(), "psd");
}

#[test]
fn every_panel_alias_resolves_to_its_canonical_name() {
    use std::str::FromStr;
    for (alias, canonical) in [
        ("wave", "waveform"),
        ("spectrum", "psd"),
        ("colorbar", "db"),
    ] {
        assert_eq!(Panels::from_str(alias).unwrap().to_string(), canonical);
    }
}

#[test]
fn panel_lists_reject_what_cannot_be_drawn() {
    use std::str::FromStr;
    // The spectrogram is not a panel: it is always there.
    let err = Panels::from_str("spectrogram").unwrap_err().to_string();
    assert_eq!(
        err,
        "unknown panel `spectrogram`, expected one of: waveform, psd, db, none"
    );

    let err = Panels::from_str(" , ").unwrap_err().to_string();
    assert_eq!(
        err,
        "no panels given; use `none` for the spectrogram on its own"
    );

    let err = Panels::from_str("none,waveform").unwrap_err().to_string();
    assert_eq!(err, "`none` cannot be combined with other panels");
}

#[test]
fn orientation_names_round_trip() {
    use std::str::FromStr;
    for name in ["horizontal", "vertical"] {
        assert_eq!(Orientation::from_str(name).unwrap().to_string(), name);
    }
    assert_eq!(
        Orientation::from_str(" H ").unwrap(),
        Orientation::Horizontal
    );
    assert_eq!(Orientation::from_str("v").unwrap(), Orientation::Vertical);
    assert_eq!(
        Orientation::from_str("sideways").unwrap_err().to_string(),
        "unknown orientation `sideways`, expected one of: horizontal, vertical"
    );
}

/// A spectrogram carrying only the axis extents a render would give it: these
/// tests are about where the labels land, not about what is drawn under them.
fn extents(time: (f64, f64), frequency: (f64, f64)) -> SpectrogramImage {
    let mut img = SpectrogramImage::new(1, 1);
    img.t0 = time.0;
    img.t1 = time.1;
    img.f0 = frequency.0;
    img.f1 = frequency.1;
    img
}

/// The label geometry the plot draws with, so a test can measure the same
/// boxes the renderer does.
struct Ruler {
    text: TextRenderer,
    ink: f64,
    gap: f64,
    rise: i64,
}

impl Ruler {
    fn new() -> Self {
        let text = TextRenderer::new();
        let ink = f64::from(text.digit_height(FONT_SIZE));
        let gap = f64::from(text.width("00", FONT_SIZE));
        Self {
            rise: (ink / 2.0).round() as i64,
            text,
            ink,
            gap,
        }
    }

    /// Labels centred on their ticks along the axis, which starts at `start`
    /// on a canvas `canvas` pixels wide.
    fn check_along(&self, marks: &[Tick], start: i64, canvas: u32, what: &str) {
        let box_of = |tick: &Tick| {
            let centre = f64::from(u32::try_from(start + tick.offset).unwrap_or(0));
            let half = f64::from(self.text.width(&tick.label, FONT_SIZE)) / 2.0;
            (centre - half, centre + half)
        };
        for pair in marks.windows(2) {
            let clear = box_of(&pair[1]).0 - box_of(&pair[0]).1;
            assert!(
                clear >= self.gap,
                "{what}: {:?} and {:?} leave {clear:.1}px",
                pair[0].label,
                pair[1].label
            );
        }
        if let (Some(first), Some(last)) = (marks.first(), marks.last()) {
            assert!(box_of(first).0 >= 0.0, "{what}: {:?} is off the left", first.label);
            assert!(
                box_of(last).1 <= f64::from(canvas - 1),
                "{what}: {:?} is off the right",
                last.label
            );
        }
    }

    /// Both of a plot's axes, measured where the renderer would draw them.
    ///
    /// Returns whether there was a plot to check at all, so the matrix below
    /// can prove it is not quietly skipping the combinations it enumerates.
    fn check_layout(&self, layout: &Layout, ranges: ((f64, f64), (f64, f64)), what: &str) -> bool {
        let Some(spec) = layout.spectrogram else {
            return false;
        };
        let img = extents(ranges.0, ranges.1);
        let clock = time_ticks(layout, &img, &self.text, self.rise);
        let hertz = frequency_ticks(layout, &img, &self.text, self.rise);
        match layout.orientation {
            Orientation::Horizontal => {
                self.check_along(&clock, spec.x, layout.width, what);
                self.check_stacked(&hertz, spec.x, what);
            }
            Orientation::Vertical => {
                self.check_stacked(&clock, spec.x, what);
                self.check_along(&hertz, spec.x, layout.width, what);
            }
        }
        true
    }

    /// Labels stacked in the gutter left of a plot whose edge is at `x`.
    fn check_stacked(&self, marks: &[Tick], x: i64, what: &str) {
        for pair in marks.windows(2) {
            let clear = (pair[1].offset - pair[0].offset) as f64 - self.ink;
            assert!(
                clear >= self.gap,
                "{what}: {:?} and {:?} leave {clear:.1}px",
                pair[0].label,
                pair[1].label
            );
        }
        for tick in marks {
            let left = (x - LABEL_PAD) as f64 - f64::from(self.text.width(&tick.label, FONT_SIZE));
            assert!(left >= 0.0, "{what}: {:?} starts at {left:.1}", tick.label);
        }
    }
}

/// Every panel set there is. Which panels are up decides what neighbours an
/// axis has, and so how much room its outermost labels can borrow, which is
/// exactly what the three obvious sets would not have exercised.
const PANEL_SETS: [Panels; 8] = [
    Panels::NONE,
    WAVEFORM,
    PSD,
    DB,
    Panels {
        waveform: true,
        psd: true,
        db: false,
    },
    Panels {
        waveform: true,
        psd: false,
        db: true,
    },
    Panels {
        waveform: false,
        psd: true,
        db: true,
    },
    EVERYTHING,
];

/// One plot to lay out and check: what its axes span, which way time runs, and
/// how wide a decibel window the colour bar was asked for.
struct Case {
    time: (f64, f64),
    frequency: (f64, f64),
    decibels: (f64, f64),
    orientation: Orientation,
}

impl Case {
    fn lay_out(&self, width: u32, height: u32, panels: Panels) -> Layout {
        let gutters = Gutters::measure(self.time, self.frequency, self.decibels);
        Layout::compute(width, height, panels, self.orientation, gutters)
    }
}

const fn case(
    time: (f64, f64),
    frequency: (f64, f64),
    decibels: (f64, f64),
    orientation: Orientation,
) -> Case {
    Case {
        time,
        frequency,
        decibels,
        orientation,
    }
}

/// Extents worth checking every panel set against, in both orientations.
const CASES: [Case; 10] = [
    // Half an hour of complex baseband.
    case((0.0, 1800.0), (-12_000.0, 12_000.0), DECIBELS, Orientation::Horizontal),
    case((0.0, 1800.0), (-12_000.0, 12_000.0), DECIBELS, Orientation::Vertical),
    // Five seconds of a real capture.
    case((0.0, 5.0), (0.0, 6_300.0), DECIBELS, Orientation::Horizontal),
    case((0.0, 5.0), (0.0, 6_300.0), DECIBELS, Orientation::Vertical),
    // Over an hour, tuned to HF.
    case((0.0, 4_320.0), (12_567_000.0, 12_591_000.0), DECIBELS, Orientation::Horizontal),
    case((0.0, 4_320.0), (12_567_000.0, 12_591_000.0), DECIBELS, Orientation::Vertical),
    // A subsecond window an hour into the recording.
    case((3_600.0, 3_600.2), (-8_000.0, 8_000.0), DECIBELS, Orientation::Horizontal),
    case((3_600.0, 3_600.2), (-8_000.0, 8_000.0), DECIBELS, Orientation::Vertical),
    // A decibel window far wider than the default, which widens the colour
    // bar's labels and narrows everything else.
    case((0.0, 300.0), (-12_000.0, 12_000.0), (-10_000.0, 0.0), Orientation::Horizontal),
    case((0.0, 300.0), (-12_000.0, 12_000.0), (-10_000.0, 0.0), Orientation::Vertical),
];

#[test]
fn no_two_labels_overlap_at_any_supported_size() {
    let ruler = Ruler::new();
    let mut checked = 0;
    // From the smallest image that still leaves a plot up to the default.
    for (w, h) in [
        (240, 120),
        (320, 160),
        (560, 300),
        (900, 320),
        (1600, 500),
        (2048, 512),
        (700, 900),
    ] {
        for panels in PANEL_SETS {
            for case in CASES {
                let layout = case.lay_out(w, h, panels);
                let what = format!("{w}x{h} {panels} {} {:?}", layout.orientation, case.time);
                checked += usize::from(ruler.check_layout(
                    &layout,
                    (case.time, case.frequency),
                    &what,
                ));
            }
        }
    }

    // A layout too small for a plot has nothing to measure and is skipped, so
    // the count is asserted: without it a change that stopped producing plots
    // would leave this test passing on an empty matrix.
    let combinations = 7 * PANEL_SETS.len() * CASES.len();
    assert!(
        checked > combinations * 3 / 4,
        "only {checked} of {combinations} layouts had a plot to check"
    );
}

#[test]
fn a_wider_image_gets_more_time_labels() {
    let ruler = Ruler::new();
    let (time, frequency) = ((0.0, 1800.0), (-12_000.0, 12_000.0));
    let img = extents(time, frequency);
    let counts: Vec<usize> = [600, 1000, 1600, 2048]
        .into_iter()
        .map(|w| {
            let gutters = Gutters::measure(time, frequency, DECIBELS);
            let layout = Layout::compute(w, 400, Panels::NONE, Orientation::Horizontal, gutters);
            time_ticks(&layout, &img, &ruler.text, ruler.rise).len()
        })
        .collect();
    // The clock ladder is coarse -- a half-hour axis steps from a mark a
    // minute to one every thirty seconds and nothing between -- so density
    // climbs in stairs rather than continuously, but it only ever climbs.
    assert!(
        counts.windows(2).all(|c| c[1] >= c[0]),
        "a wider image lost labels: {counts:?}"
    );
    assert!(
        counts[counts.len() - 1] > counts[0] * 3,
        "the extra width bought almost nothing: {counts:?}"
    );
}

#[test]
fn a_megahertz_frequency_label_gets_a_gutter_that_holds_it() {
    let ruler = Ruler::new();
    let (time, tuned) = ((0.0, 1800.0), (12_567_000.0, 12_591_000.0));
    let gutters = Gutters::measure(time, tuned, DECIBELS);
    let layout = Layout::compute(2048, 512, Panels::NONE, Orientation::Horizontal, gutters);
    let spec = layout.spectrogram.expect("spectrogram");

    let hertz = frequency_ticks(&layout, &extents(time, tuned), &ruler.text, ruler.rise);
    assert!(hertz.iter().any(|t| t.label.ends_with(" MHz")), "{hertz:?}");
    ruler.check_stacked(&hertz, spec.x, "tuned to HF");

    // The 78-pixel gutter this replaced could not hold them, and a baseband
    // axis does not need anything like as much.
    assert!(gutters.frequency + LABEL_PAD > 78, "{gutters:?}");
    let baseband = Gutters::measure(time, (-12_000.0, 12_000.0), DECIBELS);
    assert!(
        gutters.frequency > baseband.frequency,
        "{gutters:?} against {baseband:?}"
    );
}

#[test]
fn panels_that_share_an_axis_draw_the_same_grid() {
    let dir = TempDir::new("render-shared");
    let layout = laid_out(1600, 500, EVERYTHING, Orientation::Horizontal);
    // Five seconds, so the clock has whole seconds to mark, and quiet against
    // full scale, so the strip's trace stays near its centre line and leaves
    // the grid at the top of the panel to read.
    let values = real_tone(5 * RATE as usize, RATE, TONE_HZ, 0.3);
    let a = analysis_of(&dir, "shared.wav", &values, false, &layout);
    let canvas = render(
        &layout,
        &PlotInput {
            analysis: &a,
            title: "title",
            footer: "footer",
            colormap: Colormap::Grayscale,
            waveform_full_scale: 1.0,
        },
    );

    let spec = layout.spectrogram.unwrap();
    let strip = layout.waveform.unwrap();
    let psd = layout.psd.unwrap();
    let is_grid = |x: i64, y: i64| canvas.get_pixel(x as u32, y as u32).0 == Theme::GRID.0;
    let columns = |rect: Rect, y: i64| -> Vec<i64> {
        (rect.x..rect.right()).filter(|x| is_grid(*x, y)).collect()
    };

    // Time: one grid column per accepted label, and the strip carries the
    // same columns and no others.
    let ruler = Ruler::new();
    let clock = time_ticks(&layout, &a.spectrogram, &ruler.text, ruler.rise);
    let labelled: Vec<i64> = clock.iter().map(|t| spec.x + t.offset).collect();
    assert!(labelled.len() > 4, "{labelled:?}");
    let on_spectrogram = columns(spec, spec.y + 2);
    assert_eq!(
        on_spectrogram, labelled,
        "a grid line without a label, or a label without one"
    );
    assert_eq!(
        columns(strip, strip.y + 2),
        on_spectrogram,
        "the strip's time grid drifted off the spectrogram's"
    );

    // Frequency: the spectrum panel carries the same rows, at the same values.
    let hertz = frequency_ticks(&layout, &a.spectrogram, &ruler.text, ruler.rise);
    let mut expected: Vec<i64> = hertz.iter().map(|t| spec.bottom() - 1 - t.offset).collect();
    expected.sort_unstable();
    assert!(expected.len() > 4, "{expected:?}");
    let on_psd: Vec<i64> = (psd.y..psd.bottom()).filter(|y| is_grid(psd.x, *y)).collect();
    assert_eq!(on_psd, expected, "the spectrum panel's grid drifted");
}

#[test]
fn the_colour_bar_labels_its_own_scale_and_stays_on_the_canvas() {
    let dir = TempDir::new("render-colorbar");
    let layout = laid_out(900, 700, EVERYTHING, Orientation::Horizontal);
    let a = analysis(&dir, "bar.wav", true, &layout);
    let canvas = plot(&layout, &a);
    let bar = layout.colorbar.expect("colorbar");

    let ruler = Ruler::new();
    let axis = Axis {
        length: bar.h,
        min: f64::from(a.spectrogram.db_min),
        max: f64::from(a.spectrogram.db_max),
        lead: ruler.rise,
        trail: ruler.rise,
    };
    let marks = ticks::ticks(
        AxisKind::DecibelsWithUnit,
        axis,
        &LabelMetrics::new(&ruler.text, FONT_SIZE, LabelRun::Down),
    );
    // The fixed five-tick target this replaced gave the same count whatever
    // the height was; a 700-pixel image has room for a good many more.
    assert!(marks.len() > 8, "{marks:?}");

    for tick in &marks {
        assert!(tick.label.ends_with(" dB"), "{:?}", tick.label);
        let right = (bar.right() + LABEL_PAD) as f32 + ruler.text.width(&tick.label, FONT_SIZE);
        assert!(
            right <= canvas.width() as f32,
            "{:?} ends at {right}, past the canvas",
            tick.label
        );
    }
}

#[test]
fn the_colour_bar_gutter_follows_the_reference_it_was_given() {
    // Under `--ref fs` the window's top is 0 dBFS, so `-d 60` cannot print
    // wider than `-60 dB`. Under `--ref peak` the top is this file's own peak,
    // which is not known yet and can sit anywhere down to the f32 floor.
    let full_scale = Gutters::measure(TIME, BASEBAND, (-60.0, 0.0));
    let peak = Gutters::measure(TIME, BASEBAND, (DB_FLOOR - 60.0, 0.0));
    assert!(
        peak.colorbar > full_scale.colorbar,
        "a peak reference needs the wider gutter: {peak:?} against {full_scale:?}"
    );

    let text = TextRenderer::new();
    let widest = ticks::widest_labels(AxisKind::DecibelsWithUnit, DB_FLOOR - 60.0, 0.0);
    for label in &widest {
        assert!(
            peak.colorbar >= text.width(label, FONT_SIZE).ceil() as i64,
            "{peak:?} does not hold {label:?}"
        );
    }
}

#[test]
fn the_spectrum_gutter_ignores_what_the_colour_bar_was_asked_for() {
    // The spectrum panel's scale follows its own trace, so opening the colour
    // bar's window must not widen the gutter beside the waterfall.
    let default = Gutters::measure(TIME, BASEBAND, (-60.0, 0.0));
    let wide = Gutters::measure(TIME, BASEBAND, (-10_000.0, 0.0));
    assert_eq!(default.decibels, wide.decibels, "{default:?} vs {wide:?}");
    assert!(wide.colorbar > default.colorbar, "{default:?} vs {wide:?}");
}

#[test]
fn a_vertical_plot_reserves_the_spectrum_gutter_only_when_it_has_one() {
    // Built rather than measured, so the rule is tested and not whichever of
    // the two labels happens to be wider today.
    let gutters = Gutters {
        time: 10,
        decibels: 40,
        ..gutters()
    };
    assert_eq!(gutters.down(Panels::NONE), 10 + LABEL_PAD);
    assert_eq!(gutters.down(WAVEFORM), 10 + LABEL_PAD);
    assert_eq!(gutters.down(PSD), 40 + LABEL_PAD);
    assert_eq!(gutters.down(EVERYTHING), 40 + LABEL_PAD);

    // Which reaches the layout: the waterfall keeps the room.
    let with = Layout::compute(700, 900, PSD, Orientation::Vertical, gutters);
    let without = Layout::compute(700, 900, Panels::NONE, Orientation::Vertical, gutters);
    let (with, without) = (with.spectrogram.unwrap(), without.spectrogram.unwrap());
    assert!(
        without.x < with.x,
        "a waterfall with no spectrum panel still paid for its labels"
    );
}
