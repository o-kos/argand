use super::*;

use argand_core::testutil::DejaVuSans;

/// A window-sized panel, in logical pixels.
fn panel(width: f32, height: f32) -> Size<Pixels> {
    size(px(width), px(height))
}

/// The two-sided span of a 24 kHz I/Q capture tuned to 12.579 MHz, half an
/// hour long -- the repository's own fixture.
const HFDL: Extents = Extents {
    time: crate::time_ruler::Ruler::CLOCK,
    seconds: (0.0, 1800.0),
    hertz: (12_567_000.0, 12_591_000.0),
};

/// An unscaled display, which is what every test here but one describes.
const UNSCALED: f32 = 1.0;

fn measure(panel: Size<Pixels>, extents: Extents) -> Frame {
    Frame::measure(panel, UNSCALED, extents, &DejaVuSans, None).expect("a panel this size holds a plot")
}

#[test]
fn the_picture_gets_what_the_labels_leave() {
    let frame = measure(panel(1200.0, 800.0), HFDL);

    // The gutter is wide enough for the widest label the axis could print,
    // which at these frequencies is six digits and a point.
    let widest = DejaVuSans.width("12.591000", 11.0);
    assert!(
        1200.0 - frame.plot.right() >= widest + LABEL_PAD,
        "a {widest} pixel label does not fit in a {} pixel gutter",
        1200.0 - frame.plot.right()
    );
    assert!(frame.plot.x >= OUTER_PAD);
    assert!(frame.plot.right() < 1200.0);
    assert!(
        frame.plot.y + frame.plot.height < 800.0,
        "the time labels need a row"
    );
    assert_eq!(frame.plot.y, 0., "the image meets the minimap separator");
}

#[test]
fn both_axes_are_marked_and_every_mark_lands_inside_the_plot() {
    let frame = measure(panel(1200.0, 800.0), HFDL);

    assert!(!frame.time.is_empty(), "half an hour should be marked");
    assert!(!frame.frequency.is_empty(), "24 kHz should be marked");
    for tick in frame.time.iter().chain(&frame.frequency) {
        assert!(
            tick.offset >= 0,
            "{:?} is off the low end of its axis",
            tick
        );
    }
    for tick in &frame.time {
        assert!((tick.offset as f32) < frame.plot.width, "{tick:?}");
    }
    for tick in &frame.frequency {
        assert!((tick.offset as f32) < frame.plot.height, "{tick:?}");
    }
}

#[test]
fn a_tuned_capture_reads_in_the_unit_its_digits_need() {
    let frame = measure(panel(1200.0, 800.0), HFDL);
    assert_eq!(frame.caption, Some("MHz"));

    // Baseband: the same span with nothing added to it is kilohertz.
    let baseband = measure(
        panel(1200.0, 800.0),
        Extents { time: crate::time_ruler::Ruler::CLOCK,
            seconds: (0.0, 1800.0),
            hertz: (-12_000.0, 12_000.0),
        },
    );
    assert_eq!(baseband.caption, Some("kHz"));
}

#[test]
fn a_real_capture_is_labelled_from_zero_up_and_a_complex_one_either_side() {
    let complex = measure(panel(1200.0, 800.0), HFDL);
    let lowest: f64 = complex
        .frequency
        .first()
        .map_or(f64::NAN, |tick| tick.value);
    assert!(
        lowest < 12_579_000.0,
        "a two-sided spectrum is marked below its centre too, not from {lowest}"
    );

    let real = measure(
        panel(1200.0, 800.0),
        Extents { time: crate::time_ruler::Ruler::CLOCK,
            seconds: (0.0, 10.0),
            hertz: (0.0, 12_000.0),
        },
    );
    assert!(
        real.frequency.iter().all(|tick| tick.value >= 0.0),
        "a one-sided spectrum has no negative frequency to mark"
    );
}

