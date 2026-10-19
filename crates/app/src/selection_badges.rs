//! A selection's edges and extent, in badges over the plot.
//!
//! Along each selected axis the two edges are labelled just outside
//! themselves on one side of the plot, and the extent over the middle of the
//! visible selection on the other side.

use super::*;

/// Room between a badge and the plot's edge it stands on.
const MARGIN: f32 = 4.;
/// The least gap the far edge's badge keeps from the near edge's badge it is pushed past.
const GAP: f32 = 2.;
/// The badge's corner radius, as the Alt badges have it.
const RADIUS: f32 = 3.;

/// A selection's readouts on the plot.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SelectionBadges {
    pub time: Option<AxisBadges>,
    pub band: Option<AxisBadges>,
}

/// One selected axis's readouts and where its edges are, as fractions along the axis on screen.
///
/// Fractions run down or to the right, so the first edge is the one met first
/// that way: the start in time, the high edge of a band shown upright and its
/// low edge with frequency running right.
#[derive(Debug, Clone, PartialEq)]
pub struct AxisBadges {
    /// The first edge, the second edge and the extent.
    pub texts: [String; 3],
    /// The first edge, none when it is not shown.
    pub first: Option<f64>,
    pub second: Option<f64>,
    /// The visible part of the selection.
    pub visible: (f64, f64),
}

/// Which way an axis runs on the screen and on which side its edges are labelled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Layout {
    /// The axis runs across the screen, not down it.
    pub across: bool,
    /// The edges stand at the far side of the other axis, the bottom or the right.
    pub edges_far: bool,
}

impl Layout {
    /// Time runs right with its edges at the bottom, or down with its edges at the left.
    fn time(vertical: bool) -> Self {
        Self {
            across: !vertical,
            edges_far: !vertical,
        }
    }

    /// Frequency runs up with its edges at the left, or right with its edges at the top.
    fn band(vertical: bool) -> Self {
        Self {
            across: vertical,
            edges_far: false,
        }
    }
}

impl SelectionBadges {
    pub fn paint(
        &self,
        frame: &Frame,
        panel: Point<Pixels>,
        labels: &Labels,
        colors: [Hsla; 2],
        window: &mut Window,
        cx: &mut App,
    ) {
        let height = labels.digit_height(LABEL_SIZE) + 2. * BADGE_PAD;
        let width = |text: &String| labels.width(text, LABEL_SIZE).ceil() + 2. * BADGE_PAD;
        let vertical = frame.orientation.vertical();
        let mut shown: Vec<(&str, Rect)> = Vec::new();
        let axes = [
            (&self.time, Layout::time(vertical)),
            (&self.band, Layout::band(vertical)),
        ];
        for (badges, layout) in axes {
            let Some(badges) = badges else { continue };
            let middle = Some((badges.visible.0 + badges.visible.1) / 2.);
            let places = places(
                frame.plot,
                layout,
                [badges.first, badges.second, middle],
                badges.texts.each_ref().map(width),
                height,
            );
            shown.extend(
                badges
                    .texts
                    .iter()
                    .zip(places)
                    .filter_map(|(text, rect)| Some((text.as_str(), rect?))),
            );
        }
        let clip = Bounds::new(
            panel + point(px(frame.plot.x), px(frame.plot.y)),
            size(px(frame.plot.width), px(frame.plot.height)),
        );
        window.with_content_mask(Some(gpui_kit::ContentMask { bounds: clip }), |window| {
            for (text, rect) in &shown {
                badge(text, *rect, panel, labels, colors, window, cx);
            }
        });
    }
}

