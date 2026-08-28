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
const EVERYTHING: Panels = Panels {
    waveform: true,
    psd: true,
    db: true,
};

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
        let layout = Layout::compute(900, 500, WAVEFORM, orientation);
        assert!(layout.spectrogram.is_some(), "{orientation}");
        assert!(layout.waveform.is_some(), "{orientation}");
        assert!(layout.psd.is_none(), "{orientation}: psd is opt-in");
        assert!(layout.colorbar.is_none(), "{orientation}: db is opt-in");
    }
}

#[test]
fn the_spectrogram_is_drawn_whatever_the_panels_say() {
    for panels in [Panels::NONE, WAVEFORM, EVERYTHING] {
        let layout = Layout::compute(900, 500, panels, Orientation::Horizontal);
        assert!(layout.spectrogram.is_some(), "{panels}");
    }
}

#[test]
fn a_horizontal_strip_sits_above_the_spectrogram_on_the_same_time_axis() {
    let layout = Layout::compute(1600, 500, EVERYTHING, Orientation::Horizontal);
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
    let layout = Layout::compute(700, 900, EVERYTHING, Orientation::Vertical);
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
    let layout = Layout::compute(1600, 500, EVERYTHING, Orientation::Horizontal);
    let spec = layout.spectrogram.unwrap();
    let psd = layout.psd.expect("psd");
    let bar = layout.colorbar.expect("colorbar");
    assert!(spec.right() < psd.x);
    assert!(psd.right() <= bar.x, "the colour bar is furthest right");
    assert_eq!((psd.y, psd.h), (spec.y, spec.h));
    assert!(spec.w > psd.w * 2, "the waterfall keeps most of the width");

    // Vertical: a row above the waterfall, column for column with it.
    let layout = Layout::compute(700, 900, EVERYTHING, Orientation::Vertical);
    let spec = layout.spectrogram.unwrap();
    let psd = layout.psd.expect("psd");
    assert!(psd.bottom() < spec.y);
    assert_eq!((psd.x, psd.w), (spec.x, spec.w));
    assert!(spec.h > psd.h);
}

#[test]
fn dropping_a_panel_gives_its_room_to_the_spectrogram() {
    let all = Layout::compute(1600, 500, EVERYTHING, Orientation::Horizontal);
    let bare = Layout::compute(1600, 500, Panels::NONE, Orientation::Horizontal);

    let (all_spec, bare_spec) = (all.spectrogram.unwrap(), bare.spectrogram.unwrap());
    assert!(bare_spec.w > all_spec.w, "the spectrum column came back");
    assert!(bare_spec.h > all_spec.h, "so did the strip's height");
    assert!(bare.waveform.is_none() && bare.psd.is_none() && bare.colorbar.is_none());
}

#[test]
fn the_transform_is_asked_for_the_pixels_it_will_fill() {
    let horizontal = Layout::compute(1600, 500, EVERYTHING, Orientation::Horizontal);
    let rect = horizontal.spectrogram.unwrap();
    assert_eq!(horizontal.transform_size(), (rect.w as usize, rect.h as usize));

    // Turning the waterfall on its side swaps which axis is time.
    let vertical = Layout::compute(700, 900, EVERYTHING, Orientation::Vertical);
    let rect = vertical.spectrogram.unwrap();
    assert_eq!(vertical.transform_size(), (rect.h as usize, rect.w as usize));
}

#[test]
fn an_image_too_small_for_a_plot_yields_no_panels() {
    for (w, h) in [(1, 1), (40, 40), (200, 20)] {
        let layout = Layout::compute(w, h, EVERYTHING, Orientation::Horizontal);
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
    let layout = Layout::compute(900, 320, EVERYTHING, Orientation::Horizontal);
    let a = analysis(&dir, "a.wav", true, &layout);
    let canvas = plot(&layout, &a);

    assert_eq!((canvas.width(), canvas.height()), (900, 320));
    let distinct: std::collections::HashSet<_> = canvas.pixels().map(|p| p.0).collect();
    assert!(distinct.len() > 8, "expected a drawn plot, got {distinct:?}");
}

#[test]
fn the_waterfall_lands_inside_its_panel_and_nowhere_else() {
    let dir = TempDir::new("render-bounds");
    let layout = Layout::compute(900, 320, Panels::NONE, Orientation::Horizontal);
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
    let layout = Layout::compute(400, 700, Panels::NONE, Orientation::Vertical);
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
    let layout = Layout::compute(900, 320, WAVEFORM, Orientation::Horizontal);
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
    let layout = Layout::compute(900, 320, WAVEFORM, Orientation::Horizontal);
    let a = analysis(&dir, "real.wav", false, &layout);
    assert_eq!(a.waveform.as_ref().unwrap().channels, 1);

    let canvas = plot(&layout, &a);
    let strip = layout.waveform.unwrap();
    assert!(trace_colors(&canvas, strip).1 > 200, "the trace is missing");

    // The gutter beside the strip carries axis labels and nothing else.
    let gutter = Rect {
        x: strip.x - FREQ_LABEL_W,
        w: FREQ_LABEL_W - 2,
        ..strip
    };
    assert_eq!(trace_colors(&canvas, gutter), (0, 0), "the trace leaked out");
}

#[test]
fn the_strip_reaches_its_edge_at_the_reference_level() {
    let dir = TempDir::new("render-scale");
    let layout = Layout::compute(900, 320, WAVEFORM, Orientation::Horizontal);
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
fn decimal_ticks_land_on_round_numbers() {
    let ticks = nice_ticks(-12_000.0, 12_000.0, 6);
    assert!(ticks.contains(&0.0), "{ticks:?}");
    assert!(ticks.iter().all(|v| (v / 5000.0).fract().abs() < 1e-9), "{ticks:?}");
    assert!(ticks.len() >= 4 && ticks.len() <= 12, "{ticks:?}");

    assert!(nice_ticks(5.0, 5.0, 6).is_empty(), "degenerate span");
    assert!(nice_ticks(f64::NAN, 1.0, 6).is_empty());
    assert!(nice_ticks(0.0, 1.0, 0).is_empty());
}

#[test]
fn time_ticks_use_clock_steps_not_decimal_ones() {
    // Half an hour: decimal steps would mark every 250 seconds ("8m20").
    let ticks = nice_time_ticks(0.0, 1800.0, 8);
    assert!(ticks.len() >= 4, "{ticks:?}");
    for value in &ticks {
        assert_eq!(
            value % 300.0,
            0.0,
            "{value} is not a round number of minutes: {ticks:?}"
        );
    }
    assert!(ticks.contains(&0.0));
}

#[test]
fn short_spans_fall_back_to_decimal_time_steps() {
    let ticks = nice_time_ticks(0.0, 0.2, 4);
    assert!(ticks.len() >= 3, "{ticks:?}");
    assert!(ticks.iter().all(|v| *v >= 0.0 && *v <= 0.2));
}

#[test]
fn the_zero_tick_is_bare() {
    assert_eq!(time_label(0.0), "0");
    assert_eq!(time_label(300.0), "5m");
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
