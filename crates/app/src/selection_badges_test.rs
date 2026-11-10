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
    let [start, end, length] = places(PLOT, Layout::time(false), spanning(0.25, 0.75), WIDTHS, 16.);
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
    let [start, end, length] = places(PLOT, Layout::time(false), spanning(0.5, 0.6), WIDTHS, 16.);
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
    let [start, end, length] = places(PLOT, Layout::time(true), spanning(0.25, 0.75), WIDTHS, 16.);
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
    let [high, low, width] = places(PLOT, Layout::band(false), spanning(0.25, 0.75), WIDTHS, 16.);
    let (high, low, width) = (high.unwrap(), low.unwrap(), width.unwrap());
    assert_eq!((high.x, high.y), (14., 70.));
    assert_eq!((low.x, low.y + low.height), (14., 170.));
    assert_eq!((width.x + width.width, width.y), (406., 112.));
}

#[test]
fn vertical_band_edges_stand_at_the_top_and_the_width_at_the_bottom() {
    let [low, high, width] = places(PLOT, Layout::band(true), spanning(0.25, 0.75), WIDTHS, 16.);
    let (low, high, width) = (low.unwrap(), high.unwrap(), width.unwrap());
    assert_eq!((low.x, low.y), (110., 24.));
    assert_eq!((high.x + high.width, high.y), (310., 24.));
    assert_eq!((width.x, width.y + width.height), (190., 216.));
}
