use super::*;

#[test]
fn an_image_addresses_a_pixel_only_inside_its_shape() {
    let mut img = SpectrogramImage::new(3, 2);
    for x in 0..3 {
        img.put(x, 0, [x as u8, 0, 0]);
        img.put(x, 1, [0, x as u8, 0]);
    }

    // One past the end of a row is a valid offset into the next one, so asking
    // for it has to fail rather than answer with the neighbour's pixel.
    assert_eq!(img.get(3, 0), None, "the column past the width");
    assert_eq!(img.get(3, 1), None);
    assert_eq!(img.put(3, 0, [9, 9, 9]), None);
    assert_eq!(img.get(0, 1), Some([0, 0, 0, 255]), "row 0 was untouched");

    // The same for the row below the last one.
    assert_eq!(img.get(0, 2), None, "the row past the height");
    assert_eq!(img.put(0, 2, [9, 9, 9]), None);
    assert_eq!(img.get(2, 1), Some([0, 2, 0, 255]), "the last row is intact");

    // The far corner cannot wrap around into the buffer either.
    assert_eq!(img.get(usize::MAX, usize::MAX), None);
    assert_eq!(img.put(usize::MAX, 1, [9, 9, 9]), None);
    assert_eq!(img.rgba.len(), 3 * 2 * 4, "no write landed");
}

#[test]
fn an_image_whose_shape_does_not_fit_its_buffer_answers_nothing() {
    // The fields are the caller's to set, so a shape the buffer cannot hold
    // reaches these accessors, and a height this size overflows the offset
    // before any comparison against the length could catch it.
    let mut broken = SpectrogramImage {
        width: 2,
        height: usize::MAX,
        rgba: vec![0; 8],
        ..SpectrogramImage::new(0, 0)
    };
    assert_eq!(broken.shape(), None);
    assert_eq!(broken.get(0, 0), None);
    assert_eq!(broken.get(1, 1), None);
    assert_eq!(broken.put(0, 0, [1, 2, 3]), None);
    assert_eq!(broken.rgba, vec![0; 8]);

    // A shape that multiplies without overflowing but the buffer still falls
    // short of is refused on the same ground.
    let mut short = SpectrogramImage {
        width: 4,
        height: 4,
        rgba: vec![0; 8],
        ..SpectrogramImage::new(0, 0)
    };
    assert_eq!(short.shape(), None);
    assert_eq!(short.get(0, 0), None);
    assert_eq!(short.get(3, 3), None);
    assert_eq!(short.put(0, 0, [1, 2, 3]), None);
    assert_eq!(short.rgba, vec![0; 8]);

    // The first pixel is refused too because the image does not hold it.
    assert_eq!(broken.get(0, 0), None);
}

fn psd(freqs: &[f64], db: &[f32]) -> Psd {
    Psd {
        freqs_hz: freqs.to_vec(),
        db: db.to_vec(),
        segments: 1,
    }
}

#[test]
fn floor_ignores_a_lone_strong_carrier() {
    let p = psd(&[0.0, 1.0, 2.0, 3.0, 4.0], &[-87.0, -86.0, -87.2, -88.0, 0.0]);
    assert_eq!(p.floor_db().unwrap(), -87.0);
}

