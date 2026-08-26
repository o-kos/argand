use super::*;
use std::str::FromStr;

const ALL: [Window; 4] = [
    Window::Hann,
    Window::Hamming,
    Window::BlackmanHarris,
    Window::Rect,
];

#[test]
fn every_name_round_trips() {
    for name in WINDOW_NAMES {
        assert_eq!(Window::from_str(name).unwrap().to_string(), name);
    }
    assert_eq!(Window::from_str("HANNING").unwrap(), Window::Hann);
    assert_eq!(Window::from_str("bh").unwrap(), Window::BlackmanHarris);
    assert!(Window::from_str("kaiser").is_err());
}

#[test]
fn windows_are_periodic_not_symmetric() {
    // The periodic form starts at zero and never returns to it: w[N-1] != w[0].
    // The symmetric form would make the last point zero again.
    let w = Window::Hann.coefficients(8);
    assert!(w[0].abs() < 1e-6, "{w:?}");
    assert!(w[7] > 0.1, "last point should not close the window: {w:?}");
    assert!((w[4] - 1.0).abs() < 1e-6, "peak at N/2: {w:?}");
}

#[test]
fn rect_is_flat_and_costs_nothing() {
    let t = WindowTable::new(Window::Rect, 16);
    assert!(t.coefficients.iter().all(|&v| v == 1.0));
    assert!((t.coherent_gain - 1.0).abs() < 1e-6);
    assert!((t.enbw_bins - 1.0).abs() < 1e-6);
}

#[test]
fn coherent_gain_matches_the_textbook_values() {
    let expected = [
        (Window::Hann, 0.5),
        (Window::Hamming, 0.54),
        (Window::BlackmanHarris, 0.35875),
        (Window::Rect, 1.0),
    ];
    for (kind, want) in expected {
        let table = WindowTable::new(kind, 4096);
        assert!(
            (table.coherent_gain - want).abs() < 1e-3,
            "{kind}: {} vs {want}",
            table.coherent_gain
        );
    }
}

#[test]
fn enbw_matches_the_textbook_values() {
    let expected = [
        (Window::Hann, 1.5),
        (Window::Hamming, 1.3628),
        (Window::BlackmanHarris, 2.0044),
        (Window::Rect, 1.0),
    ];
    for (kind, want) in expected {
        let table = WindowTable::new(kind, 4096);
        assert!(
            (table.enbw_bins - want).abs() < 1e-2,
            "{kind}: {} vs {want}",
            table.enbw_bins
        );
    }
}

#[test]
fn enbw_in_hertz_scales_with_the_transform_size() {
    // A hann window over 1024 bins at 24 kHz spreads noise over 1.5 bins.
    let table = WindowTable::new(Window::Hann, 1024);
    let want = 1.5 * 24_000.0 / 1024.0;
    assert!((table.enbw_hz(24_000.0) - want).abs() < 1e-3);
}

#[test]
fn a_zero_length_window_does_not_divide_by_zero() {
    for kind in ALL {
        let table = WindowTable::new(kind, 0);
        assert!(table.is_empty());
        assert!(table.coherent_gain.is_finite() && table.coherent_gain > 0.0);
        assert!(table.enbw_bins.is_finite());
        assert_eq!(table.enbw_hz(24_000.0), 0.0);
    }
}
