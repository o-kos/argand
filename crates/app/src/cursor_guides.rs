//! Transient ruler readouts; no analysis or layout changes on modifier input.

use super::*;
use std::{cell::RefCell, rc::Rc};

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
        let badges = [
            badge_bounds(
                point(pointer.x, px(frame.time_row - LINE_HEIGHT / 2.)),
                panel.size,
                widths[0],
            ),
            badge_bounds(
                point(panel.size.width, pointer.y - px(LINE_HEIGHT / 2.)),
                panel.size,
                widths[1],
            ),
        ];
        window.with_content_mask(Some(gpui::ContentMask { bounds: panel }), |window| {
            paint_lines(panel.origin + pointer, panel.origin, badges, window);
            for (text, rect) in [(&readout.time, badges[0]), (&readout.frequency, badges[1])] {
                self.badge(text, rect, panel, labels, window, cx);
            }
        });
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
            .corner_radii(px(3.)),
        );
        // Optical correction for the digit ink inside the filled badge.
        let top = labels.line_top(rect.height / 2., &shaped) + 1.;
        let left = (px(rect.width) - shaped.width) / 2.;
        let _ = shaped.paint(origin + point(left, px(top)), px(LINE_HEIGHT), window, cx);
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
                *width = width.max(labels.width(&text, LABEL_SIZE).ceil() + 6.);
            }
        }
    }
    widths
}

fn paint_lines(
    pointer: Point<Pixels>,
    origin: Point<Pixels>,
    badges: [Rect; 2],
    window: &mut Window,
) {
    let scale = window.scale_factor();
    let x = px((f32::from(pointer.x) * scale).floor() / scale);
    let y = px((f32::from(pointer.y) * scale).floor() / scale);
    // Overlap the rounded backgrounds so their corners cannot expose a gap.
    let bottom = origin.y + px(badges[0].y + 3.);
    let right = origin.x + px(badges[1].x + 3.);
    for (width, color) in [(3., gpui::white()), (1., gpui::black())] {
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

fn badge_bounds(anchor: Point<Pixels>, panel: Size<Pixels>, width: f32) -> Rect {
    let width = width.min(f32::from(panel.width));
    let height = LINE_HEIGHT.min(f32::from(panel.height));
    Rect {
        x: (f32::from(anchor.x) - width / 2.).clamp(0., f32::from(panel.width) - width),
        y: f32::from(anchor.y).clamp(0., f32::from(panel.height) - height),
        width,
        height,
    }
}

struct Readout {
    time: String,
    frequency: String,
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
        let time_span = extents.seconds.1 - extents.seconds.0;
        let time = extents.seconds.0 + (x - plot.x) as f64 / plot.width as f64 * time_span;
        let frequency_span = extents.hertz.1 - extents.hertz.0;
        let frequency = extents.hertz.1 - (y - plot.y) as f64 / plot.height as f64 * frequency_span;
        let time_label = extents.time.readout(
            time,
            time_span,
            (plot.width * scale) as f64,
            (x - plot.x) as f64 / plot.width as f64,
        );
        let unit =
            axis::caption(AxisKind::Frequency, extents.hertz.0, extents.hertz.1).unwrap_or("Hz");
        let divisor = match unit {
            "GHz" => 1e9,
            "MHz" => 1e6,
            "kHz" => 1e3,
            _ => 1.,
        };
        let precision = crate::navigation::time_precision(
            frequency_span / (plot.height * scale) as f64 / divisor,
        );
        Some(Self {
            time: time_label,
            frequency: crate::numbers::text(&format!(
                "{:.*} {unit}",
                precision,
                frequency / divisor
            )),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn physical_values_follow_requested_extents_and_inverted_frequency_axis() {
        let plot = Rect {
            x: 10.,
            y: 20.,
            width: 1000.,
            height: 500.,
        };
        let extents = Extents {
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
                time: crate::time_ruler::Ruler::CLOCK,
                seconds: (0., 1.),
                hertz: (-24000., 24000.),
            },
            scale: 1.,
            font: gpui::font("test"),
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
        changed.font = gpui::font("other");
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
                }
            }
        }
    }

    fn assert_badge_widths(extents: Extents, scale: f32) {
        use argand_core::testutil::DejaVuSans;

        let panel = size(px(1200.), px(800.));
        let frame = Frame::measure(panel, scale, extents, &DejaVuSans, None, 48.).unwrap();
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
                let rect = badge_bounds(pointer, panel, width);
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
            let rect = badge_bounds(anchor, size(px(100.), px(80.)), 70.);
            assert!(rect.x >= 0. && rect.y >= 0.);
            assert!(rect.right() <= 100. && rect.bottom() <= 80.);
        }
    }
}
