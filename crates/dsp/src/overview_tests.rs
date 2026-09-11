#[test]
fn overview_matches_exact_analysis_when_every_frame_and_bin_fits_the_cache() {
    for domain in [Domain::Real, Domain::Iq] {
        for reduce in [Reduce::Max, Reduce::MeanPower] {
            check_exact_overview(domain, reduce);
        }
    }
}

fn check_exact_overview(domain: Domain, reduce: Reduce) {
            let values = match domain {
                Domain::Real => real_tone(FFT * 32 + 71, TONE_HZ, 0.4),
                Domain::Iq => iq_tone(FFT * 32 + 71, TONE_HZ, 0.4),
            };
            let mut source = VecSource::new(domain, values, 12345.0);
            let mut req = request(5, 7, SampleRange::new(13, source.meta.len_samples - 13));
            req.reduce = reduce;
            req.waveform_columns = Some(5);
            let mut overview = analyze_overview(&mut source, &req, ProgressiveOptions::default(),
                &|| Flow::Continue, &mut |state, _| {
                    let preview = state.render(11, 9, Some(11)).unwrap();
                    assert!(preview.frames > 0);
                    Flow::Continue
                }).unwrap();
            for (width, height) in [(1, 1), (7, 33), (127, 300), (200, 1100), (5, 7)] {
                req.width = width;
                req.height = height;
                req.waveform_columns = Some(width);
                let exact = analyze(&mut source, &req, &mut |_, _| {}).unwrap();
                let cached = overview.render(width, height, Some(width)).unwrap();
                assert_eq!(cached.frames, exact.frames);
                assert_eq!(cached.time_peak, exact.time_peak);
                assert_eq!(cached.enbw_hz, exact.enbw_hz);
                assert_eq!((cached.db.t0, cached.db.t1, cached.db.f0, cached.db.f1),
                    (exact.db.t0, exact.db.t1, exact.db.f0, exact.db.f1));
                for (a, b) in cached.db.values.iter().zip(&exact.db.values) {
                    assert!((a - b).abs() < 0.0001, "{domain:?} {reduce:?} {width}x{height}: {a} vs {b}");
                }
                for (a, b) in cached.psd.db.iter().zip(&exact.psd.db) {
                    assert!((a - b).abs() < 0.0001);
                }
                let a = cached.waveform.unwrap();
                let b = exact.waveform.unwrap();
                assert_eq!(a.min, b.min);
                assert_eq!(a.max, b.max);
            }

}

#[test]
fn overview_cancellation_and_invalid_reducers_are_explicit() {
    let mut source = VecSource::new(Domain::Real, real_tone(FFT * 64, TONE_HZ, 0.4), 0.0);
    let mut req = request(30, 20, SampleRange::new(0, source.meta.len_samples));
    assert!(matches!(analyze_overview(&mut source, &req, ProgressiveOptions::default(),
        &|| Flow::Continue, &mut |_, _| Flow::Stop), Err(DspError::Cancelled)));
    req.reduce = Reduce::Mean;
    assert!(matches!(analyze_overview(&mut source, &req, ProgressiveOptions::default(),
        &|| Flow::Continue, &mut |_, _| panic!("must reject before publication")), Err(DspError::BadOverviewReduction)));
}

#[test]
fn overview_resize_preserves_resolved_colour_scale() {
    let mut source = VecSource::new(Domain::Real, real_tone(FFT * 32, TONE_HZ, 0.4), 0.0);
    let mut req = request(10, 10, SampleRange::new(0, source.meta.len_samples));
    req.reduce = Reduce::MeanPower;
    req.dynamic_range = DynamicRange::Auto;
    let mut overview = analyze_overview(&mut source, &req, ProgressiveOptions::default(),
        &|| Flow::Continue, &mut |_, _| Flow::Continue).unwrap();
    let first = overview.render(10, 10, None).unwrap();
    let larger = overview.render(100, 100, None).unwrap();
    assert_eq!(first.dynamic_range, larger.dynamic_range);
    assert_eq!(first.spectrogram.db_min, larger.spectrogram.db_min);
    assert_eq!(first.spectrogram.db_max, larger.spectrogram.db_max);
    assert!(matches!(overview.render(0, 5, None), Err(DspError::BadOutputSize { .. })));
}

#[test]
fn overview_large_fft_fallback_matches_aligned_frequency_groups() {
    let mut source = VecSource::new(Domain::Iq, iq_tone(32768, TONE_HZ, 0.4), 0.0);
    for reduce in [Reduce::Max, Reduce::MeanPower] {
        let mut req = request(4, 1024, SampleRange::new(0, source.meta.len_samples));
        req.cfg = StftConfig::new(8192, Window::Hann);
        req.reduce = reduce;
        let mut cached = analyze_overview(&mut source, &req, ProgressiveOptions::default(),
            &|| Flow::Continue, &mut |_, _| Flow::Continue).unwrap();
        assert_eq!(cached.dimensions(), (13, 2048));
        let result = cached.render(4, 1024, None).unwrap();
        let exact = analyze(&mut source, &req, &mut |_, _| {}).unwrap();
        for (a,b) in result.db.values.iter().zip(&exact.db.values) {
            assert!((a-b).abs() < 0.0001, "{reduce:?}: {a} vs {b}");
        }
    }
}

