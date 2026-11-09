use super::*;
use argand_core::{Adjustment, Domain, SampleFormat, SampleType};
use argand_edit::{Clip, Piece, SourceId};
use argand_io::testutil::{TempDir, write_wav};
use argand_io::{Normalize, OpenHints};

fn request(dir: &TempDir, sample_type: SampleType, values: &[f32], hints: OpenHints) -> Request {
    request_at_rate(dir, sample_type, values, hints, 8000)
}

fn request_at_rate(
    dir: &TempDir,
    sample_type: SampleType,
    values: &[f32],
    hints: OpenHints,
    rate: u32,
) -> Request {
    let path = write_wav(&dir.join("input.wav"), sample_type, rate, values, 1.0);
    let (source, stamp) = argand_io::open_stamped(&path, &hints).unwrap();
    let input = source.meta().clone();
    Request {
        capture: Capture::whole(SourceId(0), input.len_samples),
        sources: vec![Some(SourceFile {
            meta: input.clone(),
            hints,
            stamp,
            encoding: None,
        })],
        output: input.clone(),
        carrier: None,
        input,
        target: dir.join("output.wav"),
        protected: Vec::new(),
        container: argand_io::write::Container::Wave,
        level: Level::Preserve,
        full_scale: f64::from(sample_type.full_scale()),
        copy: None,
    }
}

fn read(request: &Request) -> (SignalMeta, Vec<f32>) {
    let mut source = argand_io::open(
        &request.target,
        &OpenHints {
            normalize: Some(Normalize::None),
            ..OpenHints::default()
        },
    )
    .unwrap();
    let meta = source.meta().clone();
    let mut values = vec![0.0; meta.len_samples as usize * meta.channels()];
    let mut done = 0;
    while done < values.len() {
        done += source.read(&mut values[done..]).unwrap();
    }
    (meta, values)
}

#[test]
fn preserve_removes_reader_normalization_once_and_keeps_capture_adjustments_for_all_types() {
    for token in argand_core::SAMPLE_TYPE_TOKENS {
        let dir = TempDir::new(token);
        let sample_type: SampleType = token.parse().unwrap();
        let channels = sample_type.channels();
        let values = [0.25, -0.5, 0.5, 0.25, 0.0, 0.0, -0.25, 0.5];
        let mut request = request(
            &dir,
            sample_type,
            &values[..4 * channels],
            OpenHints {
                normalize: Some(Normalize::Auto),
                ..OpenHints::default()
            },
        );
        request.output.sample_type = SampleType::new(sample_type.domain, SampleFormat::F32);
        let adjustment = Adjustment::new(true, 0.5).unwrap();
        request.capture = request.capture.with_adjustment(adjustment);
        run(&request, &mut |_, _| {}, &AtomicBool::new(false)).unwrap();
        let (_, actual) = read(&request);
        let mut expected = values[..4 * channels].to_vec();
        adjustment.apply(&mut expected, channels, 0);
        for (&a, &b) in actual.iter().zip(&expected) {
            assert!((a - b).abs() < 1e-6, "{token}: {actual:?} != {expected:?}");
        }
    }
}

#[test]
fn arbitrary_float_units_are_preserved_despite_auto_normalization() {
    for domain in [Domain::Real, Domain::Iq] {
        let dir = TempDir::new("conversion-unscaled");
        let mut request = request(
            &dir,
            SampleType::new(domain, SampleFormat::F16x8),
            &[16.0, -8.0, 4.0, 0.0],
            OpenHints::default(),
        );
        request.output.sample_type.format = SampleFormat::F32;
        run(&request, &mut |_, _| {}, &AtomicBool::new(false)).unwrap();
        assert_eq!(read(&request).1, [16.0, -8.0, 4.0, 0.0]);
    }
}

#[test]
fn normalize_measures_output_magnitude_with_one_iq_gain_and_preserves_silence() {
    for values in [[0.3, 0.4, -0.3, -0.4], [0.0; 4]] {
        let dir = TempDir::new("conversion-normalize");
        let mut request = request(
            &dir,
            SampleType::new(Domain::Iq, SampleFormat::F32),
            &values,
            OpenHints::default(),
        );
        request.level = Level::Normalize;
        let result = run(&request, &mut |_, _| {}, &AtomicBool::new(false)).unwrap();
        let (_, actual) = read(&request);
        let target = if values[0] == 0.0 {
            0.0
        } else {
            10f64.powf(crate::levelling::TARGET / 20.0)
        };
        assert!((f64::from(actual[0]).hypot(f64::from(actual[1])) - target).abs() < 1e-6);
        let expected_gain = if target == 0.0 { 1.0 } else { target / 0.5 };
        assert!((f64::from(actual[0]) - f64::from(values[0]) * expected_gain).abs() < 1e-6);
        assert_eq!(result.clipped, 0);
    }
}

