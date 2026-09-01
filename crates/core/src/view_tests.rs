use super::*;

#[test]
fn image_starts_transparent_and_stores_pixels() {
    let mut img = SpectrogramImage::new(4, 3);
    assert_eq!(img.rgba.len(), 4 * 3 * 4);
    assert_eq!(img.get(0, 0), [0, 0, 0, 0]);

    img.put(3, 2, [10, 20, 30]);
    assert_eq!(img.get(3, 2), [10, 20, 30, 255]);
    assert_eq!(img.get(2, 2), [0, 0, 0, 0]);
}

fn psd(freqs: &[f64], db: &[f32]) -> Psd {
    Psd {
        freqs_hz: freqs.to_vec(),
        db: db.to_vec(),
        segments: 1,
    }
}

#[test]
fn peak_reports_bin_offset_and_absolute_frequency() {
    let center = 12_579_000.0;
    let p = psd(
        &[center - 2000.0, center, center + 2404.0],
        &[-40.0, -60.0, -11.4],
    );
    let peak = p.peak(center).unwrap();
    assert_eq!(peak.bin, 2);
    assert_eq!(peak.offset_hz, 2404.0);
    assert_eq!(peak.freq_hz, center + 2404.0);
    assert!((peak.db - -11.4).abs() < 1e-6);
    // -11.4 dB is about 0.269 of full scale.
    assert!((peak.magnitude - 0.2692).abs() < 1e-3, "{}", peak.magnitude);
}

#[test]
fn empty_spectrum_has_no_peak_or_floor() {
    let p = psd(&[], &[]);
    assert!(p.peak(0.0).is_none());
    assert!(p.floor_db().is_none());
}

#[test]
fn floor_ignores_a_lone_strong_carrier() {
    let p = psd(&[0.0, 1.0, 2.0, 3.0, 4.0], &[-87.0, -86.0, -87.2, -88.0, 0.0]);
    assert_eq!(p.floor_db().unwrap(), -87.0);
}

#[test]
fn an_envelope_addresses_channels_within_a_column() {
    let mut env = WaveformEnvelope::new(3, 2);
    assert_eq!(env.min.len(), 6);
    assert_eq!(env.column(0, 0), Some((0.0, 0.0)));

    // Column 1: I spans [-0.5, 0.5], Q spans [-0.1, 0.9].
    env.min[2] = -0.5;
    env.max[2] = 0.5;
    env.min[3] = -0.1;
    env.max[3] = 0.9;
    assert_eq!(env.column(1, 0), Some((-0.5, 0.5)));
    assert_eq!(env.column(1, 1), Some((-0.1, 0.9)));

    assert_eq!(env.column(1, 2), None, "no third channel");
    assert_eq!(env.column(3, 0), None, "no fourth column");
    assert_eq!(env.peak(), 0.9);
}

#[test]
fn a_grid_addresses_a_bin_within_a_column() {
    // Column-major: a whole column of bins, then the next column.
    let grid = DbGrid {
        width: 3,
        height: 2,
        values: vec![-10.0, -20.0, -30.0, -40.0, -50.0, -60.0],
        t0: 0.0,
        t1: 1.0,
        f0: -12_000.0,
        f1: 12_000.0,
    };
    assert_eq!(grid.value(0, 0), Some(-10.0));
    assert_eq!(grid.value(0, 1), Some(-20.0));
    assert_eq!(grid.value(2, 1), Some(-60.0));
    assert_eq!(grid.column(1), Some([-30.0, -40.0].as_slice()));

    // One bin past a column is a valid offset into the next one, so asking
    // for it has to fail rather than answer with the neighbour's value.
    assert_eq!(grid.value(0, 2), None);
    assert_eq!(grid.value(3, 0), None);
    assert_eq!(grid.column(3), None);
}
