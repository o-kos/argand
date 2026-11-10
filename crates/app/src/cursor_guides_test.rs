use super::*;

#[test]
fn shared_readout_honours_all_ruler_modes_and_frequency_units() {
    use crate::time_ruler::Mode;

    for (mode, time) in [
        (Mode::Clock, "0:12.0005000"),
        (Mode::Seconds, "12.0005000 s"),
        (Mode::Samples, "#24,001,000"),
    ] {
        for (hertz, frequency) in [
            ((-0.5, 0.5), "-0.250 Hz"),
            ((-24000., 24000.), "-12.000 kHz"),
            ((99e6, 101e6), "99.500 MHz"),
            ((1e9, 3e9), "1.500 GHz"),
        ] {
            let extents = Extents {
                orientation: crate::orientation::Mode::Horizontal,
                time: crate::time_ruler::Ruler {
                    mode,
                    view: crate::navigation::View {
                        start: 24_000_000,
                        len: 2000,
                    },
                    total: 48_000_000,
                },
                seconds: (12., 12.001),
                hertz,
            };
            let readout = Readout::from_fractions((0.5, 0.75), extents, (1000., 500.));
            assert_eq!(readout.time, time);
            assert_eq!(readout.frequency, frequency);
        }
    }
}

#[test]
fn physical_values_follow_requested_extents_and_inverted_frequency_axis() {
    let plot = Rect {
        x: 10.,
        y: 20.,
        width: 1000.,
        height: 500.,
    };
    let extents = Extents {
        orientation: crate::orientation::Mode::Horizontal,
        time: crate::time_ruler::Ruler::CLOCK,
        seconds: (12., 12.001),
        hertz: (99e6, 101e6),
    };
    let center = Readout::at(plot, point(px(510.), px(270.)), extents, 1.).unwrap();
    assert_eq!(center.time, "0:12.0005000");
    assert_eq!(center.frequency, "100.000 MHz");
    let top = Readout::at(plot, point(px(10.), px(20.)), extents, 1.).unwrap();
    assert_eq!(top.frequency, "101.000 MHz");
    assert!(Readout::at(plot, point(px(510.), px(19.)), extents, 1.).is_none());
    assert!(Readout::at(plot, point(px(1011.), px(270.)), extents, 1.).is_none());
}

#[test]
fn hidpi_badges_resolve_adjacent_device_pixels() {
    let plot = Rect {
        x: 0.,
        y: 0.,
        width: 500.,
        height: 500.,
    };
    let extents = Extents {
        orientation: crate::orientation::Mode::Horizontal,
        time: crate::time_ruler::Ruler::CLOCK,
        seconds: (0., 0.5),
        hertz: (0., 0.5),
    };
    let first = Readout::at(plot, point(px(0.5), px(0.5)), extents, 2.).unwrap();
    let next = Readout::at(plot, point(px(1.), px(1.)), extents, 2.).unwrap();
    assert_eq!(first.time, "0:00.0005");
    assert_eq!(next.time, "0:00.0010");
    assert_eq!(first.frequency, "0.4995 Hz");
    assert_eq!(next.frequency, "0.4990 Hz");
}

#[test]
fn badge_measurements_are_reused_until_ruler_or_font_changes() {
    struct CountingMeasure(std::cell::Cell<usize>);
    impl LabelMeasure for CountingMeasure {
        fn width(&self, text: &str, size: f32) -> f32 {
            self.0.set(self.0.get() + 1);
            argand_core::testutil::DejaVuSans.width(text, size)
        }
        fn digit_height(&self, size: f32) -> f32 {
            size
        }
    }
    let labels = CountingMeasure(std::cell::Cell::new(0));
    let metrics = BadgeMetrics::default();
    let key = BadgeKey {
        plot: Rect {
            x: 0.,
            y: 0.,
            width: 1000.,
            height: 500.,
        },
        extents: Extents {
            orientation: crate::orientation::Mode::Horizontal,
            time: crate::time_ruler::Ruler::CLOCK,
            seconds: (0., 1.),
            hertz: (-24000., 24000.),
        },
        scale: 1.,
        font: gpui_kit::font("test"),
    };
    let expected = metrics.get(key.clone(), &labels);
    for _ in 0..1000 {
        assert_eq!(metrics.clone().get(key.clone(), &labels), expected);
    }
    assert_eq!(labels.0.get(), 4);
    let mut changed = key;
    changed.extents.seconds = (0., 1000.);
    metrics.get(changed.clone(), &labels);
    assert_eq!(labels.0.get(), 8);
    changed.extents.time.mode = crate::time_ruler::Mode::Samples;
    metrics.get(changed.clone(), &labels);
    assert_eq!(labels.0.get(), 12);
    changed.plot.width = 2000.;
    metrics.get(changed.clone(), &labels);
    assert_eq!(labels.0.get(), 16);
    changed.scale = 2.;
    metrics.get(changed.clone(), &labels);
    assert_eq!(labels.0.get(), 20);
    changed.font = gpui_kit::font("other");
    metrics.get(changed, &labels);
    assert_eq!(labels.0.get(), 24);
}

#[test]
fn sample_width_keeps_exact_large_capture_indices() {
    let plot = Rect {
        x: 0.,
        y: 0.,
        width: 1000.,
        height: 500.,
    };
    let extents = Extents {
        orientation: crate::orientation::Mode::Horizontal,
        time: crate::time_ruler::Ruler {
            mode: crate::time_ruler::Mode::Samples,
            view: crate::navigation::View {
                start: u64::MAX - 4096,
                len: 4096,
            },
            total: u64::MAX,
        },
        seconds: (0., 1.),
        hertz: (-1., 1.),
    };
    let widths = measure_badges(plot, extents, 1., &argand_core::testutil::DejaVuSans);
    let last = format!("#{}", crate::numbers::number(u64::MAX - 1));
    assert_eq!(
        widths[0],
        argand_core::testutil::DejaVuSans
            .width(&last, LABEL_SIZE)
            .ceil()
            + 6.
    );
}

#[test]
fn vertical_readouts_follow_time_down_and_frequency_right() {
    let extents = Extents {
        orientation: crate::orientation::Mode::Vertical,
        time: crate::time_ruler::Ruler::CLOCK,
        seconds: (10., 20.),
        hertz: (-12000., 12000.),
    };
    let plot = Rect {
        x: 10.,
        y: 20.,
        width: 400.,
        height: 200.,
    };
    let center = Readout::at(plot, point(px(210.), px(120.)), extents, 1.).unwrap();
    let left = Readout::at(plot, point(px(10.), px(120.)), extents, 1.).unwrap();
    let bottom = Readout::at(plot, point(px(210.), px(220.)), extents, 1.).unwrap();
    assert_eq!(center.time, left.time);
    assert_eq!(center.frequency, "0.000 kHz");
    assert_eq!(left.frequency, "-12.000 kHz");
    assert_ne!(center.time, bottom.time);
    assert_eq!(center.frequency, bottom.frequency);
}