#[test]
fn a_panel_with_no_room_left_for_a_picture_is_not_one() {
    // Narrower than the gutter the frequency labels need, and shorter than the
    // two label rows: there is no rectangle to draw into.
    assert!(Frame::measure(panel(20.0, 400.0), UNSCALED, HFDL, &DejaVuSans, None).is_none());
    assert!(Frame::measure(panel(1200.0, 8.0), UNSCALED, HFDL, &DejaVuSans, None).is_none());
}

#[test]
fn a_gutter_reserved_from_zeros_holds_whatever_digits_turn_up_in_it() {
    // The bound the gutter is built from stands in for a number nobody has
    // picked yet, and every label the axis then chooses has to fit inside it.
    let frame = measure(panel(1600.0, 900.0), HFDL);
    for tick in &frame.frequency {
        let width = DejaVuSans.width(&tick.label, 11.0);
        assert!(
            width + LABEL_PAD <= 1600.0 - frame.plot.right(),
            "{:?} measures {width} in a {} pixel gutter",
            tick.label,
            1600.0 - frame.plot.right()
        );
    }
}

#[test]
fn the_plot_lands_on_whole_device_pixels_so_the_picture_is_not_resampled() {
    // A fractional panel on a 1.5x display, which is what a tiled window on a
    // scaled desktop hands over.
    let scale = 1.5;
    let frame = Frame::measure(panel(1237.0, 803.5), scale, HFDL, &DejaVuSans, None)
        .expect("a panel this size holds a plot");

    for (name, edge) in [
        ("left", frame.plot.x),
        ("top", frame.plot.y),
        ("right", frame.plot.x + frame.plot.width),
        ("bottom", frame.plot.y + frame.plot.height),
    ] {
        let device = edge * scale;
        assert!(
            (device - device.round()).abs() < 1e-3,
            "the {name} edge is at device pixel {device}"
        );
    }
}

#[test]
fn time_labels_follow_ticks_without_overlapping_or_entering_the_right_gutter() {
    for width in [100.0, 180.0, 640.0, 1200.0] {
        for seconds in [(0.0, 30.456), (590.0, 650.0), (0.0, 4350.0)] {
            let frame = measure(panel(width, 400.0), Extents { seconds, ..HFDL });
            assert_time_labels_fit(&frame);
            assert!(width < 180.0 || !frame.time.is_empty());
            if width >= 180.0 && seconds.0 == 0.0 {
                assert_eq!(frame.time[0].offset, 0, "the first label needs no left gutter");
            }
        }
    }
}

fn assert_time_labels_fit(frame: &Frame) {
    for tick in &frame.time {
        let start = frame.plot.x + tick.offset as f32 + LABEL_PAD;
        let end = start + DejaVuSans.width(&tick.label, LABEL_SIZE);
        assert!(start > frame.plot.x + tick.offset as f32);
        assert!(end <= frame.plot.right(), "{tick:?}");
    }
    for pair in frame.time.windows(2) {
        let clear = (pair[1].offset - pair[0].offset) as f32
            - DejaVuSans.width(&pair[0].label, LABEL_SIZE);
        assert!(clear >= DejaVuSans.width("00", LABEL_SIZE));
    }
}

struct WideDigits;

impl LabelMeasure for WideDigits {
    fn width(&self, text: &str, size: f32) -> f32 {
        DejaVuSans.width(text, size) * 1.04
    }

    fn digit_height(&self, size: f32) -> f32 {
        DejaVuSans.digit_height(size)
    }
}

#[test]
fn fractional_dpi_keeps_complete_frequency_labels_inside_the_panel() {
    let extents = Extents { time: crate::time_ruler::Ruler::CLOCK,
        seconds: (0.0, 30.0),
        hertz: (5_000_001.0, 5_000_004.0),
    };
    // Slightly wider figures leave little slack below the next whole pixel.
    for (width, scale) in [(640.0, 1.25), (640.0, 1.5), (619.4, 1.75), (300.2, 2.0)] {
        let frame = Frame::measure(panel(width, 400.0), scale, extents, &WideDigits, None)
            .expect("the panel holds a plot");
        assert!(!frame.frequency.is_empty());
        for tick in &frame.frequency {
            let end = frame.plot.right() + LABEL_PAD + WideDigits.width(&tick.label, LABEL_SIZE);
            assert!(end <= width, "{} ends at {end} beyond {width}px at {scale}x", tick.label);
        }
    }
}

