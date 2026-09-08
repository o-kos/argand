#[test]
fn progressive_final_matches_plain_analysis_in_both_domains_and_reducers() {
    for domain in [Domain::Real, Domain::Iq] {
        let mut values = match domain {
            Domain::Real => real_tone(FFT * 64 + 73, TONE_HZ, 0.4),
            Domain::Iq => iq_tone(FFT * 64 + 73, TONE_HZ, 0.4),
        };
        values[FFT * 3 + 27] = 0.95;
        *values.last_mut().unwrap() = -0.8;
        for reduce in [Reduce::Max, Reduce::Mean] {
            for width in [1, 7, 128, 257] {
                assert_progressive_matches(domain, reduce, width, &values);
            }
        }
    }
}

fn assert_progressive_matches(domain: Domain, reduce: Reduce, width: usize, values: &[f32]) {
    assert_progressive_options_match(domain, reduce, width, values, ProgressiveOptions::default());
}

fn assert_progressive_options_match(
    domain: Domain,
    reduce: Reduce,
    width: usize,
    values: &[f32],
    options: ProgressiveOptions,
) {
    let mut source = VecSource::new(domain, values.to_vec(), 100_000.0);
    let mut request = request(
        width,
        33,
        SampleRange::new(17, source.meta.len_samples - 17),
    );
    request.waveform_columns = Some(width);
    request.reduce = reduce;
    request.dynamic_range = DynamicRange::Auto;
    let plain = analyze(&mut source, &request, &mut |_, _| {}).unwrap();
    let mut previews = 0;
    let refined = analyze_progressive_with_options(
        &mut source,
        &request,
        options,
        &|| Flow::Continue,
        &mut |preview, coverage| {
            assert_eq!(coverage.width, width);
            assert_eq!(preview.db.t0, plain.db.t0);
            assert_eq!(preview.db.t1, plain.db.t1);
            assert_eq!(
                preview.psd.peak(100_000.0).unwrap().bin,
                plain.psd.peak(100_000.0).unwrap().bin
            );
            let shading = Shading {
                colormap: request.colormap,
                db_min: preview.spectrogram.db_min,
                db_max: preview.spectrogram.db_max,
            };
            assert_eq!(preview.spectrogram.rgba, shade(&preview.db, shading).rgba);
            previews += 1;
            Flow::Continue
        },
    )
    .unwrap();
    assert!(previews > 0);
    assert_eq!(refined.frames, plain.frames);
    assert_eq!(refined.time_peak, plain.time_peak);
    assert_eq!(refined.enbw_hz, plain.enbw_hz);
    let a = refined.waveform.as_ref().unwrap();
    let b = plain.waveform.as_ref().unwrap();
    assert_eq!(a.min, b.min);
    assert_eq!(a.max, b.max);
    assert_eq!(
        (a.t0, a.t1, a.columns, a.channels),
        (b.t0, b.t1, b.columns, b.channels)
    );
    assert_eq!(refined.dynamic_range, plain.dynamic_range);
    for (a, b) in refined.db.values.iter().zip(&plain.db.values) {
        assert!(
            (a - b).abs() < 0.0001 || a == b,
            "{domain:?}/{reduce:?}/{width}: {a} != {b}"
        );
    }
    for (a, b) in refined.psd.db.iter().zip(&plain.psd.db) {
        assert!((a - b).abs() < 0.0001, "PSD: {a} != {b}");
    }
}

#[test]
fn progressive_preview_reads_only_selected_frames_and_can_stop_before_refinement() {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    struct Counted {
        inner: VecSource,
        read: Arc<AtomicUsize>,
    }
    impl SampleSource for Counted {
        fn meta(&self) -> &SignalMeta {
            self.inner.meta()
        }
        fn seek(&mut self, sample: u64) -> Result<(), SourceError> {
            self.inner.seek(sample)
        }
        fn read(&mut self, buf: &mut [f32]) -> Result<usize, SourceError> {
            let got = self.inner.read(buf)?;
            self.read.fetch_add(got, Ordering::Relaxed);
            Ok(got)
        }
    }
    let read = Arc::new(AtomicUsize::new(0));
    let mut source = Counted {
        inner: VecSource::new(Domain::Iq, iq_tone(FFT * 1000, TONE_HZ, 0.4), 0.0),
        read: read.clone(),
    };
    let request = request(320, 32, SampleRange::new(0, source.meta().len_samples));
    let result = analyze_progressive(
        &mut source,
        &request,
        &|| Flow::Continue,
        &mut |_, coverage| {
            assert_eq!(coverage.refined_columns, 0);
            assert_eq!(read.load(Ordering::Relaxed), 128 * FFT * 2);
            Flow::Stop
        },
    );
    assert!(matches!(result, Err(DspError::Cancelled)));
    assert_eq!(read.load(Ordering::Relaxed), 128 * FFT * 2);
}

