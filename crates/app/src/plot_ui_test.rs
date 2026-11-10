use super::*;

const ORIENTATIONS: [crate::orientation::Mode; 2] = [
    crate::orientation::Mode::Horizontal,
    crate::orientation::Mode::Vertical,
];

#[test]
fn corner_zones_vanish_together_on_small_spectrums() {
    for orientation in ORIENTATIONS {
        let tiny = Bounds::new(point(px(0.), px(0.)), size(px(40.), px(40.)));
        for zone in corner_zones(tiny, orientation, true) {
            assert!(zone.is_none(), "a 40-pixel spectrum fits no pair");
        }
        // Either side alone being too small drops both pairs.
        let narrow = Bounds::new(point(px(0.), px(0.)), size(px(40.), px(400.)));
        for zone in corner_zones(narrow, orientation, true) {
            assert!(zone.is_none(), "a 40-pixel side fits no pair");
        }
        // Below the minimum the two corner pairs would overlap in the
        // middle of the picture, so the minimum is exact.
        let cramped = Bounds::new(point(px(0.), px(0.)), size(px(82.), px(82.)));
        for zone in corner_zones(cramped, orientation, true) {
            assert!(zone.is_none(), "an 82-pixel spectrum still overlaps");
        }
        let snug = Bounds::new(point(px(0.), px(0.)), size(px(83.), px(83.)));
        let [time, frequency] = corner_zones(snug, orientation, true);
        let (time, frequency) = (time.unwrap(), frequency.unwrap());
        assert!(
            time.x + time.width <= frequency.x || frequency.x + frequency.width <= time.x,
            "the horizontal spans do not overlap"
        );
        assert!(
            frequency.y + frequency.height <= time.y || time.y + time.height <= frequency.y,
            "the vertical spans do not overlap"
        );
    }
}
