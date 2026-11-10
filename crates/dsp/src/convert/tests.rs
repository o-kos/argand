use super::*;
use argand_core::{Domain, SampleFormat, SampleType};

fn meta(iq: bool, rate: f64, reference: f64) -> SignalMeta {
    SignalMeta {
        sample_rate: rate,
        center_freq: reference,
        sample_type: SampleType::new(
            if iq { Domain::Iq } else { Domain::Real },
            SampleFormat::F32,
        ),
        len_samples: 16_384,
        container: "wav",
        divisor: 1.0,
        source: "tone.wav".into(),
    }
}

fn tone(input: &SignalMeta, frequency: f64) -> Vec<f32> {
    (0..input.len_samples)
        .flat_map(|n| {
            let phase = std::f64::consts::TAU * frequency * n as f64 / input.sample_rate;
            let values = [0.5 * phase.cos() as f32, 0.5 * phase.sin() as f32];
            values.into_iter().take(input.channels())
        })
        .collect()
}

fn convert(input: &SignalMeta, output: &SignalMeta, frequency: f64, chunk: usize) -> Vec<[f32; 2]> {
    let plan = ConversionPlan::new(input, output).unwrap();
    let mut stream = plan.stream(input.len_samples).unwrap();
    let mut values = Vec::new();
    let mut sink = |block: &[[f32; 2]]| -> Result<(), ()> {
        assert!(block.len() <= 4096);
        values.extend_from_slice(block);
        Ok(())
    };
    for block in tone(input, frequency).chunks(chunk * input.channels()) {
        stream.push(block, input.channels(), &mut sink).unwrap();
    }
    stream.finish(&mut sink).unwrap();
    assert_eq!(values.len() as u64, plan.samples);
    values
}

#[test]
fn tones_keep_physical_frequency_and_amplitude_in_every_domain_direction() {
    for (input, output, frequency) in [
        (
            meta(true, 8000.0, 100_000.0),
            meta(true, 6000.25, 99_500.0),
            -1500.0,
        ),
        (
            meta(true, 8000.0, 100_000.0),
            meta(false, 16000.0, 96_000.0),
            1500.0,
        ),
        (
            meta(false, 8000.0, 100_000.0),
            meta(true, 5000.0, 102_000.0),
            1500.0,
        ),
        (
            meta(false, 8000.0, 100_000.0),
            meta(false, 6000.0, 100_500.0),
            1500.0,
        ),
    ] {
        let values = convert(&input, &output, frequency, 137);
        for (n, &[i, q]) in values
            .iter()
            .enumerate()
            .skip(1000)
            .take(values.len() - 2000)
        {
            let phase = std::f64::consts::TAU
                * (input.center_freq + frequency - output.center_freq)
                * n as f64
                / output.sample_rate;
            assert!(
                (f64::from(i) - 0.5 * phase.cos()).abs() < 0.01,
                "n={n} i={i} phase={phase}"
            );
            if output.is_iq() {
                assert!((f64::from(q) - 0.5 * phase.sin()).abs() < 0.01);
            } else {
                assert_eq!(q, 0.0);
            }
        }
    }
}

#[test]
fn block_boundaries_do_not_change_samples() {
    let input = meta(true, 8000.0, 100_000.0);
    let output = meta(true, 3000.0, 99_000.0);
    let expected = convert(&input, &output, -1200.0, 4096);
    assert_eq!(convert(&input, &output, -1200.0, 1), expected);
    assert_eq!(convert(&input, &output, -1200.0, 17), expected);
}

#[test]
fn excluded_tones_do_not_alias_into_the_output() {
    let input = meta(true, 8000.0, 100_000.0);
    let output = meta(true, 2000.0, 100_000.0);
    let values = convert(&input, &output, 3000.0, 4096);
    let peak = values[500..values.len() - 500]
        .iter()
        .map(|v| v[0].hypot(v[1]))
        .fold(0.0, f32::max);
    assert!(peak < 0.0005, "peak={peak}");
}

#[test]
fn invalid_bands_and_rates_are_refused_and_identity_is_unfiltered() {
    let input = meta(true, 8000.0, 100_000.0);
    assert!(ConversionPlan::new(&input, &input).unwrap().is_identity());
    for output in [
        meta(false, 8000.0, 200_000.0),
        meta(true, f64::NAN, 100_000.0),
        meta(true, 8000.0, f64::INFINITY),
    ] {
        assert!(ConversionPlan::new(&input, &output).is_err());
    }
}

