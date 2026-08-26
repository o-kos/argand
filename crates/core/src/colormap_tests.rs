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