#[test]
fn cancelled_request_reads_no_samples() {
    let mut source = VecSource::new(Domain::Real, real_tone(FFT * 2, TONE_HZ, 0.5), 0.0);
    let request = request(8, 16, SampleRange::new(0, source.meta.len_samples));
    let result = analyze_progressive(&mut source, &request, &|| Flow::Stop, &mut |_, _| {
        panic!("cancelled preview")
    });
    assert!(matches!(result, Err(DspError::Cancelled)));
    assert_eq!(source.pos, 0);
}

#[test]
fn refinement_holds_preview_scales_until_completion() {
    use std::cell::Cell;
    let mut values = real_tone(FFT * 128, TONE_HZ, 0.1);
    values[FFT + 100] = 1000.0;
    let mut source = VecSource::new(Domain::Real, values, 0.0);
    let mut request = request(4, 32, SampleRange::new(0, source.meta.len_samples));
    request.dynamic_range = DynamicRange::Fixed(40.0);
    let pause = Cell::new(false);
    let mut initial = None;
    let mut snapshots = 0;
    let result = analyze_progressive_with_options(
        &mut source,
        &request,
        ProgressiveOptions::new(16).unwrap(),
        &|| {
            if pause.replace(false) {
                std::thread::sleep(std::time::Duration::from_millis(55));
            }
            Flow::Continue
        },
        &mut |snapshot, _| {
            let scale = (snapshot.spectrogram.db_min, snapshot.spectrogram.db_max);
            if let Some(initial) = initial {
                assert_eq!(scale, initial);
            } else {
                initial = Some(scale);
                pause.set(true);
            }
            snapshots += 1;
            Flow::Continue
        },
    )
    .unwrap();
    assert!(snapshots > 1);
    assert_eq!(result.time_peak, 1000.0);
    assert!(result.spectrogram.db_max > initial.unwrap().1);
}

#[test]
fn scheduling_preserves_fft_counts_psd_waveform_and_both_reducers() {
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();
    pool.install(|| {
        for domain in [Domain::Real, Domain::Iq] {
            let values = match domain {
                Domain::Real => real_tone(FFT * 320 + 73, TONE_HZ, 0.4),
                Domain::Iq => iq_tone(FFT * 320 + 73, TONE_HZ, 0.4),
            };
            check_scheduling_options(domain, &values);
        }
    });
}

fn check_scheduling_options(domain: Domain, values: &[f32]) {
    for reduce in [Reduce::Max, Reduce::Mean] {
        for batch in [1, 16, 256, 1024, 4096] {
            assert_progressive_options_match(
                domain,
                reduce,
                7,
                values,
                ProgressiveOptions::new(batch).unwrap(),
            );
        }
    }
}

#[test]
fn progressive_buffer_allocates_only_its_requested_capacity() {
    let mut source = VecSource::new(Domain::Iq, vec![0.0; 8192], 0.0);
    let block = Block::with_capacity(&mut source, 2048, 2, 4096);
    assert_eq!(block.buf.len(), 4096);
    assert_eq!(block.buf.capacity(), 4096);
}

#[test]
fn high_overlap_refinement_can_cancel_after_a_bounded_batch() {
    use argand_core::AccessPattern;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    struct Counted {
        inner: VecSource,
        sequential: bool,
        read: Arc<AtomicUsize>,
    }
    impl SampleSource for Counted {
        fn meta(&self) -> &SignalMeta {
            self.inner.meta()
        }
        fn seek(&mut self, sample: u64) -> Result<(), SourceError> {
            self.inner.seek(sample)
        }
        fn access_pattern(&mut self, pattern: AccessPattern) {
            self.sequential = matches!(pattern, AccessPattern::Sequential);
        }
        fn read(&mut self, buf: &mut [f32]) -> Result<usize, SourceError> {
            let got = self.inner.read(buf)?;
            if self.sequential {
                self.read.fetch_add(got, Ordering::Relaxed);
            }
            Ok(got)
        }
    }
    let read = Arc::new(AtomicUsize::new(0));
    let size = 262144;
    let mut source = Counted {
        inner: VecSource::new(Domain::Real, vec![0.1; size * 2], 0.0),
        sequential: false,
        read: read.clone(),
    };
    let mut request = request(1, 8, SampleRange::new(0, source.meta().len_samples));
    request.cfg = StftConfig {
        fft_size: size,
        hop: 1,
        window: Window::Hann,
    };
    let result = analyze_progressive_with_options(
        &mut source,
        &request,
        ProgressiveOptions::new(4096).unwrap(),
        &|| {
            if read.load(Ordering::Relaxed) > 0 {
                Flow::Stop
            } else {
                Flow::Continue
            }
        },
        &mut |_, _| Flow::Continue,
    );
    assert!(matches!(result, Err(DspError::Cancelled)));
    assert_eq!(read.load(Ordering::Relaxed), size + 3);
}
