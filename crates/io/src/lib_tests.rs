use super::*;
use crate::testutil::{TempDir, all_sample_types, iq_tone, write_raw, write_wav};
use argand_core::{Domain, SampleFormat};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root")
}

/// Find a real capture by name.
///
/// Looks in the repository's own `tests/signals` first, then wherever
/// `ARGAND_EXTRA_FIXTURES` points, then at the sibling sgvr checkout. Real
/// captures are not committed, so a missing one is reported and skipped
/// rather than failed.
fn optional_fixture(name: &str) -> Option<PathBuf> {
    let mut bases = vec![repo_root().join("tests/signals")];
    if let Ok(extra) = std::env::var("ARGAND_EXTRA_FIXTURES") {
        bases.push(PathBuf::from(extra));
    }
    bases.push(repo_root().join("../sgvr/cli/tests"));

    match bases.iter().map(|b| b.join(name)).find(|p| p.exists()) {
        Some(path) => Some(path),
        None => {
            eprintln!("skipping: no fixture named {name} in {bases:?}");
            None
        }
    }
}

/// `Box<dyn SampleSource>` is not `Debug`, so `unwrap_err` is unavailable.
fn err_of(result: Result<Box<dyn SampleSource>, IoError>) -> IoError {
    match result {
        Ok(_) => panic!("expected an error, got a reader"),
        Err(e) => e,
    }
}

fn drain(src: &mut dyn SampleSource) -> Vec<f32> {
    let mut out = Vec::new();
    let mut buf = [0.0f32; 1024];
    loop {
        let n = src.read(&mut buf).unwrap();
        if n == 0 {
            return out;
        }
        out.extend_from_slice(&buf[..n]);
    }
}

#[test]
fn detects_every_wav_sample_type_from_content() {
    let dir = TempDir::new("open-wav");
    let values = [0.25f32, -0.25, 0.5, -0.5];

    for st in all_sample_types() {
        // Deliberately misleading extension: detection must go by content.
        let path = write_wav(&dir.join(&format!("{st}.dat")), st, 24_000, &values, 1.0);
        let src = open(&path, &OpenHints::default()).unwrap();
        assert_eq!(src.meta().sample_type, st, "{st}");
        assert_eq!(src.meta().container, "wav", "{st}");
        assert_eq!(src.meta().sample_rate, 24_000.0, "{st}");
        assert_eq!(src.meta().sample_rate, 24_000.0, "{st}");
    }
}

#[test]
fn raw_needs_a_spec_and_honours_the_offset() {
    let dir = TempDir::new("open-raw");
    let values = [0.5f32, -0.5, 0.25, -0.25];
    let path = write_raw(&dir.join("dump.bin"), SampleFormat::I16, &values, 1.0);

    // Without --raw there is nothing to go on.
    let err = err_of(open(&path, &OpenHints::default()));
    assert!(matches!(err, IoError::UnknownContainer { .. }), "{err}");

    let hints = OpenHints {
        raw: Some("iq_i16@24k".parse().unwrap()),
        ..Default::default()
    };
    let mut src = open(&path, &hints).unwrap();
    assert_eq!(src.meta().container, "raw");
    assert_eq!(src.meta().sample_rate, 24_000.0);
    assert_eq!(src.meta().len_samples, 2); // four values, two I/Q pairs
    assert_eq!(drain(src.as_mut()).len(), 4);
}

#[test]
fn raw_without_a_rate_anywhere_is_an_error() {
    let dir = TempDir::new("open-raw-rate");
    let path = write_raw(&dir.join("dump.bin"), SampleFormat::I16, &[0.5], 1.0);

    let hints = OpenHints {
        raw: Some("rl_i16".parse().unwrap()),
        ..Default::default()
    };
    assert!(matches!(
        err_of(open(&path, &hints)),
        IoError::MissingRate { .. }
    ));

    // --rate supplies what the spec left out.
    let hints = OpenHints {
        sample_rate: Some(8000.0),
        ..hints
    };
    assert_eq!(open(&path, &hints).unwrap().meta().sample_rate, 8000.0);
}

