use super::*;

#[test]
fn fractional_time_cells_use_frame_counts_and_keep_boundary_peaks() {
    let mut store = ColumnStore::new(3, 1, Reduce::MeanPower);
    store.counts = vec![4, 3, 3];
    store.power = vec![4.0, 27.0, 75.0];
    let time = Overlaps::new(10, 3, 2, true);
    let freq = Overlaps::new(1, 1, 1, false);
    let db = rebin(&store, &time, &freq);
    for (actual, power) in db.iter().zip([2.6f64, 18.6]) {
        assert!((f64::from(*actual) - 10.0 * power.log10()).abs() < 1e-5);
    }
    store.reduce = Reduce::Max;
    store.values = vec![1.0, 3.0, 5.0];
    let db = rebin(&store, &time, &freq);
    assert!((db[0] - 20.0 * 3.0f32.log10()).abs() < 1e-5);
    assert!((db[1] - 20.0 * 5.0f32.log10()).abs() < 1e-5);
}

#[test]
fn unequal_frequency_groups_are_weighted_by_bin_overlap() {
    let mut store = ColumnStore::new(1, 2, Reduce::MeanPower);
    store.counts = vec![2];
    store.power = vec![2.0, 18.0];
    let values = rebin(
        &store,
        &Overlaps::new(2, 1, 1, true),
        &Overlaps::new(5, 2, 3, false),
    );
    for (actual, power) in values.iter().zip([1.0f64, 5.0, 9.0]) {
        assert!((f64::from(*actual) - 10.0 * power.log10()).abs() < 1e-5);
    }
    let whole = rebin(
        &store,
        &Overlaps::new(2, 1, 1, true),
        &Overlaps::new(5, 2, 1, false),
    );
    assert!((f64::from(whole[0]) - 10.0 * 5.8f64.log10()).abs() < 1e-5);
}

#[test]
fn envelope_rebin_keeps_a_peak_crossing_a_cache_boundary() {
    let source = WaveformEnvelope {
        columns: 3,
        channels: 2,
        min: vec![0.0, 0.0, -0.8, -0.4, 0.0, 0.0],
        max: vec![0.0, 0.0, 0.9, 0.3, 0.0, 0.0],
        t0: 0.0,
        t1: 10.0,
    };
    let view = rebin_waveform(source, 2, 10);
    assert_eq!(view.min, vec![-0.8, -0.4, -0.8, -0.4]);
    assert_eq!(view.max, vec![0.9, 0.3, 0.9, 0.3]);
}
