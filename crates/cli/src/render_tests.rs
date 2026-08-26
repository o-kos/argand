use super::*;
use argand_core::{SampleRange, SampleType};
use argand_dsp::{DbReference, Reduce, StftConfig, Window, analyze};
use argand_io::OpenHints;
use argand_io::testutil::{TempDir, iq_tone, real_tone, write_wav};

const RATE: f64 = 24_000.0;
const FFT: usize = 256;
const TONE_BIN: usize = 40;
const TONE_HZ: f64 = TONE_BIN as f64 * RATE / FFT as f64;

fn analysis(dir: &TempDir, name: &str, iq: bool, w: usize, h: usize) -> Analysis {
    let st: SampleType = if iq { "iq_f32" } else { "rl_f32" }.parse().unwrap();
    let values = if iq {
        iq_tone(8192, RATE, TONE_HZ, 0.8)
    } else {
        real_tone(8192, RATE, TONE_HZ, 0.8)
    };
    let path = write_wav(&dir.join(name), st, RATE as u32, &values, 1.0);
    let mut src = argand_io::open(&path, &OpenHints::default()).unwrap();
    let range = SampleRange::new(0, src.meta().len_samples);
    analyze(
        src.as_mut(),
        &StftConfig::new(FFT, Window::Hann),
        range,
        w,
        h,
        Reduce::Max,
        Colormap::Grayscale,
        60.0,
        DbReference::Peak,
        &mut |_, _| {},
    )
    .unwrap()
}

fn plot(layout: &Layout, analysis: &Analysis) -> RgbImage {
    render(
        layout,
        &PlotInput {
            analysis,
            title: "title",
            footer: "footer",
            colormap: Colormap::Grayscale,
        },
    )
}

#[test]
fn both_mode_lays_out_three_panels_side_by_side() {
    let layout = Layout::compute(1600, 500, Mode::Both, Orientation::Horizontal);
    let spec = layout.spectrogram.expect("spectrogram");
    let psd = layout.psd.expect("psd");
    let bar = layout.colorbar.expect("colorbar");

    assert!(spec.right() < psd.x, "psd sits to the right of the waterfall");
    assert!(psd.right() <= bar.x, "colour bar is furthest right");
    assert_eq!(spec.y, psd.y, "panels share the frequency axis");
    assert_eq!(spec.h, psd.h);
    assert!(spec.w > psd.w * 2, "the waterfall gets most of the width");
    assert!(bar.right() < 1600);
}

#[test]
fn vertical_mode_stacks_the_spectrum_above_the_waterfall() {
    let layout = Layout::compute(700, 900, Mode::Both, Orientation::Vertical);
    let spec = layout.spectrogram.expect("spectrogram");
    let psd = layout.psd.expect("psd");

    assert!(psd.bottom() < spec.y, "spectrum sits above the waterfall");
    assert_eq!(spec.x, psd.x, "panels share the frequency axis");
    assert_eq!(spec.w, psd.w);
    assert!(spec.h > psd.h);
}

#[test]
fn single_panel_modes_drop_what_they_do_not_draw() {
    let only_spec = Layout::compute(800, 300, Mode::Spectrogram, Orientation::Horizontal);
    assert!(only_spec.spectrogram.is_some());
    assert!(only_spec.psd.is_none());
    assert!(only_spec.colorbar.is_some());

    let only_psd = Layout::compute(800, 300, Mode::Psd, Orientation::Horizontal);
    assert!(only_psd.spectrogram.is_none());
    assert!(only_psd.psd.is_some());
    assert!(only_psd.colorbar.is_none(), "no colours to explain");
}

#[test]
fn the_transform_is_asked_for_the_pixels_it_will_fill() {
    let horizontal = Layout::compute(1600, 500, Mode::Both, Orientation::Horizontal);
    let rect = horizontal.spectrogram.unwrap();
    assert_eq!(horizontal.transform_size(), (rect.w as usize, rect.h as usize));

    // Turning the waterfall on its side swaps which axis is time.
    let vertical = Layout::compute(700, 900, Mode::Both, Orientation::Vertical);
    let rect = vertical.spectrogram.unwrap();
    assert_eq!(vertical.transform_size(), (rect.h as usize, rect.w as usize));
}

#[test]
fn an_image_too_small_for_a_plot_yields_no_panels() {
    for (w, h) in [(1, 1), (40, 40), (200, 20)] {
        let layout = Layout::compute(w, h, Mode::Both, Orientation::Horizontal);
        assert!(
            layout.spectrogram.is_none() || layout.spectrogram.unwrap().is_valid(),
            "{w}x{h} produced a degenerate rect"
        );
    }
}

#[test]
fn rendering_fills_the_requested_canvas() {
    let dir = TempDir::new("render-size");
    let layout = Layout::compute(900, 320, Mode::Both, Orientation::Horizontal);
    let (tw, th) = layout.transform_size();
    let a = analysis(&dir, "a.wav", true, tw, th);
    let canvas = plot(&layout, &a);

    assert_eq!((canvas.width(), canvas.height()), (900, 320));
    let distinct: std::collections::HashSet<_> = canvas.pixels().map(|p| p.0).collect();
    assert!(distinct.len() > 8, "expected a drawn plot, got {distinct:?}");
}

#[test]
fn the_waterfall_lands_inside_its_panel_and_nowhere_else() {
    let dir = TempDir::new("render-bounds");
    let layout = Layout::compute(900, 320, Mode::Spectrogram, Orientation::Horizontal);
    let (tw, th) = layout.transform_size();
    let a = analysis(&dir, "b.wav", true, tw, th);
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
    let layout = Layout::compute(400, 700, Mode::Spectrogram, Orientation::Vertical);
    let (tw, th) = layout.transform_size();
    let a = analysis(&dir, "c.wav", false, tw, th);
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
fn mode_and_orientation_names_round_trip() {
    use std::str::FromStr;
    for name in MODE_NAMES {
        assert_eq!(Mode::from_str(name).unwrap().to_string(), name);
    }
    for name in ORIENTATION_NAMES {
        assert_eq!(Orientation::from_str(name).unwrap().to_string(), name);
    }
    assert_eq!(Orientation::from_str("v").unwrap(), Orientation::Vertical);
    assert_eq!(Mode::from_str("SPEC").unwrap(), Mode::Spectrogram);
    assert!(Mode::from_str("waterfall").is_err());
    assert!(Orientation::from_str("sideways").is_err());
}
