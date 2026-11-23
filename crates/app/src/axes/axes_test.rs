use super::*;

use argand_core::testutil::DejaVuSans;

/// A window-sized panel, in logical pixels.
fn panel(width: f32, height: f32) -> Size<Pixels> {
    size(px(width), px(height))
}

/// The two-sided span of a 24 kHz I/Q capture tuned to 12.579 MHz, half an
/// hour long -- the repository's own fixture.
const HFDL: Extents = Extents {
    frequency_unit: argand_core::frequency::FrequencyUnit::MHz,
    frequency_offset: 0.,
    orientation: crate::navigation::orientation::Mode::Horizontal,
    time: crate::navigation::time_ruler::Ruler::CLOCK,
    seconds: (0.0, 1800.0),
    hertz: (12_567_000.0, 12_591_000.0),
};

/// An unscaled display, which is what every test here but one describes.
const UNSCALED: f32 = 1.0;

fn measure(panel: Size<Pixels>, extents: Extents) -> Frame {
    Frame::measure(panel, UNSCALED, extents, &DejaVuSans, None)
        .expect("a panel this size holds a plot")
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
            frequency_unit: argand_core::frequency::FrequencyUnit::MHz,
            frequency_offset: 0.,
            orientation: crate::navigation::orientation::Mode::Horizontal,
            time: crate::navigation::time_ruler::Ruler::CLOCK,
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
                assert_eq!(
                    frame.time[0].offset, 0,
                    "the first label needs no left gutter"
                );
            }
        }
    }
}

fn assert_time_labels_fit(frame: &Frame) {
    let visible: Vec<_> = frame
        .time
        .iter()
        .filter(|tick| !tick.label.is_empty())
        .collect();
    for tick in &visible {
        let start = frame.plot.x + tick.offset as f32 + BOTTOM_LABEL_START;
        let end = start + DejaVuSans.width(&tick.label, LABEL_SIZE);
        assert!(start > frame.plot.x + tick.offset as f32);
        assert!(end <= frame.plot.right(), "{tick:?}");
    }
    for pair in visible.windows(2) {
        let clear =
            (pair[1].offset - pair[0].offset) as f32 - DejaVuSans.width(&pair[0].label, LABEL_SIZE);
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
        frequency_unit: argand_core::frequency::FrequencyUnit::MHz,
        frequency_offset: 0.,
        orientation: crate::navigation::orientation::Mode::Horizontal,
        time: crate::navigation::time_ruler::Ruler::CLOCK,
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
            assert!(
                end <= width,
                "{} ends at {end} beyond {width}px at {scale}x",
                tick.label
            );
        }
    }
}

#[test]
fn unit_resolution_uses_device_pixels_current_view_and_caption_units() {
    use crate::navigation::time_ruler::Mode;
    let mut extents = HFDL;
    extents.seconds = (10., 12.);
    extents.hertz = (-12_000., 12_000.);
    extents.frequency_unit = argand_core::frequency::FrequencyUnit::KHz;
    extents.time.view = crate::navigation::View {
        start: 10_000,
        len: 48_000,
    };
    for mode in [Mode::Clock, Mode::Seconds, Mode::Samples] {
        extents.time.mode = mode;
        let frame = Frame::measure(panel(800., 600.), 2., extents, &DejaVuSans, None).unwrap();
        let [time, frequency] = frame.unit_hints(&DejaVuSans).map(Option::unwrap);
        let span = if mode == Mode::Samples { 48_000. } else { 2. };
        assert!((time.per_pixel * f64::from((frame.plot.width * 2.).round()) - span).abs() < 1e-8);
        assert!(
            (frequency.per_pixel * f64::from((frame.plot.height * 2.).round()) - 24.).abs() < 1e-8
        );
        assert_eq!(frequency.units, "kHz");
        assert_eq!(
            time.units,
            if mode == Mode::Samples {
                "samples"
            } else {
                "s"
            }
        );
        assert_eq!(
            frequency.bounds.y + frequency.bounds.height / 2.,
            frame.caption_row
        );
        assert_eq!(frame.plot.y, 0.);
        let tiny = UnitHint {
            per_pixel: 1e-12,
            ..time
        };
        assert!(tiny.resolution().contains("1.000e-12"));
    }
}

