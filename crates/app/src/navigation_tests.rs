use super::*;

#[test]
fn zoom_keeps_the_pointer_sample_and_obeys_both_bounds() {
    let full = View::full(100_000);
    let zoom = full.zoom(0.5, 0.25, 100_000, 2048);
    assert_eq!(zoom, View { start: 12_500, len: 50_000 });
    assert_eq!(zoom.start + zoom.len / 4, full.len / 4);
    assert_eq!(zoom.zoom(0.00001, 0.5, 100_000, 2048).len, 2048);
    assert_eq!(zoom.zoom(100.0, 0.5, 100_000, 2048), full);
}

#[test]
fn panning_clamps_without_changing_the_span() {
    let view = View { start: 1000, len: 2000 };
    assert_eq!(view.pan(-10.0, 10_000), View { start: 0, len: 2000 });
    assert_eq!(view.pan(10.0, 10_000), View { start: 8000, len: 2000 });
    assert_eq!(view.pan(0.25, 10_000).start, 1500);
}

#[test]
fn restoration_handles_short_changed_empty_and_corrupt_captures() {
    let saved = View { start: u64::MAX, len: u64::MAX };
    assert_eq!(saved.bounded(500, 2048), View::full(500));
    assert_eq!(saved.bounded(0, 2048), View::full(0));
    assert_eq!(View { start: 500, len: 0 }.bounded(10_000, 2048), View::full(10_000));
    assert_eq!(View { start: 9500, len: 100 }.bounded(10_000, 2048), View { start: 7952, len: 2048 });
}

#[test]
fn sample_math_does_not_overflow_at_the_end_of_a_large_capture() {
    let view = View { start: u64::MAX - 16384, len: 16384 };
    assert_eq!(view.pan(100.0, u64::MAX), view);
    assert_eq!(view.zoom(0.5, 1.0, u64::MAX, 2048).start, u64::MAX - 8192);
}

#[test]
fn placeholder_and_cursor_use_the_same_physical_time_and_row() {
    assert_eq!(image_mapping((10.0, 20.0), (15.0, 25.0)), (-0.5, 1.0));
    assert_eq!(image_mapping((10.0, 20.0), (12.0, 14.0)), (-1.0, 5.0));
    let grid = DbGrid { width: 2, height: 2, values: vec![-10.0, -20.0, -30.0, -40.0],
        t0: 10.0, t1: 20.0, f0: -500.0, f1: 500.0 };
    assert_eq!(level_at(&grid, 15.0, 0.0), Some(-40.0));
    assert_eq!(level_at(&grid, 14.999, 0.99), Some(-10.0));
    assert_eq!(level_at(&grid, 20.0, 0.5), None);
    assert_eq!(level_at(&grid, 9.0, 0.5), None);
    assert_eq!(level_at(&grid, 15.0, 1.0), None);
}


#[test]
fn deep_zoom_clips_source_columns_before_gpu_conversion() {
    let held = (0.0, 86_400.0);
    let shown = (43_199.999_999, 43_200.000_001);
    assert_eq!(visible_columns(held, shown, 2000), 999..1001);
    assert_eq!(column_mapping(held, shown, 2000, 999), (0.0, 0.5));
    assert_eq!(column_mapping(held, shown, 2000, 1000), (0.5, 1.0));
    assert_eq!(visible_columns(held, (90_000.0, 90_001.0), 2000), 2000..2000);
}


#[test]
fn huge_capture_views_keep_distinct_seconds_after_zoom_and_restore() {
    for total in [1_u64 << 53, 1_u64 << 60, u64::MAX] {
        for rate in [1.0, 24_000_000.0, 1_000_000_000.0] {
            let view = View { start: total - 2, len: 2 }.bounded(total, 2);
            let (lo, hi) = view.seconds(rate);
            assert!(hi > lo, "{total} at {rate}: {view:?}");
            let zoom = view.zoom(0.01, 1.0, total, 2);
            assert_eq!(zoom, view);
        }
    }
}

#[test]
fn cursor_precision_covers_rf_time_resolution() {
    assert_eq!(time_precision(40.0 / 1500.0), 3);
    assert_eq!(time_precision(2048.0 / 24e6 / 1500.0), 8);
    assert_eq!(time_precision(2.0 / 24e6 / 1500.0), 9);
}
