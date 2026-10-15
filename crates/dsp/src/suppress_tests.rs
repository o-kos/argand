use super::*;

const RATE: f64 = 48_000.0;

/// A span of `len` samples, interleaved with `channels` values each, as `sample(n)` gives them.
fn span(len: usize, channels: usize, sample: impl Fn(usize) -> [f32; 2]) -> Vec<f32> {
    (0..len)
        .flat_map(|n| sample(n).into_iter().take(channels))
        .collect()
}

/// Suppress a region with zeros beyond it, feeding blocks of `block` samples.
fn suppress(
    plan: &SuppressPlan,
    input: &[f32],
    channels: usize,
    block: usize,
    taper: (u64, u64),
) -> Vec<[f32; 2]> {
    let len = (input.len() / channels) as u64;
    let mut suppressor = Suppressor::new(plan, len, taper.0, taper.1).unwrap();
    let mut out = Vec::new();
    suppressor.push_zeros(plan.margin(), &mut out);
    for chunk in input.chunks(block * channels) {
        suppressor.push(chunk, channels, &mut out);
    }
    let wanted = suppressor.wanted();
    assert_eq!(wanted, plan.margin());
    suppressor.push_zeros(wanted, &mut out);
    assert!(suppressor.is_done());
    out
}

fn tone(frequency: f64, amplitude: f32) -> impl Fn(usize) -> [f32; 2] {
    move |n| {
        let phase = std::f64::consts::TAU * frequency * n as f64 / RATE;
        [amplitude * phase.cos() as f32, amplitude * phase.sin() as f32]
    }
}

/// The middle half of a region, clear of the edges where the filter met zeros.
fn middle<T>(values: &[T]) -> &[T] {
    &values[values.len() / 4..values.len() * 3 / 4]
}

/// The largest difference between the output and the input it came from, over the middle half.
fn largest_change(out: &[[f32; 2]], input: &[f32], channels: usize) -> f32 {
    let pairs: Vec<[f32; 2]> = input
        .chunks_exact(channels)
        .map(|value| [value[0], if channels > 1 { value[1] } else { 0.0 }])
        .collect();
    middle(out)
        .iter()
        .zip(middle(&pairs))
        .map(|(a, b)| (a[0] - b[0]).hypot(a[1] - b[1]))
        .fold(0.0, f32::max)
}

fn largest(out: &[[f32; 2]]) -> f32 {
    middle(out)
        .iter()
        .map(|[i, q]| i.hypot(*q))
        .fold(0.0, f32::max)
}

/// At least 75 dB under an amplitude of 0.5.
const SUPPRESSED: f32 = 0.5 * 1.8e-4;
/// Within the stopband's ripple of an amplitude of 0.5, 74 dB under it.
const UNCHANGED: f32 = 0.5 * 2e-4;

#[test]
fn a_complex_band_is_suppressed_and_what_lies_beyond_its_transitions_is_kept() {
    let plan = SuppressPlan::new(RATE, 5_000.0, 7_000.0, false).unwrap();
    for frequency in [5_050.0, 6_000.0, 6_950.0] {
        let input = span(48_000, 2, tone(frequency, 0.5));
        let out = suppress(&plan, &input, 2, 4096, (0, 0));
        assert!(largest(&out) < SUPPRESSED, "{frequency} Hz stays at {}", largest(&out));
    }
    for frequency in [-6_000.0, 0.0, 3_000.0, 4_400.0, 7_600.0, 20_000.0] {
        let input = span(48_000, 2, tone(frequency, 0.5));
        let out = suppress(&plan, &input, 2, 4096, (0, 0));
        let change = largest_change(&out, &input, 2);
        assert!(change < UNCHANGED, "{frequency} Hz changes by {change}");
    }
}

#[test]
fn a_real_band_is_suppressed_without_its_mirror_counting_twice() {
    let real = |frequency: f64| {
        move |n: usize| {
            let phase = std::f64::consts::TAU * frequency * n as f64 / RATE;
            [0.5 * phase.cos() as f32, 0.0]
        }
    };
    let cases = [
        // A band inside, from 0 Hz and to Fs/2.
        (5_000.0, 7_000.0, vec![5_100.0, 6_000.0], vec![3_000.0, 10_000.0]),
        (0.0, 2_000.0, vec![0.0, 1_000.0, 1_950.0], vec![3_000.0, 12_000.0]),
        (20_000.0, 24_000.0, vec![20_100.0, 23_000.0, 24_000.0], vec![12_000.0, 18_500.0]),
    ];
    for (low, high, inside, outside) in cases {
        let plan = SuppressPlan::new(RATE, low, high, true).unwrap();
        for frequency in inside {
            let input = span(48_000, 1, real(frequency));
            let out = suppress(&plan, &input, 1, 4096, (0, 0));
            let peak = largest(&out);
            assert!(peak < SUPPRESSED, "{low}..{high}: {frequency} Hz stays at {peak}");
        }
        for frequency in outside {
            let input = span(48_000, 1, real(frequency));
            let out = suppress(&plan, &input, 1, 4096, (0, 0));
            let change = largest_change(&out, &input, 1);
            assert!(change < UNCHANGED, "{low}..{high}: {frequency} Hz changes by {change}");
        }
    }
}

