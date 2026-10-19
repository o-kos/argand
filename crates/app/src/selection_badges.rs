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
/// How far an arrowhead of the dimension line reaches along it.
const ARROW_LENGTH: f32 = 6.;
/// Half the width of an arrowhead across the line.
const ARROW_HALF: f32 = 3.;
/// Room the dimension line leaves on each side of the extent badge.
const LINE_GAP: f32 = 3.;
/// How far the dimension line runs on past an arrowhead turned outside a narrow selection.
const STUB: f32 = 8.;

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
    /// What the selection covers of the other axis, as fractions across the screen, which the badges stand beside.
    pub around: (f64, f64),
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
        let mut lines: Vec<Dimension> = Vec::new();
        let axes = [
            (&self.time, Layout::time(vertical)),
            (&self.band, Layout::band(vertical)),
        ];
        for (badges, layout) in axes {
            let Some(badges) = badges else { continue };
            let places = places(
                frame.plot,
                layout,
                Extent {
                    edges: [badges.first, badges.second],
                    visible: badges.visible,
                    around: badges.around,
                },
                badges.texts.each_ref().map(width),
                height,
            );
            if let Some(extent) = places[2] {
                let ends = [badges.first.is_some(), badges.second.is_some()];
                lines.push(dimension(frame.plot, layout, badges.visible, ends, extent));
            }
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
            for line in &lines {
                line.paint(panel, colors, window);
            }
            for (text, rect) in &shown {
                badge(text, *rect, panel, labels, colors, window, cx);
            }
        });
    }
}

/// A line from edge to edge of the visible selection through its extent badge, with an arrowhead at each edge shown.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Dimension {
    pub across: bool,
    /// Where the line runs across the other axis, in panel coordinates.
    pub at: f32,
    /// The stretches along the axis the line is drawn over, either side of the badge.
    pub segments: Vec<(f32, f32)>,
    /// Arrow tips along the axis, each with the way it points, true towards larger values.
    pub arrows: Vec<(f32, bool)>,
}

/// The dimension line of a selection `visible` along `layout`'s axis, broken by the extent badge `extent`.
pub(crate) fn dimension(
    plot: Rect,
    layout: Layout,
    visible: (f64, f64),
    ends: [bool; 2],
    extent: Rect,
) -> Dimension {
    let (origin, length, badge_from, badge_to, at) = match layout.across {
        true => (
            plot.x,
            plot.width,
            extent.x,
            extent.x + extent.width,
            extent.y + extent.height / 2.,
        ),
        false => (
            plot.y,
            plot.height,
            extent.y,
            extent.y + extent.height,
            extent.x + extent.width / 2.,
        ),
    };
    let from = origin + visible.0 as f32 * length;
    let to = origin + visible.1 as f32 * length;
    // A narrow selection takes its arrowheads outside, pointing in, with the line running on past them.
    let inside = fits_between(to - from, badge_to - badge_from);
    let (line_from, line_to) = match inside {
        true => (from, to),
        false => (from - ARROW_LENGTH - STUB, to + ARROW_LENGTH + STUB),
    };
    let segments = [
        (line_from, badge_from - LINE_GAP),
        (badge_to + LINE_GAP, line_to),
    ]
    .into_iter()
    .filter(|(start, end)| end > start)
    .collect();
    let arrows = [(ends[0], from, !inside), (ends[1], to, inside)]
        .into_iter()
        .filter(|(shown, _, _)| *shown)
        .map(|(_, tip, forward)| (tip, forward))
        .collect();
    Dimension {
        across: layout.across,
        at,
        segments,
        arrows,
    }
}

