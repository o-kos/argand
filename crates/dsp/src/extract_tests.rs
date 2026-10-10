use super::*;

const RATE: f64 = 48_000.0;

/// A span of `len` samples, interleaved with `channels` values each, as `sample(n)` gives them.
fn span(len: usize, channels: usize, sample: impl Fn(usize) -> [f32; 2]) -> Vec<f32> {
    (0..len)
        .flat_map(|n| sample(n).into_iter().take(channels))
        .collect()
}

/// Extract a span with zeros beyond it, feeding blocks of `block` samples.
fn extract(plan: &ExtractPlan, input: &[f32], channels: usize, block: usize) -> Vec<[f32; 2]> {
    let len = (input.len() / channels) as u64;
    let mut extractor = Extractor::new(plan.clone(), len).unwrap();
    let mut out = Vec::new();
    extractor.push_zeros(plan.margin(), &mut out);
    for chunk in input.chunks(block * channels) {
        extractor.push(chunk, channels, &mut out);
    }
    let wanted = extractor.wanted();
    extractor.push_zeros(wanted, &mut out);
    assert!(extractor.is_done());
    out
}

fn tone(frequency: f64, amplitude: f32) -> impl Fn(usize) -> [f32; 2] {
    move |n| {
        let phase = std::f64::consts::TAU * frequency * n as f64 / RATE;
        [amplitude * phase.cos() as f32, amplitude * phase.sin() as f32]
    }
}

/// The middle half of an output, clear of the edges where the filter met zeros.
fn middle(out: &[[f32; 2]]) -> &[[f32; 2]] {
    &out[out.len() / 4..out.len() * 3 / 4]
}

fn magnitude([i, q]: [f32; 2]) -> f32 {
    i.hypot(q)
}

#[test]
fn a_tone_inside_the_band_comes_out_at_its_offset_with_its_amplitude() {
    let plan = ExtractPlan::new(RATE, 5_000.0, 7_000.0, false).unwrap();
    assert_eq!(plan.decimation(), 19);
    let input = span(48_000, 2, tone(6_500.0, 0.5));
    let out = extract(&plan, &input, 2, 4096);
    let middle = middle(&out);
    for &sample in middle {
        assert!((magnitude(sample) - 0.5).abs() < 0.005, "{sample:?}");
    }
    // Each output turns by the tone's offset from the band centre.
    let expected = std::f64::consts::TAU * 500.0 / plan.output_rate();
    for pair in middle.windows(2) {
        let [a, b] = [pair[0], pair[1]];
        let turn = f64::from(b[1] * a[0] - b[0] * a[1]).atan2(f64::from(b[0] * a[0] + b[1] * a[1]));
        assert!((turn - expected).abs() < 1e-3, "{turn} against {expected}");
    }
}

#[test]
fn a_tone_outside_the_band_is_stopped() {
    let plan = ExtractPlan::new(RATE, 5_000.0, 7_000.0, false).unwrap();
    for frequency in [-6_000.0, 0.0, 3_000.0, 9_000.0, 20_000.0] {
        let input = span(48_000, 2, tone(frequency, 0.5));
        let out = extract(&plan, &input, 2, 4096);
        let peak = middle(&out).iter().map(|&s| magnitude(s)).fold(0.0, f32::max);
        // At least 75 dB under the tone's own amplitude.
        assert!(peak < 0.5 * 1.8e-4, "{frequency} Hz leaks {peak}");
    }
}

#[test]
fn the_output_has_one_sample_for_every_decimated_input_sample() {
    let plan = ExtractPlan::new(RATE, -1_000.0, 1_000.0, false).unwrap();
    let d = plan.decimation() as usize;
    for len in [1, d - 1, d, d + 1, 10 * d + 3] {
        let input = span(len, 2, |_| [0.0; 2]);
        let out = extract(&plan, &input, 2, 64);
        assert_eq!(out.len() as u64, plan.output_len(len as u64), "{len} samples");
    }
}

#[test]
fn output_n_lines_up_with_input_n_times_d() {
    let plan = ExtractPlan::new(RATE, -1_000.0, 1_000.0, false).unwrap();
    let d = plan.decimation() as usize;
    let at = 40 * d;
    let input = span(100 * d, 2, |n| if n == at { [1.0, 0.0] } else { [0.0; 2] });
    let out = extract(&plan, &input, 2, 333);
    let peak = (0..out.len())
        .max_by(|&a, &b| magnitude(out[a]).total_cmp(&magnitude(out[b])))
        .unwrap();
    assert_eq!(peak, 40);
}

