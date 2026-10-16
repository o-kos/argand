//! The start, end and length of a time selection, in badges over the plot.

use super::*;
use crate::time_ruler::Counter;

/// Room between a badge and the plot's edge it stands on.
const MARGIN: f32 = 4.;
/// The least gap the end badge keeps from the start badge it is pushed past.
const GAP: f32 = 2.;
/// The badge's corner radius, as the Alt badges have it.
const RADIUS: f32 = 3.;
/// How strongly the insignificant lead of a value shows against the badge.
const DIM: f32 = 0.45;

/// A time selection's readouts and where its edges are, as fractions of the visible time.
#[derive(Debug, Clone, PartialEq)]
pub struct SelectionBadges {
    /// Start, end and length.
    pub counters: [Counter; 3],
    /// The start edge, none when it is outside the visible time.
    pub start: Option<f64>,
    pub end: Option<f64>,
    /// The visible part of the selection.
    pub visible: (f64, f64),
}

impl SelectionBadges {
    pub fn paint(
        &self,
        frame: &Frame,
        panel: Point<Pixels>,
        labels: &Labels,
        [ink, paper]: [Hsla; 2],
        window: &mut Window,
        cx: &mut App,
    ) {
        let height = labels.digit_height(LABEL_SIZE) + 2. * BADGE_PAD;
        let widths = self
            .counters
            .clone()
            .map(|counter| labels.width(&counter.text, LABEL_SIZE).ceil() + 2. * BADGE_PAD);
        let places = places(
            frame.plot,
            frame.orientation.vertical(),
            [
                self.start,
                self.end,
                Some((self.visible.0 + self.visible.1) / 2.),
            ],
            widths,
            height,
        );
        let clip = Bounds::new(
            panel + point(px(frame.plot.x), px(frame.plot.y)),
            size(px(frame.plot.width), px(frame.plot.height)),
        );
        window.with_content_mask(Some(gpui_kit::ContentMask { bounds: clip }), |window| {
            for (counter, rect) in self.counters.iter().zip(places) {
                let Some(rect) = rect else { continue };
                badge(counter, rect, panel, labels, [ink, paper], window, cx);
            }
        });
    }
}

fn badge(
    counter: &Counter,
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
    let dim = paper.opacity(DIM);
    let text = &counter.text;
    let runs = [
        (counter.dim.start, paper),
        (counter.dim.len(), dim),
        (text.len() - counter.dim.end, paper),
    ];
    let shaped = labels.shape_runs(text, &runs);
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

/// Where the start, end and length badges go, each none where it is not shown.
///
/// `at` holds the start edge, the end edge and the middle of the visible
/// selection as fractions of the visible time. The edge badges stand outside
/// their edges on the low-frequency side, the length badge over the middle on
/// the other side. Badges stay inside the plot, the end badge goes past the
/// start badge where they would overlap, and a badge with no room is dropped.
pub(crate) fn places(
    plot: Rect,
    vertical: bool,
    at: [Option<f64>; 3],
    widths: [f32; 3],
    height: f32,
) -> [Option<Rect>; 3] {
    // Along the time axis and across it, as if the plot lay horizontally.
    let (length, depth) = match vertical {
        false => (plot.width, plot.height),
        true => (plot.height, plot.width),
    };
    let along = |index: usize| match vertical {
        false => widths[index],
        true => height,
    };
    let across = |index: usize| match vertical {
        false => height,
        true => widths[index],
    };
    let fits = |index: usize| along(index) <= length && across(index) + 2. * MARGIN <= depth;
    let clamp = |from: f32, index: usize| from.clamp(0., length - along(index));
    let start = at[0]
        .filter(|_| fits(0))
        .map(|edge| clamp(edge as f32 * length - along(0), 0));
    let mut end = at[1]
        .filter(|_| fits(1))
        .map(|edge| clamp(edge as f32 * length, 1));
    if let (Some(start), Some(from)) = (start, end) {
        let past = start + along(0) + GAP;
        end = (from < past).then_some(past).or(Some(from));
        end = end.filter(|&from| from + along(1) <= length);
    }
    let middle = at[2]
        .filter(|_| fits(2))
        .map(|middle| clamp(middle as f32 * length - along(2) / 2., 2));
    // The low-frequency side is the bottom, or the left with frequency running right.
    let low = |index: usize| match vertical {
        false => depth - MARGIN - across(index),
        true => MARGIN,
    };
    let high = |index: usize| match vertical {
        false => MARGIN,
        true => depth - MARGIN - across(index),
    };
    let rect = |index: usize, from: f32, side: f32| {
        let (x, y) = match vertical {
            false => (from, side),
            true => (side, from),
        };
        Rect {
            x: plot.x + x,
            y: plot.y + y,
            width: widths[index],
            height,
        }
    };
    [
        start.map(|from| rect(0, from, low(0))),
        end.map(|from| rect(1, from, low(1))),
        middle.map(|from| rect(2, from, high(2))),
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

    #[test]
    fn edge_badges_stand_outside_their_edges_at_the_bottom_and_the_length_on_top() {
        let [start, end, length] = places(
            PLOT,
            false,
            [Some(0.25), Some(0.75), Some(0.5)],
            [60., 60., 40.],
            16.,
        );
        let (start, end, length) = (start.unwrap(), end.unwrap(), length.unwrap());
        assert_eq!((start.x + start.width, start.y), (110., 200.));
        assert_eq!((end.x, end.y), (310., 200.));
        assert_eq!((length.x, length.y), (190., 24.));
    }

    #[test]
    fn a_narrow_selection_pushes_the_end_badge_past_the_start_badge() {
        let [start, end, _] = places(
            PLOT,
            false,
            [Some(0.), Some(0.05), None],
            [60., 60., 40.],
            16.,
        );
        let (start, end) = (start.unwrap(), end.unwrap());
        assert_eq!(end.x, start.x + start.width + GAP);
        let [_, end, _] = places(
            PLOT,
            false,
            [Some(0.98), Some(0.99), None],
            [60., 60., 40.],
            16.,
        );
        assert_eq!(end, None, "no room past the start badge");
    }

    #[test]
    fn badges_stay_inside_the_plot_and_hidden_edges_have_none() {
        let [start, end, length] = places(
            PLOT,
            false,
            [Some(0.), None, Some(0.)],
            [60., 60., 40.],
            16.,
        );
        assert_eq!(start.unwrap().x, PLOT.x);
        assert_eq!(end, None);
        assert_eq!(length.unwrap().x, PLOT.x);
        let [start, ..] = places(PLOT, false, [Some(0.5), None, None], [500., 60., 40.], 16.);
        assert_eq!(start, None, "wider than the plot");
    }

    #[test]
    fn vertical_plots_put_the_edges_on_the_left_and_the_length_on_the_right() {
        let [start, end, length] = places(
            PLOT,
            true,
            [Some(0.25), Some(0.75), Some(0.5)],
            [60., 60., 40.],
            16.,
        );
        let (start, end, length) = (start.unwrap(), end.unwrap(), length.unwrap());
        assert_eq!((start.x, start.y + start.height), (14., 70.));
        assert_eq!((end.x, end.y), (14., 170.));
        assert_eq!((length.x + length.width, length.y), (406., 112.));
    }
}
