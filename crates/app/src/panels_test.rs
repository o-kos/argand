use super::*;

const DEFAULT: MinimapSize = MinimapSize::Rem(3.0);

#[test]
fn a_small_panel_bounds_the_minimap() {
    assert_eq!(waveform_height(20.0, 16.0, DEFAULT, 1.0), 20.0);
    assert_eq!(waveform_height(-5.0, 16.0, DEFAULT, 1.0), 0.0);
}

#[test]
fn the_minimap_stays_on_device_pixels_at_fractional_dpi() {
    for scale in [1.0, 1.25, 1.5, 2.0] {
        let height = waveform_height(316.0, 15.3, DEFAULT, scale);
        assert!((height * scale).fract().abs() < 1e-5);
    }
}

#[test]
fn unusable_sizes_are_refused() {
    for text in [
        "", "4", "rem", "four rem", "4 em", "0 rem", "-4 rem", "nan rem", "inf rem", "21 rem",
        "8 px", "1000 px", "4 rem px", "4 remx",
    ] {
        assert!(text.parse::<MinimapSize>().is_err(), "{text}");
    }
}