#[test]
fn a_real_tone_comes_out_complex_with_its_amplitude() {
    let plan = ExtractPlan::new(RATE, 5_000.0, 7_000.0, true).unwrap();
    let input = span(48_000, 1, tone(6_500.0, 0.5));
    let out = extract(&plan, &input, 1, 4096);
    for &sample in middle(&out) {
        assert!((magnitude(sample) - 0.5).abs() < 0.005, "{sample:?}");
    }
}

#[test]
fn the_block_size_does_not_change_the_output() {
    let plan = ExtractPlan::new(RATE, 1_000.0, 4_000.0, false).unwrap();
    let input = span(5_000, 2, tone(2_200.0, 0.25));
    let whole = extract(&plan, &input, 2, input.len());
    assert_eq!(extract(&plan, &input, 2, 1), whole);
    assert_eq!(extract(&plan, &input, 2, 7), whole);
}

#[test]
fn a_band_outside_the_capture_or_too_narrow_is_refused() {
    assert!(matches!(
        ExtractPlan::new(RATE, -1_000.0, 1_000.0, true),
        Err(ExtractError::OutsideCapture { .. })
    ));
    assert!(matches!(
        ExtractPlan::new(RATE, 20_000.0, 25_000.0, false),
        Err(ExtractError::OutsideCapture { .. })
    ));
    assert!(matches!(
        ExtractPlan::new(RATE, 1_000.0, 1_000.5, false),
        Err(ExtractError::TooNarrow { .. })
    ));
}

#[test]
fn a_band_as_wide_as_the_capture_is_kept_whole() {
    let plan = ExtractPlan::new(RATE, -24_000.0, 24_000.0, false).unwrap();
    assert_eq!(plan.decimation(), 1);
    assert_eq!(plan.taps(), [1.0]);
}

#[test]
fn a_wide_band_kept_at_the_full_rate_is_still_filtered() {
    let plan = ExtractPlan::new(RATE, -10_000.0, 10_000.0, false).unwrap();
    assert_eq!(plan.decimation(), 1);
    let input = span(24_000, 2, tone(20_000.0, 0.5));
    let out = extract(&plan, &input, 2, 4096);
    let peak = middle(&out).iter().map(|&s| magnitude(s)).fold(0.0, f32::max);
    assert!(peak < 0.5 * 1.8e-4, "leaks {peak}");
}

#[test]
fn a_real_band_from_0_hz_stops_its_mirror() {
    let plan = ExtractPlan::new(RATE, 0.0, 2_000.0, true).unwrap();
    // The mirror of a 100 Hz tone lands 200 Hz below it, beyond the guard.
    let input = span(48_000, 1, tone(100.0, 0.5));
    let out = extract(&plan, &input, 1, 4096);
    for &sample in middle(&out) {
        assert!((magnitude(sample) - 0.5).abs() < 0.005, "the mirror beats with the tone {sample:?}");
    }
}

#[test]
fn a_real_band_up_to_the_nyquist_rate_stops_its_mirror() {
    let plan = ExtractPlan::new(RATE, 21_000.0, 24_000.0, true).unwrap();
    let input = span(48_000, 1, tone(23_800.0, 0.5));
    let out = extract(&plan, &input, 1, 4096);
    for &sample in middle(&out) {
        assert!((magnitude(sample) - 0.5).abs() < 0.005, "{sample:?}");
    }
}

#[test]
fn a_wide_complex_band_short_of_the_whole_capture_is_filtered() {
    let plan = ExtractPlan::new(RATE, -21_600.0, 21_600.0, false).unwrap();
    assert_eq!(plan.decimation(), 1);
    let input = span(24_000, 2, tone(23_000.0, 0.5));
    let out = extract(&plan, &input, 2, 4096);
    let peak = middle(&out).iter().map(|&s| magnitude(s)).fold(0.0, f32::max);
    assert!(peak < 0.5 * 1.8e-4, "leaks {peak}");
}

#[test]
fn a_span_whose_input_cannot_be_counted_is_refused() {
    let plan = ExtractPlan::new(RATE, -10_000.0, 10_000.0, false).unwrap();
    assert!(matches!(Extractor::new(plan, u64::MAX), Err(ExtractError::TooLong(_))));
}

#[test]
fn a_real_band_just_above_0_hz_stops_the_mirror_of_its_lowest_tone() {
    let plan = ExtractPlan::new(RATE, 1.0, 2_001.0, true).unwrap();
    // A 2 Hz tone's mirror lies 4 Hz below it, which the transition follows.
    let input = span(96_000, 1, tone(2.0, 0.5));
    let out = extract(&plan, &input, 1, 4096);
    for &sample in middle(&out) {
        assert!((magnitude(sample) - 0.5).abs() < 0.005, "the mirror beats with the tone {sample:?}");
    }
}

