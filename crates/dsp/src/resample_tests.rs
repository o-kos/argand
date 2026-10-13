use super::*;

fn signal(rate: f64, frequency: f64, count: usize) -> Vec<[f32; 2]> {
    (0..count).map(|n| {
        let phase = std::f64::consts::TAU * frequency * n as f64 / rate;
        [phase.cos() as f32 * 0.5, phase.sin() as f32 * 0.5]
    }).collect()
}

fn render(input: &[[f32; 2]], rate: f64, target: f64, block: usize) -> Vec<[f32; 2]> {
    let plan = Plan::new(rate, target, 2_000.0).unwrap();
    let mut stream = Stream::new(plan, Span { offset: 100.25, samples: 2_000 }).unwrap();
    let mut output = Vec::new();
    for chunk in input.chunks(block) {
        let mut taken = 0;
        while taken < chunk.len() {
            let mut out = Vec::new();
            taken += stream.push(&chunk[taken..], &mut out);
            output.extend(out);
        }
    }
    while !stream.is_done() {
        let mut out = Vec::new();
        stream.finish(&mut out);
        output.extend(out);
    }
    output
}

#[test]
fn a_fractional_rate_preserves_amplitude_frequency_and_alignment() {
    for rate in [2_526.315789473684, 4_000.0] {
        let input = signal(rate, 700.0, 4_000);
        let output = render(&input, rate, 3_000.0, 137);
        assert_eq!(output.len(), 2_000);
        for (n, &[i, q]) in output.iter().enumerate() {
            let phase = std::f64::consts::TAU * 700.0 * (100.25 / rate + n as f64 / 3_000.0);
            assert!((f64::from(i) - 0.5 * phase.cos()).abs() < 0.001, "{rate}, {n}");
            assert!((f64::from(q) - 0.5 * phase.sin()).abs() < 0.001, "{rate}, {n}");
        }
    }
}

#[test]
fn input_block_boundaries_do_not_change_the_reconstruction() {
    let input = signal(4_000.0, -800.0, 4_000);
    let whole = render(&input, 4_000.0, 3_000.0, input.len());
    assert_eq!(render(&input, 4_000.0, 3_000.0, 1), whole);
    assert_eq!(render(&input, 4_000.0, 3_000.0, 17), whole);
}

#[test]
fn downsampling_stops_an_out_of_band_tone() {
    let output = render(&signal(4_000.0, 1_800.0, 4_000), 4_000.0, 3_000.0, 4096);
    let peak = output.iter().map(|&[i, q]| i.hypot(q)).fold(0.0, f32::max);
    assert!(peak < 0.5 * 1.8e-4, "{peak}");
}

#[test]
fn output_and_retained_input_are_bounded_even_with_large_offsets_and_upsampling() {
    let plan = Plan::new(100.0, 100_000.0, 50.0).unwrap();
    let margin = plan.margin() as usize;
    let mut stream = Stream::new(plan, Span { offset: 10_000.0, samples: 20_000 }).unwrap();
    let input = vec![[0.25, 0.0]; 10_100];
    let mut taken = 0;
    let mut total = 0;
    while taken < input.len() {
        let mut out = Vec::new();
        taken += stream.push(&input[taken..], &mut out);
        total += out.len();
        assert!(out.len() <= OUTPUT_BATCH);
        assert!(stream.buffer.len() <= 2 * margin + 1);
    }
    while !stream.is_done() {
        let mut out = Vec::new();
        stream.finish(&mut out);
        total += out.len();
        assert!(out.len() <= OUTPUT_BATCH);
    }
    assert_eq!(total, 20_000);
}

#[test]
fn export_bands_have_space_for_fractional_reconstruction() {
    for width in [48_000.0, 47_999.0, 47_990.0] {
        let plan = Plan::new(96_000.0, 60_000.0, width).unwrap();
        assert!(plan.margin() < 1024);
    }
}