#[test]
fn vertical_rulers_fit_labels_and_report_resolution_on_the_corresponding_axis() {
    use crate::navigation::orientation::Mode;
    for mode in [
        crate::navigation::time_ruler::Mode::Clock,
        crate::navigation::time_ruler::Mode::Seconds,
        crate::navigation::time_ruler::Mode::Samples,
    ] {
        for scale in [1., 1.25, 2.] {
            let extents = Extents {
                frequency_unit: argand_core::frequency::FrequencyUnit::MHz,
                orientation: Mode::Vertical,
                time: crate::navigation::time_ruler::Ruler {
                    mode,
                    view: crate::navigation::View {
                        start: 0,
                        len: 94_080,
                    },
                    total: 94_080,
                },
                ..HFDL
            };
            let frame =
                Frame::measure(panel(800., 600.), scale, extents, &DejaVuSans, None).unwrap();
            assert_eq!(frame.plot.x, 0.);
            assert_eq!(frame.plot.y, 0., "vertical spectrum meets the panel top");
            for tick in frame.time.iter().filter(|tick| !tick.label.is_empty()) {
                assert!(
                    frame.plot.right() + LABEL_PAD + DejaVuSans.width(&tick.label, LABEL_SIZE)
                        <= 796.
                );
            }
            for tick in frame.frequency.iter().filter(|tick| !tick.label.is_empty()) {
                let start = tick.offset as f32 + BOTTOM_LABEL_START;
                assert!(start + DejaVuSans.width(&tick.label, LABEL_SIZE) <= frame.plot.width);
            }
            let hints = frame.unit_hints(&DejaVuSans);
            assert_eq!(
                hints[0].unwrap().per_pixel,
                (extents.time.bounds(extents.seconds).1 - extents.time.bounds(extents.seconds).0)
                    / (frame.plot.height * scale).round() as f64
            );
            assert_eq!(
                hints[1].unwrap().per_pixel,
                (extents.hertz.1 - extents.hertz.0)
                    / (frame.plot.width * scale).round() as f64
                    / 1e6
            );
            let time_unit = hints[0].unwrap().bounds;
            let frequency_unit = hints[1].unwrap().bounds;
            assert_eq!(time_unit.y, 0.);
            assert!(time_unit.x > frame.plot.right() + TICK_LEN);
            assert!(frequency_unit.x > frame.plot.right());
            let unit_ink = frequency_unit.y + frequency_unit.height / 2.
                - DejaVuSans.digit_height(LABEL_SIZE) / 2.;
            assert!(unit_ink >= frame.plot.bottom() + 1. + BOTTOM_LABEL_DROP - 1e-4);
            assert!(frequency_unit.x + frequency_unit.width <= 800.);
            assert_vertical_time_clearance(&frame);
        }
    }
}

#[test]
fn frequency_resolution_keeps_the_common_unit_and_auxiliary_precision() {
    let hint = measure(panel(800., 600.), HFDL).unit_hints(&DejaVuSans)[1].unwrap();
    for (units, value, expected) in [
        ("Hz", 1000., "1,000 Hz"),
        ("kHz", 0.000496, "0.000496 kHz"),
        ("MHz", 0.000000496, "0.000000496 MHz"),
        ("GHz", 0.000000000496, "4.96e-10 GHz"),
        ("Hz", 0., "0 Hz"),
    ] {
        assert_eq!(
            UnitHint {
                per_pixel: value,
                units,
                ..hint
            }
            .resolution(),
            format!("Resolution: {expected}/px")
        );
    }
    let localized = crate::numbers::Numbers::new("ru-RU").text(
        &UnitHint {
            per_pixel: 0.000496,
            units: "kHz",
            ..hint
        }
        .resolution(),
    );
    assert_eq!(localized, "Resolution: 0,000496 kHz/px");
}

fn assert_vertical_time_clearance(frame: &Frame) {
    let half_ink = DejaVuSans.digit_height(LABEL_SIZE) / 2.;
    let top = frame.time_caption_row + half_ink;
    let bottom = frame.caption_row - half_ink;
    let mut previous_end = f32::NEG_INFINITY;
    for tick in frame.time.iter().filter(|tick| !tick.label.is_empty()) {
        let center = tick.offset as f32 + 0.5;
        assert!(center - half_ink >= top + OUTER_PAD);
        assert!(center + half_ink <= bottom - OUTER_PAD);
        assert!(center - half_ink - previous_end >= DejaVuSans.width("00", LABEL_SIZE));
        previous_end = center + half_ink;
    }
}

#[test]
fn common_frequency_unit_stabilizes_after_first_layout_near_precision_thresholds() {
    use crate::navigation::orientation::Mode;
    use argand_core::frequency::FrequencyUnit;
    for orientation in [Mode::Horizontal, Mode::Vertical] {
        for span in [499_999., 500_000., 500_001., 599., 600., 601., 24_000.] {
            let mut extents = Extents {
                orientation,
                hertz: (-span / 2., span / 2.),
                frequency_offset: 12_590_000.,
                frequency_unit: FrequencyUnit::MHz,
                ..HFDL
            };
            let mut previous = None;
            for pass in 0..4 {
                let frame =
                    Frame::measure(panel(800., 600.), 1., extents, &DejaVuSans, None).unwrap();
                let pixels = if orientation.vertical() {
                    frame.plot.width
                } else {
                    frame.plot.height
                };
                let unit = FrequencyUnit::coordinates(
                    12_590_000. - span / 2.,
                    12_590_000. + span / 2.,
                    span / f64::from(pixels.round()),
                );
                if pass >= 2 {
                    assert_eq!(Some(unit), previous, "{orientation:?}, span {span}");
                }
                previous = Some(unit);
                extents.frequency_unit = unit;
            }
        }
    }
}