#[test]
fn a_complex_band_at_the_capture_edge_is_suppressed() {
    let plan = SuppressPlan::new(RATE, 20_000.0, 24_000.0, false).unwrap();
    for frequency in [21_000.0, 23_900.0, -24_000.0] {
        let input = span(48_000, 2, tone(frequency, 0.5));
        let out = suppress(&plan, &input, 2, 4096, (0, 0));
        assert!(largest(&out) < SUPPRESSED, "{frequency} Hz stays");
    }
    for frequency in [-10_000.0, 0.0, 18_500.0] {
        let input = span(48_000, 2, tone(frequency, 0.5));
        let out = suppress(&plan, &input, 2, 4096, (0, 0));
        assert!(largest_change(&out, &input, 2) < UNCHANGED, "{frequency} Hz changes");
    }
}

#[test]
fn the_whole_complex_band_suppresses_everything() {
    let plan = SuppressPlan::new(RATE, -24_000.0, 24_000.0, false).unwrap();
    assert_eq!(plan.margin(), 0);
    // Limited in time, it still fades rather than cutting.
    assert_eq!(plan.taper(), SHORTEST_TAPER);
    let input = span(1000, 2, tone(3_000.0, 0.5));
    let out = suppress(&plan, &input, 2, 64, (0, 0));
    assert!(out.iter().all(|&[i, q]| i.hypot(q) < 1e-6));
}

#[test]
fn the_output_has_one_sample_for_every_input_sample_whatever_the_blocks() {
    let plan = SuppressPlan::new(RATE, 1_000.0, 3_000.0, false).unwrap();
    let input = span(20_000, 2, tone(2_000.0, 0.25));
    let whole = suppress(&plan, &input, 2, 20_000, (0, 0));
    for block in [1, 7, 4096] {
        assert_eq!(suppress(&plan, &input, 2, block, (0, 0)), whole, "blocks of {block}");
    }
    for len in [1, 2, 100, 5_000] {
        let input = span(len, 2, tone(2_000.0, 0.25));
        assert_eq!(suppress(&plan, &input, 2, 64, (0, 0)).len(), len);
    }
}

#[test]
fn a_tapered_region_fades_the_suppression_in_and_out() {
    let plan = SuppressPlan::new(RATE, 5_000.0, 7_000.0, false).unwrap();
    let taper = plan.taper();
    assert!(taper > 0);
    let len = 40_000;
    let input = span(len, 2, tone(6_000.0, 0.5));
    let out = suppress(&plan, &input, 2, 4096, (taper, taper));
    // The first and last outputs are barely touched, the middle suppressed.
    let first = out[0][0].hypot(out[0][1]);
    let last = out[len - 1][0].hypot(out[len - 1][1]);
    assert!(first > 0.45 && last > 0.45, "{first} {last}");
    let inner = &out[taper as usize + plan.margin() as usize..len - taper as usize];
    let peak = inner.iter().map(|[i, q]| i.hypot(*q)).fold(0.0, f32::max);
    assert!(peak < SUPPRESSED, "{peak}");
    // The fade is monotonic for a steady tone.
    let fading: Vec<f32> = out[plan.margin() as usize..taper as usize]
        .iter()
        .map(|[i, q]| i.hypot(*q))
        .collect();
    assert!(fading.windows(2).all(|pair| pair[1] <= pair[0] + 1e-3));
}

#[test]
fn a_band_too_narrow_or_outside_the_capture_is_refused() {
    let narrow = RATE / 40_000.0;
    assert!(matches!(
        SuppressPlan::check(RATE, 1_000.0, 1_000.0 + narrow, false),
        Err(ExtractError::TooNarrow { .. })
    ));
    assert!(matches!(
        SuppressPlan::check(RATE, -1_000.0, 1_000.0, true),
        Err(ExtractError::OutsideCapture { .. })
    ));
    assert!(matches!(
        SuppressPlan::check(RATE, 1_000.0, 25_000.0, false),
        Err(ExtractError::OutsideCapture { .. })
    ));
    let plan = SuppressPlan::new(RATE, 1_000.0, 2_000.0, false).unwrap();
    assert!(Suppressor::new(&plan, 10, 6, 5).is_err());
}

