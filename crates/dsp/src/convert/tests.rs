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
            values[..input.channels()].to_vec()
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
    let input = meta(false, 8000.0, 100_000.0);
    let output = meta(true, 10000.0, 102_000.0);
    for frequency in [0.0, 4000.0] {
        let values = convert(&input, &output, frequency, 4096);
        for (n, &[i, q]) in values
            .iter()
            .enumerate()
            .skip(2000)
            .take(values.len() - 4000)
        {
            let phase = std::f64::consts::TAU
                * (input.center_freq + frequency - output.center_freq)
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
#[ignore = "manual release-mode conversion cost measurement"]
fn conversion_cost_for_one_million_input_frames() {
    for (iq, output_rate, reference) in [
        (true, 50_000.0, 0.0),
        (true, 1_000_000.0, 100_000.0),
        (false, 1_000_000.0, 250_000.0),
    ] {
        let mut input = meta(iq, 1_000_000.0, 0.0);
        input.len_samples = 1_000_000;
        let output = meta(true, output_rate, reference);
        let started = std::time::Instant::now();
        let plan = ConversionPlan::new(&input, &output).unwrap();
        let mut stream = plan.stream(input.len_samples).unwrap();
        let prepared = started.elapsed();
        let block = vec![0.125; 4096 * input.channels()];
        let mut count = 0u64;
        let mut sink = |values: &[[f32; 2]]| -> Result<(), ()> {
            count += values.len() as u64;
            Ok(())
        };
        let mut remaining = input.len_samples as usize;
        while remaining > 0 {
            let take = remaining.min(4096);
            stream
                .push(
                    &block[..take * input.channels()],
                    input.channels(),
                    &mut sink,
                )
                .unwrap();
            remaining -= take;
        }
        stream.finish(&mut sink).unwrap();
        assert_eq!(count, plan.samples);
        eprintln!(
            "input IQ={iq}, output {output_rate} Hz ref={reference}: prepare={prepared:?}, total={:?}, frames={count}",
            started.elapsed()
        );
    }
    if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
        for line in status
            .lines()
            .filter(|line| line.starts_with("VmHWM:") || line.starts_with("VmRSS:"))
        {
            eprintln!("{line}");
        }
    }
}
