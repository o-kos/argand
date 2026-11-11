use super::*;
use argand_core::{Domain, SampleFormat, SampleSpan, SampleType};
use argand_edit::{Clip, Piece, SourceId};
use argand_io::OpenHints;
use argand_io::testutil::{TempDir, write_wav};
use std::sync::atomic::Ordering;

fn indexed(
    dir: &TempDir,
    name: &str,
    iq: bool,
    values: &[f32],
    block: usize,
) -> (SourceFile, Arc<Index>, Vec<f32>) {
    let kind = SampleType::new(
        if iq { Domain::Iq } else { Domain::Real },
        SampleFormat::F32,
    );
    let path = write_wav(&dir.join(name), kind, 48000, values, 1.);
    let hints = OpenHints::default();
    let (mut reader, stamp) = argand_io::open_stamped(&path, &hints).unwrap();
    let file = SourceFile {
        meta: reader.meta().clone(),
        hints,
        stamp,
        encoding: None,
    };
    let channels = file.meta.channels();
    let mut index = Index::new(
        file.meta.len_samples,
        channels,
        reader.original_sample_units().map(|(factor, _)| factor),
    );
    let mut decoded = vec![0.; values.len()];
    for (part, buffer) in decoded.chunks_mut(block * channels).enumerate() {
        assert_eq!(reader.read(buffer).unwrap(), buffer.len());
        index.fold(buffer, (part * block) as u64);
    }
    (file, Arc::new(index.finish()), decoded)
}

fn expected(values: &[f32], channels: usize, factor: f64, gain: f64) -> Metrics {
    let mut peak: f64 = 0.;
    let mut energy = 0.;
    for frame in values.chunks_exact(channels) {
        let squared: f64 = frame
            .iter()
            .map(|&v| f64::from((f64::from(v) * gain) as f32) * factor)
            .map(|v| v * v)
            .sum();
        energy += squared;
        peak = peak.max(squared.sqrt());
    }
    Metrics {
        peak,
        rms: (energy / (values.len() / channels) as f64).sqrt(),
    }
}

fn near(actual: Metrics, expected: Metrics) {
    for (a, b) in [(actual.peak, expected.peak), (actual.rms, expected.rms)] {
        assert!(
            (a - b).abs() <= b.abs() * 2e-7 + f64::from(f32::from_bits(1)),
            "{actual:?} != {expected:?}"
        );
    }
}

#[test]
fn chunks_partial_cut_paste_and_current_encoding_match_independent_samples_with_bounded_cached_io()
{
    let dir = TempDir::new("sample-statistics");
    for iq in [false, true] {
        let channels = if iq { 2 } else { 1 };
        let values: Vec<f32> = (0..200003 * channels)
            .map(|at| (at % 97) as f32 / 128. - 0.375)
            .collect();
        let (file, index, decoded) = indexed(
            &dir,
            if iq { "iq.wav" } else { "real.wav" },
            iq,
            &values,
            113,
        );
        assert!(index.tree.len() * std::mem::size_of::<Aggregate>() <= 262144);
        let mut request = Request {
            capture: Capture::whole(SourceId(0), 200003),
            sources: vec![Some(file)],
            indices: vec![Some(index.clone())],
            adjustment: Adjustment::IDENTITY,
        };
        let cache = Cache::default();
        near(
            measure(&request, &cache, &|| true).unwrap(),
            expected(&decoded, channels, index.factor.unwrap(), 1.),
        );
        assert_eq!(
            cache.1.load(Ordering::Relaxed),
            0,
            "whole-source summaries require no sample IO"
        );
        let piece = Piece {
            source: SourceId(0),
            start: 137,
            len: 176406,
        };
        request.capture = Capture::whole(SourceId(0), 0).insert(0, &Clip::new(vec![piece, piece]));
        request.sources[0].as_mut().unwrap().encoding = Some(argand_io::write::Encoding {
            factor: 32768.,
            offset: 128.,
        });
        request.adjustment = Adjustment::new(true, 1.37).unwrap();
        let selected = decoded[137 * channels..176543 * channels].repeat(2);
        near(
            measure(&request, &cache, &|| true).unwrap(),
            expected(&selected, channels, 32768., 1.37),
        );
        let read = cache.1.load(Ordering::Relaxed);
        assert!(
            read <= 2 * 200003_u64.div_ceil(4096),
            "boundary frames read: {read}"
        );
        request.adjustment = Adjustment::new(false, 0.75).unwrap();
        near(
            measure(&request, &cache, &|| true).unwrap(),
            expected(&selected, channels, 32768., 0.75),
        );
        assert_eq!(
            cache.1.load(Ordering::Relaxed),
            read,
            "gain and duplicate pieces reuse boundary summaries"
        );
        assert!(measure(&request, &cache, &|| false).is_err());
    }
}

