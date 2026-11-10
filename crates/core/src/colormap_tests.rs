use super::*;

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
    for map in ALL {
        for dark in [true, false] {
            let ink = map.waveform_ink(dark);
            let active = HSL::from_rgb(&unpack(ink.active));
            let muted = HSL::from_rgb(&unpack(ink.muted));
            assert!(
                (active.l - muted.l).abs() >= MIN_LIGHTNESS_SEPARATION,
                "{map} dark={dark} ink={ink:?}"
            );
        }
    }
}

#[test]
fn gradient_ends_land_on_the_declared_stops_and_get_brighter() {
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
        let lum = |c: [u8; 3]| c[0] as u32 + c[1] as u32 + c[2] as u32;
        assert!(
            lum(g[GRADIENT_SIZE - 1]) > lum(g[0]),
            "{map} should ramp dark to bright"
        );
    }
}
