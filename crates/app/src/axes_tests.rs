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

fn measure(panel: Size<Pixels>, extents: Extents) -> Frame {
    Frame::measure(panel, extents, &DejaVuSans).expect("a panel this size holds a plot")
}

#[test]
fn the_picture_gets_what_the_labels_leave() {
    let frame = measure(panel(1200.0, 800.0), HFDL);

    // The gutter is wide enough for the widest label the axis could print,
    // which at these frequencies is six digits and a point.
    let widest = DejaVuSans.width("12.591000", 11.0);
    assert!(
        frame.plot.x >= widest,
        "a {widest} pixel label does not fit in a {} pixel gutter",
        frame.plot.x
    );
    assert!(frame.plot.x + frame.plot.width <= 1200.0);
    assert!(
        frame.plot.y + frame.plot.height < 800.0,
        "the time labels need a row"
    );
    assert!(frame.plot.y > 0.0, "the unit needs a row above the axis");
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
    assert!(Frame::measure(panel(20.0, 400.0), HFDL, &DejaVuSans).is_none());
    assert!(Frame::measure(panel(1200.0, 8.0), HFDL, &DejaVuSans).is_none());
}

#[test]
fn a_gutter_reserved_from_zeros_holds_whatever_digits_turn_up_in_it() {
    // The bound the gutter is built from stands in for a number nobody has
    // picked yet, and every label the axis then chooses has to fit inside it.
    let frame = measure(panel(1600.0, 900.0), HFDL);
    for tick in &frame.frequency {
        let width = DejaVuSans.width(&tick.label, 11.0);
        assert!(
            width <= frame.plot.x,
            "{:?} measures {width} in a {} pixel gutter",
            tick.label,
            frame.plot.x
        );
    }
}