#[test]
fn a_complex_band_a_few_hertz_short_of_the_whole_capture_is_filtered() {
    let plan = ExtractPlan::new(RATE, -23_990.0, 23_990.0, false).unwrap();
    assert!(plan.taps().len() > 1);
    let input = span(96_000, 2, tone(23_997.0, 0.5));
    let out = extract(&plan, &input, 2, 4096);
    let peak = middle(&out).iter().map(|&s| magnitude(s)).fold(0.0, f32::max);
    assert!(peak < 0.5 * 1.8e-4, "leaks {peak}");
}


/// The filter's gain at `frequency` hertz from the band centre.
fn response(plan: &ExtractPlan, frequency: f64) -> f64 {
    let (mut re, mut im) = (0.0f64, 0.0f64);
    for (k, &tap) in plan.taps().iter().enumerate() {
        let phase = std::f64::consts::TAU * frequency * k as f64 / RATE;
        re += f64::from(tap) * phase.cos();
        im -= f64::from(tap) * phase.sin();
    }
    re.hypot(im)
}

#[test]
fn a_real_capture_s_dc_and_nyquist_rate_keep_their_amplitude() {
    // Its own mirror, DC lies on the band's edge, where the doubled half gain is one.
    let plan = ExtractPlan::new(RATE, 0.0, 2_000.0, true).unwrap();
    assert!((response(&plan, 1_000.0) - 1.0).abs() < 0.01);
    let plan = ExtractPlan::new(RATE, 22_000.0, 24_000.0, true).unwrap();
    assert!((response(&plan, 1_000.0) - 1.0).abs() < 0.01);
    // A band clear of both keeps the analytic gain to its edges.
    let plan = ExtractPlan::new(RATE, 5_000.0, 7_000.0, true).unwrap();
    assert!((response(&plan, 1_000.0) - 2.0).abs() < 0.01);
}

#[test]
fn export_interpolation_preserves_a_tone_near_the_native_nyquist_boundary() {
    let plan = ExtractPlan::for_export(RATE, -23_990.0, 23_990.0, false).unwrap();
    assert_eq!(plan.output_rate(), 2.0 * RATE);
    let input = span(96_000, 2, tone(23_980.0, 0.5));
    let out = extract(&plan, &input, 2, 137);
    for &sample in middle(&out) {
        assert!((magnitude(sample) - 0.5).abs() < 0.005, "{sample:?}");
    }
}


#[test]
fn half_band_interpolation_keeps_native_samples_and_input_chunking_exact() {
    let mut plan = ExtractPlan::new(RATE, -RATE / 2.0, RATE / 2.0, false).unwrap();
    plan.interpolation = 2;
    plan.whole_export = true;
    plan.taps = half_sample_delay(1.0 / 1024.0);
    let input = span(100_000, 2, tone(23_900.0, 0.5));
    let out = extract(&plan, &input, 2, 4096);
    for (n, sample) in input.chunks_exact(2).enumerate() {
        assert_eq!(out[2 * n].map(f32::to_bits), [sample[0].to_bits(), sample[1].to_bits()]);
    }
    for block in [1, 7, 17, 333] {
        assert_eq!(extract(&plan, &input, 2, block), out);
    }
    let expected_turn = std::f64::consts::TAU * 23_900.0 / plan.output_rate();
    for pair in middle(&out).windows(2) {
        let [a, b] = [pair[0], pair[1]];
        let turn = f64::from(b[1] * a[0] - b[0] * a[1]).atan2(f64::from(b[0] * a[0] + b[1] * a[1]));
        assert!((magnitude(a) - 0.5).abs() < 0.001, "{a:?}");
        assert!((turn - expected_turn).abs() < 0.001, "{turn}");
    }
}

#[test]
fn a_whole_rf_band_keeps_both_native_edges_beyond_the_declared_guard() {
    let rate = 10_000_000.0;
    let frequency = rate * (0.5 - NARROWEST_TRANSITION);
    let plan = ExtractPlan::for_export(rate, -rate / 2.0, rate / 2.0, false).unwrap();
    let input = span(4_000_000, 2, |n| {
        let phase = std::f64::consts::TAU * frequency * n as f64 / rate;
        [0.6 * phase.cos() as f32, 0.2 * phase.sin() as f32]
    });
    let out = extract(&plan, &input, 2, 137);
    for (n, &[i, q]) in out.iter().enumerate().skip(out.len() / 4).take(out.len() / 2) {
        let phase = std::f64::consts::TAU * frequency * n as f64 / plan.output_rate();
        assert!((f64::from(i) - 0.6 * phase.cos()).abs() < 0.001, "{n}: {i}");
        assert!((f64::from(q) - 0.2 * phase.sin()).abs() < 0.001, "{n}: {q}");
    }
}