#[test]
fn mixed_computed_pieces_use_document_encoding_without_a_second_scale() {
    let dir = TempDir::new("conversion-computed");
    let mut request = request(
        &dir,
        "iq_i16".parse().unwrap(),
        &[0.25, 0.0, 0.5, 0.0],
        OpenHints {
            normalize: Some(Normalize::Auto),
            ..OpenHints::default()
        },
    );
    let scratch = write_wav(
        &dir.join("computed.wav"),
        "iq_f32".parse().unwrap(),
        8000,
        &[0.125, 0.0],
        1.0,
    );
    let (source, stamp) = argand_io::open_stamped(&scratch, &OpenHints::default()).unwrap();
    request.sources.push(Some(SourceFile {
        meta: source.meta().clone(),
        hints: OpenHints::default(),
        stamp,
        encoding: Some(argand_io::write::Encoding {
            factor: 65536.0,
            offset: 0.0,
        }),
    }));
    request.capture = request.capture.insert(
        1,
        &Clip::new(vec![Piece {
            source: SourceId(1),
            start: 0,
            len: 1,
        }]),
    );
    request.output.sample_type.format = SampleFormat::F32;
    run(&request, &mut |_, _| {}, &AtomicBool::new(false)).unwrap();
    assert_eq!(read(&request).1, [0.25, 0.0, 0.25, 0.0, 0.5, 0.0]);
}

#[test]
fn failures_cancellation_and_changed_sources_do_not_publish_or_modify_input() {
    let dir = TempDir::new("conversion-safety");
    let mut request = request(
        &dir,
        "rl_f32".parse().unwrap(),
        &[0.25; 64],
        OpenHints::default(),
    );
    let before = std::fs::read(&request.input.source).unwrap();
    std::fs::write(&request.target, b"kept").unwrap();
    assert!(matches!(
        run(&request, &mut |_, _| {}, &AtomicBool::new(true)),
        Err(WriteError::Cancelled)
    ));
    request.target = request.input.source.clone();
    assert!(run(&request, &mut |_, _| {}, &AtomicBool::new(false)).is_err());
    assert_eq!(std::fs::read(&request.input.source).unwrap(), before);
    request.target = dir.join("output.wav");
    request.sources[0].as_mut().unwrap().stamp = None;
    assert!(run(&request, &mut |_, _| {}, &AtomicBool::new(false)).is_err());
    assert_eq!(std::fs::read(&request.target).unwrap(), b"kept");
}

#[test]
fn conversion_preserves_exact_fractional_rate_reference_and_duration() {
    let dir = TempDir::new("conversion-metadata");
    let values: Vec<f32> = (0..8000)
        .flat_map(|n| {
            let phase = std::f64::consts::TAU * 1000.0 * n as f64 / 8000.0;
            [0.5 * phase.cos() as f32, 0.5 * phase.sin() as f32]
        })
        .collect();
    let mut request = request(
        &dir,
        "iq_f32".parse().unwrap(),
        &values,
        OpenHints::default(),
    );
    request.output.sample_rate = 6000.125;
    request.output.center_freq = 123.456;
    run(&request, &mut |_, _| {}, &AtomicBool::new(false)).unwrap();
    let (meta, _) = read(&request);
    assert_eq!(meta.sample_rate, 6000.125);
    assert_eq!(meta.center_freq, 123.456);
    assert_eq!(meta.len_samples, 6001);
}

#[test]
fn identity_copy_keeps_i32_low_bits_and_full_band_endpoints_byte_exact() {
    let dir = TempDir::new("conversion-i32-copy");
    let mut request = request(
        &dir,
        "iq_i32".parse().unwrap(),
        &[0.0; 8],
        OpenHints::default(),
    );
    let mut bytes = std::fs::read(&request.input.source).unwrap();
    let start = bytes.windows(4).position(|v| v == b"data").unwrap() + 8;
    let ints = [i32::MIN, i32::MAX, -1, 1, 16777217, -16777217, 0, 123456789];
    for (at, value) in ints.into_iter().enumerate() {
        bytes[start + at * 4..start + (at + 1) * 4].copy_from_slice(&value.to_le_bytes());
    }
    std::fs::write(&request.input.source, &bytes).unwrap();
    request.sources[0].as_mut().unwrap().stamp =
        argand_io::write::SourceStamp::of(&request.input.source).ok();
    request.copy = Some(argand_io::write::SaveRequest::span(
        request.sources[0].clone().unwrap(),
        None,
        request.target.clone(),
    ));
    let result = run(&request, &mut |_, _| {}, &AtomicBool::new(false)).unwrap();
    assert_eq!(result.clipped, 0);
    let output = std::fs::read(&request.target).unwrap();
    let output_start = output.windows(4).position(|v| v == b"data").unwrap() + 8;
    assert_eq!(
        &output[output_start..output_start + 32],
        &bytes[start..start + 32]
    );
}