#[test]
fn overrides_beat_the_header() {
    let dir = TempDir::new("open-override");
    let st = SampleType::new(Domain::Iq, SampleFormat::I16);
    let path = write_wav(&dir.join("x.wav"), st, 24_000, &[0.5, 0.5, 0.25, 0.25], 1.0);

    let hints = OpenHints {
        sample_rate: Some(48_000.0),
        center_freq: 12_579_000.0,
        sample_type: Some(SampleType::new(Domain::Real, SampleFormat::I32)),
        ..Default::default()
    };
    let src = open(&path, &hints).unwrap();
    assert_eq!(src.meta().sample_rate, 48_000.0);
    assert_eq!(src.meta().center_freq, 12_579_000.0);
    assert_eq!(src.meta().sample_type.format, SampleFormat::I32);
    // Reinterpreting 16-bit stereo as 32-bit mono halves the sample count.
    assert_eq!(src.meta().len_samples, 2);
}

#[test]
fn centre_frequency_shifts_the_reported_span() {
    let dir = TempDir::new("open-centre");
    let st = SampleType::new(Domain::Iq, SampleFormat::I16);
    let path = write_wav(&dir.join("c.wav"), st, 24_000, &[0.0; 8], 1.0);

    let hints = OpenHints {
        center_freq: 12_579_000.0,
        ..Default::default()
    };
    let (lo, hi) = open(&path, &hints).unwrap().meta().frequency_span();
    assert_eq!((lo, hi), (12_567_000.0, 12_591_000.0));
}

#[test]
fn reports_useful_errors_for_files_it_cannot_read() {
    let dir = TempDir::new("open-errors");

    let missing = dir.join("nope.wav");
    assert!(matches!(
        err_of(open(&missing, &OpenHints::default())),
        IoError::Open { .. }
    ));

    let empty = dir.join("empty.bin");
    std::fs::write(&empty, []).unwrap();
    assert!(matches!(
        err_of(open(&empty, &OpenHints::default())),
        IoError::Empty { .. }
    ));

    // Looks like RIFF, has no usable chunks.
    let broken = dir.join("broken.wav");
    std::fs::write(&broken, b"RIFF\x04\x00\x00\x00WAVE").unwrap();
    let err = err_of(open(&broken, &OpenHints::default()));
    assert!(matches!(err, IoError::Wav { .. }), "{err}");
    assert!(err.to_string().contains("wav"), "{err}");
}

#[test]
fn the_unknown_container_error_points_at_the_way_out() {
    let dir = TempDir::new("open-hint");
    let path = dir.join("mystery.bin");
    std::fs::write(&path, [1u8; 64]).unwrap();
    let msg = err_of(open(&path, &OpenHints::default())).to_string();
    assert!(msg.contains("--raw"), "{msg}");
}

#[test]
fn every_repository_capture_reads_end_to_end() {
    // Deliberately not pinned to one file: the fixture directory gains and
    // loses captures, so the test checks the invariant -- every sample in the
    // file is accounted for -- rather than a count that goes stale.
    let dir = repo_root().join("tests/signals");
    let Ok(entries) = std::fs::read_dir(&dir) else {
        eprintln!("skipping: no {}", dir.display());
        return;
    };

    let mut checked = 0;
    for path in entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "iqw" || e == "wavs"))
    {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let file_len = std::fs::metadata(&path).unwrap().len();
        let mut src = open(&path, &OpenHints::default()).unwrap();
        let meta = src.meta().clone();

        // Both extensions are RIFF underneath, which is the point of probing
        // by content instead of by name.
        assert_eq!(meta.container, "wav", "{name}");
        assert_eq!(meta.sample_type.to_string(), "iq_i16", "{name}");
        assert!(meta.sample_rate > 0.0, "{name}");

        let stride = meta.sample_type.bytes_per_sample() as u64;
        let payload = meta.len_samples * stride;
        assert!(
            payload <= file_len && file_len - payload < 4096,
            "{name}: {payload} bytes of samples in a {file_len} byte file"
        );

        // Reading works at both ends without dragging the file through memory.
        let mut buf = [0.0f32; 64];
        assert_eq!(src.read(&mut buf).unwrap(), 64, "{name}");
        assert!(buf.iter().all(|v| v.abs() <= 1.0), "{name}: {buf:?}");

        src.seek(meta.len_samples - 8).unwrap();
        assert_eq!(src.read(&mut buf).unwrap(), 16, "{name} at the end");
        assert!(src.seek(meta.len_samples + 1).is_err(), "{name} past the end");

        checked += 1;
    }
    assert!(checked > 0, "no captures found in {}", dir.display());
    eprintln!("checked {checked} repository captures");
}

#[test]
fn the_half_hour_capture_has_the_length_its_header_claims() {
    let Some(path) = optional_fixture("12.579000_25_08_26_06_09_10.iqw") else {
        return;
    };
    let src = open(&path, &OpenHints::default()).unwrap();
    let meta = src.meta();
    assert_eq!(meta.sample_rate, 24_000.0);
    assert_eq!(meta.len_samples, 43_200_000);
    assert_eq!(meta.duration_seconds(), 1800.0);
}

