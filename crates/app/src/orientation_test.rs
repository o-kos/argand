use super::*;

#[test]
fn vertical_time_runs_down_and_frequency_runs_right() {
    assert_eq!(Mode::Vertical.fractions(0., 0.), (0., 1.));
    assert_eq!(Mode::Vertical.fractions(1., 1.), (1., 0.));
    assert_eq!(
        Mode::Vertical.rect([0.25, 0.5, 0.5, 0.25]),
        [0.25, 0.25, 0.25, 0.5]
    );
    assert_eq!(Mode::Vertical.axes(800, 600), (600, 800));
    for mode in [Mode::Horizontal, Mode::Vertical] {
        assert_eq!(mode.toggled().toggled(), mode);
        assert_eq!(mode.rect([0., 0., 1., 1.]), [0., 0., 1., 1.]);
    }
}
