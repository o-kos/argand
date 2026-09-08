use super::*;

fn constant_frames(domain: Domain, amplitudes: &[f32], fft_size: usize) -> VecSource {
    let mut data = Vec::new();
    for &amplitude in amplitudes {
        for _ in 0..fft_size {
            data.push(amplitude);
            if domain == Domain::Iq {
                data.push(0.0);
            }
        }
    }
    VecSource::new(domain, data, 0.0)
}

#[test]
fn mean_power_preserves_duty_cycle_and_averages_frequency_bins() {
    let fft_size = 16;
    let mut amplitudes = vec![0.0; 100];
    amplitudes[0] = 1.0;
    for domain in [Domain::Real, Domain::Iq] {
        let bins = if domain == Domain::Iq { fft_size } else { fft_size / 2 + 1 };
        for height in [1, bins] {
            let mut source = constant_frames(domain, &amplitudes, fft_size);
            let mut request = request(1, height, SampleRange::new(0, source.meta.len_samples));
            request.cfg = StftConfig { fft_size, hop: fft_size, window: Window::Rect };
            request.waveform_columns = Some(1);
            let max = analyze(&mut source, &request, &mut |_, _| {}).unwrap();
            request.reduce = Reduce::MeanPower;
            let mean = analyze(&mut source, &request, &mut |_, _| {}).unwrap();
            let dc_row = if domain == Domain::Iq { height / 2 } else { 0 };
            let expected = -20.0 - if height == 1 { 10.0 * (bins as f32).log10() } else { 0.0 };
            assert!((mean.db.values[dc_row] - expected).abs() < 0.0001);
            assert_eq!(max.db.values[dc_row], 0.0);
            assert_eq!(mean.psd.db, max.psd.db);
            assert_eq!(mean.psd.freqs_hz, max.psd.freqs_hz);
            assert_eq!(mean.psd.segments, max.psd.segments);
            let mean_waveform = mean.waveform.as_ref().unwrap();
            let max_waveform = max.waveform.as_ref().unwrap();
            assert_eq!(mean_waveform.min, max_waveform.min);
            assert_eq!(mean_waveform.max, max_waveform.max);
            assert_eq!(mean.time_peak, max.time_peak);
            request.reduce = Reduce::Mean;
            let legacy = analyze(&mut source, &request, &mut |_, _| {}).unwrap();
            assert!((legacy.db.values[dc_row] - -297.0).abs() < 0.0001);
        }
    }
}

#[test]
fn progressive_mean_power_weights_frames_once_in_unequal_columns() {
    let amplitudes: Vec<f32> = (1..=17).map(|i| i as f32 / 20.0).collect();
    for domain in [Domain::Real, Domain::Iq] {
        let mut source = constant_frames(domain, &amplitudes, 16);
        let bins = if domain == Domain::Iq { 16 } else { 9 };
        let dc = if domain == Domain::Iq { 8 } else { 0 };
        let mut request = request(3, bins, SampleRange::new(0, source.meta.len_samples));
        request.cfg = StftConfig { fft_size: 16, hop: 16, window: Window::Rect };
        request.reduce = Reduce::MeanPower;
        let mut first = true;
        let result = analyze_progressive_with_options(
            &mut source, &request, ProgressiveOptions::new(1).unwrap(),
            &|| Flow::Continue,
            &mut |preview, _| {
                if !std::mem::take(&mut first) {
                    return Flow::Continue;
                }
                // One lattice frame per column: frames 0, 6 and 12.
                for (column, frame) in [0, 6, 12].into_iter().enumerate() {
                    let expected = 20.0 * amplitudes[frame].log10();
                    assert!((preview.db.values[column * bins + dc] - expected).abs() < 0.0001);
                }
                Flow::Continue
            },
        ).unwrap();
        assert_eq!(result.frames, 17);
        for (column, range) in [0..6, 6..12, 12..17].into_iter().enumerate() {
            let powers: Vec<f64> = amplitudes[range].iter().map(|&v| f64::from(v).powi(2)).collect();
            let expected = 10.0 * (powers.iter().sum::<f64>() / powers.len() as f64).log10();
            assert!((f64::from(result.db.values[column * bins + dc]) - expected).abs() < 0.0001);
        }
    }
}

#[test]
fn mean_power_silence_stays_finite_when_the_image_upsamples_frames_and_bins() {
    for domain in [Domain::Real, Domain::Iq] {
        let mut source = constant_frames(domain, &[0.0; 3], 16);
        let mut request = request(29, 37, SampleRange::new(0, source.meta.len_samples));
        request.cfg = StftConfig { fft_size: 16, hop: 16, window: Window::Rect };
        request.reduce = Reduce::MeanPower;
        let plain = analyze(&mut source, &request, &mut |_, _| {}).unwrap();
        let progressive = analyze_progressive(&mut source, &request, &|| Flow::Continue, &mut |_, _| Flow::Continue).unwrap();
        assert!(plain.db.values.iter().all(|&db| db == DB_FLOOR));
        assert_eq!(plain.db.values, progressive.db.values);
    }
}
