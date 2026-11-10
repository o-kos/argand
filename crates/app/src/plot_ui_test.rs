use super::*;

fn spectrum() -> Bounds<Pixels> {
    Bounds::new(point(px(30.), px(20.)), size(px(640.), px(300.)))
}

const ORIENTATIONS: [crate::orientation::Mode; 2] = [
    crate::orientation::Mode::Horizontal,
    crate::orientation::Mode::Vertical,
];

fn bottom_left_row() -> axes::Rect {
    axes::Rect {
        x: 30. + SCALE_INSET,
        y: 20. + 300. - SCALE_INSET - SCALE_BUTTON,
        width: SCALE_PAIR,
        height: SCALE_BUTTON,
    }
}

fn top_right_column() -> axes::Rect {
    axes::Rect {
        x: 30. + 640. - SCALE_INSET - SCALE_BUTTON,
        y: 20. + SCALE_INSET,
        width: SCALE_BUTTON,
        height: SCALE_PAIR,
    }
}

#[test]
fn horizontal_corner_zones_put_time_bottom_left_and_frequency_top_right() {
    let zones = corner_zones(spectrum(), crate::orientation::Mode::Horizontal, true);
    assert_eq!(zones, [Some(bottom_left_row()), Some(top_right_column())]);
}

#[test]
fn vertical_corner_zones_put_time_top_right_and_frequency_bottom_left() {
    // Time runs down the right ruler and frequency along the bottom one.
    let zones = corner_zones(spectrum(), crate::orientation::Mode::Vertical, true);
    assert_eq!(zones, [Some(top_right_column()), Some(bottom_left_row())]);
}

#[test]
fn hidden_scale_controls_leave_no_zones() {
    for orientation in ORIENTATIONS {
        for zone in corner_zones(spectrum(), orientation, false) {
            assert!(zone.is_none(), "a hidden toggle leaves no pair");
        }
    }
}

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
