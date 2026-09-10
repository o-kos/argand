//! Transient ruler readouts; no analysis or layout changes on modifier input.

use super::*;

pub struct CursorGuides {
    pub extents: Extents,
    pub ink: Hsla,
    pub paper: Hsla,
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
        let at = |x, y| panel.origin + point(px(x), px(y));
        let x = f32::from(pointer.x);
        let y = f32::from(pointer.y);
        window.with_content_mask(Some(gpui::ContentMask { bounds: panel }), |window| {
            for bounds in [
                Bounds::new(at(x, y), size(px(1.), px(frame.plot.bottom() - y))),
                Bounds::new(at(x, y), size(px(frame.plot.right() - x), px(1.))),
            ] {
                window.paint_quad(fill(bounds, self.ink));
            }
            self.badge(
                &readout.time,
                point(px(x), px(frame.time_row - LINE_HEIGHT / 2.)),
                panel,
                labels,
                window,
                cx,
            );
            self.badge(
                &readout.frequency,
                point(panel.size.width, px(y - LINE_HEIGHT / 2.)),
                panel,
                labels,
                window,
                cx,
            );
        });
    }

    fn badge(
        &self,
        text: &str,
        anchor: Point<Pixels>,
        panel: Bounds<Pixels>,
        labels: &Labels,
        window: &mut Window,
        cx: &mut App,
    ) {
        let shaped = labels.shape(text, self.paper);
        let width = f32::from(shaped.width) + 6.;
        let rect = badge_bounds(anchor, panel.size, width);
        let origin = panel.origin + point(px(rect.x), px(rect.y));
        window.paint_quad(fill(
            Bounds::new(origin, size(px(rect.width), px(rect.height))),
            self.ink,
        ));
        let top = labels.line_top(LINE_HEIGHT / 2., &shaped);
        let _ = shaped.paint(origin + point(px(3.), px(top)), px(LINE_HEIGHT), window, cx);
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
        let decimals = crate::navigation::time_precision(time_span / (plot.width * scale) as f64);
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
            time: format!("{time:.decimals$} s"),
            frequency: format!("{:.*} {unit}", precision, frequency / divisor),
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
            seconds: (12., 12.001),
            hertz: (99e6, 101e6),
        };
        let center = Readout::at(plot, point(px(510.), px(270.)), extents, 1.).unwrap();
        assert_eq!(center.time, "12.0005000 s");
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
            seconds: (0., 1.),
            hertz: (-24000., 24000.),
        };
        let bottom = Readout::at(plot, point(px(1000.), px(500.)), extents, 1.).unwrap();
        assert_eq!(bottom.frequency, "-24.000 kHz");
        assert_eq!(bottom.time, "1.000 s");
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
            seconds: (0., 0.5),
            hertz: (0., 0.5),
        };
        let first = Readout::at(plot, point(px(0.5), px(0.5)), extents, 2.).unwrap();
        let next = Readout::at(plot, point(px(1.), px(1.)), extents, 2.).unwrap();
        assert_eq!(first.time, "0.0005 s");
        assert_eq!(next.time, "0.0010 s");
        assert_eq!(first.frequency, "0.4995 Hz");
        assert_eq!(next.frequency, "0.4990 Hz");
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
