use super::*;

#[test]
fn physical_bounds_and_anchor_hold_for_real_iq_and_offset_captures() {
    for full in [(0., 12000.), (-12000., 12000.), (14000000., 14024000.)] {
        for anchor in [0., 0.25, 0.5, 1.] {
            let view = View::default().zoom(0.25, anchor, 1024);
            let hz = view.hertz(full);
            assert!(
                (hz.0 + anchor * (hz.1 - hz.0) - (full.0 + anchor * (full.1 - full.0))).abs()
                    < 1e-8
            );
            assert_eq!(view.pan(-1e20).hertz(full).0, full.0);
            assert_eq!(view.pan(1e20).hertz(full).1, full.1);
            assert_eq!(view.zoom(1e20, anchor, 1024), View::default());
        }
    }
}

#[test]
fn minimum_span_tracks_retained_resolution_and_rebounds_after_fft_changes() {
    let view = View::default().zoom(1e-20, 1., 2048);
    assert_eq!(view.span, 1. / 2048.);
    assert_eq!(view.zoom(1., 0.5, 128).span, 1. / 128.);
    assert_eq!(view.zoom(f64::NAN, 0.5, 1), view);
}
#[test]
fn physical_rounding_never_expands_the_full_band_or_collapses_a_narrow_view() {
    let full = (-2040503.3724825433, -329272.3859428355);
    assert_eq!(View::default().hertz(full), full);
    let full = (1e12, 1e12 + 0.001);
    let view = View::default().zoom(1e-20, 0.5, cells(full, 2048));
    let band = view.hertz(full);
    assert!(band.0 < band.1 && band.0 >= full.0 && band.1 <= full.1);
}
