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
fn the_dialog_opens_in_the_source_folder() {
    assert_eq!(directory(Path::new("/data/pass.wav")), PathBuf::from("/data"));
    assert_eq!(directory(Path::new("pass.wav")), PathBuf::from("."));
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
