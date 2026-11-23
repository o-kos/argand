use super::*;

fn input(len: usize) -> Vec<Complex32> {
    (0..len)
        .map(|n| {
            Complex32::new(
                (((n * 71) ^ (n >> 5)) % 10007) as f32 / 10007.0 - 0.5,
                (((n * 37) ^ (n >> 3)) % 5003) as f32 / 5003.0 - 0.5,
            )
        })
        .collect()
}

fn reference(plan: &ExtractPlan, input: &[Complex32]) -> Vec<[f32; 2]> {
    let sparse: Vec<Vec<_>> = [&plan.taps, &plan.half_taps]
        .iter()
        .map(|taps| {
            taps.iter()
                .copied()
                .enumerate()
                .filter(|&(_, v)| v != 0.0)
                .collect()
        })
        .collect();
    (0..plan.output_len(input.len() as u64))
        .map(|n| {
            let q = n * plan.decimation;
            let k = (q / 2) as i64;
            let mut value = Complex32::default();
            for &(j, tap) in &sparse[(q % 2) as usize] {
                let at = k + plan.margin() as i64 - j as i64;
                if at < 0 || at as usize >= input.len() {
                    continue;
                }
                let phase = -std::f64::consts::TAU * plan.centre * at as f64 / plan.rate;
                let (sin, cos) = phase.sin_cos();
                let source = input[at as usize];
                let mixed = Complex32::new(
                    (f64::from(source.re) * cos - f64::from(source.im) * sin) as f32,
                    (f64::from(source.re) * sin + f64::from(source.im) * cos) as f32,
                );
                value += mixed * tap;
            }
            [value.re, value.im]
        })
        .collect()
}

fn render(
    plan: &ExtractPlan,
    input: &[Complex32],
    partitioned: bool,
    chunk: usize,
) -> Vec<[f32; 2]> {
    let margin = plan.margin();
    let support = margin..margin + input.len() as u64;
    let mut engine =
        ExportExtractor::new(plan.clone(), input.len() as u64, Some(support.clone())).unwrap();
    if partitioned != matches!(engine.engine, Convolution::Partitioned(_)) {
        engine.engine = if partitioned {
            let mut phases = vec![plan.taps.as_slice()];
            if !plan.half_taps.is_empty() {
                phases.push(&plan.half_taps);
            }
            Convolution::Partitioned(Box::new(Partitioned::polyphase(&phases)))
        } else {
            Convolution::Direct {
                support,
                input: vec![Complex32::default(); input.len()],
            }
        };
    }
    let mut out = Vec::new();
    engine.push_zeros(margin, &mut out);
    for block in input.chunks(chunk) {
        let values: Vec<_> = block.iter().flat_map(|v| [v.re, v.im]).collect();
        engine.push(&values, 2, &mut out);
    }
    engine.push_zeros(engine.wanted(), &mut out);
    assert!(engine.is_done());
    out
}

fn assert_close(actual: &[[f32; 2]], expected: &[[f32; 2]]) {
    assert_eq!(actual.len(), expected.len());
    for (n, (a, b)) in actual.iter().zip(expected).enumerate() {
        for channel in 0..2 {
            assert!(
                (a[channel] - b[channel]).abs() < 2e-6,
                "{n}: {a:?} vs {b:?}"
            );
        }
    }
}

#[test]
fn odd_and_even_decimation_match_independent_fir_phases_and_mixing() {
    let input = input(PARTITION + 13);
    for d in [1, 2, 3, 6] {
        let mut plan = ExtractPlan::for_export(48_000.0, 5_000.0, 7_000.0, false).unwrap();
        plan.decimation = d;
        plan.taps = (0..19)
            .map(|n| ((n * 17 % 23) as f32 - 11.0) / 97.0)
            .collect();
        plan.half_taps = if d % 2 == 0 {
            Vec::new()
        } else {
            (0..18)
                .map(|n| ((n * 11 % 19) as f32 - 9.0) / 71.0)
                .collect()
        };
        for input in [&input[..], &input[..1], &input[..2], &input[..7]] {
            let expected = reference(&plan, input);
            for partitioned in [false, true] {
                let actual = render(&plan, input, partitioned, 137);
                assert_close(&actual, &expected);
            }
        }
    }
}

