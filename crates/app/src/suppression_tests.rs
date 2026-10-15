use super::*;
use argand_core::{Domain, SampleFormat, SampleType};
use argand_edit::{Clip, EditedSource, Piece, SourceId};
use argand_io::OpenHints;
use argand_io::testutil::{TempDir, write_wav};

const RATE: f64 = 48_000.0;
const REFERENCE: f64 = 10_000_000.0;
const LEN: usize = 48_000;

/// A complex capture of a tone at 6 kHz, to be suppressed, and one at -9 kHz, to be kept.
fn capture(dir: &TempDir) -> SourceFile {
    let values: Vec<f32> = (0..LEN)
        .flat_map(|n| {
            let t = n as f64 / RATE;
            let inside = std::f64::consts::TAU * 6_000.0 * t;
            let outside = std::f64::consts::TAU * -9_000.0 * t;
            [
                (0.4 * inside.cos() + 0.2 * outside.cos()) as f32,
                (0.4 * inside.sin() + 0.2 * outside.sin()) as f32,
            ]
        })
        .collect();
    let path = write_wav(
        &dir.join("capture.wav"),
        SampleType::new(Domain::Iq, SampleFormat::F32),
        RATE as u32,
        &values,
        1.0,
    );
    let (source, stamp) = argand_io::open_stamped(&path, &OpenHints::default()).unwrap();
    SourceFile {
        meta: source.meta().clone(),
        hints: OpenHints::default(),
        stamp,
        encoding: None,
    }
}

fn request(dir: &TempDir, source: SourceFile, time: Option<SampleSpan>) -> SuppressRequest {
    let mut meta = source.meta.clone();
    meta.center_freq = REFERENCE;
    SuppressRequest {
        capture: Capture::whole(SourceId(0), source.meta.len_samples),
        sources: vec![Some(source)],
        meta,
        band: FrequencyBand::between(REFERENCE + 5_000.0, REFERENCE + 7_000.0).unwrap(),
        time,
        target: dir.join("computed.wav"),
    }
}

/// The capture with the computed samples in place of their span, read whole.
fn edited(request: &SuppressRequest, suppressed: &Suppressed) -> Vec<f32> {
    let computed = SourceFile {
        meta: suppressed.meta.clone(),
        hints: OpenHints::default(),
        stamp: suppressed.stamp,
        encoding: None,
    };
    let clip = Clip::new(vec![Piece {
        source: SourceId(1),
        start: 0,
        len: suppressed.span.count(),
    }]);
    let capture = request.capture.replace(suppressed.span, &clip);
    let sources = vec![request.sources[0].clone(), Some(computed)];
    let mut source = crate::extraction::open(&capture, &sources, &request.meta).unwrap();
    let mut values = vec![0.0; LEN * request.meta.channels()];
    let mut filled = 0;
    while filled < values.len() {
        filled += source.read(&mut values[filled..]).unwrap();
    }
    values
}

fn original(request: &SuppressRequest) -> Vec<f32> {
    let mut source: EditedSource =
        crate::extraction::open(&request.capture, &request.sources, &request.meta).unwrap();
    let mut values = vec![0.0; LEN * request.meta.channels()];
    let mut filled = 0;
    while filled < values.len() {
        filled += source.read(&mut values[filled..]).unwrap();
    }
    values
}

/// The level of `frequency` over samples `from..to`, by correlation.
fn level(values: &[f32], frequency: f64, from: usize, to: usize) -> f64 {
    let (mut re, mut im) = (0.0, 0.0);
    for n in from..to {
        let turn = -std::f64::consts::TAU * frequency * n as f64 / RATE;
        let (sin, cos) = turn.sin_cos();
        let (i, q) = (f64::from(values[2 * n]), f64::from(values[2 * n + 1]));
        re += i * cos - q * sin;
        im += i * sin + q * cos;
    }
    re.hypot(im) / (to - from) as f64
}

#[test]
fn a_band_is_suppressed_over_the_whole_capture() {
    let dir = TempDir::new("suppress-band");
    let request = request(&dir, capture(&dir), None);
    let suppressed = run(&request, &mut |_, _| {}, &AtomicBool::new(false)).unwrap();
    assert_eq!(suppressed.span, SampleSpan::between(0, LEN as u64).unwrap());
    assert_eq!(suppressed.meta.len_samples, LEN as u64);
    let values = edited(&request, &suppressed);
    let quarter = LEN / 4;
    assert!(level(&values, 6_000.0, quarter, 3 * quarter) < 0.4 * 1e-4);
    assert!((level(&values, -9_000.0, quarter, 3 * quarter) - 0.2).abs() < 1e-4);
}

