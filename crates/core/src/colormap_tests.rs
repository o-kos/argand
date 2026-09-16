use super::*;
use std::str::FromStr;

const ALL: [Colormap; 6] = [
    Colormap::Oceanic,
    Colormap::Grayscale,
    Colormap::Inferno,
    Colormap::Viridis,
    Colormap::Synthwave,
    Colormap::Sunset,
];

#[test]
fn waveform_ink_separates_active_and_muted_lightness_in_both_themes() {
    const MIN_LIGHTNESS_SEPARATION: f64 = 0.39;
    const RGB_ROUNDING_TOLERANCE: f64 = 1.0 / 255.0;
    for map in ALL {
        for (dark, active_lightness, muted_lightness) in
            [(true, 0.68, 0.26), (false, 0.36, 0.76)]
        {
            let ink = map.waveform_ink(dark);
            let active = HSL::from_rgb(&unpack(ink.active));
            let muted = HSL::from_rgb(&unpack(ink.muted));
            assert!(
                (active.l - muted.l).abs() >= MIN_LIGHTNESS_SEPARATION,
                "{map} dark={dark} ink={ink:?}"
            );
            assert!((active.l - active_lightness).abs() <= RGB_ROUNDING_TOLERANCE);
            assert!((muted.l - muted_lightness).abs() <= RGB_ROUNDING_TOLERANCE);
        }
    }
}

#[test]
fn waveform_ink_preserves_signature_hue_with_bounded_saturation() {
    for (map, signature) in ALL.into_iter().zip([
        0x4DA4D5, 0x9AA3AD, 0xF98E09, 0x5EC962, 0xA537FD, 0xE03A22,
    ]) {
        let signature = HSL::from_rgb(&unpack(signature));
        for dark in [true, false] {
            let ink = map.waveform_ink(dark);
            let active = HSL::from_rgb(&unpack(ink.active));
            let muted = HSL::from_rgb(&unpack(ink.muted));
            for colour in [active, muted] {
                let hue_distance = (colour.h - signature.h + 180.0).rem_euclid(360.0) - 180.0;
                assert!(hue_distance.abs() <= 3.0, "{map} dark={dark}");
            }
            assert!((active.s - signature.s.min(0.85)).abs() <= 0.01);
            assert!((muted.s - active.s * 0.5).abs() <= 0.01);
        }
    }
}

#[test]
fn oceanic_dark_waveform_ink_stays_close_to_the_previous_colours() {
    const MAX_CHANNEL_DISTANCE: u8 = 32;
    let ink = Colormap::Oceanic.waveform_ink(true);
    for (actual, previous) in [(ink.active, 0x78C8FF), (ink.muted, 0x243C4D)] {
        for (actual, previous) in unpack(actual).into_iter().zip(unpack(previous)) {
            assert!(actual.abs_diff(previous) <= MAX_CHANNEL_DISTANCE);
        }
    }
}

#[test]
fn every_name_round_trips() {
    for name in COLORMAP_NAMES {
        assert_eq!(Colormap::from_str(name).unwrap().to_string(), name);
    }
    assert_eq!(Colormap::from_str("GreyScale").unwrap(), Colormap::Grayscale);
    assert!(Colormap::from_str("plasma").is_err());
}

#[test]
fn gradient_ends_land_on_the_declared_stops() {
    for map in ALL {
        let g = map.gradient();
        let stops = map.stops();
        // HSL round-tripping can move a channel by one unit.
        for (got, want) in g[0].iter().zip(unpack(stops[0]).iter()) {
            assert!(got.abs_diff(*want) <= 1, "{map}: start {g:?}");
        }
        let last = unpack(stops[stops.len() - 1]);
        for (got, want) in g[GRADIENT_SIZE - 1].iter().zip(last.iter()) {
            assert!(got.abs_diff(*want) <= 1, "{map}: end");
        }
    }
}

#[test]
fn gradient_gets_brighter_from_start_to_end() {
    for map in ALL {
        let g = map.gradient();
        let lum = |c: [u8; 3]| c[0] as u32 + c[1] as u32 + c[2] as u32;
        assert!(
            lum(g[GRADIENT_SIZE - 1]) > lum(g[0]),
            "{map} should ramp dark to bright"
        );
    }
}

#[test]
fn index_clamps_outside_the_unit_range() {
    assert_eq!(gradient_index(0.0), 0);
    assert_eq!(gradient_index(1.0), GRADIENT_SIZE - 1);
    assert_eq!(gradient_index(-5.0), 0);
    assert_eq!(gradient_index(5.0), GRADIENT_SIZE - 1);
    assert_eq!(gradient_index(f32::NAN), 0);
}
