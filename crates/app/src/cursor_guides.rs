//! Transient ruler readouts; no analysis or layout changes on modifier input.

use super::*;
use std::{cell::RefCell, rc::Rc};

/// The badge's corner radius.
const BADGE_RADIUS: f32 = 3.;
/// The width of the paper-coloured ring around a badge.
const BADGE_RING: f32 = 1.;

pub struct CursorGuides {
    pub extents: Extents,
    pub metrics: BadgeMetrics,
    pub ink: Hsla,
    pub paper: Hsla,
}

#[derive(Clone, Default)]
pub struct BadgeMetrics(Rc<RefCell<Option<MeasuredBadges>>>);

#[derive(Clone, PartialEq)]
struct BadgeKey {
    plot: Rect,
    extents: Extents,
    scale: f32,
    font: Font,
}

struct MeasuredBadges {
    key: BadgeKey,
    widths: [f32; 2],
}

impl BadgeMetrics {
    fn get(&self, key: BadgeKey, labels: &dyn LabelMeasure) -> [f32; 2] {
        let mut cached = self.0.borrow_mut();
        if let Some(measured) = cached.as_ref().filter(|measured| measured.key == key) {
            return measured.widths;
        }
        // The numeric locale is immutable after startup; it needs no invalidation.
        let widths = measure_badges(key.plot, key.extents, key.scale, labels);
        *cached = Some(MeasuredBadges { key, widths });
        widths
    }
}

impl CursorGuides {
    pub fn paint(
        &self,
        frame: &Frame,
        panel: Bounds<Pixels>,
        labels: &Labels,
        window: &mut Window,
        cx: &mut App,
    ) {
        if !window.modifiers().alt || !window.is_window_active() {
            return;
        }
        let pointer = window.mouse_position() - panel.origin;
        let Some(readout) = Readout::at(frame.plot, pointer, self.extents, window.scale_factor())
        else {
            return;
        };
        let widths = self.metrics.get(
            BadgeKey {
                plot: frame.plot,
                extents: self.extents,
                scale: window.scale_factor(),
                font: labels.font.clone(),
            },
            labels,
        );
        let vertical = frame.orientation.vertical();
        let ordered_widths = if vertical {
            [widths[1], widths[0]]
        } else {
            widths
        };
        let positions = badge_rects(
            frame,
            panel.size,
            pointer,
            ordered_widths,
            labels.digit_height(LABEL_SIZE),
            window.scale_factor(),
        );
        let badges = if vertical {
            [positions[1], positions[0]]
        } else {
            positions
        };
        window.with_content_mask(Some(gpui_kit::ContentMask { bounds: panel }), |window| {
            // Rings first, so the guide lines run over them into the badge fill.
            for rect in badges {
                self.ring(rect, panel, window);
            }
            let colors = [self.paper, self.ink];
            paint_lines(
                panel.origin + pointer,
                panel.origin,
                positions,
                colors,
                window,
            );
            for (text, rect) in [(&readout.time, badges[0]), (&readout.frequency, badges[1])] {
                self.badge(text, rect, panel, labels, window, cx);
            }
        });
    }

    /// A one-pixel ring in the paper colour, so a badge keeps its edge over a picture of its own shade.
    fn ring(&self, rect: Rect, panel: Bounds<Pixels>, window: &mut Window) {
        let origin = panel.origin + point(px(rect.x - BADGE_RING), px(rect.y - BADGE_RING));
        let size = size(
            px(rect.width + 2. * BADGE_RING),
            px(rect.height + 2. * BADGE_RING),
        );
        window.paint_quad(
            fill(Bounds::new(origin, size), self.paper).corner_radii(px(BADGE_RADIUS + BADGE_RING)),
        );
    }

    fn badge(
        &self,
        text: &str,
        rect: Rect,
        panel: Bounds<Pixels>,
        labels: &Labels,
        window: &mut Window,
        cx: &mut App,
    ) {
        let shaped = labels.shape(text, self.paper);
        let origin = panel.origin + point(px(rect.x), px(rect.y));
        window.paint_quad(
            fill(
                Bounds::new(origin, size(px(rect.width), px(rect.height))),
                self.ink,
            )
            .corner_radii(px(BADGE_RADIUS)),
        );
        // Centred like the ruler labels, so an unclamped badge shares their text row.
        let top = labels.line_top(rect.height / 2., &shaped);
        let left = (px(rect.width) - shaped.width) / 2.;
        let _ = shaped.paint(
            origin + point(left, px(top)),
            px(LINE_HEIGHT),
            gpui_kit::TextAlign::Left,
            None,
            window,
            cx,
        );
    }
}

/// Endpoint labels bound the number of digits, grouping separators and signs.
/// The label measure uses the widest digit, independently of the pointer value.
fn measure_badges(plot: Rect, extents: Extents, scale: f32, labels: &dyn LabelMeasure) -> [f32; 2] {
    let mut widths = [0_f32; 2];
    for pointer in [
        point(px(plot.x), px(plot.y)),
        point(px(plot.right()), px(plot.bottom())),
    ] {
        if let Some(readout) = Readout::at(plot, pointer, extents, scale) {
            for (width, text) in widths.iter_mut().zip([readout.time, readout.frequency]) {
                *width = width.max(labels.width(&text, LABEL_SIZE).ceil() + 2. * BADGE_PAD);
            }
        }
    }
    widths
}