#[test]
fn reads_the_external_format_matrix() {
    // codec, expected sample type, expected container
    let cases = [
        ("iq_i16-hfdl.iqw", "iq_i16", "wav"),
        ("rl_i16-hfdl.wav", "rl_i16", "wav"),
        ("rl_f32-hfdl.wav", "rl_f32", "wav"),
        ("iq_f16x8-ntx.wav", "iq_f16x8", "wav"),
        ("rl_f16x8-hfdl.wav", "rl_f16x8", "wav"),
        ("iq_f32-ft8.flac", "iq_i16", "flac"),
        ("rl_f32-hfdl.flac", "rl_i16", "flac"),
    ];

    for (name, want_type, want_container) in cases {
        let Some(path) = optional_fixture(name) else {
            continue;
        };
        let mut src = open(&path, &OpenHints::default()).unwrap();
        let meta = src.meta().clone();
        assert_eq!(meta.sample_type.to_string(), want_type, "{name}");
        assert_eq!(meta.container, want_container, "{name}");
        assert!(meta.sample_rate > 0.0, "{name}");
        assert!(meta.len_samples > 0, "{name}");

        let mut buf = [0.0f32; 256];
        let n = src.read(&mut buf).unwrap();
        assert!(n > 0, "{name} produced no samples");
        assert!(
            buf[..n].iter().all(|v| v.is_finite() && v.abs() <= 1.5),
            "{name} produced out-of-range values"
        );
    }
}

#[test]
fn an_iq_capture_survives_a_round_trip_through_the_reader() {
    let dir = TempDir::new("open-tone");
    let st = SampleType::new(Domain::Iq, SampleFormat::I16);
    let values = iq_tone(1024, 24_000.0, 2404.0, 0.5);
    let path = write_wav(&dir.join("tone.wav"), st, 24_000, &values, 1.0);

    let mut src = open(&path, &OpenHints::default()).unwrap();
    let got = drain(src.as_mut());
    assert_eq!(got.len(), values.len());
    for (got, want) in got.iter().zip(values.iter()) {
        assert!((got - want).abs() < 1e-3, "{got} vs {want}");
    }
}

#[test]
fn a_64_bit_wave_is_read_and_named_as_such() {
    use crate::riff::F16X8_MAGIC;
    let _ = F16X8_MAGIC;

    let dir = TempDir::new("open-rf64");
    let values = iq_tone(2048, 24_000.0, 3000.0, 0.8);
    let st = SampleType::new(Domain::Iq, SampleFormat::I16);

    // Start from a normal wav and rewrite the header in place: same samples,
    // 64-bit lengths, so any difference in the result is the header's doing.
    let plain = write_wav(&dir.join("plain.wav"), st, 24_000, &values, 1.0);
    let bytes = std::fs::read(&plain).unwrap();
    let layout = crate::riff::parse(&bytes).unwrap();
    let data = &bytes[layout.data_offset..];

    for (magic, want) in [(b"RF64", "rf64"), (b"BW64", "bw64")] {
        let mut file = Vec::new();
        file.extend_from_slice(magic);
        file.extend_from_slice(&u32::MAX.to_le_bytes());
        file.extend_from_slice(b"WAVE");

        let mut ds64 = Vec::new();
        ds64.extend_from_slice(&0u64.to_le_bytes());
        ds64.extend_from_slice(&(data.len() as u64).to_le_bytes());
        ds64.extend_from_slice(&(values.len() as u64 / 2).to_le_bytes());
        ds64.extend_from_slice(&0u32.to_le_bytes());
        file.extend_from_slice(b"ds64");
        file.extend_from_slice(&(ds64.len() as u32).to_le_bytes());
        file.extend_from_slice(&ds64);

        file.extend_from_slice(&bytes[12..layout.data_offset - 8]);
        file.extend_from_slice(b"data");
        file.extend_from_slice(&u32::MAX.to_le_bytes());
        file.extend_from_slice(data);

        let path = dir.join(&format!("{want}.wav"));
        std::fs::write(&path, &file).unwrap();

        let mut src = open(&path, &OpenHints::default()).unwrap();
        assert_eq!(src.meta().container, want);
        assert_eq!(src.meta().sample_type, st);
        assert_eq!(src.meta().len_samples, 2048, "{want}");
        assert_eq!(drain(src.as_mut()).len(), values.len(), "{want}");
    }
}
