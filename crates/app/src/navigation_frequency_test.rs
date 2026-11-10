use super::*;

#[test]
fn cropped_frequency_lookup_and_backdrop_holes_share_physical_coordinates() {
    let grid = DbGrid {
        width: 2,
        height: 4,
        values: vec![-10., -20., -30., -40., -50., -60., -70., -80.],
        t0: 0.,
        t1: 2.,
        f0: -200.,
        f1: 200.,
    };
    let shown = PictureView {
        time: (0., 2.),
        frequency: (-100., 100.),
    };
    assert_eq!(level_in_view(&grid, shown, 0.25, 0.25), grid.value(0, 2));
    assert_eq!(level_in_view(&grid, shown, 0.75, 0.75), grid.value(1, 1));
    let outside = PictureView {
        frequency: (200., 400.),
        ..shown
    };
    assert_eq!(level_in_view(&grid, outside, 0.5, 0.5), None);
    let held = PictureView {
        time: (0.5, 1.5),
        frequency: (-50., 50.),
    };
    assert_eq!(
        uncovered_picture(shown, Some(held)),
        vec![
            (0., 0., 0.25, 1.),
            (0.75, 0., 1., 1.),
            (0.25, 0.75, 0.75, 1.),
            (0.25, 0., 0.75, 0.25),
        ]
    );
}
#[test]
fn distant_or_touching_windows_are_rejected_before_gpu_coordinate_mapping() {
    let held = PictureView {
        time: (0., 0.001),
        frequency: (-12000., 12000.),
    };
    for shown in [
        PictureView {
            time: (1e12, 1e12 + 0.001),
            ..held
        },
        PictureView {
            time: (0.001, 0.002),
            ..held
        },
        PictureView {
            frequency: (12000., 13000.),
            ..held
        },
        PictureView {
            frequency: (0., 0.),
            ..held
        },
    ] {
        assert!(!held.intersects(shown));
    }
    assert!(held.intersects(PictureView {
        time: (0.0005, 0.0015),
        frequency: (1000., 2000.)
    }));
}
