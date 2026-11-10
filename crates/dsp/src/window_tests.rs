use super::*;

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
fn coherent_gain_and_enbw_match_the_textbook_values() {
    let expected = [
        (Window::Hann, 0.5, 1.5),
        (Window::Hamming, 0.54, 1.3628),
        (Window::BlackmanHarris, 0.35875, 2.0044),
        (Window::Rect, 1.0, 1.0),
    ];
    for (kind, gain, enbw) in expected {
        let table = WindowTable::new(kind, 4096);
        assert!(
            (table.coherent_gain - gain).abs() < 1e-3,
            "{kind}: {} vs {gain}",
            table.coherent_gain
        );
        assert!(
            (table.enbw_bins - enbw).abs() < 1e-2,
            "{kind}: {} vs {enbw}",
            table.enbw_bins
        );
    }
}