#[test]
fn overview_sparse_budget_keeps_every_frame_when_time_cells_are_compressed() {
    let mut source = VecSource::new(Domain::Real, real_tone(FFT * 1200 + 37, TONE_HZ, 0.4), 0.0);
    let mut req = request(17, 9, SampleRange::new(0, source.meta.len_samples));
    for reduce in [Reduce::Max, Reduce::MeanPower] {
        req.reduce = reduce;
        let mut first = true;
        let mut cached = analyze_overview(&mut source, &req, ProgressiveOptions::default(), &|| Flow::Continue,
            &mut |state, _| {
                if first {
                    assert_eq!(state.render(17, 9, None).unwrap().frames, 128);
                    first = false;
                }
                Flow::Continue
            }).unwrap();
        assert_eq!(cached.dimensions(), (4096, 513));
        let result = cached.render(17, 9, None).unwrap();
        let exact = analyze(&mut source, &req, &mut |_, _| {}).unwrap();
        assert_eq!(result.frames, exact.frames);
        for (a,b) in result.psd.db.iter().zip(&exact.psd.db) { assert!((a-b).abs() < 0.0001); }
    }
}

#[test]
fn explicit_style_refresh_bypasses_the_periodic_snapshot_interval() {
    let mut source = VecSource::new(Domain::Iq, iq_tone(FFT * 400, TONE_HZ, 0.4), 0.0);
    let req = request(20, 20, SampleRange::new(0, source.meta.len_samples));
    let mut refined = 0;
    let mut previews = 0;
    let mut state = analyze_overview_with_refresh(&mut source, &req,
        ProgressiveOptions::new(1).unwrap(), &|| Flow::Continue, &|| true,
        &mut |state, coverage| {
            if coverage.refined_columns > 0 { refined += 1; } else { previews += 1; }
            state.set_style(Colormap::Inferno, DynamicRange::Fixed(40.0)).unwrap();
            let view = state.render(20, 20, None).unwrap();
            assert_eq!(view.dynamic_range.requested, DynamicRange::Fixed(40.0));
            Flow::Continue
        }).unwrap();
    assert!(previews > 2, "refresh must be checked during the remaining sparse preview");
    assert!(refined > 10, "explicit refresh must not wait for the timer");
    let final_view = state.render(20, 20, None).unwrap();
    let expected = analyze(&mut source, &req, &mut |_, _| {}).unwrap();
    assert_eq!(final_view.frames, expected.frames);
    for (actual, expected) in final_view.db.values.iter().zip(&expected.db.values) {
        assert!((actual - expected).abs() < 0.0001);
    }
}

#[test]
fn final_only_overview_matches_progressive_for_serial_and_parallel_workloads() {
    for domain in [Domain::Real, Domain::Iq] {
        for reduce in [Reduce::Max, Reduce::MeanPower] {
            for frames in [1, 16, 128, 257] {
                check_final_only(domain, reduce, frames);
            }
        }
    }
}

fn check_final_only(domain: Domain, reduce: Reduce, frames: usize) {
    let len = FFT + FFT / 4 * (frames - 1) + 93;
    let mut values = match domain {
        Domain::Real => real_tone(len, TONE_HZ, 0.4),
        Domain::Iq => iq_tone(len, TONE_HZ, 0.4),
    };
    *values.last_mut().unwrap() = 0.9;
    let mut source = VecSource::new(domain, values, 12500000.0);
    let mut req = request(41, 33, SampleRange::new(17, len as u64 - 17));
    req.reduce = reduce;
    req.waveform_columns = Some(41);
    let mut progressive = analyze_overview(&mut source, &req, ProgressiveOptions::default(),
        &|| Flow::Continue, &mut |_, _| Flow::Continue).unwrap();
    let expected = progressive.render(41, 33, Some(41)).unwrap();
    let mut final_only = analyze_overview(&mut source, &req, ProgressiveOptions::default().final_only(),
        &|| Flow::Continue, &mut |_, _| panic!("navigation must not publish intermediate pictures")).unwrap();
    let actual = final_only.render(41, 33, Some(41)).unwrap();
    assert_eq!(actual.frames, expected.frames);
    assert_eq!(actual.time_peak, 0.9);
    let a = actual.waveform.as_ref().unwrap();
    let b = expected.waveform.as_ref().unwrap();
    assert_eq!(a.min, b.min);
    assert_eq!(a.max, b.max);
    assert_eq!((a.t0, a.t1, a.columns, a.channels), (b.t0, b.t1, b.columns, b.channels));
    assert_eq!((actual.db.t0, actual.db.t1), (expected.db.t0, expected.db.t1));
    for (a, b) in actual.db.values.iter().zip(&expected.db.values) {
        assert!((a-b).abs() < 0.0001, "{domain:?} {reduce:?} {frames}: {a} != {b}");
    }
    for (a, b) in actual.psd.db.iter().zip(&expected.psd.db) {
        assert!((a-b).abs() < 0.0001);
    }
    assert_eq!(actual.spectrogram.rgba, expected.spectrogram.rgba);
}