#[test]
fn normalization_happens_after_domain_conversion_and_reports_integer_clips_in_preserve() {
    let dir = TempDir::new("conversion-normalized-transform");
    let values: Vec<f32> = (0..8000)
        .map(|n| (std::f64::consts::TAU * 1000.0 * n as f64 / 8000.0).cos() as f32 * 2.0)
        .collect();
    let mut request = request(
        &dir,
        "rl_f32".parse().unwrap(),
        &values,
        OpenHints::default(),
    );
    request.output.sample_type = "iq_i16".parse().unwrap();
    request.output.sample_rate = 6000.0;
    request.output.center_freq = 1500.0;
    let result = run(&request, &mut |_, _| {}, &AtomicBool::new(false)).unwrap();
    assert!(result.clipped > 0);
    request.level = Level::Normalize;
    let result = run(&request, &mut |_, _| {}, &AtomicBool::new(false)).unwrap();
    assert_eq!(result.clipped, 0);
    let (_, values) = read(&request);
    let peak = values
        .chunks_exact(2)
        .map(|v| v[0].hypot(v[1]))
        .fold(0.0, f32::max);
    assert!((f64::from(peak) - 10f64.powf(crate::levelling::TARGET / 20.0)).abs() < 0.001);
}

#[test]
fn nonfinite_data_never_replaces_an_existing_target_even_in_identity_mode() {
    for bad in [f32::NAN, f32::INFINITY] {
        let dir = TempDir::new("conversion-nonfinite");
        let request = request(
            &dir,
            "rl_f32".parse().unwrap(),
            &[0.25, bad],
            OpenHints::default(),
        );
        std::fs::write(&request.target, b"kept").unwrap();
        assert!(run(&request, &mut |_, _| {}, &AtomicBool::new(false)).is_err());
        assert_eq!(std::fs::read(&request.target).unwrap(), b"kept");
    }
}

#[test]
fn cancellation_and_source_change_after_a_block_leave_the_existing_target_alone() {
    for change in [false, true] {
        let dir = TempDir::new("conversion-interrupted");
        let request = request(
            &dir,
            "rl_f32".parse().unwrap(),
            &[0.25; 10000],
            OpenHints::default(),
        );
        std::fs::write(&request.target, b"kept").unwrap();
        let cancel = AtomicBool::new(false);
        let mut changed = false;
        let result = run(
            &request,
            &mut |done, _| {
                if done > 0 && !changed {
                    changed = true;
                    if change {
                        use std::io::Write;
                        std::fs::OpenOptions::new()
                            .append(true)
                            .open(&request.input.source)
                            .unwrap()
                            .write_all(&[0])
                            .unwrap();
                    } else {
                        cancel.store(true, Ordering::Relaxed);
                    }
                }
            },
            &cancel,
        );
        assert!(result.is_err());
        assert_eq!(std::fs::read(&request.target).unwrap(), b"kept");
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 2);
    }
}