#[test]
fn native_real_dc_and_nyquist_are_kept_when_making_an_analytic_signal() {
    for (rate, reference, output_reference) in [
        (8000.0, 100_000.0, 102_000.0),
        (8000.1, 1_000_000_000.0, 1_000_002_000.0),
        (8000.15, 10_000_000_000.0, 10_000_000_000.0),
    ] {
        let input = meta(false, rate, reference);
        let output = meta(true, 10000.0, output_reference);
        for frequency in [0.0, rate / 2.0] {
            let values = convert(&input, &output, frequency, 4096);
            for (n, &[i, q]) in values
                .iter()
                .enumerate()
                .skip(2000)
                .take(values.len() - 4000)
            {
                let phase = std::f64::consts::TAU
                    * (frequency - (output.center_freq - input.center_freq))
                    * n as f64
                    / output.sample_rate;
                assert!(
                    (f64::from(i) - 0.5 * phase.cos()).abs() < 0.01,
                    "f={frequency} n={n} i={i}"
                );
                assert!(
                    (f64::from(q) - 0.5 * phase.sin()).abs() < 0.01,
                    "f={frequency} n={n} q={q}"
                );
            }
        }
    }
}

#[test]
fn cropped_real_output_stops_a_tone_just_outside_its_target_band() {
    let input = meta(true, 8000.0, 100_000.0);
    let output = meta(false, 4000.0, 99_000.0);
    // Target is 99..101 kHz. A 101.05 kHz tone is outside the one-percent transition.
    let values = convert(&input, &output, 1050.0, 4096);
    let peak = values[1000..values.len() - 1000]
        .iter()
        .map(|v| v[0].abs())
        .fold(0.0, f32::max);
    assert!(peak < 0.0005, "peak={peak}");
}

#[test]
fn fractional_native_edges_survive_physical_reference_subtraction() {
    for rate in [8000.1, 16000.0, 44100.123] {
        for reference in [100_000.0, 1_000_000_000.0] {
            for input_iq in [false, true] {
                check_fractional_edges(rate, reference, input_iq);
            }
        }
    }
}

fn check_fractional_edges(rate: f64, reference: f64, input_iq: bool) {
    let input = meta(input_iq, rate, reference);
    for output_iq in [false, true] {
        for output_rate in [rate, rate / 2.0, 8000.1] {
            let output_reference = if !input_iq && output_iq {
                reference + output_rate / 4.0
            } else {
                reference
            };
            let output = meta(output_iq, output_rate, output_reference);
            let plan = ConversionPlan::new(&input, &output).unwrap();
            assert!(plan.width <= output_rate / if output_iq { 1.0 } else { 2.0 });
            plan.stream(input.len_samples).unwrap();
        }
    }
    let outside = meta(false, rate, input.frequency_span().1 + 0.001);
    assert!(ConversionPlan::new(&input, &outside).is_err());
}

#[test]
fn native_real_endpoints_and_their_sum_survive_a_tight_fractional_iq_band() {
    let mut input = meta(false, 8000.0, 0.0);
    input.len_samples = 65_536;
    let output = meta(true, 4001.0, 2000.0);
    for frequencies in [&[0.0][..], &[4000.0][..], &[0.0, 4000.0][..]] {
        let plan = ConversionPlan::new(&input, &output).unwrap();
        let mut stream = plan.stream(input.len_samples).unwrap();
        let mut values = Vec::new();
        let mut sink = |block: &[[f32; 2]]| -> Result<(), ()> {
            values.extend_from_slice(block);
            Ok(())
        };
        for start in (0..input.len_samples).step_by(4096) {
            let samples: Vec<f32> = (start..(start + 4096).min(input.len_samples))
                .map(|n| {
                    frequencies
                        .iter()
                        .map(|frequency| {
                            0.5 * (std::f64::consts::TAU * frequency * n as f64 / input.sample_rate)
                                .cos()
                        })
                        .sum::<f64>() as f32
                })
                .collect();
            stream.push(&samples, 1, &mut sink).unwrap();
        }
        stream.finish(&mut sink).unwrap();
        for (n, &[i, q]) in values
            .iter()
            .enumerate()
            .skip(3000)
            .take(values.len() - 6000)
        {
            let expected = frequencies.iter().fold([0.0, 0.0], |mut sum, frequency| {
                let phase = std::f64::consts::TAU * (frequency - output.center_freq) * n as f64
                    / output.sample_rate;
                sum[0] += 0.5 * phase.cos();
                sum[1] += 0.5 * phase.sin();
                sum
            });
            assert!(
                (f64::from(i) - expected[0]).abs() < 0.01,
                "{frequencies:?} n={n} i={i} expected={expected:?}"
            );
            assert!(
                (f64::from(q) - expected[1]).abs() < 0.01,
                "{frequencies:?} n={n} q={q} expected={expected:?}"
            );
        }
    }
}