/// Draw both guide lines as a paper-coloured stroke around an ink core, matching the badges.
fn paint_lines(
    pointer: Point<Pixels>,
    origin: Point<Pixels>,
    badges: [Rect; 2],
    [paper, ink]: [Hsla; 2],
    window: &mut Window,
) {
    let scale = window.scale_factor();
    let x = px((f32::from(pointer.x) * scale).floor() / scale);
    let y = px((f32::from(pointer.y) * scale).floor() / scale);
    // Overlap the rounded backgrounds so their corners cannot expose a gap.
    let bottom = origin.y + px(badges[0].y + BADGE_RADIUS);
    let right = origin.x + px(badges[1].x + BADGE_RADIUS);
    for (width, color) in [(3., paper), (1., ink)] {
        let offset = px((width - 1.) / 2.);
        for bounds in [
            Bounds::new(
                point(x - offset, y),
                size(px(width), (bottom - y).max(px(0.))),
            ),
            Bounds::new(
                point(x, y - offset),
                size((right - x).max(px(0.)), px(width)),
            ),
        ] {
            window.paint_quad(fill(bounds, color));
        }
    }
}

/// The bottom and right badges, each centred where its ruler centres a label.
///
/// The bottom badge sits on the bottom ruler's text row and the right badge on
/// the guide line, as labels sit on their ticks. The right badge stands as far
/// from the panel's right edge as the bottom badge from its bottom edge.
/// Clamping moves a badge only where the panel has no room for the aligned
/// position.
fn badge_rects(
    frame: &Frame,
    panel: Size<Pixels>,
    pointer: Point<Pixels>,
    widths: [f32; 2],
    ink: f32,
    scale: f32,
) -> [Rect; 2] {
    let height = ink + 2. * BADGE_PAD;
    let snap = |value: Pixels| (f32::from(value) * scale).floor() / scale;
    let bottom = badge_bounds(
        point(px(snap(pointer.x) + 0.5), px(frame.time_row - height / 2.)),
        panel,
        widths[0],
        height,
    );
    // The right badge keeps the same margin to the panel edge as the bottom one.
    let margin = (f32::from(panel.height) - bottom.bottom()).max(0.);
    let right_edge = f32::from(panel.width) - margin;
    let right = badge_bounds(
        point(
            px(right_edge - widths[1] / 2.),
            px(snap(pointer.y) + 0.5 - height / 2.),
        ),
        panel,
        widths[1],
        height,
    );
    [bottom, right]
}

fn badge_bounds(anchor: Point<Pixels>, panel: Size<Pixels>, width: f32, height: f32) -> Rect {
    let width = width.min(f32::from(panel.width));
    let height = height.min(f32::from(panel.height));
    Rect {
        x: (f32::from(anchor.x) - width / 2.).clamp(0., f32::from(panel.width) - width),
        y: f32::from(anchor.y).clamp(0., f32::from(panel.height) - height),
        width,
        height,
    }
}

pub(crate) struct Readout {
    pub time: String,
    pub frequency: String,
}

impl Readout {
    fn at(plot: Rect, pointer: Point<Pixels>, extents: Extents, scale: f32) -> Option<Self> {
        let x = f32::from(pointer.x);
        let y = f32::from(pointer.y);
        if plot.width <= 0.
            || plot.height <= 0.
            || x < plot.x
            || x > plot.right()
            || y < plot.y
            || y > plot.bottom()
        {
            return None;
        }
        let (time_fraction, frequency_fraction) = extents.orientation.fractions(
            (x - plot.x) as f64 / plot.width as f64,
            (y - plot.y) as f64 / plot.height as f64,
        );
        let (time_length, frequency_length) = extents.orientation.axes(plot.width, plot.height);
        Some(Self::from_fractions(
            (time_fraction, frequency_fraction),
            extents,
            (
                (time_length * scale) as f64,
                (frequency_length * scale) as f64,
            ),
        ))
    }

    pub fn from_fractions(
        (time_fraction, frequency_fraction): (f64, f64),
        extents: Extents,
        (time_pixels, frequency_pixels): (f64, f64),
    ) -> Self {
        let time_span = extents.seconds.1 - extents.seconds.0;
        let time = extents.seconds.0 + time_fraction * time_span;
        let frequency_span = extents.hertz.1 - extents.hertz.0;
        let frequency = extents.hertz.1 - frequency_fraction * frequency_span;
        let time_label = extents
            .time
            .readout(time, time_span, time_pixels, time_fraction);
        let unit =
            axis::caption(AxisKind::Frequency, extents.hertz.0, extents.hertz.1).unwrap_or("Hz");
        let divisor = match unit {
            "GHz" => 1e9,
            "MHz" => 1e6,
            "kHz" => 1e3,
            _ => 1.,
        };
        let precision =
            crate::navigation::time_precision(frequency_span / frequency_pixels / divisor);
        Self {
            time: time_label,
            frequency: crate::numbers::text(&format!(
                "{:.*} {unit}",
                precision,
                frequency / divisor
            )),
        }
    }
}

#[cfg(test)]
mod tests {
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
}
