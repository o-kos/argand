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
        let frequency =
            extents.hertz.1 - frequency_fraction * frequency_span + extents.frequency_offset;
        let time_label = extents
            .time
            .readout(time, time_span, time_pixels, time_fraction);
        let (unit, divisor) = frequency_unit(extents.displayed_hertz());
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

/// The frequency ruler's unit for the visible band, with the hertz in one of it.
pub fn frequency_unit((low, high): (f64, f64)) -> (&'static str, f64) {
    let unit = axis::caption(AxisKind::Frequency, low, high).unwrap_or("Hz");
    let divisor = match unit {
        "GHz" => 1e9,
        "MHz" => 1e6,
        "kHz" => 1e3,
        _ => 1.,
    };
    (unit, divisor)
}

#[cfg(test)]
#[path = "cursor_guides_test.rs"]
mod tests;