#[test]
fn sparse_three_partition_phases_keep_history_overlap_and_final_partial() {
    let input = input(2 * PARTITION + 13);
    let mut plan = ExtractPlan::for_export(48_000.0, 5_000.0, 7_000.0, false).unwrap();
    plan.taps = vec![0.0; 2 * PARTITION + 39];
    plan.half_taps = vec![0.0; 2 * PARTITION + 38];
    for (phase, taps) in [&mut plan.taps, &mut plan.half_taps]
        .into_iter()
        .enumerate()
    {
        let last = taps.len() - 1;
        for (at, value) in [
            (0, -0.13),
            (PARTITION - 1, 0.25),
            (PARTITION, 0.11),
            (PARTITION + 1, -0.17),
            (2 * PARTITION, 0.3),
            (last, 0.1),
        ] {
            taps[at] = value * (phase + 1) as f32;
        }
    }
    for d in [1, 3] {
        plan.decimation = d;
        let expected = reference(&plan, &input);
        for chunk in [1, PARTITION + 5] {
            assert_close(&render(&plan, &input, true, chunk), &expected);
        }
    }
}

fn response(
    taps: &[f32],
    offset: f64,
    frequency: f64,
    rate: f64,
) -> rustfft::num_complex::Complex64 {
    taps.iter()
        .enumerate()
        .map(|(j, &tap)| {
            let angle = -std::f64::consts::TAU * frequency / rate * (j as f64 - offset);
            rustfft::num_complex::Complex64::from_polar(f64::from(tap), angle)
        })
        .sum()
}

fn assert_response(plan: &ExtractPlan, frequency: f64, expected: f64, scale: f64) {
    let m = plan.margin() as f64;
    let a = response(&plan.taps, m, frequency, plan.rate);
    let b = response(&plan.half_taps, m - 0.5, frequency, plan.rate);
    let wanted = (a + b) / 2.0;
    let image = (a - b) / 2.0;
    assert!(
        (wanted.re - expected).abs() < scale * 1e-4,
        "{frequency}: wanted={wanted:?}, expected={expected}"
    );
    assert!(wanted.im.abs() < scale * 1e-4, "{frequency}: {wanted:?}");
    assert!(image.norm() < scale * 1e-4, "{frequency}: image={image:?}");
}

#[test]
fn combined_phases_keep_passband_real_edge_half_gain_and_80db_images() {
    let real = ExtractPlan::for_export(48_000.0, 0.0, 2_000.0, true).unwrap();
    let upper = ExtractPlan::for_export(48_000.0, 22_000.0, 24_000.0, true).unwrap();
    assert_eq!(real.taps, upper.taps);
    assert_eq!(real.half_taps, upper.half_taps);
    for (frequency, gain) in [
        (0.0, 2.0),
        (999.0, 2.0),
        (1000.0, 1.0),
        (1001.0, 0.0),
        (24_000.0, 0.0),
    ] {
        assert_response(&real, frequency, gain, 2.0);
    }
    let iq = ExtractPlan::for_export(48_000.0, -23_990.0, 23_990.0, false).unwrap();
    for (frequency, gain) in [
        (0.0, 1.0),
        (23_980.0, 1.0),
        (23_990.0, 1.0),
        (23_997.0, 0.0),
        (24_000.0, 0.0),
    ] {
        assert_response(&iq, frequency, gain, 1.0);
    }
}

#[test]
fn conversion_keeps_the_spectral_route_and_even_export_has_no_half_bank() {
    let conversion = ExtractPlan::for_conversion(48_000.0, -1_000.0, 1_000.0, false).unwrap();
    assert!(!conversion.export);
    assert!(conversion.half_taps.is_empty());
    assert!(matches!(
        Extractor::new(conversion, 17).unwrap().0,
        Engine::Spectral(_)
    ));
    let export = ExtractPlan::for_export(48_000.0, -500.0, 500.0, false).unwrap();
    assert!(export.decimation.is_multiple_of(2));
    assert!(export.half_taps.is_empty());
    assert!(matches!(
        Extractor::new(export, 17).unwrap().0,
        Engine::Export(_)
    ));
}
