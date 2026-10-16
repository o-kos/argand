use super::*;
use argand_core::{Adjustment, Domain, SampleFormat, SampleType};
use argand_edit::SourceId;
use argand_io::OpenHints;
use argand_io::testutil::{TempDir, write_wav};

fn request(dir: &TempDir, domain: Domain, values: &[f32]) -> LevelRequest {
    let path = write_wav(
        &dir.join("capture.wav"),
        SampleType::new(domain, SampleFormat::F32),
        48_000,
        values,
        1.0,
    );
    let (source, stamp) = argand_io::open_stamped(&path, &OpenHints::default()).unwrap();
    let meta = source.meta().clone();
    LevelRequest {
        capture: Capture::whole(SourceId(0), meta.len_samples),
        sources: vec![Some(SourceFile {
            meta: meta.clone(),
            hints: OpenHints::default(),
            stamp,
            encoding: None,
        })],
        meta,
    }
}

#[test]
fn the_peak_of_a_complex_capture_is_its_largest_magnitude() {
    let dir = TempDir::new("level-iq");
    let request = request(&dir, Domain::Iq, &[0.3, 0.4, -0.45, 0.0, 0.1, -0.2]);
    let peak = run(&request, &mut |_, _| {}, &AtomicBool::new(false)).unwrap();
    assert!((peak - 0.5).abs() < 1e-6);
}

#[test]
fn the_peak_of_a_real_capture_is_its_largest_absolute_value_whatever_its_adjustment() {
    let dir = TempDir::new("level-real");
    let mut request = request(&dir, Domain::Real, &[0.25, -0.75, 0.5]);
    request.capture = request
        .capture
        .with_adjustment(Adjustment::new(true, 4.0).unwrap());
    let peak = run(&request, &mut |_, _| {}, &AtomicBool::new(false)).unwrap();
    assert!((peak - 0.75).abs() < 1e-6);
}

#[test]
fn a_cancelled_measurement_ends_without_a_peak() {
    let dir = TempDir::new("level-cancel");
    let request = request(&dir, Domain::Real, &[0.5; 64]);
    let result = run(&request, &mut |_, _| {}, &AtomicBool::new(true));
    assert!(matches!(result, Err(WriteError::Cancelled)));
}

#[test]
fn the_gain_puts_the_peak_just_below_full_scale() {
    let gain = gain(0.25, 1.0).unwrap();
    assert!((20.0 * (gain * 0.25).log10() - TARGET).abs() < 1e-9);
    assert!((super::gain(8_192.0, 32_768.0).unwrap() - 4.0 * 10f64.powf(TARGET / 20.0)).abs() < 1e-12);
    assert_eq!(super::gain(0.0, 1.0), None);
}

/// An I/Q 16-bit file holding `peak` of full scale, opened with `hints`.
fn i16_file(dir: &TempDir, name: &str, peak: f32, hints: OpenHints) -> SourceFile {
    let iq_i16: SampleType = "iq_i16".parse().unwrap();
    let path = write_wav(&dir.join(name), iq_i16, 48_000, &[peak, 0.0, 0.0, -peak / 2.0], 1.0);
    let (source, stamp) = argand_io::open_stamped(&path, &hints).unwrap();
    SourceFile {
        meta: source.meta().clone(),
        hints,
        stamp,
        encoding: None,
    }
}

#[test]
fn files_read_on_different_scales_are_measured_in_stored_units() {
    let dir = TempDir::new("level-scales");
    let louder = OpenHints {
        normalize: Some(argand_io::Normalize::Factor(16_384.0)),
        ..Default::default()
    };
    let quiet = i16_file(&dir, "a.wav", 0.1, louder);
    let loud = i16_file(&dir, "b.wav", 0.9, OpenHints::default());
    let clip = argand_edit::Clip::new(vec![argand_edit::Piece {
        source: SourceId(1),
        start: 0,
        len: 2,
    }]);
    let request = LevelRequest {
        capture: Capture::whole(SourceId(0), 2).insert(2, &clip),
        meta: quiet.meta.clone(),
        sources: vec![Some(quiet), Some(loud)],
    };
    let peak = run(&request, &mut |_, _| {}, &AtomicBool::new(false)).unwrap();
    assert!((peak - 0.9 * 32_768.0).abs() < 1.0, "{peak}");
}