#[test]
fn a_rectangle_is_suppressed_over_its_span_and_fades_outside_it() {
    let dir = TempDir::new("suppress-rectangle");
    let time = SampleSpan::between(20_000, 30_000).unwrap();
    let request = request(&dir, capture(&dir), Some(time));
    let suppressed = run(&request, &mut |_, _| {}, &AtomicBool::new(false)).unwrap();
    let taper = request.plan().unwrap().taper();
    assert_eq!(
        suppressed.span,
        SampleSpan::between(20_000 - taper, 30_000 + taper).unwrap()
    );
    let values = edited(&request, &suppressed);
    let before = original(&request);
    assert!(level(&values, 6_000.0, 20_000, 30_000) < 0.4 * 1e-4);
    assert!((level(&values, -9_000.0, 20_000, 30_000) - 0.2).abs() < 1e-4);
    let outside = |n: usize| !(20_000 - taper as usize..30_000 + taper as usize).contains(&n);
    for n in (0..LEN).filter(|&n| outside(n)) {
        assert_eq!(values[2 * n..2 * n + 2], before[2 * n..2 * n + 2], "sample {n}");
    }
}

#[test]
fn a_rectangle_at_the_capture_edge_needs_no_taper_there() {
    let dir = TempDir::new("suppress-edge");
    let time = SampleSpan::between(0, 1_000).unwrap();
    let request = request(&dir, capture(&dir), Some(time));
    let suppressed = run(&request, &mut |_, _| {}, &AtomicBool::new(false)).unwrap();
    assert_eq!(suppressed.span.start(), 0);
    assert_eq!(suppressed.span.end(), 1_000 + request.plan().unwrap().taper());
}

#[test]
fn a_cancelled_suppression_leaves_no_file() {
    let dir = TempDir::new("suppress-cancel");
    let request = request(&dir, capture(&dir), None);
    let result = run(&request, &mut |_, _| {}, &AtomicBool::new(true));
    assert!(matches!(result, Err(WriteError::Cancelled)));
    let left: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert_eq!(left, ["capture.wav"]);
}

#[test]
fn a_band_too_narrow_is_refused_before_any_work() {
    let dir = TempDir::new("suppress-narrow");
    let mut request = request(&dir, capture(&dir), None);
    request.band = FrequencyBand::between(REFERENCE + 1_000.0, REFERENCE + 1_000.5).unwrap();
    assert!(request.check().is_err());
}

#[test]
fn a_band_of_an_inverted_capture_is_suppressed_where_it_is_shown() {
    let dir = TempDir::new("suppress-inverted");
    let mut request = request(&dir, capture(&dir), None);
    request.capture = request
        .capture
        .with_adjustment(argand_core::Adjustment::IDENTITY.inverted());
    request.band = FrequencyBand::between(REFERENCE + 8_000.0, REFERENCE + 10_000.0).unwrap();
    let suppressed = run(&request, &mut |_, _| {}, &AtomicBool::new(false)).unwrap();
    let values = edited(&request, &suppressed);
    let quarter = LEN / 4;
    assert!(level(&values, 9_000.0, quarter, 3 * quarter) < 0.2 * 1e-4);
    assert!((level(&values, -6_000.0, quarter, 3 * quarter) - 0.4).abs() < 1e-4);
}

#[test]
fn a_rectangle_of_an_inverted_real_capture_keeps_what_is_shown_around_it() {
    let dir = TempDir::new("suppress-inverted-real");
    let values: Vec<f32> = (0..LEN)
        .map(|n| {
            let t = n as f64 / RATE;
            (0.3 * (std::f64::consts::TAU * 3_000.0 * t).cos()
                + 0.3 * (std::f64::consts::TAU * 9_000.0 * t).cos()) as f32
        })
        .collect();
    let path = write_wav(
        &dir.join("capture.wav"),
        SampleType::new(Domain::Real, SampleFormat::F32),
        RATE as u32,
        &values,
        1.0,
    );
    let (source, stamp) = argand_io::open_stamped(&path, &OpenHints::default()).unwrap();
    let file = SourceFile {
        meta: source.meta().clone(),
        hints: OpenHints::default(),
        stamp,
        encoding: None,
    };
    let mut request = request(&dir, file, SampleSpan::between(20_001, 30_001));
    request.meta.center_freq = 0.0;
    request.capture = request
        .capture
        .with_adjustment(argand_core::Adjustment::new(true, 2.0).unwrap());
    // Shown inverted, the 9 kHz tone sits at 15 kHz and the 3 kHz one at 21 kHz.
    request.band = FrequencyBand::between(14_000.0, 16_000.0).unwrap();
    let suppressed = run(&request, &mut |_, _| {}, &AtomicBool::new(false)).unwrap();
    let (before, after) = (original(&request), edited(&request, &suppressed));
    let start = suppressed.span.start() as usize;
    assert_eq!(after[..start], before[..start]);
    let real_level = |values: &[f32], frequency: f64| {
        let (mut re, mut im) = (0.0, 0.0);
        for (n, value) in values.iter().enumerate().take(29_000).skip(22_000) {
            let turn = std::f64::consts::TAU * frequency * n as f64 / RATE;
            re += f64::from(*value) * turn.cos();
            im += f64::from(*value) * turn.sin();
        }
        2.0 * re.hypot(im) / 7_000.0
    };
    assert!(real_level(&after, 15_000.0) < 1e-3, "{}", real_level(&after, 15_000.0));
    assert!((real_level(&after, 21_000.0) - 0.6).abs() < 1e-3);
}