#[test]
fn nonfinite_only_poison_included_samples_empty_is_unavailable_and_gain_overflow_is_detected() {
    let dir = TempDir::new("sample-statistics-invalid");
    let (file, mut index, _) = indexed(&dir, "invalid.wav", false, &[0.25; 5001], 7);
    let mut raw = vec![0.25; 5001];
    raw[2500] = f32::NAN;
    let mut invalid = Index::new(5001, 1, Some(1.));
    invalid.fold(&raw, 0);
    *Arc::make_mut(&mut index) = invalid.finish();
    let mut request = Request {
        capture: Capture::whole(SourceId(0), 5001),
        sources: vec![Some(file)],
        indices: vec![Some(index)],
        adjustment: Adjustment::IDENTITY,
    };
    let cache = Cache::default();
    assert!(measure(&request, &cache, &|| true).is_err());
    request.capture = request
        .capture
        .delete(SampleSpan::between(2490, 2510).unwrap());
    let good = measure(&request, &cache, &|| true).unwrap();
    near(
        good,
        Metrics {
            peak: 0.25,
            rms: 0.25,
        },
    );
    request.adjustment = Adjustment::new(false, f64::from(f32::MAX) * 8.).unwrap();
    assert!(measure(&request, &cache, &|| true).is_err());
    request.capture = Capture::whole(SourceId(0), 0);
    assert!(measure(&request, &cache, &|| true).is_err());
}

#[test]
fn source_mutation_is_detected_even_when_every_summary_was_cached() {
    use std::io::Write;
    let dir = TempDir::new("statistics-stamp");
    let (file, index, _) = indexed(&dir, "source.wav", false, &[0.25; 100], 13);
    let path = file.meta.source.clone();
    let request = Request {
        capture: Capture::whole(SourceId(0), 100),
        sources: vec![Some(file)],
        indices: vec![Some(index)],
        adjustment: Adjustment::IDENTITY,
    };
    let cache = Cache::default();
    assert!(measure(&request, &cache, &|| true).is_ok());
    std::fs::OpenOptions::new()
        .append(true)
        .open(path)
        .unwrap()
        .write_all(&[0])
        .unwrap();
    assert!(measure(&request, &cache, &|| true).is_err());
    assert_eq!(cache.1.load(Ordering::Relaxed), 0);
}

#[test]
fn gain_overflow_uses_actual_f32_rounding_at_the_maximum_boundary() {
    let dir = TempDir::new("statistics-rounding");
    let (file, index, _) = indexed(&dir, "source.wav", false, &[0.25], 1);
    let mut request = Request {
        capture: Capture::whole(SourceId(0), 1),
        sources: vec![Some(file)],
        indices: vec![Some(index)],
        adjustment: Adjustment::new(false, 4. * f64::from(f32::MAX) * (1. + 2f64.powi(-26)))
            .unwrap(),
    };
    let cache = Cache::default();
    near(
        measure(&request, &cache, &|| true).unwrap(),
        Metrics {
            peak: f64::from(f32::MAX),
            rms: f64::from(f32::MAX),
        },
    );
    request.adjustment =
        Adjustment::new(false, 4. * f64::from(f32::MAX) * (1. + 2f64.powi(-22))).unwrap();
    assert!(measure(&request, &cache, &|| true).is_err());
}

#[test]
fn native_units_survive_initial_normalization_and_capture_gain_for_all_formats() {
    use argand_io::Normalize;
    let dir = TempDir::new("statistics-native-units");
    for kind in argand_io::testutil::all_sample_types() {
        let channels = kind.channels();
        let arbitrary = kind.format == SampleFormat::F16x8;
        let stored_scale = if arbitrary {
            16.
        } else {
            kind.format.full_scale()
        };
        let values = [0., 0., 0.25, -0.5, 0., 0., 0.125, -0.25];
        let path = write_wav(
            &dir.join(&format!("{kind}.wav")),
            kind,
            48000,
            &values,
            if arbitrary { 16. } else { 1. },
        );
        let hints = OpenHints {
            normalize: Some(if arbitrary {
                Normalize::Auto
            } else {
                Normalize::Factor(7.)
            }),
            gain_db: 6.,
            ..OpenHints::default()
        };
        let (mut source, stamp) = argand_io::open_stamped(&path, &hints).unwrap();
        let file = SourceFile {
            meta: source.meta().clone(),
            hints,
            stamp,
            encoding: None,
        };
        let mut decoded = [0.; 8];
        assert_eq!(source.read(&mut decoded).unwrap(), 8);
        let mut index = Index::new(
            file.meta.len_samples,
            channels,
            source.original_sample_units().map(|(factor, _)| factor),
        );
        index.fold(&decoded, 0);
        let mut request = Request {
            capture: Capture::whole(SourceId(0), file.meta.len_samples),
            sources: vec![Some(file)],
            indices: vec![Some(Arc::new(index.finish()))],
            adjustment: Adjustment::new(true, 1.37).unwrap(),
        };
        near(
            measure(&request, &Default::default(), &|| true).unwrap(),
            expected(&values, channels, f64::from(stored_scale), 1.37),
        );
        request.capture = Capture::whole(SourceId(0), 2 / channels as u64);
        near(
            measure(&request, &Default::default(), &|| true).unwrap(),
            Metrics { peak: 0., rms: 0. },
        );
    }
}
