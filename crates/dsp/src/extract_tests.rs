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
