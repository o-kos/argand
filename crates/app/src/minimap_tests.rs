use super::*;

#[test]
fn rebin_keeps_single_sample_extrema_and_both_iq_channels() {
    let mut source = WaveformEnvelope::new(65536, 2);
    source.min[15999] = -0.9;
    source.max[63999] = 0.7;
    for width in [1, 31, 1000, 65537] {
        let result = rebin(&source, width);
        assert_eq!(result.min.iter().copied().fold(0., f32::min), -0.9);
        assert_eq!(result.max.iter().copied().fold(0., f32::max), 0.7);
        assert!(result.min.iter().step_by(2).all(|&value| value == 0.));
    }
}

#[test]
fn viewport_remains_visible_and_bounded_at_both_edges_and_extreme_counts() {
    for total in [1000, u64::MAX] {
        for view in [View::full(total), View { start: 0, len: 1 }, View { start: total - 1, len: 1 }] {
            let (left, right) = viewport(view, total, 1000);
            assert!(left >= 0. && right <= 1.);
            assert!(right - left >= 0.001 - 1e-12);
        }
    }
    assert_eq!(viewport(View { start: 200, len: 300 }, 1000, 1000), (0.2, 0.5));
    assert_eq!(center(View { start: 200, len: 300 }, 0., 1000).start, 0);
    assert_eq!(center(View { start: 200, len: 300 }, 1., 1000).start, 700);
}

struct Source {
    meta: argand_core::SignalMeta,
    values: Vec<f32>,
    offset: usize,
    reads: std::cell::Cell<usize>,
    values_read: usize,
    short: bool,
}

impl Source {
    fn new(iq: bool, samples: usize) -> Self {
        use argand_core::{Domain, SampleFormat, SampleType};
        let sample_type = SampleType::new(if iq { Domain::Iq } else { Domain::Real }, SampleFormat::F32);
        let mut values = vec![0.; samples * sample_type.channels()];
        values[0] = -0.5;
        values[samples / 2 * sample_type.channels()] = 0.8;
        *values.last_mut().unwrap() = -0.9;
        Self {
            meta: argand_core::SignalMeta { sample_rate: 48000., center_freq: 0., sample_type,
                len_samples: samples as u64, container: "test", divisor: 1., source: "test.wav".into() },
            values, offset: 0, reads: Default::default(), values_read: 0, short: false,
        }
    }
}

impl SampleSource for Source {
    fn meta(&self) -> &argand_core::SignalMeta { &self.meta }
    fn seek(&mut self, sample: u64) -> Result<(), argand_core::SourceError> {
        self.offset = sample as usize * self.meta.channels();
        Ok(())
    }
    fn read(&mut self, buffer: &mut [f32]) -> Result<usize, argand_core::SourceError> {
        assert!(buffer.len() <= BLOCK_VALUES);
        assert_eq!(buffer.len() % self.meta.channels(), 0);
        self.reads.set(self.reads.get() + 1);
        let count = if self.short { 0 } else { buffer.len() };
        buffer[..count].copy_from_slice(&self.values[self.offset..self.offset + count]);
        self.offset += count;
        self.values_read += count;
        Ok(count)
    }
}

#[test]
fn full_capture_scan_is_bounded_and_keeps_every_real_or_iq_extremum() {
    for iq in [false, true] {
        let mut source = Source::new(iq, 150001);
        let mut snapshots = Vec::new();
        build(&mut source, &|| true, &mut |snapshot| { snapshots.push(snapshot); true }).unwrap();
        assert_eq!(snapshots.len(), 2);
        assert!(!snapshots[0].complete);
        let full = &snapshots[1];
        assert!(full.complete);
        assert_eq!(full.envelope.columns, COLUMNS);
        assert_eq!(full.full_scale, 0.9);
        assert_eq!(full.envelope.t0, 0.);
        assert_eq!(full.envelope.t1, source.meta.duration_seconds());
        let channels = source.meta.channels();
        assert_eq!(source.values_read, source.values.len() + PREVIEW_BLOCKS as usize * PREVIEW_SAMPLES as usize * channels);
        for column in 0..COLUMNS {
            let first = (column * 150001).div_ceil(COLUMNS);
            let end = ((column + 1) * 150001).div_ceil(COLUMNS);
            for channel in 0..channels {
                let values = (first..end).map(|sample| source.values[sample * channels + channel]);
                let low = values.clone().fold(f32::INFINITY, f32::min);
                let high = values.fold(f32::NEG_INFINITY, f32::max);
                assert_eq!(full.envelope.column(column, channel), Some((low, high)));
            }
        }
    }
}

#[test]
fn cancellation_stops_before_reading_and_after_the_preview_without_a_final_snapshot() {
    let mut source = Source::new(true, 150001);
    build(&mut source, &|| false, &mut |_| panic!("cancelled")).unwrap();
    assert_eq!(source.reads.get(), 0);
    let active = std::cell::Cell::new(true);
    build(&mut source, &|| active.get(), &mut |snapshot| {
        assert!(!snapshot.complete);
        active.set(false);
        true
    }).unwrap();
    assert_eq!(source.reads.get(), PREVIEW_BLOCKS as usize);
}

#[test]
fn small_capture_has_one_exact_delivery_and_truncated_input_is_an_error() {
    let mut source = Source::new(false, 101);
    let mut count = 0;
    build(&mut source, &|| true, &mut |snapshot| {
        assert!(snapshot.complete);
        assert_eq!(snapshot.envelope.columns, 101);
        count += 1;
        true
    }).unwrap();
    assert_eq!(count, 1);
    source.short = true;
    assert!(build(&mut source, &|| true, &mut |_| panic!("incomplete")).is_err());
}

#[test]
fn minimap_clicks_share_keyboard_divisions_and_only_doubleclick_centers() {
    for (fraction, direction) in [(0.1, -1), (0.9, 1)] {
        assert_eq!(click(fraction, (0.2, 0.5), false, 1), Click::Step(direction));
        assert_eq!(click(fraction, (0.2, 0.5), true, 1), Click::Step(5 * direction));
        assert_eq!(click(fraction, (0.2, 0.5), false, 2), Click::Center);
    }
    for fraction in [0.2, 0.3, 0.5] {
        for count in [1, 2] {
            assert_eq!(click(fraction, (0.2, 0.5), false, count), Click::Grab);
            assert_eq!(click(fraction, (0.2, 0.5), true, count), Click::Grab);
        }
    }
    for count in [1, 2] {
        assert_eq!(click(0.3, (0., 1.), false, count), Click::Grab);
    }
    assert_eq!(viewport(View::full(1000), 1000, 2000), (0., 1.));
}

#[test]
fn viewport_uses_exact_sample_bounds_and_shared_device_pixel_edges() {
    assert_eq!(viewport(View { start: 201, len: 298 }, 1000, 10), (0.2, 0.5));
    for columns in [1, 17, 1000, 2000] {
        assert_eq!(viewport(View { start: u64::MAX - 1, len: 1 }, u64::MAX, columns), ((columns - 1) as f64 / columns as f64, 1.));
        assert_eq!(viewport(View { start: 0, len: 1 }, u64::MAX, columns), (0., 1. / columns as f64));
    }
}
