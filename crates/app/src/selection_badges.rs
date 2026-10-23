//! A selection's edges and extent, in badges over the plot.
//!
//! Along each selected axis the two edges are labelled just outside
//! themselves on one side of the plot, and the extent over the middle of the
//! visible selection on the other side.

use super::*;
use crate::selection_entry::Field;

/// Room between a badge and the plot's edge it stands on.
const MARGIN: f32 = 4.;
/// How far an edge's badge stands from the selection, room for the pointer reaching it.
const EDGE_GAP: f32 = MARGIN + 6.;
/// The width of a pointer's base at its badge.
const POINTER_BASE: f32 = 8.;
/// The shortest tail worth drawing, from the badge to the tip.
const SHORTEST_TAIL: f32 = 2.;
/// The paper ring around a badge and its pointer.
pub(crate) const RING: f32 = 1.;
/// The gap an input typed over a badge keeps from the selection.
const ENTRY_GAP: f32 = 1.;
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
    /// The values those texts show.
    pub fields: [Field; 3],
    /// The first edge, none when it is not shown.
    pub first: Option<f64>,
    pub second: Option<f64>,
    /// The visible part of the selection.
    pub visible: (f64, f64),
    /// What the selection covers of the other axis, as fractions across the screen, which the badges stand beside.
    pub around: (f64, f64),
    /// Whether this is the only axis selected, whose edge badges then meet no others.
    pub alone: bool,
}

/// Which way an axis runs on the screen and on which side its edges are labelled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Layout {
    /// The axis runs across the screen, not down it.
    pub across: bool,
    /// The edges stand at the far side of the other axis, the bottom or the right.
    pub edges_far: bool,
    /// A narrow selection's extent goes before its first edge, away from the other axis's extent.
    pub narrow_before: bool,
}

impl Layout {
    /// Time runs right with its edges at the bottom, or down with its edges at the left.
    fn time(vertical: bool) -> Self {
        Self {
            across: !vertical,
            edges_far: !vertical,
            narrow_before: true,
        }
    }

    /// Frequency runs up with its edges at the left, or right with its edges at the top.
    fn band(vertical: bool) -> Self {
        Self {
            across: vertical,
            edges_far: false,
            narrow_before: vertical,
        }
    }
}

/// Where a selection's badges and dimension lines go, in panel coordinates.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PlacedBadges {
    /// Each shown badge with the value it holds.
    pub badges: Vec<(Field, String, Rect)>,
    lines: Vec<Dimension>,
    /// The tails by which edge badges point at the edges they name, with the badge each belongs to.
    pointers: Vec<(Field, [(f32, f32); 4])>,
}

impl SelectionBadges {
    /// Settle every badge in the plot, time's first, and the dimension lines through the extents.
    ///
    /// `entry` is the width an input typed over a badge adds to it and the
    /// input's own height, which a badge pressed against the selection keeps clear of it.
    pub fn place(&self, frame: &Frame, labels: &Labels, entry: (f32, f32)) -> PlacedBadges {
        let height = labels.digit_height(LABEL_SIZE) + 2. * BADGE_PAD;
        let width = |text: &String| labels.width(text, LABEL_SIZE).ceil() + 2. * BADGE_PAD;
        let vertical = frame.orientation.vertical();
        let mut placed = PlacedBadges::default();
        let mut taken: Vec<Rect> = Vec::new();
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
                    alone: badges.alone,
                    clear: clearance(layout, entry, height),
                },
                badges.texts.each_ref().map(width),
                height,
            );
            let places = settle(places, &mut taken, layout, frame.plot);
            let edges = [badges.first, badges.second];
            for ((edge, rect), field) in edges.into_iter().zip(places).zip(badges.fields) {
                if let (Some(edge), Some(rect)) = (edge, rect) {
                    placed.pointers.extend(
                        pointer(frame.plot, layout, edge, badges.around, rect)
                            .map(|tail| (field, tail)),
                    );
                }
            }
            if let Some(extent) = places[2] {
                let ends = [badges.first.is_some(), badges.second.is_some()];
                placed
                    .lines
                    .push(dimension(frame.plot, layout, badges.visible, ends, extent));
            }
            placed.badges.extend(
                badges
                    .fields
                    .into_iter()
                    .zip(badges.texts.iter().cloned())
                    .zip(places)
                    .filter_map(|((field, text), rect)| Some((field, text, rect?))),
            );
        }
        placed
    }
}