#[test]
fn conversion_extractor_rejects_cropped_bands_across_downsampling_ratios() {
    let mut input = meta(true, 8000.0, 100_000.0);
    input.len_samples = 65_536;
    for rate in [2000.0, 2000.125, 333.3] {
        let output = meta(true, rate, input.center_freq);
        for sign in [-1.0, 1.0] {
            let values = convert(&input, &output, sign * rate * 0.6, 4096);
            let peak = values[500..values.len() - 500]
                .iter()
                .map(|v| v[0].hypot(v[1]))
                .fold(0.0, f32::max);
            assert!(peak < 0.0005, "rate={rate} sign={sign} peak={peak}");
        }
    }
}

#[test]
fn local_reference_math_keeps_long_capture_phase_and_fractional_shift_width() {
    let input = meta(true, 1_000_000.0, 0.0);
    let output = meta(true, 1000.1, 100_000.0);
    ConversionPlan::new(&input, &output)
        .unwrap()
        .stream(input.len_samples)
        .unwrap();
    let input = meta(false, 8000.15, 10_000_000_000.0);
    let output = meta(true, 10000.0, input.center_freq);
    let plan = ConversionPlan::new(&input, &output).unwrap();
    let mut stream = plan.stream(input.len_samples).unwrap();
    let shift = input.sample_rate / 4.0;
    assert_eq!(stream.step, shift / output.sample_rate);
    for n in [0, 108_000_000, 1u64 << 40] {
        stream.given = n;
        let mut value = [[1.0, 0.0]];
        stream.translate(&mut value);
        let cycles = (n as f64 * shift / output.sample_rate).rem_euclid(1.0);
        let phase = std::f64::consts::TAU * cycles;
        let actual = f64::from(value[0][1]).atan2(f64::from(value[0][0]));
        let error = (actual - phase + std::f64::consts::PI).rem_euclid(std::f64::consts::TAU)
            - std::f64::consts::PI;
        assert!(error.abs() < 0.002, "n={n} phase error={error}");
    }
}

#[test]
fn explicit_carrier_delta_keeps_local_band_and_long_phase_at_large_references() {
    for (rate, reference) in [(24000., 1e20), (8000.15, 1e10)] {
        let input = meta(true, rate, reference);
        let output = meta(false, rate * 2., reference - rate / 2.);
        let metadata_plan = ConversionPlan::new(&input, &output).unwrap();
        assert_eq!(
            metadata_plan.reference_delta,
            output.center_freq - input.center_freq
        );
        assert!(
            !metadata_plan.retains_input_band(),
            "legacy API still follows rounded metadata"
        );
        let plan = ConversionPlan::with_reference_delta(&input, &output, -rate / 2.).unwrap();
        assert!(plan.retains_input_band());
        assert_eq!(plan.local_band, (-rate / 2., rate / 2.));
        assert_eq!(plan.reference_delta, -rate / 2.);
        let mut stream = plan.stream(input.len_samples).unwrap();
        assert_eq!(stream.step, 0.25);
        stream.given = (1u64 << 40) + 13;
        let mut value = [[0.4, 0.3]];
        stream.translate(&mut value);
        assert!((value[0][0] + 0.3).abs() < 1e-6);
        assert_eq!(value[0][1], 0.);
        let reverse = ConversionPlan::with_reference_delta(&output, &input, rate / 2.).unwrap();
        assert!(reverse.retains_input_band());
        assert_eq!(reverse.stream(output.len_samples).unwrap().step, 0.);
    }
}