impl Dimension {
    /// A paper stroke around an ink core, as the Alt guides are drawn, and filled arrowheads.
    fn paint(&self, panel: Point<Pixels>, [ink, paper]: [Hsla; 2], window: &mut Window) {
        let screen = |along: f32, across: f32| match self.across {
            true => panel + point(px(along), px(across)),
            false => panel + point(px(across), px(along)),
        };
        for (width, color) in [(3., paper), (1., ink)] {
            for &(start, end) in &self.segments {
                let corner = screen(start, self.at - width / 2.);
                let extent = match self.across {
                    true => size(px(end - start), px(width)),
                    false => size(px(width), px(end - start)),
                };
                window.paint_quad(fill(Bounds::new(corner, extent), color));
            }
        }
        for &(tip, forward) in &self.arrows {
            let back = if forward { -ARROW_LENGTH } else { ARROW_LENGTH };
            for (grow, color) in [(1., paper), (0., ink)] {
                let points = [
                    screen(tip - back.signum() * grow, self.at),
                    screen(tip + back, self.at - ARROW_HALF - grow),
                    screen(tip + back, self.at + ARROW_HALF + grow),
                ];
                let mut path = gpui_kit::PathBuilder::fill();
                path.add_polygon(&points, true);
                if let Ok(path) = path.build() {
                    window.paint_path(path, color);
                }
            }
        }
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

/// Where a selection lies along one axis and across the other, as fractions of the plot.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Extent {
    /// The two edges along the axis, none for one not shown.
    pub edges: [Option<f64>; 2],
    /// The visible part along the axis.
    pub visible: (f64, f64),
    /// The part of the other axis it covers.
    pub around: (f64, f64),
}

/// Whether an extent badge `badge` long and its arrowheads fit between edges `length` apart.
fn fits_between(length: f32, badge: f32) -> bool {
    length >= badge + 2. * (ARROW_LENGTH + LINE_GAP)
}

/// Where the first edge's, the second edge's and the extent's badges go, each none where it is not shown.
///
/// The edge badges stand outside their edges and just beyond the selection on
/// the side `layout` names, the extent badge beyond it on the other side,
/// over the middle of the visible selection, or past its second edge where it
/// and its arrowheads do not fit between the edges. Badges stay inside the
/// plot, pressed against the selection where there is no room beyond it, the
/// second edge's badge goes past the first's where they would overlap, and a
/// badge with no room is dropped.
pub(crate) fn places(
    plot: Rect,
    layout: Layout,
    extent: Extent,
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
    let first = extent.edges[0]
        .filter(|_| fits(0))
        .map(|edge| clamp(edge as f32 * length - along(0), 0));
    let mut second = extent.edges[1]
        .filter(|_| fits(1))
        .map(|edge| clamp(edge as f32 * length, 1));
    if let (Some(first), Some(from)) = (first, second) {
        let past = first + along(0) + GAP;
        second = Some(from.max(past)).filter(|&from| from + along(1) <= length);
    }
    let (start, end) = (
        extent.visible.0 as f32 * length,
        extent.visible.1 as f32 * length,
    );
    let beside = ARROW_LENGTH + STUB + LINE_GAP;
    let middle = match fits_between(end - start, along(2)) {
        true => (start + end - along(2)) / 2.,
        false if end + beside + along(2) <= length => end + beside,
        false => start - beside - along(2),
    };
    let middle = fits(2).then_some(clamp(middle, 2));
    // Beyond the selection on the far side or the near one, pressed back inside the plot.
    let beyond = |index: usize, far: bool| match far {
        true => (extent.around.1 as f32 * depth + MARGIN).min(depth - MARGIN - thick(index)),
        false => (extent.around.0 as f32 * depth - MARGIN - thick(index)).max(MARGIN),
    };
    let rect = |index: usize, from: f32| {
        let edge = index < 2;
        let side = beyond(index, edge == layout.edges_far);
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

    /// A selection between `from` and `to` along the axis, covering all of the other.
    fn spanning(from: f64, to: f64) -> Extent {
        Extent {
            edges: [Some(from), Some(to)],
            visible: (from, to),
            around: (0., 1.),
        }
    }

    #[test]
    fn time_edges_stand_outside_themselves_at_the_bottom_and_the_length_on_top() {
        let [start, end, length] =
            places(PLOT, Layout::time(false), spanning(0.25, 0.75), WIDTHS, 16.);
        let (start, end, length) = (start.unwrap(), end.unwrap(), length.unwrap());
        assert_eq!((start.x + start.width, start.y), (110., 200.));
        assert_eq!((end.x, end.y), (310., 200.));
        assert_eq!((length.x, length.y), (190., 24.));
    }

    #[test]
    fn a_rectangle_s_badges_stand_beside_it_rather_than_at_the_plot_s_edges() {
        let extent = Extent {
            around: (0.25, 0.75),
            ..spanning(0.25, 0.75)
        };
        let [start, _, length] = places(PLOT, Layout::time(false), extent, WIDTHS, 16.);
        assert_eq!(
            start.unwrap().y,
            20. + 150. + MARGIN,
            "just under the rectangle"
        );
        assert_eq!(length.unwrap().y, 20. + 50. - MARGIN - 16., "just over it");
        let [high, _, width] = places(PLOT, Layout::band(false), extent, WIDTHS, 16.);
        assert_eq!(high.unwrap().x, 10. + 100. - MARGIN - 60., "left of it");
        assert_eq!(width.unwrap().x, 10. + 300. + MARGIN, "right of it");
    }

    #[test]
    fn a_narrow_selection_puts_its_extent_past_its_second_edge() {
        let [_, _, length] = places(PLOT, Layout::time(false), spanning(0.5, 0.52), WIDTHS, 16.);
        assert_eq!(
            length.unwrap().x,
            10. + 208. + ARROW_LENGTH + STUB + LINE_GAP
        );
        let [_, _, length] = places(PLOT, Layout::time(false), spanning(0.97, 0.98), WIDTHS, 16.);
        assert!(
            length.unwrap().x + 40. <= 10. + 388.,
            "before the first edge where nothing is left past the second"
        );
    }

    #[test]
    fn a_narrow_selection_pushes_the_second_badge_past_the_first() {
        // The first badge is held inside the plot over the start of the selection.
        let [start, end, _] = places(PLOT, Layout::time(false), spanning(0., 0.05), WIDTHS, 16.);
        let (start, end) = (start.unwrap(), end.unwrap());
        assert_eq!(end.x, start.x + start.width + GAP);
        let [_, end, _] = places(PLOT, Layout::time(false), spanning(0.98, 0.99), WIDTHS, 16.);
        assert_eq!(end, None, "no room past the first badge");
    }

    #[test]
    fn badges_stay_inside_the_plot_and_hidden_edges_have_none() {
        let extent = Extent {
            edges: [Some(0.), None],
            ..spanning(0., 1.)
        };
        let [start, end, _] = places(PLOT, Layout::time(false), extent, WIDTHS, 16.);
        assert_eq!(start.unwrap().x, PLOT.x);
        assert_eq!(end, None);
        let [start, ..] = places(
            PLOT,
            Layout::time(false),
            spanning(0.5, 0.6),
            [500., 60., 40.],
            16.,
        );
        assert_eq!(start, None, "wider than the plot");
    }

    #[test]
    fn vertical_time_puts_the_edges_on_the_left_and_the_length_on_the_right() {
        let [start, end, length] =
            places(PLOT, Layout::time(true), spanning(0.25, 0.75), WIDTHS, 16.);
        let (start, end, length) = (start.unwrap(), end.unwrap(), length.unwrap());
        assert_eq!((start.x, start.y + start.height), (14., 70.));
        assert_eq!((end.x, end.y), (14., 170.));
        assert_eq!((length.x + length.width, length.y), (406., 112.));
    }

    #[test]
    fn the_dimension_line_runs_between_the_edges_around_the_badge() {
        let extent = Rect {
            x: 190.,
            y: 24.,
            width: 40.,
            height: 16.,
        };
        let line = dimension(
            PLOT,
            Layout::time(false),
            (0.25, 0.75),
            [true, true],
            extent,
        );
        assert_eq!(line.at, 32.);
        assert_eq!(line.segments, [(110., 187.), (233., 310.)]);
        assert_eq!(line.arrows, [(110., false), (310., true)]);
        let hidden = dimension(PLOT, Layout::time(false), (0., 0.75), [false, true], extent);
        assert_eq!(
            hidden.arrows,
            [(310., true)],
            "no arrow at an edge out of view"
        );
    }

    #[test]
    fn a_narrow_selection_takes_its_arrowheads_outside_pointing_in() {
        let beside = Rect {
            x: 10. + 208. + 17.,
            y: 24.,
            width: 40.,
            height: 16.,
        };
        let line = dimension(PLOT, Layout::time(false), (0.5, 0.52), [true, true], beside);
        assert_eq!(line.arrows, [(210., true), (218., false)]);
        assert_eq!(
            line.segments,
            [(210. - ARROW_LENGTH - STUB, 235. - LINE_GAP)],
            "the line runs on to the badge past the second edge"
        );
    }

    #[test]
    fn band_edges_stand_at_the_left_and_the_width_at_the_right() {
        let [high, low, width] =
            places(PLOT, Layout::band(false), spanning(0.25, 0.75), WIDTHS, 16.);
        let (high, low, width) = (high.unwrap(), low.unwrap(), width.unwrap());
        assert_eq!((high.x, high.y + high.height), (14., 70.));
        assert_eq!((low.x, low.y), (14., 170.));
        assert_eq!((width.x + width.width, width.y), (406., 112.));
    }

    #[test]
    fn vertical_band_edges_stand_at_the_top_and_the_width_at_the_bottom() {
        let [low, high, width] =
            places(PLOT, Layout::band(true), spanning(0.25, 0.75), WIDTHS, 16.);
        let (low, high, width) = (low.unwrap(), high.unwrap(), width.unwrap());
        assert_eq!((low.x + low.width, low.y), (110., 24.));
        assert_eq!((high.x, high.y), (310., 24.));
        assert_eq!((width.x, width.y + width.height), (190., 216.));
    }
}