#[test]
fn carrier_translation_preserves_two_sided_tones_amplitude_and_physical_metadata() {
    for (rate, physical_reference) in [
        (24000., 0.),
        (24000.125, 1_000_000_000.5),
        (24000., 1e20),
        (8000.15, 1e10),
    ] {
        let dir = TempDir::new("carrier-roundtrip");
        let count = 32768;
        let values: Vec<f32> = (0..count)
            .flat_map(|n| {
                let phase = std::f64::consts::TAU * n as f64 / 4.;
                [(0.35 * phase.cos()) as f32, (-0.05 * phase.sin()) as f32]
            })
            .collect();
        let mut forward = request_at_rate(
            &dir,
            "iq_f32".parse().unwrap(),
            &values,
            OpenHints {
                normalize: Some(Normalize::None),
                ..OpenHints::default()
            },
            24000,
        );
        // The source reader's exact fractional metadata is part of the input contract.
        forward.input.sample_rate = rate;
        forward.input.center_freq = physical_reference;
        forward.output = forward.input.clone();
        forward.output.sample_type = "rl_f32".parse().unwrap();
        forward.output.sample_rate = rate * 2.;
        forward.output.center_freq =
            carrier_reference(&forward.input, &forward.output, rate / 2.).unwrap();
        forward.carrier = Some(rate / 2.);
        let plan = forward.plan().unwrap();
        assert!(plan.retains_input_band());
        assert_eq!(
            plan.band, plan.input_band,
            "default retains the nominal whole band"
        );
        run(&forward, &mut |_, _| {}, &AtomicBool::new(false)).unwrap();
        let (real_meta, real) = read(&forward);
        for (frequency, expected) in [(0.125, 0.2), (0.375, 0.15)] {
            let amplitude = carrier_tone_amplitude(&real, 1, frequency, 8192, count);
            assert!((20. * (amplitude / expected).log10()).abs() < 0.1);
        }
        assert_eq!(real_meta.sample_rate, rate * 2.);
        assert_eq!(real_meta.center_freq, physical_reference - rate / 2.);
        assert_eq!(real.len(), count * 2);
        for (n, &actual) in real.iter().enumerate().skip(8192).take(count) {
            let phase = std::f64::consts::TAU * n as f64 / 8.;
            let expected = 0.2 * phase.cos() + 0.15 * (3. * phase).cos();
            assert!(
                (f64::from(actual) - expected).abs() < 0.002,
                "forward {n}: {actual} vs {expected}"
            );
        }
        let mut reverse = carrier_reverse_source(&forward);
        reverse.output = reverse.input.clone();
        reverse.output.sample_type = "iq_f32".parse().unwrap();
        reverse.output.sample_rate = rate;
        reverse.output.center_freq =
            carrier_reference(&reverse.input, &reverse.output, rate / 2.).unwrap();
        reverse.carrier = Some(rate / 2.);
        reverse.target = dir.join("roundtrip.wav");
        run(&reverse, &mut |_, _| {}, &AtomicBool::new(false)).unwrap();
        let (iq_meta, iq) = read(&reverse);
        for (frequency, expected) in [(-0.25, 0.2), (0.25, 0.15)] {
            let amplitude = carrier_tone_amplitude(&iq, 2, frequency, 4096, count - 8192);
            assert!((20. * (amplitude / expected).log10()).abs() < 0.2);
        }
        assert_eq!(iq_meta.sample_rate, rate);
        assert_eq!(iq_meta.center_freq, physical_reference);
        assert_eq!(iq.len(), values.len());
        for (frame, expected) in iq
            .chunks_exact(2)
            .zip(values.chunks_exact(2))
            .skip(4096)
            .take(count - 8192)
        {
            assert!((frame[0] - expected[0]).abs() < 0.003);
            assert!((frame[1] - expected[1]).abs() < 0.003);
        }
    }
}

#[test]
fn carrier_bounds_are_the_real_arrays_nyquist_not_the_metadata_origin() {
    let dir = TempDir::new("carrier-bounds");
    let input = request(
        &dir,
        "iq_f32".parse().unwrap(),
        &[0., 0.],
        OpenHints::default(),
    )
    .input;
    let mut output = input.clone();
    output.sample_type = "rl_f32".parse().unwrap();
    output.sample_rate = 16000.;
    for invalid in [-1., 8000.1, f64::INFINITY, f64::NAN] {
        assert!(carrier_reference(&input, &output, invalid).is_err());
    }
    assert_eq!(
        carrier_reference(&input, &output, 4000.).unwrap(),
        input.center_freq - 4000.
    );
}

fn carrier_tone_amplitude(
    values: &[f32],
    channels: usize,
    frequency: f64,
    skip: usize,
    count: usize,
) -> f64 {
    let mut sum = [0., 0.];
    for (n, frame) in values
        .chunks_exact(channels)
        .enumerate()
        .skip(skip)
        .take(count)
    {
        let phase = std::f64::consts::TAU * frequency * n as f64;
        let (sin, cos) = phase.sin_cos();
        let i = f64::from(frame[0]);
        let q = if channels == 2 {
            f64::from(frame[1])
        } else {
            0.
        };
        sum[0] += i * cos + q * sin;
        sum[1] += q * cos - i * sin;
    }
    sum[0].hypot(sum[1]) / count as f64 * if channels == 1 { 2. } else { 1. }
}

fn carrier_reverse_source(forward: &Request) -> Request {
    let (source, stamp) = argand_io::open_stamped(
        &forward.target,
        &OpenHints {
            normalize: Some(Normalize::None),
            ..OpenHints::default()
        },
    )
    .unwrap();
    let mut reverse = forward.clone();
    reverse.input = source.meta().clone();
    reverse.capture = Capture::whole(SourceId(0), reverse.input.len_samples);
    reverse.sources = vec![Some(SourceFile {
        meta: reverse.input.clone(),
        hints: OpenHints {
            normalize: Some(Normalize::None),
            ..OpenHints::default()
        },
        stamp,
        encoding: None,
    })];
    reverse
}