/// White noise of `amplitude` per sample, complex or real, from a fixed seed.
fn white(len: usize, channels: usize, seed: u64, amplitude: impl Fn(usize) -> f32) -> Vec<f32> {
    let mut noise = Noise::new(seed);
    let scale = if channels == 2 { std::f32::consts::FRAC_1_SQRT_2 } else { 1.0 };
    (0..len)
        .flat_map(|n| {
            let a = amplitude(n) * scale;
            let pair = [noise.gaussian() * a, noise.gaussian() * a];
            pair.into_iter().take(channels)
        })
        .collect()
}

/// Measure the noise beside `plan`'s band in `input`, as an application does before suppressing.
fn measure(plan: &SuppressPlan, input: &[f32], channels: usize) -> Option<NoiseFill> {
    let len = (input.len() / channels) as u64;
    let mut floor = NoiseFloor::new(plan, len);
    let size = floor.segment_len();
    for start in floor.segments() {
        let from = start as usize * channels;
        floor.push(start, &input[from..from + size * channels], channels);
    }
    floor.finish()
}

fn suppress_filled(plan: &SuppressPlan, input: &[f32], channels: usize) -> Vec<[f32; 2]> {
    let len = (input.len() / channels) as u64;
    let fill = measure(plan, input, channels).expect("neighbours measured");
    let mut suppressor = Suppressor::new(plan, len, 0, 0).unwrap().filled_with(fill);
    let mut out = Vec::new();
    suppressor.push_zeros(plan.margin(), &mut out);
    suppressor.push(input, channels, &mut out);
    let wanted = suppressor.wanted();
    suppressor.push_zeros(wanted, &mut out);
    out
}

/// The mean power per transform bin of `values` between `low` and `high` hertz, over samples `from..to`.
fn band_power(values: &[[f32; 2]], low: f64, high: f64, from: usize, to: usize) -> f64 {
    let size = 1024;
    let fft = FftPlanner::new().plan_fft_forward(size);
    let window: Vec<f32> = (0..size)
        .map(|n| (0.5 - 0.5 * (std::f64::consts::TAU * n as f64 / size as f64).cos()) as f32)
        .collect();
    let power: f32 = window.iter().map(|w| w * w).sum();
    let (mut sum, mut count) = (0.0f64, 0usize);
    let mut start = from;
    while start + size <= to {
        let mut work: Vec<Complex32> = values[start..start + size]
            .iter()
            .zip(&window)
            .map(|(&[i, q], &w)| Complex32::new(i, q) * w)
            .collect();
        fft.process(&mut work);
        for (k, bin) in work.iter().enumerate() {
            let frequency = bin_frequency(k, size, RATE, false).unwrap_or(0.0);
            if (low..high).contains(&frequency) {
                sum += f64::from(bin.norm_sqr() / power);
                count += 1;
            }
        }
        start += size;
    }
    sum / count as f64
}

fn db(ratio: f64) -> f64 {
    10.0 * ratio.log10()
}

#[test]
fn a_suppressed_band_is_filled_with_noise_as_loud_as_its_neighbours() {
    let plan = SuppressPlan::new(RATE, 5_000.0, 7_000.0, false).unwrap();
    let len = 96_000;
    let noise = white(len, 2, 1, |_| 0.01);
    let tone = span(len, 2, tone(6_000.0, 0.5));
    let input: Vec<f32> = noise.iter().zip(&tone).map(|(a, b)| a + b).collect();
    let out = suppress_filled(&plan, &input, 2);
    let inside = band_power(&out, 5_200.0, 6_800.0, 10_000, 86_000);
    let beside = band_power(&out, 1_000.0, 4_000.0, 10_000, 86_000);
    assert!(db(inside / beside).abs() < 1.0, "{} dB", db(inside / beside));
    // No trace of the tone stays above the noise.
    let peak = middle(&out)
        .iter()
        .enumerate()
        .fold((0.0, 0.0), |(re, im), (n, &[i, q])| {
            let turn = -std::f64::consts::TAU * 6_000.0 * n as f64 / RATE;
            let (sin, cos) = turn.sin_cos();
            (re + f64::from(i) * cos - f64::from(q) * sin, im + f64::from(i) * sin + f64::from(q) * cos)
        });
    let level = peak.0.hypot(peak.1) / middle(&out).len() as f64;
    assert!(level < 0.001, "{level}");
}

