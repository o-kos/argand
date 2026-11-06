use super::*;

fn meta(source: &str, rate: f64) -> SignalMeta {
    SignalMeta {
        sample_rate: rate,
        center_freq: 0.0,
        sample_type: "iq_i16".parse().unwrap(),
        len_samples: 1_000_000,
        container: "wav",
        divisor: 1.0,
        source: PathBuf::from(source),
    }
}

#[test]
fn a_selection_is_named_by_its_bounds_in_seconds() {
    let span = SampleSpan::between(1_200_000, 3_600_000);
    assert_eq!(
        suggested_name(&meta("/data/pass.iqw", 2_400_000.0), false, span),
        "pass_0.500-1.500s.iqw"
    );
}

#[test]
fn the_whole_capture_keeps_its_name_and_a_headerless_one_becomes_wave() {
    let capture = meta("/data/pass.flac", 48_000.0);
    assert_eq!(suggested_name(&capture, false, None), "pass.flac");
    assert_eq!(suggested_name(&meta("/data/dump.bin", 1e6), true, None), "dump.wav");
    assert_eq!(suggested_name(&meta("/data/dump", 1e6), false, None), "dump.wav");
}

#[test]
fn the_dialog_uses_the_nearest_existing_source_folder() {
    let dir = std::env::temp_dir().join(format!("argand-chooser-directory-{}", std::process::id()));
    std::fs::create_dir_all(dir.join("existing")).unwrap();
    assert_eq!(directory(&dir.join("existing/pass.wav")), dir.join("existing"));
    assert_eq!(directory(&dir.join("existing/deleted/nested/pass.wav")), dir.join("existing"));
    assert!(directory(Path::new("pass.wav")).is_dir());
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_job_reports_progress_and_the_saved_file() {
    let dir = std::env::temp_dir().join(format!("argand-saving-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let source = dir.join("a.raw");
    std::fs::write(&source, vec![0u8; 4000]).unwrap();
    let hints = argand_io::OpenHints {
        raw: Some("iq_i16@1M".parse().unwrap()),
        ..Default::default()
    };
    let opened = argand_io::open(&source, &hints).unwrap();
    let target = dir.join("b.wav");
    let (_job, updates) = start(SaveRequest::span(
        argand_io::write::SourceFile {
            meta: opened.meta().clone(),
            hints: hints.clone(),
            stamp: None,
            encoding: None,
        },
        SampleSpan::between(10, 20),
        target.clone(),
    ), false);
    let mut finished = None;
    while let Ok(update) = updates.recv_blocking() {
        if let Update::Finished(outcome) = update {
            finished = Some(outcome);
        }
    }
    let Some(Outcome::Saved(saved)) = finished else {
        panic!("not saved: {finished:?}");
    };
    assert_eq!((saved.path, saved.samples), (target, 10));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn the_same_file_is_recognised_through_a_different_spelling() {
    let dir = std::env::temp_dir();
    let name = format!("argand-same-{}.wav", std::process::id());
    let file = dir.join(&name);
    std::fs::write(&file, b"x").unwrap();
    assert!(same_path(&file, &dir.join(".").join(&name)));
    assert!(!same_path(&file, &dir.join("argand-other.wav")));
    std::fs::remove_file(&file).unwrap();
}

#[test]
fn a_band_is_named_by_its_edges_and_a_rectangle_by_its_span_too() {
    let meta = meta("/captures/hfdl.flac", 2e6);
    let band = argand_core::FrequencyBand::between(12_578_500.0, 12_581_250.0).unwrap();
    assert_eq!(
        suggested_band_name(&meta, None, band),
        "hfdl_12.579-12.581MHz.wav"
    );
    let span = SampleSpan::between(2_000_000, 3_000_000);
    assert_eq!(
        suggested_band_name(&meta, span, band),
        "hfdl_1.000-1.500s_12.579-12.581MHz.wav"
    );
    let audio = argand_core::FrequencyBand::between(300.0, 2_700.0).unwrap();
    assert_eq!(suggested_band_name(&meta, None, audio), "hfdl_0.300-2.700kHz.wav");
    let narrow = argand_core::FrequencyBand::between(10_000_000.0, 10_000_100.0).unwrap();
    assert_eq!(
        suggested_band_name(&meta, None, narrow),
        "hfdl_10.0000-10.0001MHz.wav"
    );
    let hair = argand_core::FrequencyBand::between(999_999_999.6, 1_000_000_000.4).unwrap();
    assert_eq!(
        suggested_band_name(&meta, None, hair),
        "hfdl_999999.9996-1000000.0004kHz.wav"
    );
}

#[test]
fn flac_band_and_rectangle_exports_are_encoded_before_publication() {
    use argand_core::{Domain, FrequencyBand, SampleFormat, SampleType};
    use argand_edit::{Capture, SourceId};
    use argand_io::testutil::{TempDir, iq_tone, write_wav};
    use argand_io::write::SourceFile;
    let dir = TempDir::new("flac-band-export");
    let path = write_wav(&dir.join("capture.wav"), SampleType::new(Domain::Iq, SampleFormat::I16),
        8000, &iq_tone(8000, 8000.0, 300.0, 0.5), 1.0);
    let original = std::fs::read(&path).unwrap();
    let hints = argand_io::OpenHints { normalize: Some(argand_io::Normalize::None), ..Default::default() };
    let (source, stamp) = argand_io::open_stamped(&path, &hints).unwrap();
    let meta = source.meta().clone();
    drop(source);
    for (name, start, end) in [("band", 0, 8000), ("rectangle", 2000, 6000)] {
        let target = dir.join(&format!("{name}.flac"));
        let request = crate::extraction::ExtractRequest {
            capture: Capture::whole(SourceId(0), 8000),
            sources: vec![Some(SourceFile { meta: meta.clone(), hints: hints.clone(), stamp, encoding: None })],
            meta: meta.clone(), span: SampleSpan::between(start, end).unwrap(),
            band: FrequencyBand::between(-1000.0, 1000.0).unwrap(),
            target: target.clone(), protected: Vec::new(),
        };
        let (saved, clipped) = extract_flac(request.clone(), &mut |_, _| {}, &AtomicBool::new(false)).unwrap();
        assert_eq!(clipped, 0);
        assert_eq!(saved.container, "flac");
        assert_eq!(&std::fs::read(&target).unwrap()[..4], b"fLaC");
        let mut output = argand_io::open(&target, &hints).unwrap();
        assert_eq!(output.meta().sample_rate, 3000.0);
        assert_eq!(output.meta().center_freq, 0.0);
        assert_eq!(output.meta().len_samples, (end - start) * 3 / 8);
        assert_eq!(output.meta().sample_type, meta.sample_type);
        let mut samples = vec![0.0; output.meta().len_samples as usize * 2];
        let mut at = 0;
        while at < samples.len() { at += output.read(&mut samples[at..]).unwrap(); }
        let middle = &samples[samples.len()/4..samples.len()*3/4];
        assert!(middle.chunks_exact(2).all(|iq| (iq[0].hypot(iq[1]) - 0.5).abs() < 0.01));
        let bytes = std::fs::read(&target).unwrap();
        assert!(matches!(extract_flac(request, &mut |_, _| {}, &AtomicBool::new(true)), Err(WriteError::Cancelled)));
        assert_eq!(std::fs::read(&target).unwrap(), bytes);
    }
    assert_eq!(std::fs::read(path).unwrap(), original);
}