#[test]
fn axis_labels_clear_adjacent_panels_and_the_window_edges() {
    for scale in [1.0, 1.25, 1.5, 2.0] {
        let extents = Extents { time: crate::time_ruler::Ruler::CLOCK, seconds: (0.0, 30.456), hertz: (-12_000.0, 12_000.0) };
        let frame = Frame::measure(panel(300.0, 240.0), scale, extents, &DejaVuSans, None)
            .expect("the panel holds a plot and its labels");
        assert_axis_bands_fit(&frame, 300.0, 240.0);
    }
}

fn assert_axis_bands_fit(frame: &Frame, width: f32, height: f32) {
    let half_line = LINE_HEIGHT / 2.0;
    assert_eq!(frame.plot.x, 4.0);
    assert_eq!(frame.plot.y, 0.0, "the image has no gap below the minimap");
    assert!(frame.caption_row - half_line >= -48., "caption fits beside the minimap");
    assert_eq!(frame.caption_row + DejaVuSans.digit_height(LABEL_SIZE) / 2., frame.plot.y);
    assert!(frame.time_row - half_line > frame.plot.bottom());
    assert!(frame.time_row + half_line <= height - 4.0, "time labels touch the status bar");
    let half_ink = DejaVuSans.digit_height(LABEL_SIZE) / 2.0;
    assert!(!frame.frequency.is_empty());
    for tick in &frame.frequency {
        let center = frame.plot.bottom() - tick.offset as f32 + 0.5;
        assert!(center - half_ink >= frame.plot.y, "frequency label stays inside the plot");
        assert!(center + half_ink <= frame.plot.bottom(), "frequency label enters the time row");
        let right = frame.plot.right() + LABEL_PAD + DejaVuSans.width(&tick.label, LABEL_SIZE);
        assert!(right <= width - 4.0);
    }
}

#[test]
fn time_modes_preserve_plot_geometry_and_sample_origin() {
    use crate::{navigation::View, time_ruler::{Mode, Ruler}};
    let view = View { start: 2_400_000, len: 24_000 };
    for width in [180., 640., 1200.] {
        let baseline = measure(panel(width, 400.), HFDL);
        for mode in [Mode::Clock, Mode::Seconds, Mode::Samples] {
            let extents = Extents {
                time: Ruler { mode, view, total: 43_200_000 },
                seconds: view.seconds(24_000.),
                ..HFDL
            };
            let frame = measure(panel(width, 400.), extents);
            assert_eq!(frame.plot, baseline.plot);
            assert_eq!(frame.frequency, baseline.frequency);
            assert_time_labels_fit(&frame);
            let bounds = extents.time.bounds(extents.seconds);
            for tick in &frame.time {
                assert!((bounds.0..=bounds.1).contains(&tick.value));
                assert_eq!(tick.label.starts_with('#'), mode == Mode::Samples);
            }
        }
    }
}

#[test]
fn localized_time_labels_fit_and_units_do_not_resize_the_plot() {
    struct Localized(crate::numbers::Numbers);
    impl LabelMeasure for Localized {
        fn localize(&self, text: &str, kind: AxisKind) -> String { self.0.axis_label(text, kind) }
        fn width(&self, text: &str, size: f32) -> f32 {
            DejaVuSans.width(&text.replace(['\u{a0}', '\u{202f}'], " "), size)
        }
        fn digit_height(&self, size: f32) -> f32 { DejaVuSans.digit_height(size) }
    }
    for locale in ["en-US", "ru-RU", "de-DE", "hi-IN"] {
        let labels = Localized(crate::numbers::Numbers::new(locale));
        let mut previous_plot = None;
        for mode in [crate::time_ruler::Mode::Clock, crate::time_ruler::Mode::Seconds, crate::time_ruler::Mode::Samples] {
            let extents = Extents {
                time: crate::time_ruler::Ruler { mode, view: crate::navigation::View { start: 1_234_000, len: 20_000 }, total: 2_000_000 },
                seconds: (1234., 1254.), hertz: (-100., 100.),
            };
            let frame = Frame::measure(panel(700., 400.), 1., extents, &labels, None).unwrap();
            if let Some(plot) = previous_plot { assert_eq!(frame.plot, plot); }
            previous_plot = Some(frame.plot);
            assert_eq!(frame.time_caption, mode.caption());
            assert!(!frame.time.is_empty());
            let mut end = 0.;
            for tick in frame.time {
                assert!(!tick.label.contains(['#', 's']));
                let left = tick.offset as f32 + LABEL_PAD;
                assert!(left >= end);
                end = left + labels.width(&tick.label, LABEL_SIZE);
                assert!(end <= frame.plot.width);
            }
        }
    }
}

