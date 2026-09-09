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
