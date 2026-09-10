use super::*;

#[test]
fn zoom_keeps_the_pointer_sample_and_obeys_both_bounds() {
    let full = View::full(100_000);
    let zoom = full.zoom(0.5, 0.25, 100_000, 2048, 1000);
    assert_eq!(zoom, View { start: 12_500, len: 50_000 });
    assert_eq!(zoom.start + zoom.len / 4, full.len / 4);
    assert_eq!(zoom.zoom(0.00001, 0.5, 100_000, 2048, 1000).len, 2048);
    assert_eq!(zoom.zoom(100.0, 0.5, 100_000, 2048, 1000), full);
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
    assert_eq!(saved.bounded(500, 2048, 1000), View::full(500));
    assert_eq!(saved.bounded(0, 2048, 1000), View::full(0));
    assert_eq!(View { start: 500, len: 0 }.bounded(10_000, 2048, 1000), View::full(10_000));
    assert_eq!(View { start: 9500, len: 100 }.bounded(10_000, 2048, 1000), View { start: 7952, len: 2048 });
}

#[test]
fn sample_math_does_not_overflow_at_the_end_of_a_large_capture() {
    let view = View { start: u64::MAX - 20_000_000, len: 20_000_000 };
    assert_eq!(view.pan(100.0, u64::MAX), view);
    assert_eq!(view.zoom(0.5, 1.0, u64::MAX, 2048, 1000).start, u64::MAX - 10_000_000);
}

#[test]
fn placeholder_and_cursor_use_the_same_physical_time_and_row() {
    assert_eq!(image_mapping((10.0, 20.0), (15.0, 25.0)), (-0.5, 1.0));
    assert_eq!(image_mapping((10.0, 20.0), (12.0, 14.0)), (-1.0, 5.0));
    let grid = DbGrid { width: 2, height: 2, values: vec![-10.0, -20.0, -30.0, -40.0],
        t0: 10.0, t1: 20.0, f0: -500.0, f1: 500.0 };
    assert_eq!(level_at(&grid, (15.0, 25.0), 0.0, 0.0), Some(-40.0));
    assert_eq!(level_at(&grid, (10.0, 20.0), 0.4999, 0.99), Some(-10.0));
    assert_eq!(level_at(&grid, (15.0, 25.0), 0.5, 0.5), None);
    assert_eq!(level_at(&grid, (9.0, 19.0), 0.0, 0.5), None);
    assert_eq!(level_at(&grid, (10.0, 20.0), 0.5, 1.0), None);
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
            let view = View { start: total - 2, len: 2 }.bounded(total, 2, 1000);
            let (lo, hi) = view.seconds(rate);
            assert!(hi > lo, "{total} at {rate}: {view:?}");
            let zoom = view.zoom(0.01, 1.0, total, 2, 1000);
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


#[test]
fn extreme_interior_views_pan_and_cover_every_display_column() {
    let total = u64::MAX;
    let columns = 1500;
    let view = View { start: total - 100_000_000, len: 2 }.bounded(total, 2, columns);
    for rate in [1.0, 24e6, 1e9] {
        let time = view.seconds(rate);
        let panned = view.pan(0.1, total).seconds(rate);
        let pixel_pan = view.pan(1.0 / columns as f64, total).seconds(rate);
        assert!(panned.0 > time.0 && panned.1 > time.1);
        assert!(pixel_pan.0 > time.0 && pixel_pan.1 > time.1);
        let grid = DbGrid { width: columns, height: 1,
            values: (0..columns).map(|x| x as f32).collect(),
            t0: time.0, t1: time.1, f0: 0.0, f1: 1.0 };
        let mut envelope = argand_core::WaveformEnvelope::new(columns, 1);
        envelope.t0 = time.0;
        envelope.t1 = time.1;
        envelope.min = (0..columns).map(|x| x as f32 / columns as f32).collect();
        envelope.max = envelope.min.clone();
        let spans = envelope.pixel_spans_in(columns, columns as i64, 1.0, time).collect::<Vec<_>>();
        for (x, span) in spans.iter().enumerate() {
            let at = (x as f64 + 0.5) / columns as f64;
            assert_eq!(level_at(&grid, time, at, 0.5), Some(x as f32), "column {x} at {rate}");
            assert!(span.is_some_and(|(_, high)| high == x as i64), "waveform column {x}: {span:?}");
        }
    }
}

#[test]
fn wider_picture_fills_only_time_missing_from_the_foreground() {
    assert_eq!(uncovered((0.0, 10.0), Some((2.0, 8.0))), vec![(0.0, 0.2), (0.8, 1.0)]);
    assert_eq!(uncovered((2.0, 8.0), Some((0.0, 10.0))), vec![]);
    assert_eq!(uncovered((0.0, 10.0), Some((20.0, 30.0))), vec![(0.0, 1.0)]);
    assert_eq!(uncovered((0.0, 10.0), Some((-20.0, -10.0))), vec![(0.0, 1.0)]);
    assert_eq!(uncovered((0.0, 10.0), None), vec![(0.0, 1.0)]);
    assert_eq!(uncovered((0.0, 10.0), Some((0.0, 4.0))), vec![(0.4, 1.0)]);
}

#[test]
fn keyboard_pan_accumulates_fractional_samples_without_drift() {
    let original = View { start: 10000, len: 1000 };
    let mut pan = TickPan::new(original, 7.2);
    for _ in 0..100 { pan.advance(1, 100000); }
    assert_eq!(pan.view.start, original.start + 720);
    for _ in 0..100 { pan.advance(-1, 100000); }
    assert_eq!(pan.view, original);
    pan.advance(5, 100000);
    assert_eq!(pan.view.start, original.start + 36);
}

#[test]
fn keyboard_pan_reverses_immediately_at_capture_edges() {
    let mut pan = TickPan::new(View { start: 2, len: 20 }, 7.2);
    pan.advance(-5, 100);
    assert_eq!(pan.view.start, 0);
    pan.advance(1, 100);
    assert_eq!(pan.view.start, 7);
    pan.advance(100, 100);
    assert_eq!(pan.view.start, 80);
    pan.advance(-1, 100);
    assert_eq!(pan.view.start, 73);
}

#[test]
fn sub_sample_divisions_accumulate_away_from_capture_edges() {
    let mut pan = TickPan::new(View { start: 0, len: 2 }, 0.2);
    for _ in 0..5 { pan.advance(1, 100); }
    assert_eq!(pan.view.start, 1);
    pan.advance(1000, 100);
    assert_eq!(pan.view.start, 98);
    for _ in 0..5 { pan.advance(-1, 100); }
    assert_eq!(pan.view.start, 97);
}