#[test]
fn final_only_reads_samples_once_in_order_and_cancels_between_bounded_batches() {
    use std::cell::Cell;
    struct Tracked {
        source: VecSource,
        seeks: usize,
        reads: Cell<usize>,
        values: usize,
    }
    impl SampleSource for Tracked {
        fn meta(&self) -> &SignalMeta { self.source.meta() }
        fn seek(&mut self, n: u64) -> Result<(), SourceError> { self.seeks += 1; self.source.seek(n) }
        fn read(&mut self, buf: &mut [f32]) -> Result<usize, SourceError> {
            self.reads.set(self.reads.get() + 1);
            let n = self.source.read(buf)?;
            self.values += n;
            Ok(n)
        }
        fn access_pattern(&mut self, pattern: argand_core::AccessPattern) {
            assert_eq!(pattern, argand_core::AccessPattern::Sequential);
        }
    }
    let len = FFT * 100 + 93;
    let mut source = Tracked { source: VecSource::new(Domain::Iq, iq_tone(len, TONE_HZ, 0.4), 0.0),
        seeks: 0, reads: Cell::new(0), values: 0 };
    let mut req = request(31, 19, SampleRange::new(17, len as u64 - 17));
    req.waveform_columns = Some(31);
    analyze_overview(&mut source, &req, ProgressiveOptions::new(8).unwrap().final_only(),
        &|| Flow::Continue, &mut |_, _| panic!("no preview")).unwrap();
    assert_eq!(source.seeks, 1);
    assert_eq!(source.values, (len - 17) * 2);
    let checks = Cell::new(0);
    source.reads.set(0);
    let result = analyze_overview(&mut source, &req, ProgressiveOptions::new(8).unwrap().final_only(),
        &|| { checks.set(checks.get()+1); if checks.get() > 2 { Flow::Stop } else { Flow::Continue } },
        &mut |_, _| panic!("no preview"));
    assert!(matches!(result, Err(DspError::Cancelled)));
    assert_eq!(source.reads.get(), 1);
}

#[test]
fn frequency_bands_reuse_native_bins_for_both_reducers_and_preserve_full_render() {
    for domain in [Domain::Real, Domain::Iq] {
        for reduce in [Reduce::Max, Reduce::MeanPower] {
            let values = match domain {
                Domain::Real => real_tone(FFT * 8, TONE_HZ, 0.4),
                Domain::Iq => iq_tone(FFT * 8, TONE_HZ, 0.4),
            };
            let mut source = VecSource::new(domain, values, 14000000.);
            let full = source.meta.frequency_span();
            let mut req = request(1, 1, SampleRange::new(0, source.meta.len_samples));
            req.reduce = reduce;
            let mut overview = analyze_overview(&mut source, &req, ProgressiveOptions::default(),
                &|| Flow::Continue, &mut |_, _| Flow::Continue).unwrap();
            let bins = overview.dimensions().1;
            let original = overview.render(1, bins, None).unwrap();
            for index in [0, bins / 2, bins - 1] {
                let span = full.1 - full.0;
                let band = (full.0 + index as f64 / bins as f64 * span,
                    full.0 + (index + 1) as f64 / bins as f64 * span);
                let zoomed = overview.render_band(1, 1, None, band).unwrap();
                assert!((zoomed.db.values[0] - original.db.values[index]).abs() < 0.001,
                    "{domain:?} {reduce:?} {index}: {} vs {}", zoomed.db.values[0], original.db.values[index]);
                assert_eq!(zoomed.frames, original.frames);
                assert_eq!(zoomed.psd.db, original.psd.db);
                assert_eq!(zoomed.spectrogram.db_min, original.spectrogram.db_min);
                assert_eq!(zoomed.spectrogram.db_max, original.spectrogram.db_max);
            }
            let restored = overview.render_band(1, bins, None, full).unwrap();
            assert_eq!(restored.db.values, original.db.values);
            assert_eq!(restored.spectrogram.rgba, original.spectrogram.rgba);
            for invalid in [(full.1, full.0), (full.0 - 1., full.1), (full.0, f64::NAN)] {
                assert!(matches!(overview.render_band(1, 1, None, invalid), Err(DspError::BadFrequencyBand)));
            }
        }
    }
}
