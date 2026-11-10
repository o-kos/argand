use super::*;
use crate::navigation::orientation::Mode;

#[test]
fn everything_outside_the_selection_is_dimmed_once() {
    let plot = Bounds::new(point(px(0.), px(0.)), size(px(100.), px(50.)));
    let band = Bounds::new(point(px(0.), px(10.)), size(px(100.), px(20.)));
    assert_eq!(
        outside(band, plot),
        [
            Bounds::new(point(px(0.), px(0.)), size(px(100.), px(10.))),
            Bounds::new(point(px(0.), px(30.)), size(px(100.), px(20.)))
        ],
        "a band across the whole view leaves the rows above and below"
    );
    let inside = Bounds::new(point(px(10.), px(10.)), size(px(20.), px(20.)));
    let parts = outside(inside, plot);
    let area: f32 = parts
        .iter()
        .map(|part| f32::from(part.size.width) * f32::from(part.size.height))
        .sum();
    assert_eq!(
        area,
        100. * 50. - 20. * 20.,
        "the parts tile the rest of the plot"
    );
}

#[test]
fn a_rectangle_covers_its_time_and_its_band_in_both_orientations() {
    let bounds = Bounds::new(point(px(10.), px(20.)), size(px(800.), px(400.)));
    let selected = Selected {
        time: (0.25, 0.5),
        frequency: (0.1, 0.6),
    };
    let horizontal = selected_rect(bounds, Mode::Horizontal, selected, 1.);
    assert_eq!(
        horizontal,
        Bounds::new(point(px(210.), px(60.)), size(px(200.), px(200.)))
    );
    // Time runs down and frequency to the right, its top at the right edge.
    let vertical = selected_rect(bounds, Mode::Vertical, selected, 1.);
    assert_eq!(
        vertical,
        Bounds::new(point(px(330.), px(120.)), size(px(400.), px(100.)))
    );
}

#[test]
fn a_thin_band_at_the_far_edge_stays_inside_its_bounds() {
    let bounds = Bounds::new(point(px(10.), px(20.)), size(px(800.), px(300.)));
    for scale in [1., 2.] {
        let band = time_band(bounds, Mode::Horizontal, (0.9999999, 1.0), scale);
        assert!(band.right() <= bounds.right(), "{band:?}");
        assert!(f32::from(band.size.width) >= 1. / scale - 1e-4);
        let band = time_band(bounds, Mode::Vertical, (0.9999999, 1.0), scale);
        assert!(band.bottom() <= bounds.bottom(), "{band:?}");
        assert_eq!(band.size.width, bounds.size.width);
    }
    let whole = time_band(bounds, Mode::Horizontal, (0.0, 1.0), 1.);
    assert_eq!(whole, bounds);
}