#[test]
fn the_half_sample_kernel_has_the_declared_native_nyquist_transition() {
    let taps = half_sample_delay(NARROWEST_TRANSITION);
    let delay = (taps.len() as f64 - 1.0) / 2.0;
    for sign in [-1.0, 1.0] {
        let frequency = sign * (0.5 - NARROWEST_TRANSITION / 2.0);
        let (mut re, mut im) = (0.0, 0.0);
        for (n, &tap) in taps.iter().enumerate() {
            let phase = std::f64::consts::TAU * frequency * (n as f64 - delay);
            re += f64::from(tap) * phase.cos();
            im -= f64::from(tap) * phase.sin();
        }
        assert!((re - 1.0).abs() < 2e-4, "{re}");
        assert!(im.abs() < 2e-4, "{im}");
    }
    let edge: f64 = taps.iter().enumerate().map(|(n, &tap)| {
        f64::from(tap) * if n.is_multiple_of(2) { 1.0 } else { -1.0 }
    }).sum();
    assert!(edge.abs() < 1e-6);
}

#[test]
fn non_identity_exports_stop_energy_at_the_intermediate_fft_boundary() {
    for (low, high, real) in [
        (-23_999.0, 23_999.0, false),
        (-1_000.0, 20_000.0, false),
        (0.0, 24_000.0, true),
        (22_000.0, 24_000.0, true),
    ] {
        let plan = ExtractPlan::for_export(RATE, low, high, real).unwrap();
        assert!(!plan.whole_export);
        assert!(response(&plan, RATE / 2.0) < 1e-4);
    }
}

#[test]
fn a_tone_inside_the_native_guard_keeps_its_energy_in_tone_and_image() {
    let rate = 10_000_000.0;
    let frequency = rate / 2.0 - 20.0;
    let plan = ExtractPlan::for_export(rate, -rate / 2.0, rate / 2.0, false).unwrap();
    let input = span(4_000_000, 2, |n| {
        let phase = std::f64::consts::TAU * frequency * n as f64 / rate;
        [0.5 * phase.cos() as f32, 0.5 * phase.sin() as f32]
    });
    let out = extract(&plan, &input, 2, 4096);
    let mut wanted = rustfft::num_complex::Complex64::default();
    let mut image = rustfft::num_complex::Complex64::default();
    let count = out.len() / 2;
    for (n, &[i, q]) in out.iter().enumerate().skip(out.len() / 4).take(count) {
        assert!(i.hypot(q) <= 0.501);
        let value = rustfft::num_complex::Complex64::new(f64::from(i), f64::from(q)) / count as f64;
        let phase = std::f64::consts::TAU * frequency * n as f64 / plan.output_rate();
        let mirror = std::f64::consts::TAU * (frequency - rate) * n as f64 / plan.output_rate();
        wanted += value * rustfft::num_complex::Complex64::new(phase.cos(), -phase.sin());
        image += value * rustfft::num_complex::Complex64::new(mirror.cos(), -mirror.sin());
    }
    assert!(wanted.norm() > 0.25 && wanted.norm() < 0.5);
    assert!(image.norm() > 0.01);
    assert!((wanted.norm() + image.norm() - 0.5).abs() < 0.001, "{wanted:?}, {image:?}");
}

#[test]
fn whole_band_mixing_happens_after_interpolation() {
    let mut plan = ExtractPlan::for_export(RATE, -RATE / 2.0, RATE / 2.0 - RATE / 65536.0, false).unwrap();
    plan.taps = half_sample_delay(1.0 / 1024.0);
    let input = span(100_000, 2, tone(23_000.0, 0.5));
    let out = extract(&plan, &input, 2, 4096);
    for (n, &[i, q]) in out.iter().enumerate().skip(out.len() / 4).take(out.len() / 2) {
        let phase = std::f64::consts::TAU * (23_000.0 - plan.centre()) * n as f64 / plan.output_rate();
        assert!((f64::from(i) - 0.5 * phase.cos()).abs() < 0.001);
        assert!((f64::from(q) - 0.5 * phase.sin()).abs() < 0.001);
    }
}
