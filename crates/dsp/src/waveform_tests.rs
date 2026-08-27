use super::*;

/// One channel, folded in a single call.
fn envelope(values: &[f32], columns: usize) -> WaveformEnvelope {
    let mut builder = EnvelopeBuilder::new(columns, 1, values.len() as u64);
    builder.fold(values, 0);
    builder.finish(0.0, 1.0)
}

#[test]
fn a_single_sample_peak_survives_decimation() {
    // 390 samples to a column: a mean would bury this spike 50 dB down.
    let mut values = vec![0.0f32; 100_000];
    values[50_000] = 0.9;
    let env = envelope(&values, 256);

    assert_eq!(env.peak(), 0.9);
    let column = env
        .max
        .iter()
        .position(|v| *v == 0.9)
        .expect("the spike reached a column");
    assert_eq!(column, 50_000 * 256 / 100_000);
}

#[test]
fn columns_cover_the_range_in_order() {
    let values: Vec<f32> = (0..1000).map(|i| i as f32 / 1000.0).collect();
    let env = envelope(&values, 10);

    assert_eq!(env.columns, 10);
    assert_eq!(env.channels, 1);
    // A ramp: every column's span sits above the one before it.
    for column in 1..env.columns {
        let (prev_min, prev_max) = env.column(column - 1, 0).unwrap();
        let (min, max) = env.column(column, 0).unwrap();
        assert!(min > prev_min && max > prev_max, "column {column}");
    }
    assert_eq!(env.column(0, 0).unwrap().0, 0.0);
    assert_eq!(env.column(9, 0).unwrap().1, 0.999);
}

#[test]
fn i_and_q_keep_their_own_extremes() {
    // I swings wide, Q stays near zero: the two must not be merged.
    let mut values = Vec::new();
    for i in 0..100 {
        values.push(if i % 2 == 0 { 0.8 } else { -0.8 });
        values.push(0.05);
    }
    let mut builder = EnvelopeBuilder::new(4, 2, 100);
    builder.fold(&values, 0);
    let env = builder.finish(0.0, 1.0);

    for column in 0..4 {
        assert_eq!(env.column(column, 0), Some((-0.8, 0.8)), "I in {column}");
        assert_eq!(env.column(column, 1), Some((0.05, 0.05)), "Q in {column}");
    }
}

#[test]
fn columns_no_sample_reached_borrow_a_neighbour() {
    // Three samples across eight columns: five columns get nothing.
    let env = envelope(&[0.5, -0.25, 0.75], 8);
    assert!(
        env.min.iter().chain(env.max.iter()).all(|v| v.is_finite()),
        "{:?} / {:?}",
        env.min,
        env.max
    );
    assert_eq!(env.column(0, 0), Some((0.5, 0.5)));
    assert_eq!(env.column(7, 0), Some((0.75, 0.75)));
}

#[test]
fn an_envelope_of_nothing_is_flat_rather_than_infinite() {
    let env = envelope(&[], 4);
    assert!(env.min.iter().all(|v| *v == 0.0));
    assert!(env.max.iter().all(|v| *v == 0.0));
    assert_eq!(env.peak(), 0.0);
}

#[test]
fn block_boundaries_do_not_change_the_result() {
    let values: Vec<f32> = (0..997)
        .map(|i| ((i as f32) * 0.37).sin() * 0.9)
        .collect();
    let one_shot = envelope(&values, 64);

    // Boundaries chosen to land inside columns rather than on them.
    let mut builder = EnvelopeBuilder::new(64, 1, values.len() as u64);
    let mut at = 0usize;
    for step in [7usize, 113, 1, 400, 476] {
        builder.fold(&values[at..at + step], at as u64);
        at += step;
    }
    assert_eq!(at, values.len());
    let in_blocks = builder.finish(0.0, 1.0);

    assert_eq!(one_shot.min, in_blocks.min);
    assert_eq!(one_shot.max, in_blocks.max);
}