#[test]
fn the_fill_follows_a_noise_level_that_changes_over_time() {
    let plan = SuppressPlan::new(RATE, -2_000.0, 2_000.0, false).unwrap();
    let len = 400_000;
    let input = white(len, 2, 2, |n| if n < len / 2 { 0.02 } else { 0.002 });
    let out = suppress_filled(&plan, &input, 2);
    for (from, to) in [(20_000, 150_000), (250_000, 380_000)] {
        let inside = band_power(&out, -1_500.0, 1_500.0, from, to);
        let beside = band_power(&out, 6_000.0, 12_000.0, from, to);
        assert!(db(inside / beside).abs() < 1.5, "{from}..{to}: {} dB", db(inside / beside));
    }
}

#[test]
fn a_real_band_is_filled_with_real_noise() {
    let plan = SuppressPlan::new(RATE, 5_000.0, 7_000.0, true).unwrap();
    let len = 96_000;
    let input = white(len, 1, 3, |_| 0.01);
    let out = suppress_filled(&plan, &input, 1);
    assert!(out.iter().all(|&[_, q]| q == 0.0));
    let inside = band_power(&out, 5_200.0, 6_800.0, 10_000, 86_000);
    let beside = band_power(&out, 1_000.0, 4_000.0, 10_000, 86_000);
    assert!(db(inside / beside).abs() < 1.0, "{} dB", db(inside / beside));
}

#[test]
fn nothing_fills_a_band_without_neighbours_and_the_fill_is_the_same_every_time() {
    let whole = SuppressPlan::new(RATE, -24_000.0, 24_000.0, false).unwrap();
    let input = white(10_000, 2, 4, |_| 0.01);
    assert_eq!(measure(&whole, &input, 2), None);
    let short = SuppressPlan::new(RATE, 1_000.0, 3_000.0, false).unwrap();
    assert_eq!(measure(&short, &input[..40], 2), None);
    let plan = SuppressPlan::new(RATE, 1_000.0, 3_000.0, false).unwrap();
    assert_eq!(suppress_filled(&plan, &input, 2), suppress_filled(&plan, &input, 2));
}

#[test]
fn the_background_stays_level_through_the_transitions() {
    let plan = SuppressPlan::new(RATE, 5_000.0, 7_000.0, false).unwrap();
    let len = 192_000;
    let input = white(len, 2, 5, |_| 0.01);
    let out = suppress_filled(&plan, &input, 2);
    let reference = band_power(&out, 1_000.0, 3_000.0, 20_000, 172_000);
    let mut low = 4_200.0;
    while low < 7_800.0 {
        let slice = band_power(&out, low, low + 200.0, 20_000, 172_000);
        assert!(db(slice / reference).abs() < 1.0, "{low} Hz: {} dB", db(slice / reference));
        low += 200.0;
    }
}

#[test]
fn the_background_stays_level_through_the_fade_in_time() {
    let plan = SuppressPlan::new(RATE, -3_000.0, 3_000.0, false).unwrap();
    let len = 120_000;
    let taper = 40_000;
    let input = white(len, 2, 6, |_| 0.01);
    let fill = measure(&plan, &input, 2).unwrap();
    let mut suppressor = Suppressor::new(&plan, len as u64, taper, taper)
        .unwrap()
        .filled_with(fill);
    let mut out = Vec::new();
    suppressor.push_zeros(plan.margin(), &mut out);
    suppressor.push(&input, 2, &mut out);
    let wanted = suppressor.wanted();
    suppressor.push_zeros(wanted, &mut out);
    let reference = band_power(&out, 8_000.0, 14_000.0, 2_000, 118_000);
    for from in (2_000..110_000).step_by(8_192) {
        let slice = band_power(&out, -2_500.0, 2_500.0, from, from + 8_192);
        assert!(db(slice / reference).abs() < 1.0, "{from}: {} dB", db(slice / reference));
    }
}

#[test]
fn a_strong_tone_at_a_narrow_bands_edge_does_not_raise_the_fill() {
    let len = 262_144;
    let noise = white(len, 2, 7, |_| 0.001);
    let tone = span(len, 2, tone(6_001.0, 0.5));
    let input: Vec<f32> = noise.iter().zip(&tone).map(|(a, b)| a + b).collect();
    let plan = SuppressPlan::new(RATE, 5_991.0, 6_011.0, false).unwrap();
    let fill = measure(&plan, &input, 2).unwrap();
    for level in fill.levels {
        let error = db(f64::from(level * level) / 1e-6);
        assert!(error.abs() < 1.5, "{error} dB");
    }
    let real = SuppressPlan::new(RATE, 5_991.0, 6_011.0, true).unwrap();
    let input: Vec<f32> = input.chunks(2).map(|pair| pair[0] * std::f32::consts::SQRT_2).collect();
    let fill = measure(&real, &input, 1).unwrap();
    for level in fill.levels {
        let error = db(f64::from(level * level) / 1e-6);
        assert!(error.abs() < 1.5, "real {error} dB");
    }
}
