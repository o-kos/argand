use super::*;
use argand_core::{Adjustment, Domain, SampleFormat, SampleType};
use argand_edit::{Clip, Piece, SourceId};
use argand_io::testutil::{TempDir, write_wav};
use argand_io::{Normalize, OpenHints};

fn request(dir: &TempDir, sample_type: SampleType, values: &[f32], hints: OpenHints) -> Request {
    let path = write_wav(&dir.join("input.wav"), sample_type, 8000, values, 1.0);
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
