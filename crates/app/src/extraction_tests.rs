use super::*;
use argand_core::{Domain, SampleFormat, SampleType};
use argand_io::OpenHints;
use argand_io::testutil::{TempDir, iq_tone, write_wav};

const RATE: f64 = 48_000.0;

/// A complex tone at `frequency` on disk, as a source of the request.
fn capture(dir: &TempDir, frequency: f64) -> SourceFile {
    let values = iq_tone(48_000, RATE, frequency, 0.5);
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
    }
}

fn request(source: SourceFile, span: SampleSpan, band: FrequencyBand, target: PathBuf) -> ExtractRequest {
    let mut meta = source.meta.clone();
    meta.center_freq = 10_000_000.0;
    ExtractRequest {
        capture: Capture::whole(argand_edit::SourceId(0), source.meta.len_samples),
        sources: vec![Some(source)],
        meta,
        span,
        band,
        target,
        protected: Vec::new(),
    }
}

fn read_all(path: &std::path::Path) -> (SignalMeta, Vec<f32>) {
    let mut source = argand_io::open(path, &OpenHints::default()).unwrap();
    let meta = source.meta().clone();
    let mut values = vec![0.0; meta.len_samples as usize * 2];
    let mut filled = 0;
    while filled < values.len() {
        filled += source.read(&mut values[filled..]).unwrap();
    }
    (meta, values)
}

#[test]
fn a_band_saves_as_a_complex_capture_centred_on_it() {
    let dir = TempDir::new("extract-band");
    let source = capture(&dir, 6_500.0);
    let target = dir.join("band.wav");
    let whole = SampleSpan::between(0, 48_000).unwrap();
    let band = FrequencyBand::between(10_005_000.0, 10_007_000.0).unwrap();
    let request = request(source, whole, band, target.clone());
    let saved = run(&request, &mut |_, _| {}, &AtomicBool::new(false)).unwrap();
    let (meta, values) = read_all(&target);
    assert_eq!(meta.sample_rate, RATE / 19.0);
    assert_eq!(meta.center_freq, 10_006_000.0);
    assert_eq!(meta.len_samples, 48_000u64.div_ceil(19));
    assert_eq!(saved.samples, meta.len_samples);
    let magnitudes: Vec<f32> = values.chunks(2).map(|iq| iq[0].hypot(iq[1])).collect();
    let middle = &magnitudes[magnitudes.len() / 4..magnitudes.len() * 3 / 4];
    assert!(middle.iter().all(|m| (m - 0.5).abs() < 0.01), "{middle:?}");
}

#[test]
fn a_rectangle_saves_its_span_only() {
    let dir = TempDir::new("extract-rectangle");
    let source = capture(&dir, 6_500.0);
    let target = dir.join("rectangle.wav");
    let span = SampleSpan::between(10_000, 20_000).unwrap();
    let band = FrequencyBand::between(10_005_000.0, 10_007_000.0).unwrap();
    run(&request(source, span, band, target.clone()), &mut |_, _| {}, &AtomicBool::new(false)).unwrap();
    // The capture's own samples beyond the span feed the filter, so even the edges keep the tone.
    let (meta, values) = read_all(&target);
    assert_eq!(meta.len_samples, 10_000u64.div_ceil(19));
    let first = values[0].hypot(values[1]);
    assert!((first - 0.5).abs() < 0.01, "{first}");
}

#[test]
fn a_source_changed_since_it_was_opened_is_refused() {
    let dir = TempDir::new("extract-changed");
    let mut source = capture(&dir, 6_500.0);
    // The stamp of another file stands for the file as it was before it changed.
    std::fs::write(dir.join("other.bin"), [0u8; 3]).unwrap();
    source.stamp = argand_io::write::SourceStamp::of(&dir.join("other.bin")).ok();
    let target = dir.join("band.wav");
    let whole = SampleSpan::between(0, 48_000).unwrap();
    let band = FrequencyBand::between(10_005_000.0, 10_007_000.0).unwrap();
    let result = run(&request(source, whole, band, target.clone()), &mut |_, _| {}, &AtomicBool::new(false));
    assert!(matches!(result, Err(WriteError::SourceChanged { .. })), "{result:?}");
    assert!(!target.exists());
}

#[test]
fn a_cancelled_extraction_leaves_nothing() {
    let dir = TempDir::new("extract-cancel");
    let source = capture(&dir, 6_500.0);
    let target = dir.join("band.wav");
    let whole = SampleSpan::between(0, 48_000).unwrap();
    let band = FrequencyBand::between(10_005_000.0, 10_007_000.0).unwrap();
    let result = run(&request(source, whole, band, target.clone()), &mut |_, _| {}, &AtomicBool::new(true));
    assert!(matches!(result, Err(WriteError::Cancelled)));
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1, "only the capture is left");
}
