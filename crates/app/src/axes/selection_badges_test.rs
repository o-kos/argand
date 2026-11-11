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
