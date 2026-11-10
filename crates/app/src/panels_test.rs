use super::*;

const DEFAULT: MinimapSize = MinimapSize::Rem(3.0);

#[test]
fn a_rem_size_follows_the_font_and_not_the_window() {
    assert_eq!(waveform_height(600.0, 16.0, DEFAULT, 1.0), 48.0);
    assert_eq!(waveform_height(800.0, 16.0, DEFAULT, 1.0), 48.0);
    assert_eq!(waveform_height(600.0, 20.0, DEFAULT, 1.0), 60.0);
}

#[test]
fn a_pixel_size_ignores_the_font() {
    let size = MinimapSize::Px(64.0);
    assert_eq!(waveform_height(600.0, 16.0, size, 1.0), 64.0);
    assert_eq!(waveform_height(600.0, 20.0, size, 1.0), 64.0);
}

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
fn sizes_parse_with_either_unit_and_any_spacing() {
    for (text, size) in [
        ("4 rem", MinimapSize::Rem(4.0)),
        ("4.0rem", MinimapSize::Rem(4.0)),
        (" 2.5 REM ", MinimapSize::Rem(2.5)),
        ("64 px", MinimapSize::Px(64.0)),
        ("64px", MinimapSize::Px(64.0)),
        ("1e1 rem", MinimapSize::Rem(10.0)),
        ("+48 PX", MinimapSize::Px(48.0)),
    ] {
        assert_eq!(text.parse::<MinimapSize>(), Ok(size), "{text}");
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

#[test]
fn a_size_reads_back_what_it_prints() {
    for size in [DEFAULT, MinimapSize::Rem(2.5), MinimapSize::Px(64.0)] {
        assert_eq!(size.to_string().parse::<MinimapSize>(), Ok(size));
    }
}
