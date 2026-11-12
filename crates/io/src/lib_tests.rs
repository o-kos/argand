use super::*;
use crate::testutil::{TempDir, all_sample_types, iq_tone, write_raw, write_wav};
use argand_core::{Domain, SampleFormat};

fn repo_root() -> PathBuf {
    PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").expect("Cargo test source path"))
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
fn overrides_beat_the_header() {
    let dir = TempDir::new("open-override");
    let st = SampleType::new(Domain::Iq, SampleFormat::I16);
    let path = write_wav(&dir.join("x.wav"), st, 24_000, &[0.5, 0.5, 0.25, 0.25], 1.0);

    let hints = OpenHints {
        sample_rate: Some(48_000.0),
        center_freq: Some(12_579_000.0),
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

#[test]
fn decoder_level_scans_honour_a_budget_and_keep_unbudgeted_behaviour() {
    let dir = TempDir::new("decoded-scan-budget");
    let mut values = vec![0.25; 4096];
    values[3000] = 0.9;
    let path = write_wav(&dir.join("levels.wav"), SampleType::new(Domain::Real, SampleFormat::I16), 24_000, &values, 1.0);
    let bounded = DecodedSource::with_levels(&path, "wav", None, None, None, decoder::DecodeLevels {
        normalize: Normalize::Auto, gain_db: 0.0, scan_bytes: Some(64),
    }).unwrap();
    let ordinary = DecodedSource::open(&path, "wav", None, None, None, Normalize::Auto, 0.0).unwrap();
    assert!((bounded.meta().divisor - 0.25 * normalize::AUTO_HEADROOM).abs() < 1e-6);
    assert!((ordinary.meta().divisor - 0.9 * normalize::AUTO_HEADROOM).abs() < 0.0001);
}

#[test]
fn original_units_recover_integer_values_through_both_readers_and_gain() {
    let dir = TempDir::new("original-units");
    for (format, expected) in [
        (SampleFormat::I16, [-16384.0, 8192.0, 4096.0]),
        (SampleFormat::I24, [-4194304.0, 2097152.0, 1048576.0]),
        (SampleFormat::U8, [64.0, 160.0, 144.0]),
    ] {
        let path = write_wav(&dir.join(&format!("{format:?}.wav")),
            SampleType::new(Domain::Real, format), 24000, &[-0.5, 0.25, 0.125], 1.0);
        for normalize in [Normalize::None, Normalize::Auto, Normalize::Factor(8.0)] {
            let hints = OpenHints { normalize: Some(normalize), gain_db: 6.0, ..OpenHints::default() };
            let mmap = open(&path, &hints).unwrap();
            let decoded = Box::new(DecodedSource::open(&path, "wav", None, None, None, normalize, 6.0).unwrap());
            for mut source in [mmap, decoded as Box<dyn SampleSource>] {
                assert_original_values(source.as_mut(), &expected);
            }
        }
    }
}

#[test]
fn original_units_use_the_true_24_bit_depth() {
    let dir = TempDir::new("pcm24-units");
    let path = dir.join("pcm24.wav");
    let values = [-8_388_608i32, 8_388_607, 12345, -45678];
    let mut bytes = Vec::from(*b"RIFF");
    bytes.extend_from_slice(&48u32.to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    for value in [1u16, 1] { bytes.extend_from_slice(&value.to_le_bytes()); }
    for value in [24000u32, 72000] { bytes.extend_from_slice(&value.to_le_bytes()); }
    for value in [3u16, 24] { bytes.extend_from_slice(&value.to_le_bytes()); }
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&12u32.to_le_bytes());
    for value in values { bytes.extend_from_slice(&value.to_le_bytes()[..3]); }
    std::fs::write(&path, bytes).unwrap();
    let mut source = open(&path, &OpenHints::default()).unwrap();
    let (factor, offset) = source.original_sample_units().unwrap();
    for (actual, expected) in drain(source.as_mut()).iter().zip(values) {
        assert!((f64::from(*actual) * factor + offset - f64::from(expected)).abs() < 0.01);
    }
}

fn assert_original_values(source: &mut dyn SampleSource, expected: &[f64]) {
    let (factor, offset) = source.original_sample_units().unwrap();
    let values = drain(source);
    assert_eq!(values.len(), expected.len());
    for (actual, expected) in values.iter().zip(expected) {
        assert!((f64::from(*actual) * factor + offset - expected).abs() < 0.01);
    }
}

#[test]
fn reopening_reuses_normalization_and_preserves_independent_cursors() {
    let dir = TempDir::new("reopen");
    let values = [0.125, -0.25, 0.5, -0.75];
    for st in all_sample_types() {
        let path = write_wav(&dir.join(&format!("{st}.wav")), st, 48000, &values, 1.0);
        let hints = OpenHints { normalize: Some(Normalize::Auto), gain_db: 6., ..Default::default() };
        let (mut original, stamp) = open_stamped(&path, &hints).unwrap();
        let expected = drain(original.as_mut());
        original.seek(1).unwrap();
        let mut reopened = reopen(original.meta(), &OpenHints { level_scan_bytes: Some(0), ..hints }, stamp).unwrap();
        assert_eq!(reopened.meta().divisor, original.meta().divisor);
        assert_eq!(reopened.original_sample_units(), original.original_sample_units());
        assert_eq!(drain(reopened.as_mut()), expected);
        assert_eq!(drain(original.as_mut()), expected[st.channels()..]);
        let mut changed = reopened.meta().clone();
        changed.len_samples += 1;
        assert!(reopen(&changed, &OpenHints::default(), stamp).is_err_and(|error| error.is_changed()));
        assert!(reopen(reopened.meta(), &OpenHints::default(), None).is_err_and(|error| error.is_changed()));
    }
}

#[test]
fn unknown_length_flac_reopens_without_counting_or_normalizing_again() {
    let dir = TempDir::new("reopen-flac");
    let path = dir.join("levels.flac");
    let data = include_bytes!("../tests/fixtures/levels.flac").to_vec();
    std::fs::write(&path, &data).unwrap();
    let hints = OpenHints { normalize: Some(Normalize::Auto), gain_db: 6., ..Default::default() };
    let mut original = open(&path, &hints).unwrap();
    let expected = drain(original.as_mut());
    drop(original);
    let mut stripped = data.clone();
    stripped[21] &= 0xf0;
    stripped[22..26].fill(0);
    std::fs::write(&path, &stripped).unwrap();
    let (mut original, stamp) = open_stamped(&path, &hints).unwrap();
    assert_eq!(original.meta().len_samples, 16384);
    assert_eq!(drain(original.as_mut()), expected);
    let meta = original.meta().clone();
    let units = original.original_sample_units();
    drop(original);
    let mut reopened = reopen(&meta, &OpenHints { level_scan_bytes: Some(0), ..hints.clone() }, stamp).unwrap();
    assert_eq!(reopened.meta().len_samples, 16384);
    assert_eq!(reopened.meta().divisor, meta.divisor);
    assert_eq!(reopened.original_sample_units(), units);
    assert_eq!(drain(reopened.as_mut()), expected);
    for sample in [0, 5, 28, 1, 31, 0] {
        reopened.seek(sample).unwrap();
        let mut value = [0.];
        assert_eq!(reopened.read(&mut value).unwrap(), 1);
        assert_eq!(value[0], expected[sample as usize]);
    }
    drop(reopened);
    let mut counted = open(&path, &hints).unwrap();
    assert_eq!(counted.meta().len_samples, 16384);
    assert_eq!(drain(counted.as_mut()), expected);
    drop(counted);
    let mut default = open(&path, &OpenHints::default()).unwrap();
    assert_eq!(default.meta().len_samples, 16384);
    assert_eq!(drain(default.as_mut()).len(), 16384);
    // Unknown-length streams use the supplied count without a decoding pass.
    let known = SignalMeta { len_samples: 32768, ..meta };
    let reopened = reopen(&known, &hints, stamp).unwrap();
    assert_eq!(reopened.meta().len_samples, 32768);
}

#[test]
fn a_same_length_replacement_is_not_reopened() {
    let dir = TempDir::new("reopen-replaced");
    let values = [0.125, -0.25, 0.5, -0.75];
    let st = SampleType::new(Domain::Iq, SampleFormat::I16);
    let path = write_wav(&dir.join("a.wav"), st, 48000, &values, 1.0);
    let (original, stamp) = open_stamped(&path, &OpenHints::default()).unwrap();
    let meta = original.meta().clone();
    drop(original);
    let other = write_wav(&dir.join("b.wav"), st, 48000, &[0.5, 0.5, 0.5, 0.5], 1.0);
    // Windows stamps carry no file ID until #199, so only the time tells the files apart.
    #[cfg(windows)]
    std::fs::File::options()
        .write(true)
        .open(&other)
        .unwrap()
        .set_modified(std::time::SystemTime::now() + std::time::Duration::from_secs(5))
        .unwrap();
    std::fs::rename(&other, &path).unwrap();
    assert!(reopen(&meta, &OpenHints::default(), stamp).is_err_and(|error| error.is_changed()));
    std::fs::write(&path, b"").unwrap();
    assert!(reopen(&meta, &OpenHints::default(), stamp).is_err_and(|error| error.is_changed()), "emptied");
    std::fs::remove_file(&path).unwrap();
    assert!(reopen(&meta, &OpenHints::default(), stamp).is_err_and(|error| error.is_changed()), "removed");
}

/// The FLAC fixture written into `dir`, with its bytes.
fn flac_fixture(dir: &TempDir) -> (std::path::PathBuf, Vec<u8>) {
    let path = dir.join("levels.flac");
    let data = include_bytes!("../tests/fixtures/levels.flac").to_vec();
    std::fs::write(&path, &data).unwrap();
    (path, data)
}

/// Where the first audio frame starts, after the metadata blocks.
fn flac_audio_offset(data: &[u8]) -> usize {
    let mut at = 4;
    loop {
        let header = &data[at..at + 4];
        at += 4 + (usize::from(header[1]) << 16 | usize::from(header[2]) << 8 | usize::from(header[3]));
        if header[0] & 0x80 != 0 {
            return at;
        }
    }
}

#[test]
fn a_flac_seek_reads_the_opened_file_after_its_name_is_replaced() {
    let dir = TempDir::new("flac-renamed");
    let (path, data) = flac_fixture(&dir);
    let mut original = open(&path, &OpenHints::default()).unwrap();
    let expected = drain(original.as_mut());
    // Another file of the same length, its audio damaged, takes the name.
    let mut other = data.clone();
    let first_frame = flac_audio_offset(&data);
    other[first_frame + 16..].iter_mut().for_each(|byte| *byte = !*byte);
    let replacement = dir.join("other.flac");
    std::fs::write(&replacement, &other).unwrap();
    std::fs::rename(&replacement, &path).unwrap();
    for sample in [5, 0, 31] {
        original.seek(sample).unwrap();
        let mut value = [0.];
        assert_eq!(original.read(&mut value).unwrap(), 1);
        assert_eq!(value[0], expected[sample as usize]);
    }
}

#[test]
fn a_flac_seek_after_an_overwrite_in_place_is_refused() {
    let dir = TempDir::new("flac-overwritten");
    let (path, data) = flac_fixture(&dir);
    let mut original = open(&path, &OpenHints::default()).unwrap();
    original.seek(5).unwrap();
    let file = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
    std::io::Write::write_all(&mut &file, &data).unwrap();
    let later = std::time::SystemTime::now() + std::time::Duration::from_secs(5);
    file.set_modified(later).unwrap();
    drop(file);
    let error = original.seek(7).unwrap_err();
    assert!(matches!(error, SourceError::Changed), "{error}");
}

#[test]
fn flac_seeks_discard_old_packets_and_preserve_levels() {
    let dir = TempDir::new("flac-seeks");
    let path = dir.join("levels.flac");
    std::fs::write(&path, include_bytes!("../tests/fixtures/levels.flac")).unwrap();
    let hints = OpenHints { normalize: Some(Normalize::Auto), gain_db: 6., ..Default::default() };
    let mut source = open(&path, &hints).unwrap();
    let expected = drain(source.as_mut());
    let units = source.original_sample_units();
    assert!(matches!(source.seek(16385), Err(SourceError::SeekOutOfRange { .. })));
    for sample in [14336, 0, 512, 4096, 14336, 1024, 0, 16383, 16384, 7] {
        source.seek(sample).unwrap();
        assert_eq!(drain(source.as_mut()), expected[sample as usize..], "sample {sample}");
        assert_eq!(source.original_sample_units(), units);
    }
}

#[test]
fn a_kiwi_recording_reads_every_data_chunk() {
    let dir = TempDir::new("kiwi-open");
    let values = iq_tone(1300, 12_000.0, 1_000.0, 0.5);
    let path = crate::testutil::write_kiwi_wav(&dir.join("iq.wav"), 12_000, &values, 512);
    let mut source = open(&path, &OpenHints::default()).unwrap();
    assert_eq!(source.meta().len_samples, 1300);
    let all = drain(source.as_mut());
    for (got, want) in all.iter().zip(&values) {
        assert!((got - want).abs() < 1e-4);
    }
    assert_eq!(all.len(), values.len());
    for sample in [511u64, 512, 1023, 1299, 0] {
        source.seek(sample).unwrap();
        assert_eq!(drain(source.as_mut()), all[sample as usize * 2..], "sample {sample}");
    }
    let mut odd = [0.0f32; 7];
    source.seek(510).unwrap();
    assert_eq!(source.read(&mut odd).unwrap(), 6);
    assert_eq!(odd[..6], all[1020..1026]);
}

#[test]
fn a_kiwi_recording_cut_short_keeps_its_whole_samples() {
    let dir = TempDir::new("kiwi-cut");
    let values = iq_tone(1024, 12_000.0, 1_000.0, 0.5);
    let path = crate::testutil::write_kiwi_wav(&dir.join("iq.wav"), 12_000, &values, 512);
    let mut bytes = std::fs::read(&path).unwrap();
    bytes.truncate(bytes.len() - 1001);
    std::fs::write(&path, &bytes).unwrap();
    let source = open(&path, &OpenHints::default()).unwrap();
    assert_eq!(source.meta().len_samples, 512 + (2048 - 1001) / 4);
}

#[test]
fn auto_levels_measure_every_data_chunk() {
    let dir = TempDir::new("kiwi-levels");
    let mut values = vec![0.1f32; 2048];
    values[1500] = 0.8;
    let path = crate::testutil::write_kiwi_wav(&dir.join("iq.wav"), 12_000, &values, 256);
    let hints = OpenHints {
        normalize: Some(Normalize::Auto),
        ..Default::default()
    };
    let source = open(&path, &hints).unwrap();
    let peak = 0.8 * 32768.0 * normalize::AUTO_HEADROOM;
    assert!((source.meta().divisor - peak).abs() < 1.0, "{}", source.meta().divisor);
}

#[test]
fn a_long_kiwi_recording_reads_again_after_pages_are_released() {
    let dir = TempDir::new("kiwi-release");
    let values: Vec<f32> = (0..3_000_000).map(|n| (n % 2001) as f32 / 2001.0 - 0.5).collect();
    let path = crate::testutil::write_kiwi_wav(&dir.join("iq.wav"), 12_000, &values, 512);
    let mut source = open(&path, &OpenHints::default()).unwrap();
    let first = drain(source.as_mut());
    assert_eq!(first.len(), values.len());
    source.seek(1_000).unwrap();
    assert_eq!(drain(source.as_mut()), first[2_000..]);
}

#[test]
fn decoded_packets_replace_the_pending_buffer_and_end_cleanly() {
    let dir = TempDir::new("decoded-packets-eof");
    for st in all_sample_types().into_iter().filter(|st| st.format != SampleFormat::F16x8) {
        let values: Vec<f32> = (0..5003 * st.channels())
            .map(|n| ((n % 127) as f32 - 63.0) / 128.0)
            .collect();
        let path = write_wav(&dir.join(&format!("{st}.wav")), st, 2_400_000, &values, 1.0);
        let mut expected = open(&path, &OpenHints {
            normalize: Some(Normalize::None), ..OpenHints::default()
        }).unwrap();
        let mut decoded = DecodedSource::open(&path, "wav", None, None, None, Normalize::None, 0.0).unwrap();
        let expected = drain(expected.as_mut());
        assert_eq!(drain(&mut decoded), expected, "{st}");
        let mut buf = [f32::NAN; 17];
        assert_eq!(decoded.read(&mut buf).unwrap(), 0, "{st}");
        assert_eq!(decoded.read(&mut buf).unwrap(), 0, "{st}");
        for sample in [4097, 1, 5002] {
            decoded.seek(sample).unwrap();
            let count = decoded.read(&mut buf).unwrap();
            assert_eq!(count % st.channels(), 0);
            let start = sample as usize * st.channels();
            assert_eq!(&buf[..count], &expected[start..start + count], "{st} at {sample}");
        }
    }
}

#[test]
fn a_truncated_known_length_flac_is_an_error_instead_of_clean_eof() {
    let dir = TempDir::new("decoded-truncated-eof");
    let (path, mut bytes) = flac_fixture(&dir);
    bytes.truncate(bytes.len() - 32);
    std::fs::write(&path, bytes).unwrap();
    let mut source = open(&path, &OpenHints::default()).unwrap();
    let mut buf = [0.0; 513];
    loop {
        match source.read(&mut buf) {
            Ok(0) => panic!("truncated FLAC was reported as clean EOF"),
            Ok(_) => {}
            Err(_) => break,
        }
    }
}

#[test]
fn a_flac_without_a_complete_first_frame_is_not_clean_eof() {
    let dir = TempDir::new("decoded-truncated-first");
    let (path, data) = flac_fixture(&dir);
    let audio = flac_audio_offset(&data);
    for corrupt in [false, true] {
        let mut bytes = data[..if corrupt { 3703 } else { audio + 100 }].to_vec();
        if corrupt {
            bytes[audio + 114] ^= 1;
        }
        std::fs::write(&path, bytes).unwrap();
        let hints = OpenHints { normalize: Some(Normalize::None), ..Default::default() };
        let mut source = open(&path, &hints).unwrap();
        assert_eq!(source.meta().len_samples, 16384);
        let mut buf = [0.0; 513];
        assert!(source.read(&mut buf).is_err(), "corrupt={corrupt}");
        assert!(source.read(&mut buf).is_err(), "repeated, corrupt={corrupt}");
        source.seek(0).unwrap();
        assert!(source.read(&mut buf).is_err(), "seek zero, corrupt={corrupt}");
        let auto = OpenHints { normalize: Some(Normalize::Auto), ..Default::default() };
        assert!(open(&path, &auto).is_err(), "normalization, corrupt={corrupt}");
    }
}

#[test]
fn packed_i24_raw_offsets_and_wave_runs_keep_whole_iq_pairs() {
    let dir = TempDir::new("i24-runs");
    let path = write_wav(&dir.join("a.wav"), "iq_i24".parse().unwrap(), 2_400_000, &[0.0; 2], 1.0);
    let mut bytes = std::fs::read(&path).unwrap()[..36].to_vec();
    for data in [vec![0, 0, 128, 255, 255, 127, 1, 2, 3], vec![255, 255, 255, 0, 0, 0, 1, 0, 0, 254, 255, 255]] {
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&(data.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&data);
        if data.len() % 2 != 0 { bytes.push(0); }
    }
    let size = (bytes.len() - 8) as u32;
    bytes[4..8].copy_from_slice(&size.to_le_bytes());
    std::fs::write(&path, bytes).unwrap();
    let expected = [-1.0, 8388607.0 / 8388608.0, -1.0 / 8388608.0, 0.0, 1.0 / 8388608.0, -2.0 / 8388608.0];
    let mut source = open(&path, &OpenHints::default()).unwrap();
    assert_eq!(source.meta().sample_type.to_string(), "iq_i24");
    assert_eq!(source.meta().len_samples, 3);
    let mut odd = [f32::NAN; 3];
    assert_eq!(source.read(&mut odd).unwrap(), 2);
    assert_eq!(&odd[..2], &expected[..2]);
    assert_eq!(drain(source.as_mut()), expected[2..]);
    source.seek(1).unwrap();
    assert_eq!(drain(source.as_mut()), expected[2..]);

    let raw = dir.join("a.raw");
    std::fs::write(&raw, [vec![42], vec![0, 0, 128, 255, 255, 127], vec![7; 5]].concat()).unwrap();
    let hints = OpenHints { raw: Some("iq_i24@2.4M".parse().unwrap()), byte_offset: 1, center_freq: Some(100e6), ..Default::default() };
    let mut source = open(&raw, &hints).unwrap();
    assert_eq!(source.meta().len_samples, 1);
    assert_eq!(source.meta().sample_rate, 2_400_000.0);
    assert_eq!(source.meta().center_freq, 100e6);
    assert_eq!(drain(source.as_mut()), expected[..2]);
}

#[cfg(all(unix, target_pointer_width = "64"))]
#[test]
fn sparse_pcm24_rf64_and_bw64_read_beyond_four_gib_without_a_full_scan() {
    use std::io::{Seek, SeekFrom, Write};
    let dir = TempDir::new("i24-sparse-rf64");
    let frames = u64::from(u32::MAX) / 6 + 3;
    let data_len = frames * 6;
    for magic in [b"RF64", b"BW64"] {
        let path = write_wav(&dir.join("large.wav"), "iq_i24".parse().unwrap(), 2_400_000, &[0.0; 2], 1.0);
        let wave = std::fs::read(&path).unwrap();
        let mut header = Vec::from(*magic);
        header.extend_from_slice(&u32::MAX.to_le_bytes());
        header.extend_from_slice(b"WAVEds64");
        header.extend_from_slice(&28u32.to_le_bytes());
        header.extend_from_slice(&(data_len + 72).to_le_bytes());
        header.extend_from_slice(&data_len.to_le_bytes());
        header.extend_from_slice(&frames.to_le_bytes());
        header.extend_from_slice(&0u32.to_le_bytes());
        header.extend_from_slice(&wave[12..40]);
        header.extend_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(header.len(), 80);
        let mut file = std::fs::File::create(&path).unwrap();
        file.write_all(&header).unwrap();
        file.set_len(80 + data_len).unwrap();
        file.seek(SeekFrom::End(-6)).unwrap();
        file.write_all(&[0, 0, 128, 255, 255, 127]).unwrap();
        drop(file);
        let mut source = open(&path, &OpenHints::default()).unwrap();
        assert_eq!(source.meta().sample_type.to_string(), "iq_i24");
        assert_eq!(source.meta().len_samples, frames);
        assert_eq!(source.meta().divisor, 8388608.0);
        source.seek(frames - 1).unwrap();
        let mut pair = [0.0; 2];
        assert_eq!(source.read(&mut pair).unwrap(), 2);
        assert_eq!(pair, [-1.0, 8388607.0 / 8388608.0]);
        assert_eq!(source.read(&mut pair).unwrap(), 0);
        drop(source);
        std::fs::remove_file(path).unwrap();
    }
}
