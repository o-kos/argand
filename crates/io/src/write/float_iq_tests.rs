use super::*;

use crate::testutil::{TempDir, iq_tone, write_wav};
use argand_core::{Domain, SampleType};

fn meta(rate: f64, centre: f64) -> SignalMeta {
    SignalMeta {
        sample_rate: rate,
        center_freq: centre,
        sample_type: SampleType::new(Domain::Iq, SampleFormat::I16),
        len_samples: 0,
        container: "wav",
        divisor: 1.0,
        source: PathBuf::from("band.wav"),
    }
}

fn request(samples: u64, target: PathBuf) -> FloatIqRequest {
    FloatIqRequest {
        meta: meta(48_000.0 / 19.0, 12_585_000.0),
        samples,
        target,
        sources: Vec::new(),
        protected: Vec::new(),
    }
}

fn frames(count: usize) -> Vec<[f32; 2]> {
    (0..count).map(|n| [n as f32 / 1000.0, -(n as f32) / 2000.0]).collect()
}

fn read_back(path: &Path) -> (SignalMeta, Vec<f32>) {
    let mut source = crate::open(path, &OpenHints::default()).unwrap();
    let meta = source.meta().clone();
    let mut values = vec![0.0; meta.len_samples as usize * 2];
    let mut filled = 0;
    while filled < values.len() {
        let read = source.read(&mut values[filled..]).unwrap();
        assert!(read > 0);
        filled += read;
    }
    (meta, values)
}

#[test]
fn a_computed_capture_opens_with_its_exact_rate_and_reference_frequency() {
    let dir = TempDir::new("float-iq");
    let target = dir.join("band.wav");
    let mut file = FloatIq::create(request(1000, target.clone())).unwrap();
    let written = frames(1000);
    for chunk in written.chunks(333) {
        file.write(chunk).unwrap();
    }
    let saved = file.finish(&AtomicBool::new(false)).unwrap();
    assert_eq!((saved.samples, saved.container), (1000, "wav"));
    let (meta, values) = read_back(&target);
    assert_eq!(meta.sample_rate, 48_000.0 / 19.0);
    assert_eq!(meta.center_freq, 12_585_000.0);
    assert_eq!(meta.sample_type, SampleType::new(Domain::Iq, SampleFormat::F32));
    assert_eq!(meta.len_samples, 1000);
    let expected: Vec<f32> = written.iter().flatten().copied().collect();
    assert_eq!(values, expected);
}

#[test]
fn a_computed_capture_past_the_riff_limit_is_rf64() {
    let dir = TempDir::new("float-iq-rf64");
    let target = dir.join("band.wav");
    let mut file = FloatIq::create_with_limit(request(100, target.clone()), 512).unwrap();
    file.write(&frames(100)).unwrap();
    assert_eq!(file.finish(&AtomicBool::new(false)).unwrap().container, "rf64");
    assert_eq!(read_back(&target).0.len_samples, 100);
}

#[test]
fn a_short_or_long_capture_is_not_moved_into_place() {
    let dir = TempDir::new("float-iq-count");
    let target = dir.join("band.wav");
    let mut file = FloatIq::create(request(10, target.clone())).unwrap();
    file.write(&frames(9)).unwrap();
    assert!(matches!(file.finish(&AtomicBool::new(false)), Err(WriteError::OutOfRange)));
    let mut file = FloatIq::create(request(10, target.clone())).unwrap();
    assert!(matches!(file.write(&frames(11)), Err(WriteError::OutOfRange)));
    drop(file);
    assert!(!target.exists());
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0, "no temporary file is left");
}

#[test]
fn a_source_is_not_a_target() {
    let dir = TempDir::new("float-iq-source");
    let values = iq_tone(64, 48_000.0, 1_000.0, 0.5);
    let path = write_wav(
        &dir.join("capture.wav"),
        SampleType::new(Domain::Iq, SampleFormat::F32),
        48_000,
        &values,
        1.0,
    );
    let source = crate::open(&path, &OpenHints::default()).unwrap();
    let mut request = request(10, path.clone());
    request.sources = vec![SourceFile {
        meta: source.meta().clone(),
        hints: OpenHints::default(),
        stamp: SourceStamp::of(&path).ok(),
    }];
    assert!(matches!(
        FloatIq::create(request),
        Err(WriteError::SameFile { .. })
    ));
}

#[test]
fn a_capture_cancelled_while_it_is_finished_leaves_the_target_alone() {
    let dir = TempDir::new("float-iq-cancel");
    let target = dir.join("band.wav");
    std::fs::write(&target, b"kept").unwrap();
    let mut file = FloatIq::create(request(10, target.clone())).unwrap();
    file.write(&frames(10)).unwrap();
    assert!(matches!(file.finish(&AtomicBool::new(true)), Err(WriteError::Cancelled)));
    assert_eq!(std::fs::read(&target).unwrap(), b"kept");
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1, "no temporary file is left");
}