#[test]
fn an_envelope_refuses_a_column_past_its_shape_without_overflowing() {
    let mut env = WaveformEnvelope::new(3, 2);
    env.min[0] = -0.5;
    env.max[0] = 0.5;

    // A column at or past the declared width is refused on the shape, not by
    // landing on a later row's values, which is what the interleaved layout
    // made an unchecked read do.
    assert_eq!(env.column(3, 0), None, "the first column past the width");
    assert_eq!(env.column(9, 1), None, "far past the width");

    // A column whose offset would overflow the index arithmetic is refused
    // rather than wrapping into a valid-looking cell.
    assert_eq!(env.column(usize::MAX, 0), None, "an overflowing offset");
    assert_eq!(env.column(usize::MAX / 2, 1), None, "an overflowing offset");

    // A shape that multiplies without overflowing but the buffers fall short
    // of is reported by `shape` and answers nothing.
    let mut short = env.clone();
    short.columns = 9;
    assert_eq!(short.shape(), None);
    assert_eq!(short.column(3, 0), None, "a column past the buffers");

    // A shape whose product itself overflows is the case the checked
    // arithmetic exists for, where the column is inside the declared width
    // and the product wraps onto a cell the buffers do hold, so a wrapping
    // index would answer with that cell's values instead of refusing.
    let mut huge = env.clone();
    huge.columns = usize::MAX;
    huge.min.truncate(1);
    huge.max.truncate(1);
    assert_eq!(huge.shape(), None, "a product that overflows has no shape");
    assert_eq!(
        huge.column(1 << 63, 0),
        None,
        "a column whose offset wraps onto a cell the buffers hold"
    );
    assert_eq!(
        huge.min[0],
        -0.5,
        "the wrapped index would have read this cell"
    );

    let mut over = env.clone();
    over.channels = 0;
    assert_eq!(over.shape(), None, "no channel has no cell");

    assert_eq!(env.shape(), Some((3, 2)), "a whole envelope keeps its shape");

    // `shape` answers for the whole buffer and an accessor for the one cell it
    // was asked for, so buffers running longer than the declared shape still
    // leave every cell the shape does cover readable.
    let mut longer = env.clone();
    longer.min.push(-1.0);
    longer.max.push(1.0);
    assert_eq!(longer.shape(), None, "more cells than the shape declares");
    assert_eq!(
        longer.column(0, 0),
        Some((-0.5, 0.5)),
        "the cell the shape does cover is still readable"
    );
    assert_eq!(longer.column(3, 0), None, "the shape still bounds it");
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

#[test]
fn a_grid_whose_shape_does_not_fit_its_values_answers_nothing() {
    // The fields are the caller's to set, so a shape the values cannot hold
    // reaches these accessors, and a height this size overflows the offset
    // before any comparison against the length could catch it.
    let broken = DbGrid {
        width: 2,
        height: usize::MAX,
        values: vec![-10.0],
        t0: 0.0,
        t1: 1.0,
        f0: 0.0,
        f1: 1.0,
    };
    assert_eq!(broken.shape(), None);
    assert_eq!(broken.value(1, 1), None);
    assert_eq!(broken.column(1), None);
    // And the first column is refused too because the grid does not hold it.
    assert_eq!(broken.column(0), None);

    // A shape that multiplies without overflowing but the values still fall
    // short of is refused on the same ground.
    let short = DbGrid {
        width: 4,
        height: 4,
        values: vec![-10.0; 8],
        ..broken
    };
    assert_eq!(short.shape(), None);
    assert_eq!(short.column(3), None);
}

#[test]
fn waveform_pixels_merge_channels_join_steps_and_clip_at_full_scale() {
    let mut envelope = WaveformEnvelope::new(3, 2);
    envelope.min = vec![-0.8, 0.2, 0.9, 0.9, -2.0, -2.0];
    envelope.max = vec![-0.4, 0.6, 0.9, 0.9, -2.0, -2.0];
    assert_eq!(
        envelope.pixel_spans(3, 10, 1.0).collect::<Vec<_>>(),
        vec![Some((-8, 6)), Some((6, 9)), Some((-10, 6))]
    );
}

#[test]
fn held_waveform_mapping_preserves_time_and_leaves_uncovered_columns_empty() {
    let mut envelope = WaveformEnvelope::new(4, 1);
    envelope.t0 = 10.0;
    envelope.t1 = 14.0;
    envelope.min = vec![0.1, 0.2, 0.3, 0.4];
    envelope.max = envelope.min.clone();
    assert_eq!(
        envelope.pixel_spans_in(4, 10, 1.0, (10.0, 14.0)).collect::<Vec<_>>(),
        envelope.pixel_spans(4, 10, 1.0).collect::<Vec<_>>()
    );
    assert_eq!(
        envelope.pixel_spans_in(4, 10, 1.0, (12.0, 16.0)).collect::<Vec<_>>(),
        vec![Some((3, 3)), Some((3, 4)), None, None]
    );
}
