use super::*;

use argand_core::testutil::DejaVuSans;

/// A window-sized panel, in logical pixels.
fn panel(width: f32, height: f32) -> Size<Pixels> {
    size(px(width), px(height))
}

/// The two-sided span of a 24 kHz I/Q capture tuned to 12.579 MHz, half an
/// hour long -- the repository's own fixture.
const HFDL: Extents = Extents {
    seconds: (0.0, 1800.0),
    hertz: (12_567_000.0, 12_591_000.0),
};

/// An unscaled display, which is what every test here but one describes.
const UNSCALED: f32 = 1.0;

fn measure(panel: Size<Pixels>, extents: Extents) -> Frame {
    Frame::measure(panel, UNSCALED, extents, &DejaVuSans).expect("a panel this size holds a plot")
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
    assert!(frame.plot.y > 0.0, "the image clears the waveform panel");
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
        Extents {
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
        Extents {
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
    assert!(Frame::measure(panel(20.0, 400.0), UNSCALED, HFDL, &DejaVuSans).is_none());
    assert!(Frame::measure(panel(1200.0, 8.0), UNSCALED, HFDL, &DejaVuSans).is_none());
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
    let frame = Frame::measure(panel(1237.0, 803.5), scale, HFDL, &DejaVuSans)
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
    let extents = Extents {
        seconds: (0.0, 30.0),
        hertz: (5_000_001.0, 5_000_004.0),
    };
    // Slightly wider figures leave little slack below the next whole pixel.
    for (width, scale) in [(640.0, 1.25), (640.0, 1.5), (619.4, 1.75), (300.2, 2.0)] {
        let frame = Frame::measure(panel(width, 400.0), scale, extents, &WideDigits)
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
        let extents = Extents { seconds: (0.0, 30.456), hertz: (-12_000.0, 12_000.0) };
        let frame = Frame::measure(panel(300.0, 240.0), scale, extents, &DejaVuSans)
            .expect("the panel holds a plot and its labels");
        assert_axis_bands_fit(&frame, 300.0, 240.0);
    }
}

fn assert_axis_bands_fit(frame: &Frame, width: f32, height: f32) {
    let half_line = LINE_HEIGHT / 2.0;
    assert_eq!(frame.plot.x, 4.0);
    assert_eq!(frame.plot.y, 4.0, "the unit must not reserve a band above the image");
    assert!(frame.caption_row - half_line >= frame.plot.y, "caption touches the waveform panel");
    assert!(frame.caption_row + half_line < frame.plot.bottom());
    assert!(frame.time_row - half_line > frame.plot.bottom());
    assert!(frame.time_row + half_line <= height - 4.0, "time labels touch the status bar");
    let half_ink = DejaVuSans.digit_height(LABEL_SIZE) / 2.0;
    assert!(!frame.frequency.is_empty());
    for tick in &frame.frequency {
        let center = frame.plot.bottom() - tick.offset as f32 + 0.5;
        assert!(center - half_ink >= frame.caption_row + half_line + LABEL_PAD,
            "frequency label collides with the unit");
        assert!(center + half_ink <= frame.plot.bottom(), "frequency label enters the time row");
        let right = frame.plot.right() + LABEL_PAD + DejaVuSans.width(&tick.label, LABEL_SIZE);
        assert!(right <= width - 4.0);
    }
}
