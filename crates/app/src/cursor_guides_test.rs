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
fn badge_and_fraction_readouts_match_at_device_pixel_precision() {
    use crate::orientation::Mode;

    let plot = Rect {
        x: 10.,
        y: 20.,
        width: 500.25,
        height: 250.125,
    };
    let pointer = point(px(plot.x + plot.width / 2.), px(plot.y + plot.height / 2.));
    for orientation in [Mode::Horizontal, Mode::Vertical] {
        let extents = Extents {
            orientation,
            time: crate::time_ruler::Ruler::CLOCK,
            seconds: (0., 0.5),
            hertz: (0., 0.5),
        };
        let (time_length, frequency_length) = orientation.axes(plot.width, plot.height);
        for scale in [1., 1.25, 2.] {
            let pixels = (
                (time_length * scale) as f64,
                (frequency_length * scale) as f64,
            );
            let badge = Readout::at(plot, pointer, extents, scale).unwrap();
            let readout = Readout::from_fractions((0.5, 0.5), extents, pixels);
            assert_eq!(readout.time, badge.time);
            assert_eq!(readout.frequency, badge.frequency);
            let outside = Readout::from_fractions((0.5, -1.), extents, pixels);
            assert_eq!(outside.time, badge.time);
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
fn complex_baseband_preserves_negative_frequency() {
    let plot = Rect {
        x: 0.,
        y: 0.,
        width: 1000.,
        height: 500.,
    };
    let extents = Extents {
        orientation: crate::orientation::Mode::Horizontal,
        time: crate::time_ruler::Ruler::CLOCK,
        seconds: (0., 1.),
        hertz: (-24000., 24000.),
    };
    let bottom = Readout::at(plot, point(px(1000.), px(500.)), extents, 1.).unwrap();
    assert_eq!(bottom.frequency, "-24.000 kHz");
    assert_eq!(bottom.time, "0:01.000");
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
fn fixed_badge_widths_cover_readouts_across_ruler_ranges() {
    use crate::{
        navigation::View,
        time_ruler::{Mode, Ruler},
    };

    for mode in [Mode::Clock, Mode::Seconds, Mode::Samples] {
        for (seconds, hertz) in [
            ((0., 1001.), (-24000., 24000.)),
            ((59.9995, 60.0005), (-9.9995, 100.)),
            ((3599., 7200.), (99e6, 101e6)),
            ((999.9995, 1000.0005), (-1e9, 0.)),
        ] {
            for scale in [1., 1.25, 2.] {
                let extents = Extents {
                    orientation: crate::orientation::Mode::Horizontal,
                    time: Ruler {
                        mode,
                        view: View {
                            start: 999,
                            len: 1_000_001,
                        },
                        total: 1_001_000,
                    },
                    seconds,
                    hertz,
                };
                assert_badge_widths(extents, scale);
                assert_badge_widths(
                    Extents {
                        orientation: crate::orientation::Mode::Vertical,
                        ..extents
                    },
                    scale,
                );
            }
        }
    }
}

fn assert_badge_widths(extents: Extents, scale: f32) {
    use argand_core::testutil::DejaVuSans;

    let panel = size(px(1200.), px(800.));
    let frame = Frame::measure(panel, scale, extents, &DejaVuSans, None).unwrap();
    let widths = measure_badges(frame.plot, extents, scale, &DejaVuSans);
    for step in 0..=1000 {
        let fraction = step as f32 / 1000.;
        let pointer = point(
            px(frame.plot.x + frame.plot.width * fraction),
            px(frame.plot.y + frame.plot.height * fraction),
        );
        let readout = Readout::at(frame.plot, pointer, extents, scale).unwrap();
        for (text, width) in [readout.time, readout.frequency].iter().zip(widths) {
            assert!(
                DejaVuSans.width(text, LABEL_SIZE) + 6. <= width,
                "{text} exceeds {width}"
            );
            let rect = badge_bounds(pointer, panel, width, LINE_HEIGHT);
            assert_eq!(rect.width, width);
        }
    }
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
fn badges_stay_inside_panel_at_corners() {
    for anchor in [point(px(0.), px(-10.)), point(px(100.), px(80.))] {
        let rect = badge_bounds(anchor, size(px(100.), px(80.)), 70., LINE_HEIGHT);
        assert!(rect.x >= 0. && rect.y >= 0.);
        assert!(rect.right() <= 100. && rect.bottom() <= 80.);
    }
}
#[test]
fn badges_sit_on_their_ruler_rows_in_both_orientations() {
    let orientations = [
        crate::orientation::Mode::Horizontal,
        crate::orientation::Mode::Vertical,
    ];
    let modes = [
        crate::time_ruler::Mode::Clock,
        crate::time_ruler::Mode::Seconds,
        crate::time_ruler::Mode::Samples,
    ];
    for orientation in orientations {
        for mode in modes {
            for scale in [1., 1.25, 1.5, 2.] {
                let extents = Extents {
                    orientation,
                    time: crate::time_ruler::Ruler {
                        mode,
                        ..crate::time_ruler::Ruler::CLOCK
                    },
                    seconds: (10., 20.),
                    hertz: (-12000., 12000.),
                };
                assert_badges_on_rows(extents, scale);
            }
        }
    }
}

fn assert_badges_on_rows(extents: Extents, scale: f32) {
    use argand_core::testutil::DejaVuSans;

    let ink = DejaVuSans.digit_height(LABEL_SIZE);
    let panel = size(px(800.), px(500.));
    let frame = Frame::measure(panel, scale, extents, &DejaVuSans, None).unwrap();
    let widths = measure_badges(frame.plot, extents, scale, &DejaVuSans);
    let snap = |value: f32| (value * scale).floor() / scale + 0.5;
    let at = |fx: f32, fy: f32| {
        point(
            px(frame.plot.x + frame.plot.width * fx),
            px(frame.plot.y + frame.plot.height * fy),
        )
    };
    for pointer in [at(0.4, 0.6), at(0., 0.), at(1., 1.)] {
        let [bottom, right] = badge_rects(&frame, panel, pointer, widths, ink, scale);
        for rect in [bottom, right] {
            assert!(rect.x >= 0. && rect.y >= 0., "{rect:?}");
            assert!(rect.right() <= 800. && rect.bottom() <= 500., "{rect:?}");
            assert!((rect.height - (ink + 2. * BADGE_PAD)).abs() < 1e-4);
        }
        let row = bottom.y + bottom.height / 2.;
        assert!((row - frame.time_row).abs() < 1e-4, "text row at {scale}");
        assert!(bottom.y >= frame.plot.bottom() + 1., "ruler line visible");
        let margin = 500. - bottom.bottom();
        assert!(
            (800. - right.right() - margin).abs() < 1e-4,
            "equal margins to both edges"
        );
        assert!(margin >= BOTTOM_LABEL_FOOT - BADGE_PAD - 1e-4);
    }
    let pointer = at(0.4, 0.6);
    let [bottom, right] = badge_rects(&frame, panel, pointer, widths, ink, scale);
    let centre_x = bottom.x + bottom.width / 2.;
    let centre_y = right.y + right.height / 2.;
    assert!((centre_x - snap(f32::from(pointer.x))).abs() < 1e-4);
    assert!((centre_y - snap(f32::from(pointer.y))).abs() < 1e-4);
    let above = bottom.y - (frame.plot.bottom() + 1.);
    let below = f32::from(panel.height) - bottom.bottom();
    assert!(
        above >= BOTTOM_LABEL_DROP - BADGE_PAD - 1e-4,
        "clears the ruler line"
    );
    let extra = BOTTOM_LABEL_FOOT - BOTTOM_LABEL_DROP;
    assert!(
        (below - above - extra).abs() < 1e-4,
        "the foot adds room below"
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