#[test]
fn unit_hints_follow_visible_captions_and_stay_in_the_gutter() {
    use crate::time_ruler::Mode;
    for scale in [1., 2.] {
        for mode in [Mode::Clock, Mode::Seconds, Mode::Samples] {
            for (hertz, text) in [
                ((0., 100.), "Frequency in Hz"),
                ((0., 5000.), "Frequency in kHz"),
                ((12e6, 13e6), "Frequency in MHz"),
                ((1e9, 2e9), "Frequency in GHz"),
            ] {
                let mut extents = HFDL;
                extents.hertz = hertz;
                extents.time.mode = mode;
                let mut frame = Frame::measure(panel(800., 600.), scale, extents, &DejaVuSans, None)
                    .expect("plot fits");
                let [time, frequency] = frame.unit_hints(&DejaVuSans).map(Option::unwrap);
                assert_eq!(frequency.text, text);
                assert_eq!(time.bounds.width, DejaVuSans.width(mode.caption(), LABEL_SIZE).ceil());
                assert_unit_hint_bounds(&frame, time);
                assert_unit_hint_bounds(&frame, frequency);
                assert!(time.bounds.y > frame.plot.bottom());
                assert_eq!(frequency.bounds.y + frequency.bounds.height / 2.
                    + DejaVuSans.digit_height(LABEL_SIZE) / 2., frame.plot.y);
                frame.frequency.clear();
                assert!(frame.unit_hints(&DejaVuSans)[1].is_none());
            }
        }
    }
}

fn assert_unit_hint_bounds(frame: &Frame, hint: UnitHint) {
    assert!(hint.bounds.x > frame.plot.right());
    assert!(hint.bounds.right() <= 800. - OUTER_PAD);
    assert!(hint.bounds.y >= -48.);
    assert!(hint.bounds.bottom() <= 600. - OUTER_PAD);
}

#[test]
fn unit_resolution_uses_device_pixels_current_view_and_caption_units() {
    use crate::time_ruler::Mode;
    let mut extents = HFDL;
    extents.seconds = (10., 12.);
    extents.hertz = (-12_000., 12_000.);
    extents.time.view = crate::navigation::View { start: 10_000, len: 48_000 };
    for mode in [Mode::Clock, Mode::Seconds, Mode::Samples] {
        extents.time.mode = mode;
        let frame = Frame::measure(panel(800., 600.), 2., extents, &DejaVuSans, None).unwrap();
        let [time, frequency] = frame.unit_hints(&DejaVuSans).map(Option::unwrap);
        let span = if mode == Mode::Samples { 48_000. } else { 2. };
        assert!((time.per_pixel * f64::from((frame.plot.width * 2.).round()) - span).abs() < 1e-8);
        assert!((frequency.per_pixel * f64::from((frame.plot.height * 2.).round()) - 24.).abs() < 1e-8);
        assert_eq!(frequency.units, "kHz");
        assert_eq!(time.units, if mode == Mode::Samples { "samples" } else { "s" });
        assert_eq!(frequency.bounds.y + frequency.bounds.height / 2., frame.caption_row);
        assert_eq!(frame.plot.y, 0.);
        let tiny = UnitHint { per_pixel: 1e-12, ..time };
        assert!(tiny.resolution().contains("1.000e-12"));
    }
}