impl PlacedBadges {
    /// Paint the lines and the badges.
    pub fn paint(
        &self,
        frame: &Frame,
        panel: Point<Pixels>,
        labels: &Labels,
        colors: [Hsla; 2],
        window: &mut Window,
        cx: &mut App,
    ) {
        let clip = Bounds::new(
            panel + point(px(frame.plot.x), px(frame.plot.y)),
            size(px(frame.plot.width), px(frame.plot.height)),
        );
        window.with_content_mask(Some(gpui_kit::ContentMask { bounds: clip }), |window| {
            for line in &self.lines {
                line.paint(panel, colors, window);
            }
            for (field, text, rect) in &self.badges {
                let tail = self
                    .pointers
                    .iter()
                    .find(|(of, _)| of == field)
                    .map(|(_, tail)| tail);
                badge(text, (*rect, tail), panel, labels, colors, window, cx);
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

/// The tail by which an edge's badge points at its edge, in panel coordinates.
///
/// One side runs along the edge itself from the badge to the selection, and
/// the base runs from the edge into the badge, so the pointers of two edges
/// meeting at a corner stay apart. The tail reaches back over the badge's
/// corner at the edge, squaring it, and its tip stands back from the selection
/// by as much as the paper ring around it reaches past it. A badge pressed over
/// the selection has none.
pub(crate) fn pointer(
    plot: Rect,
    layout: Layout,
    edge: f64,
    around: (f64, f64),
    badge: Rect,
) -> Option<[(f32, f32); 4]> {
    let (along, length, depth_origin, depth) = match layout.across {
        true => (plot.x, plot.width, plot.y, plot.height),
        false => (plot.y, plot.height, plot.x, plot.width),
    };
    let ((start, end), (from, to)) = match layout.across {
        true => (
            (badge.y, badge.y + badge.height),
            (badge.x, badge.x + badge.width),
        ),
        false => (
            (badge.x, badge.x + badge.width),
            (badge.y, badge.y + badge.height),
        ),
    };
    let near = depth_origin + around.0 as f32 * depth;
    let far = depth_origin + around.1 as f32 * depth;
    let (side, tip) = if start >= far {
        (start, far)
    } else if end <= near {
        (end, near)
    } else {
        return None;
    };
    let edge = along + edge as f32 * length;
    // The badge runs from its edge towards the other, and so does the base.
    let base = match (from + to) / 2. >= edge {
        true => edge + POINTER_BASE,
        false => edge - POINTER_BASE,
    };
    tail(layout.across, edge, side, tip, base, RADIUS)
}

/// A tail from a shape's side at `side` to `tip`, one side running along `edge` and the base to `base`.
///
/// Coordinates are along and across the edge, which runs across the screen
/// when `across` holds. The tail reaches back over the shape's corner of
/// `radius` at the edge, squaring it, and its tip stands back by as much as
/// the ring around it reaches past it.
/// None where the room is too short for a tail to show.
pub(crate) fn tail(
    across: bool,
    edge: f32,
    side: f32,
    tip: f32,
    base: f32,
    radius: f32,
) -> Option<[(f32, f32); 4]> {
    let reach = (tip - side).abs();
    let inset = tip_inset(reach, (base - edge).abs());
    let length = reach - inset;
    // A reach so short that the inset comes out negative or not at all leaves none.
    if length.is_nan() || length < SHORTEST_TAIL {
        return None;
    }
    let towards = (tip - side).signum();
    let tip = tip - towards * inset;
    let corner = side - towards * radius;
    let point = |along: f32, depth: f32| match across {
        true => (along, depth),
        false => (depth, along),
    };
    Some([
        point(edge, corner),
        point(edge, tip),
        point(base, side),
        point(base, corner),
    ])
}

/// How far a tail `reach` long on a base `base` wide stands back so that the ring around its tip ends where it points.
fn tip_inset(reach: f32, base: f32) -> f32 {
    // The ring's tip lies RING / tan(half the tip's angle) beyond the tail's own.
    (0..4).fold(0., |inset: f32, _| {
        let half = (base / (reach - inset)).atan() / 2.;
        RING / half.tan()
    })
}

/// A convex polygon with every side moved `by` outward.
fn grown(points: &[(f32, f32)], by: f32) -> Vec<(f32, f32)> {
    // A corner of no radius leaves two points in one place, which make no side.
    let mut points: Vec<(f32, f32)> = points.to_vec();
    points.dedup();
    if points.len() > 1 && points.first() == points.last() {
        points.pop();
    }
    let count = points.len();
    if count < 3 {
        return points;
    }
    let n = count as f32;
    let centre = points
        .iter()
        .fold((0., 0.), |(x, y), p| (x + p.0 / n, y + p.1 / n));
    // Each side as a point on it and its direction, moved outward.
    let sides: Vec<((f32, f32), (f32, f32))> = (0..count)
        .map(|index| {
            let (a, b) = (points[index], points[(index + 1) % count]);
            let (dx, dy) = (b.0 - a.0, b.1 - a.1);
            let length = dx.hypot(dy);
            let (ux, uy) = (dx / length, dy / length);
            let outward = match uy * (centre.0 - a.0) - ux * (centre.1 - a.1) > 0. {
                true => -1.,
                false => 1.,
            };
            ((a.0 + uy * by * outward, a.1 - ux * by * outward), (ux, uy))
        })
        .collect();
    let cross = |a: (f32, f32), b: (f32, f32)| a.0 * b.1 - a.1 * b.0;
    (0..count)
        .map(|index| {
            let (p, u) = sides[(index + count - 1) % count];
            let (q, v) = sides[index];
            let t = cross((q.0 - p.0, q.1 - p.1), v) / cross(u, v);
            (p.0 + t * u.0, p.1 + t * u.1)
        })
        .collect()
}

/// Fill `rect` of corner `radius` and its `tail` with `fill` inside a ring of `ring` around both.
///
/// The ring goes under the whole shape first, so none falls between the badge and its tail.
pub(crate) fn callout(
    rect: Rect,
    radius: f32,
    tail: Option<&[(f32, f32); 4]>,
    panel: Point<Pixels>,
    [fill_color, ring]: [Hsla; 2],
    window: &mut Window,
) {
    let origin = panel + point(px(rect.x), px(rect.y));
    let outer = Bounds::new(
        origin - point(px(RING), px(RING)),
        size(px(rect.width + 2. * RING), px(rect.height + 2. * RING)),
    );
    window.paint_quad(fill(outer, ring).corner_radii(px(radius + RING)));
    if let Some(tail) = tail {
        fill_polygon(&grown(tail, RING), panel, ring, window);
    }
    window.paint_quad(
        fill(
            Bounds::new(origin, size(px(rect.width), px(rect.height))),
            fill_color,
        )
        .corner_radii(px(radius)),
    );
    if let Some(tail) = tail {
        fill_polygon(tail, panel, fill_color, window);
    }
}

/// Fill a polygon given in panel coordinates.
fn fill_polygon(points: &[(f32, f32)], panel: Point<Pixels>, color: Hsla, window: &mut Window) {
    let screen: Vec<_> = points
        .iter()
        .map(|&(x, y)| panel + point(px(x), px(y)))
        .collect();
    let mut path = gpui_kit::PathBuilder::fill();
    path.add_polygon(&screen, true);
    if let Ok(path) = path.build() {
        window.paint_path(path, color);
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

/// A badge and its tail as one shape.
fn badge(
    text: &str,
    (rect, tail): (Rect, Option<&[(f32, f32); 4]>),
    panel: Point<Pixels>,
    labels: &Labels,
    [ink, paper]: [Hsla; 2],
    window: &mut Window,
    cx: &mut App,
) {
    let origin = panel + point(px(rect.x), px(rect.y));
    callout(rect, RADIUS, tail, panel, [ink, paper], window);
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
    /// Whether no other axis is selected, so crowded edge badges turn outward rather than give way.
    pub alone: bool,
    /// How far a badge pressed against the selection stands from it.
    pub clear: f32,
}

/// How far a badge `height` high stands from the selection along `layout` so the input `entry` over it keeps clear.
///
/// The input is centred on its badge, wider by `entry.0` and `entry.1` high.
fn clearance(layout: Layout, (wider, high): (f32, f32), height: f32) -> f32 {
    let past = match layout.across {
        true => (high - height) / 2.,
        false => wider / 2.,
    };
    (past + ENTRY_GAP).max(MARGIN)
}

/// Edge badges past their edges, the first before it and the second after it, kept apart and inside `length`.
fn outward(edges: [Option<f64>; 2], length: f32, along: [f32; 2]) -> (Option<f32>, Option<f32>) {
    let first = edges[0].map(|edge| edge as f32 * length - along[0]);
    let second = edges[1].map(|edge| edge as f32 * length);
    let (Some(first), Some(second)) = (first, second) else {
        return (first, second);
    };
    // Without room for both side by side only the first is shown.
    if along[0] + GAP + along[1] > length {
        return (Some(first), None);
    }
    let first = first.max(0.);
    let second = second.max(first + along[0] + GAP);
    // Pressed against the far end, both slide back together.
    let over = (second + along[1] - length).max(0.);
    (Some(first - over), Some(second - over))
}

fn overlaps(a: Rect, b: Rect) -> bool {
    a.x < b.x + b.width && b.x < a.x + a.width && a.y < b.y + b.height && b.y < a.y + a.height
}

/// `places` settled one after another clear of `taken`, each joining it once placed.
///
/// An edge badge has nothing joining it to its edge but where it stands, so it
/// is dropped rather than moved; the extent badge moves along its axis, since its
/// dimension line follows it.
pub(crate) fn settle(
    places: [Option<Rect>; 3],
    taken: &mut Vec<Rect>,
    layout: Layout,
    plot: Rect,
) -> [Option<Rect>; 3] {
    let mut index = 0;
    places.map(|rect| {
        let edge = index < 2;
        index += 1;
        let rect = rect?;
        let rect = match edge {
            true => (!taken.iter().any(|taken| overlaps(rect, *taken))).then_some(rect)?,
            false => clear_of(rect, taken, layout, plot)?,
        };
        taken.push(rect);
        Some(rect)
    })
}

/// `rect` moved along `layout`'s axis to the nearest place clear of the badges `taken`, or none without one.
pub(crate) fn clear_of(rect: Rect, taken: &[Rect], layout: Layout, plot: Rect) -> Option<Rect> {
    let span = |rect: &Rect| match layout.across {
        true => (rect.x, rect.x + rect.width),
        false => (rect.y, rect.y + rect.height),
    };
    let (start, end) = match layout.across {
        true => (plot.x, plot.x + plot.width),
        false => (plot.y, plot.y + plot.height),
    };
    let (at, to) = span(&rect);
    let length = to - at;
    let moved = |place: f32| match layout.across {
        true => Rect { x: place, ..rect },
        false => Rect { y: place, ..rect },
    };
    // Where it stands, or just past either side of any badge already placed.
    let places = std::iter::once(at).chain(taken.iter().flat_map(|taken| {
        let (from, to) = span(taken);
        [to + GAP, from - GAP - length]
    }));
    places
        .filter(|&place| place >= start && place + length <= end)
        .map(moved)
        .filter(|moved| !taken.iter().any(|taken| overlaps(*moved, *taken)))
        .min_by(|a, b| {
            let distance = |rect: &Rect| (span(rect).0 - at).abs();
            distance(a).total_cmp(&distance(b))
        })
}

/// Where a badge `thick` deep stands beyond the selection on its far side or its near one, across the axis.
///
/// It is pressed back inside the plot as far from its edge as from the
/// selection, so the input typed over it stays inside too.
fn beyond_selection(extent: Extent, depth: f32, thick: f32, far: bool) -> f32 {
    let (from, to) = (extent.clear, depth - extent.clear - thick);
    match far {
        true => (extent.around.1 as f32 * depth + extent.clear).min(to),
        false => (extent.around.0 as f32 * depth - extent.clear - thick).max(from),
    }
}

/// Whether an extent badge `badge` long and its arrowheads fit between edges `length` apart.
fn fits_between(length: f32, badge: f32) -> bool {
    length >= badge + 2. * (ARROW_LENGTH + LINE_GAP)
}

/// Where the first edge's, the second edge's and the extent's badges go, each none where it is not shown.
///
/// The edge badges stand just beyond the selection on the side `layout` names,
/// each running from its edge towards the other, and are left out where the
/// edges are too close for both. The extent badge stands beyond the selection
/// on the other side, over the middle of the visible selection, or beside it
/// away from the other axis's extent where it and its arrowheads do not fit
/// between the edges. Badges stay inside the plot, pressed against the
/// selection where there is no room beyond it, and a badge with no room is
/// dropped.
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
    // Each edge's badge runs from its edge towards the other, so the corners stay free for the other axis.
    let first = extent.edges[0].map(|edge| edge as f32 * length);
    let second = extent.edges[1].map(|edge| edge as f32 * length - along(1));
    let crowded = match (first, second) {
        (Some(first), Some(second)) => second < first + along(0) + GAP,
        _ => false,
    };
    let (first, second) = match (crowded, extent.alone) {
        (false, _) => (first, second),
        // Edges too close for both badges show only the extent beside another axis's badges.
        (true, false) => (None, None),
        (true, true) => outward(extent.edges, length, [along(0), along(1)]),
    };
    let first = first.filter(|_| fits(0)).map(|from| clamp(from, 0));
    let second = second.filter(|_| fits(1)).map(|from| clamp(from, 1));
    let (start, end) = (
        extent.visible.0 as f32 * length,
        extent.visible.1 as f32 * length,
    );
    let beside = ARROW_LENGTH + STUB + LINE_GAP;
    let before = start - beside - along(2);
    let past = end + beside;
    let middle = match (fits_between(end - start, along(2)), layout.narrow_before) {
        (true, _) => (start + end - along(2)) / 2.,
        (false, true) if before >= 0. => before,
        (false, false) if past + along(2) <= length => past,
        (false, true) => past,
        (false, false) => before,
    };
    let middle = fits(2).then(|| clamp(middle, 2));
    // Beyond the selection on the far side or the near one, pressed back inside the plot.
    let beyond = |index: usize, far: bool| beyond_selection(extent, depth, thick(index), far);
    let extent_at = beyond(2, !layout.edges_far);
    let edge_at = |index: usize| -> (f32, bool) {
        let wanted = match layout.edges_far {
            true => extent.around.1 as f32 * depth + EDGE_GAP,
            false => extent.around.0 as f32 * depth - EDGE_GAP - thick(index),
        };
        if (MARGIN..=depth - MARGIN - thick(index)).contains(&wanted) {
            return (wanted, false);
        }
        let pressed = beyond(index, layout.edges_far);
        let on_extent = middle.is_some()
            && pressed < extent_at + thick(2)
            && pressed + thick(index) > extent_at;
        if !on_extent {
            return (pressed, false);
        }
        // Pressed into the other side's dimension line, the edge badges stand just past that line instead.
        let line = extent_at + thick(2) / 2.;
        let at = match layout.edges_far {
            true => line - LINE_GAP - thick(index),
            false => line + LINE_GAP,
        };
        (at.clamp(MARGIN, depth - MARGIN - thick(index)), true)
    };
    let edges = [
        first.map(|from| (from, edge_at(0))),
        second.map(|from| (from, edge_at(1))),
    ];
    // Beside the dimension line the edge badges share the axis with the extent badge, and give way to it.
    let clear_of_middle = |index: usize, from: f32| {
        middle.is_none_or(|middle| {
            from + along(index) + GAP <= middle || middle + along(2) + GAP <= from
        })
    };
    let blocked = edges.iter().enumerate().any(|(index, edge)| {
        edge.is_some_and(|(from, (_, shared))| shared && !clear_of_middle(index, from))
    });
    let rect = |index: usize, from: f32, side: f32| {
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
    let [first, second] = edges.map(|edge| edge.filter(|_| !blocked));
    [
        first.map(|(from, (side, _))| rect(0, from, side)),
        second.map(|(from, (side, _))| rect(1, from, side)),
        middle.map(|from| rect(2, from, extent_at)),
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
            alone: false,
            clear: MARGIN,
        }
    }

    #[test]
    fn a_badge_pressed_against_the_plot_s_edge_keeps_its_input_inside() {
        let clear = clearance(Layout::time(false), (4., 22.), 14.);
        let whole = Extent {
            clear,
            ..spanning(0.25, 0.75)
        };
        let [_, _, length] = places(PLOT, Layout::time(false), whole, WIDTHS, 14.);
        let length = length.unwrap();
        assert_eq!(
            length.y,
            20. + clear,
            "as far from the plot's top as from a selection"
        );
        let input_top = length.y - (22. - 14.) / 2.;
        assert_eq!(input_top, 20. + ENTRY_GAP, "the input a pixel inside");
    }

    #[test]
    fn a_tail_without_room_is_left_out_and_a_square_corner_grows_cleanly() {
        assert_eq!(
            tail(true, 10., 20.5, 20., 18., RADIUS),
            None,
            "half a pixel of room"
        );
        assert_eq!(
            tail(true, 10., 22., 20., 18., RADIUS),
            None,
            "less than its ring needs"
        );
        let square = tail(true, 10., 30., 20., 18., 0.).unwrap();
        let ring = grown(&square, RING);
        assert_eq!(ring.len(), 3, "a corner of no radius leaves a triangle");
        assert!(
            ring.iter().all(|(x, y)| x.is_finite() && y.is_finite()),
            "{ring:?}"
        );
        assert!((ring.iter().map(|p| p.1).fold(f32::MAX, f32::min) - 20.).abs() < 0.05);
    }

    #[test]
    fn a_badge_against_the_selection_keeps_its_input_a_pixel_clear() {
        let across = clearance(Layout::time(false), (4., 22.), 14.);
        assert_eq!(
            across,
            4. + ENTRY_GAP,
            "half the input's extra height and a pixel"
        );
        let beside = clearance(Layout::time(true), (4., 22.), 14.);
        assert_eq!(beside, MARGIN, "the caret's room is within the margin");
        let rectangle = Extent {
            around: (0.25, 0.5),
            clear: across,
            ..spanning(0.25, 0.75)
        };
        let [_, _, length] = places(PLOT, Layout::time(false), rectangle, WIDTHS, 14.);
        assert_eq!(
            length.unwrap().y + 14. + 4.,
            20. + 50. - ENTRY_GAP,
            "above it"
        );
    }

    #[test]
    fn time_edges_run_inward_from_themselves_below_and_the_length_on_top() {
        let [start, end, length] =
            places(PLOT, Layout::time(false), spanning(0.25, 0.75), WIDTHS, 16.);
        let (start, end, length) = (start.unwrap(), end.unwrap(), length.unwrap());
        assert_eq!((start.x, start.y), (110., 200.));
        assert_eq!((end.x + end.width, end.y), (310., 200.));
        assert_eq!((length.x, length.y), (190., 24.));
    }

    #[test]
    fn a_pointer_stands_on_its_badge_with_its_tip_on_the_edge() {
        let rectangle = Extent {
            around: (0.25, 0.5),
            ..spanning(0.25, 0.75)
        };
        let [start, end, _] = places(PLOT, Layout::time(false), rectangle, WIDTHS, 16.);
        let (start, end) = (start.unwrap(), end.unwrap());
        assert_eq!(
            start.y,
            20. + 100. + EDGE_GAP,
            "room for the pointer below the selection"
        );
        let tail = pointer(PLOT, Layout::time(false), 0.25, rectangle.around, start).unwrap();
        let inset = tip_inset(EDGE_GAP, POINTER_BASE);
        assert_eq!(
            tail,
            [
                (110., start.y + RADIUS),
                (110., 120. + inset),
                (118., start.y),
                (118., start.y + RADIUS)
            ],
            "along the edge from the badge's squared corner to the selection, its base into the badge"
        );
        let ring = grown(&tail, RING);
        assert!(
            (ring[1].0 - (110. - RING)).abs() < 1e-3 && (ring[1].1 - 120.).abs() < 0.05,
            "the ring follows the edge one pixel out and its tip ends on the selection: {:?}",
            ring[1]
        );
        assert!(
            (ring[0].0 - (110. - RING)).abs() < 1e-3,
            "the ring's square corner meets the badge's ring"
        );
        let tail = pointer(PLOT, Layout::time(false), 0.75, rectangle.around, end).unwrap();
        assert_eq!(
            tail[2],
            (302., end.y),
            "into the end's badge, which runs back from its edge"
        );
        let over = Rect { y: 80., ..start };
        assert_eq!(
            pointer(PLOT, Layout::time(false), 0.25, rectangle.around, over),
            None,
            "a badge over the selection points at nothing"
        );
    }

    #[test]
    fn a_selection_of_one_axis_turns_crowded_edge_badges_outward() {
        let alone = Extent {
            alone: true,
            ..spanning(0.5, 0.6)
        };
        let [start, end, _] = places(PLOT, Layout::time(false), alone, WIDTHS, 16.);
        let (start, end) = (start.unwrap(), end.unwrap());
        assert_eq!(start.x + start.width, 210.);
        assert!((end.x - 250.).abs() < 1e-3);
        let edge = Extent {
            alone: true,
            ..spanning(0.98, 0.99)
        };
        let [start, end, _] = places(PLOT, Layout::time(false), edge, WIDTHS, 16.);
        let (start, end) = (start.unwrap(), end.unwrap());
        assert_eq!(end.x + end.width, 410., "pressed against the plot's end");
        assert!(end.x >= start.x + start.width + GAP, "kept apart");
    }

    #[test]
    fn an_extent_too_long_for_its_axis_is_dropped_rather_than_clamped() {
        let short = Rect {
            height: 10.,
            ..PLOT
        };
        let [_, _, width] = places(short, Layout::band(false), spanning(0.2, 0.8), WIDTHS, 16.);
        assert_eq!(width, None);
    }

    #[test]
    fn a_crowded_one_axis_selection_without_room_for_both_keeps_the_first() {
        let low = Rect {
            height: 30.,
            ..PLOT
        };
        let alone = Extent {
            alone: true,
            ..spanning(0.45, 0.55)
        };
        let [high, low_edge, _] = places(low, Layout::band(false), alone, WIDTHS, 16.);
        assert!(high.is_some());
        assert_eq!(low_edge, None);
    }

    #[test]
    fn a_badge_of_one_axis_moves_clear_of_the_other_s() {
        let plot = Rect {
            x: 0.,
            y: 0.,
            width: 800.,
            height: 400.,
        };
        let time = Rect {
            x: 25.,
            y: 4.,
            width: 40.,
            height: 16.,
        };
        let width = Rect {
            x: 12.,
            y: 12.,
            width: 40.,
            height: 16.,
        };
        let moved = clear_of(width, &[time], Layout::band(false), plot).unwrap();
        assert_eq!((moved.x, moved.y), (12., 20. + GAP));
        assert!(!overlaps(moved, time));
    }

    #[test]
    fn edge_badges_in_the_way_are_dropped_and_the_extent_moves_clear() {
        let plot = Rect {
            x: 0.,
            y: 0.,
            width: 800.,
            height: 400.,
        };
        let badge = |x: f32, y: f32| Rect {
            x,
            y,
            width: 60.,
            height: 16.,
        };
        let mut taken = vec![badge(4., 4.)];
        let settled = settle(
            [
                Some(badge(4., 0.)),
                Some(badge(4., 24.)),
                Some(badge(4., 12.)),
            ],
            &mut taken,
            Layout::band(false),
            plot,
        );
        assert_eq!(
            settled[0], None,
            "an edge badge in the way is dropped, not moved off its edge"
        );
        assert_eq!(settled[1], Some(badge(4., 24.)));
        let extent = settled[2].unwrap();
        assert_ne!(
            extent,
            badge(4., 12.),
            "the extent moves, its line following it"
        );
        assert!(
            taken
                .iter()
                .enumerate()
                .all(|(index, a)| { taken[index + 1..].iter().all(|b| !overlaps(*a, *b)) })
        );
    }

    #[test]
    fn a_badge_finds_the_nearest_room_past_several_others() {
        let plot = Rect {
            x: 0.,
            y: 0.,
            width: 800.,
            height: 400.,
        };
        let at = |x: f32| Rect {
            x,
            y: 380.,
            width: 40.,
            height: 16.,
        };
        let taken = [at(736.), at(694.), at(652.)];
        let moved = clear_of(at(736.), &taken, Layout::band(true), plot).unwrap();
        assert_eq!(moved.x, 652. - GAP - 40.);
        assert!(taken.iter().all(|taken| !overlaps(moved, *taken)));
    }

    #[test]
    fn edges_too_close_for_both_badges_show_only_the_extent() {
        let [start, end, length] =
            places(PLOT, Layout::time(false), spanning(0.5, 0.6), WIDTHS, 16.);
        assert_eq!((start, end), (None, None));
        assert!(length.is_some());
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
            20. + 150. + EDGE_GAP,
            "under the rectangle, with room for its pointer"
        );
        assert_eq!(length.unwrap().y, 20. + 50. - MARGIN - 16., "just over it");
        let [high, _, width] = places(PLOT, Layout::band(false), extent, WIDTHS, 16.);
        assert_eq!(high.unwrap().x, 10. + 100. - EDGE_GAP - 60., "left of it");
        assert_eq!(width.unwrap().x, 10. + 300. + MARGIN, "right of it");
    }

    #[test]
    fn a_narrow_extent_stands_beside_the_selection_away_from_the_other_axis() {
        let beside = ARROW_LENGTH + STUB + LINE_GAP;
        let [_, _, length] = places(PLOT, Layout::time(false), spanning(0.5, 0.52), WIDTHS, 16.);
        assert_eq!(
            length.unwrap().x + 40.,
            10. + 200. - beside,
            "before the start, away from the width on the right"
        );
        let [_, _, length] = places(PLOT, Layout::time(false), spanning(0.01, 0.02), WIDTHS, 16.);
        assert_eq!(
            length.unwrap().x,
            10. + 8. + beside,
            "past the end without room before the start"
        );
        let [_, _, width] = places(PLOT, Layout::band(false), spanning(0.5, 0.52), WIDTHS, 16.);
        assert_eq!(
            width.unwrap().y,
            20. + 104. + beside,
            "below the band, away from the length on top"
        );
    }

    #[test]
    fn edge_badges_with_no_room_beside_a_narrow_selection_join_its_extent() {
        // A tall narrow rectangle against the plot's left edge, as a band.
        let extent = Extent {
            around: (0., 0.1),
            ..spanning(0.1, 0.9)
        };
        let [high, low, width] = places(PLOT, Layout::band(false), extent, WIDTHS, 16.);
        let (high, low, width) = (high.unwrap(), low.unwrap(), width.unwrap());
        let line = width.x + width.width / 2.;
        assert_eq!(high.x, line + LINE_GAP, "past the width's line");
        assert_eq!(low.x, line + LINE_GAP);
        assert_eq!((high.y, low.y + low.height), (40., 200.));
        let short = Extent {
            around: (0., 0.1),
            ..spanning(0.4, 0.6)
        };
        let [high, low, width] = places(PLOT, Layout::band(false), short, WIDTHS, 16.);
        assert_eq!((high, low), (None, None), "no room beside the width");
        assert!(width.is_some());
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
        assert_eq!((start.x, start.y), (14., 70.));
        assert_eq!((end.x, end.y + end.height), (14., 170.));
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
        assert_eq!((high.x, high.y), (14., 70.));
        assert_eq!((low.x, low.y + low.height), (14., 170.));
        assert_eq!((width.x + width.width, width.y), (406., 112.));
    }

    #[test]
    fn vertical_band_edges_stand_at_the_top_and_the_width_at_the_bottom() {
        let [low, high, width] =
            places(PLOT, Layout::band(true), spanning(0.25, 0.75), WIDTHS, 16.);
        let (low, high, width) = (low.unwrap(), high.unwrap(), width.unwrap());
        assert_eq!((low.x, low.y), (110., 24.));
        assert_eq!((high.x + high.width, high.y), (310., 24.));
        assert_eq!((width.x, width.y + width.height), (190., 216.));
    }
}