fn badge(
    text: &str,
    rect: Rect,
    panel: Point<Pixels>,
    labels: &Labels,
    [ink, paper]: [Hsla; 2],
    window: &mut Window,
    cx: &mut App,
) {
    let origin = panel + point(px(rect.x), px(rect.y));
    let ring = Bounds::new(
        origin - point(px(1.), px(1.)),
        size(px(rect.width + 2.), px(rect.height + 2.)),
    );
    window.paint_quad(fill(ring, paper).corner_radii(px(RADIUS + 1.)));
    window.paint_quad(
        fill(
            Bounds::new(origin, size(px(rect.width), px(rect.height))),
            ink,
        )
        .corner_radii(px(RADIUS)),
    );
    let shaped = labels.shape(text, paper);
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

/// Where the first edge's, the second edge's and the extent's badges go, each none where it is not shown.
///
/// `at` holds the two edges and the middle of the visible selection as
/// fractions along the axis. The edge badges stand outside their edges on the
/// side `layout` names, the extent badge over the middle on the other side.
/// Badges stay inside the plot, the second edge's badge goes past the first's
/// where they would overlap, and a badge with no room is dropped.
pub(crate) fn places(
    plot: Rect,
    layout: Layout,
    at: [Option<f64>; 3],
    widths: [f32; 3],
    height: f32,
) -> [Option<Rect>; 3] {
    let (length, depth) = match layout.across {
        true => (plot.width, plot.height),
        false => (plot.height, plot.width),
    };
    let along = |index: usize| match layout.across {
        true => widths[index],
        false => height,
    };
    let thick = |index: usize| match layout.across {
        true => height,
        false => widths[index],
    };
    let fits = |index: usize| along(index) <= length && thick(index) + 2. * MARGIN <= depth;
    let clamp = |from: f32, index: usize| from.clamp(0., length - along(index));
    let first = at[0]
        .filter(|_| fits(0))
        .map(|edge| clamp(edge as f32 * length - along(0), 0));
    let mut second = at[1]
        .filter(|_| fits(1))
        .map(|edge| clamp(edge as f32 * length, 1));
    if let (Some(first), Some(from)) = (first, second) {
        let past = first + along(0) + GAP;
        second = Some(from.max(past)).filter(|&from| from + along(1) <= length);
    }
    let middle = at[2]
        .filter(|_| fits(2))
        .map(|middle| clamp(middle as f32 * length - along(2) / 2., 2));
    let far = |index: usize| depth - MARGIN - thick(index);
    let side = |index: usize, edge: bool| match edge == layout.edges_far {
        true => far(index),
        false => MARGIN,
    };
    let rect = |index: usize, from: f32| {
        let side = side(index, index < 2);
        let (x, y) = match layout.across {
            true => (from, side),
            false => (side, from),
        };
        Rect {
            x: plot.x + x,
            y: plot.y + y,
            width: widths[index],
            height,
        }
    };
    [
        first.map(|from| rect(0, from)),
        second.map(|from| rect(1, from)),
        middle.map(|from| rect(2, from)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    const PLOT: Rect = Rect {
        x: 10.,
        y: 20.,
        width: 400.,
        height: 200.,
    };

    const WIDTHS: [f32; 3] = [60., 60., 40.];

    #[test]
    fn time_edges_stand_outside_themselves_at_the_bottom_and_the_length_on_top() {
        let [start, end, length] = places(
            PLOT,
            Layout::time(false),
            [Some(0.25), Some(0.75), Some(0.5)],
            WIDTHS,
            16.,
        );
        let (start, end, length) = (start.unwrap(), end.unwrap(), length.unwrap());
        assert_eq!((start.x + start.width, start.y), (110., 200.));
        assert_eq!((end.x, end.y), (310., 200.));
        assert_eq!((length.x, length.y), (190., 24.));
    }

    #[test]
    fn a_narrow_selection_pushes_the_second_badge_past_the_first() {
        // The first badge is held inside the plot over the start of the selection.
        let [start, end, _] = places(
            PLOT,
            Layout::time(false),
            [Some(0.), Some(0.05), None],
            WIDTHS,
            16.,
        );
        let (start, end) = (start.unwrap(), end.unwrap());
        assert_eq!(end.x, start.x + start.width + GAP);
        let [_, end, _] = places(
            PLOT,
            Layout::time(false),
            [Some(0.98), Some(0.99), None],
            WIDTHS,
            16.,
        );
        assert_eq!(end, None, "no room past the first badge");
    }

    #[test]
    fn badges_stay_inside_the_plot_and_hidden_edges_have_none() {
        let [start, end, length] = places(
            PLOT,
            Layout::time(false),
            [Some(0.), None, Some(0.)],
            WIDTHS,
            16.,
        );
        assert_eq!(start.unwrap().x, PLOT.x);
        assert_eq!(end, None);
        assert_eq!(length.unwrap().x, PLOT.x);
        let [start, ..] = places(
            PLOT,
            Layout::time(false),
            [Some(0.5), None, None],
            [500., 60., 40.],
            16.,
        );
        assert_eq!(start, None, "wider than the plot");
    }

    #[test]
    fn vertical_time_puts_the_edges_on_the_left_and_the_length_on_the_right() {
        let [start, end, length] = places(
            PLOT,
            Layout::time(true),
            [Some(0.25), Some(0.75), Some(0.5)],
            WIDTHS,
            16.,
        );
        let (start, end, length) = (start.unwrap(), end.unwrap(), length.unwrap());
        assert_eq!((start.x, start.y + start.height), (14., 70.));
        assert_eq!((end.x, end.y), (14., 170.));
        assert_eq!((length.x + length.width, length.y), (406., 112.));
    }

    #[test]
    fn band_edges_stand_at_the_left_and_the_width_at_the_right() {
        let [high, low, width] = places(
            PLOT,
            Layout::band(false),
            [Some(0.25), Some(0.75), Some(0.5)],
            WIDTHS,
            16.,
        );
        let (high, low, width) = (high.unwrap(), low.unwrap(), width.unwrap());
        assert_eq!((high.x, high.y + high.height), (14., 70.));
        assert_eq!((low.x, low.y), (14., 170.));
        assert_eq!((width.x + width.width, width.y), (406., 112.));
    }

    #[test]
    fn vertical_band_edges_stand_at_the_top_and_the_width_at_the_bottom() {
        let [low, high, width] = places(
            PLOT,
            Layout::band(true),
            [Some(0.25), Some(0.75), Some(0.5)],
            WIDTHS,
            16.,
        );
        let (low, high, width) = (low.unwrap(), high.unwrap(), width.unwrap());
        assert_eq!((low.x + low.width, low.y), (110., 24.));
        assert_eq!((high.x, high.y), (310., 24.));
        assert_eq!((width.x, width.y + width.height), (190., 216.));
    }
}
