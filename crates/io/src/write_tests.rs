use super::*;

use std::sync::atomic::AtomicBool;

use argand_core::{Domain, SampleType};

use crate::testutil::{TempDir, all_sample_types, encode, iq_tone, real_tone, write_raw, write_wav};
use crate::{RawSpec, open};

fn request(path: &Path, hints: OpenHints, span: Option<SampleSpan>, target: PathBuf) -> SaveRequest {
    let meta = open(path, &hints).expect("open source").meta().clone();
    SaveRequest {
        meta,
        hints,
        span,
        target,
    }
}

fn run(request: &SaveRequest) -> Result<Saved, WriteError> {
    save(request, &mut |_, _| {}, &AtomicBool::new(false))
}

/// The stored bytes of a WAVE file's samples.
fn data_bytes(path: &Path) -> Vec<u8> {
    let bytes = fs::read(path).expect("read output");
    let layout = riff::parse(&bytes).expect("parse output");
    let len = layout.data_len(bytes.len());
    bytes[layout.data_offset..layout.data_offset + len].to_vec()
}

fn read_all(path: &Path, hints: &OpenHints) -> Vec<f32> {
    let mut source = open(path, hints).expect("open output");
    let mut out = vec![0.0; source.meta().len_samples as usize * source.meta().channels()];
    let mut got = 0;
    while got < out.len() {
        let n = source.read(&mut out[got..]).expect("read");
        assert!(n > 0, "short read");
        got += n;
    }
    out
}

fn signal(sample_type: SampleType, len: usize) -> Vec<f32> {
    match sample_type.domain {
        Domain::Iq => iq_tone(len, 48_000.0, 3_000.0, 0.5),
        Domain::Real => real_tone(len, 48_000.0, 3_000.0, 0.5),
    }
}

/// Files left in a directory, to catch stray temporaries.
fn names(dir: &TempDir) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir.path())
        .expect("list dir")
        .map(|entry| entry.expect("entry").file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

#[test]
fn every_wave_sample_type_is_copied_byte_for_byte() {
    let dir = TempDir::new("write-types");
    for sample_type in all_sample_types() {
        let source = dir.join(&format!("{sample_type}.wav"));
        write_wav(&source, sample_type, 48_000, &signal(sample_type, 1000), 1.0);
        let target = dir.join(&format!("{sample_type}-cut.wav"));
        let span = SampleSpan::between(101, 734);
        let saved = run(&request(&source, OpenHints::default(), span, target.clone())).unwrap();
        assert_eq!(saved.samples, 633);
        assert_eq!(saved.container, "wav");

        let block = sample_type.bytes_per_sample();
        let original = data_bytes(&source);
        assert_eq!(data_bytes(&target), original[101 * block..734 * block], "{sample_type}");

        let reopened = open(&target, &OpenHints::default()).unwrap();
        assert_eq!(reopened.meta().sample_type, sample_type);
        assert_eq!(reopened.meta().sample_rate, 48_000.0);
        assert_eq!(reopened.meta().len_samples, 633);
    }
}

#[test]
fn whole_capture_is_saved_without_a_span() {
    let dir = TempDir::new("write-whole");
    let sample_type: SampleType = "iq_i16".parse().unwrap();
    let source = write_wav(&dir.join("a.wav"), sample_type, 2_000_000, &signal(sample_type, 513), 1.0);
    let target = dir.join("b.wav");
    run(&request(&source, OpenHints::default(), None, target.clone())).unwrap();
    assert_eq!(data_bytes(&target), data_bytes(&source));
}

#[test]
fn headerless_capture_becomes_wave_that_opens_without_hints() {
    let dir = TempDir::new("write-raw");
    for sample_type in all_sample_types() {
        let values = signal(sample_type, 300);
        let source = dir.join(&format!("{sample_type}.bin"));
        let mut bytes = vec![0xAA; 7];
        bytes.extend(encode(sample_type.format, &values));
        fs::write(&source, &bytes).unwrap();
        let hints = OpenHints {
            raw: Some(RawSpec {
                sample_type,
                sample_rate: Some(250_000.0),
            }),
            byte_offset: 7,
            ..Default::default()
        };
        let target = dir.join(&format!("{sample_type}.wav"));
        run(&request(&source, hints.clone(), SampleSpan::between(3, 299), target.clone())).unwrap();

        let block = sample_type.bytes_per_sample();
        assert_eq!(data_bytes(&target), bytes[7 + 3 * block..7 + 299 * block], "{sample_type}");
        let reopened = open(&target, &OpenHints::default()).unwrap();
        assert_eq!(reopened.meta().sample_type, sample_type);
        assert_eq!(reopened.meta().sample_rate, 250_000.0);
    }
}

#[test]
fn raw_and_wave_values_read_back_identically() {
    let dir = TempDir::new("write-values");
    let sample_type: SampleType = "iq_f32".parse().unwrap();
    let values = signal(sample_type, 200);
    let source = write_raw(&dir.join("a.raw"), sample_type.format, &values, 1.0);
    let hints = OpenHints {
        raw: Some(RawSpec {
            sample_type,
            sample_rate: Some(1e6),
        }),
        ..Default::default()
    };
    let target = dir.join("a.wav");
    run(&request(&source, hints, None, target.clone())).unwrap();
    assert_eq!(read_all(&target, &OpenHints::default()), values);
}

/// A 24-bit WAVE file, which the native reader declines and the decoder reads.
fn write_wav24(path: &Path, channels: u16, frames: usize) -> Vec<u8> {
    let mut data = Vec::new();
    for n in 0..frames * channels as usize {
        let value = (n as i32 * 7919 - 400_000).clamp(-(1 << 23), (1 << 23) - 1);
        data.extend_from_slice(&value.to_le_bytes()[..3]);
    }
    let block = channels * 3;
    let mut fmt = Vec::new();
    fmt.extend_from_slice(&1u16.to_le_bytes());
    fmt.extend_from_slice(&channels.to_le_bytes());
    fmt.extend_from_slice(&96_000u32.to_le_bytes());
    fmt.extend_from_slice(&(96_000 * u32::from(block)).to_le_bytes());
    fmt.extend_from_slice(&block.to_le_bytes());
    fmt.extend_from_slice(&24u16.to_le_bytes());
    let mut file = Vec::new();
    file.extend_from_slice(b"RIFF");
    file.extend_from_slice(&((4 + 8 + fmt.len() + 8 + data.len()) as u32).to_le_bytes());
    file.extend_from_slice(b"WAVE");
    file.extend_from_slice(b"fmt ");
    file.extend_from_slice(&(fmt.len() as u32).to_le_bytes());
    file.extend_from_slice(&fmt);
    file.extend_from_slice(b"data");
    file.extend_from_slice(&(data.len() as u32).to_le_bytes());
    file.extend_from_slice(&data);
    fs::write(path, &file).unwrap();
    data
}

#[test]
fn twenty_four_bit_wave_is_copied_rather_than_decoded() {
    let dir = TempDir::new("write-24");
    let source = dir.join("a.wav");
    let data = write_wav24(&source, 2, 400);
    let target = dir.join("b.wav");
    run(&request(&source, OpenHints::default(), SampleSpan::between(10, 390), target.clone())).unwrap();

    let bytes = fs::read(&target).unwrap();
    let chunks = riff::scan(&bytes).unwrap();
    let fmt = riff::parse_fmt(chunks.fmt).unwrap();
    assert_eq!((fmt.bits, fmt.block_align, fmt.channels), (24, 6, 2));
    let start = chunks.data_offset;
    assert_eq!(bytes[start..start + 380 * 6], data[10 * 6..390 * 6]);
    assert_eq!(open(&target, &OpenHints::default()).unwrap().meta().len_samples, 380);
}

#[test]
fn rf64_is_written_past_the_riff_limit() {
    let dir = TempDir::new("write-rf64");
    let sample_type: SampleType = "iq_i16".parse().unwrap();
    let source = write_wav(&dir.join("a.wav"), sample_type, 48_000, &signal(sample_type, 1001), 1.0);
    let target = dir.join("b.wav");
    let request = request(&source, OpenHints::default(), SampleSpan::between(0, 1001), target.clone());
    let saved = save_with_limit(&request, &mut |_, _| {}, &AtomicBool::new(false), 1000).unwrap();
    assert_eq!(saved.container, "rf64");

    let bytes = fs::read(&target).unwrap();
    let layout = riff::parse(&bytes).unwrap();
    assert_eq!(layout.container, "rf64");
    assert_eq!(layout.declared_len, Some(1001 * 4));
    assert_eq!(data_bytes(&target), data_bytes(&source));
    let ds64_riff = u64::from_le_bytes(bytes[20..28].try_into().unwrap());
    assert_eq!(ds64_riff, bytes.len() as u64 - 8);
}

#[test]
fn riff_size_and_odd_data_are_padded_correctly() {
    let dir = TempDir::new("write-odd");
    let sample_type: SampleType = "rl_u8".parse().unwrap();
    let source = write_wav(&dir.join("a.wav"), sample_type, 8_000, &signal(sample_type, 99), 1.0);
    let target = dir.join("b.wav");
    run(&request(&source, OpenHints::default(), SampleSpan::between(0, 5), target.clone())).unwrap();
    let bytes = fs::read(&target).unwrap();
    assert_eq!(bytes.len() % 2, 0);
    assert_eq!(u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize, bytes.len() - 8);
    assert_eq!(data_bytes(&target).len(), 5);
}

#[test]
fn reference_frequency_survives_exactly_and_auxi_carries_whole_hertz() {
    let dir = TempDir::new("write-freq");
    let sample_type: SampleType = "iq_i16".parse().unwrap();
    let source = write_wav(&dir.join("a.wav"), sample_type, 2_400_000, &signal(sample_type, 64), 1.0);
    let hints = OpenHints {
        center_freq: Some(433_920_000.25),
        ..Default::default()
    };
    let target = dir.join("b.wav");
    run(&request(&source, hints, None, target.clone())).unwrap();

    let reopened = open(&target, &OpenHints::default()).unwrap();
    assert_eq!(reopened.meta().center_freq, 433_920_000.25);
    let bytes = fs::read(&target).unwrap();
    let chunks = riff::scan(&bytes).unwrap();
    assert_eq!(chunks.metadata.auxi, Some(433_920_000));
    assert_eq!(chunks.metadata.argd, Some((433_920_000.25, 2_400_000.0)));

    let overridden = OpenHints {
        center_freq: Some(-5.0),
        ..Default::default()
    };
    assert_eq!(open(&target, &overridden).unwrap().meta().center_freq, -5.0);
}

#[test]
fn frequency_beyond_auxi_is_kept_only_in_argd() {
    let dir = TempDir::new("write-ghz");
    let sample_type: SampleType = "rl_f32".parse().unwrap();
    let source = write_wav(&dir.join("a.wav"), sample_type, 48_000, &signal(sample_type, 64), 1.0);
    let hints = OpenHints {
        center_freq: Some(10.5e9),
        ..Default::default()
    };
    let target = dir.join("b.wav");
    run(&request(&source, hints, None, target.clone())).unwrap();
    let bytes = fs::read(&target).unwrap();
    assert_eq!(riff::scan(&bytes).unwrap().metadata.auxi, None);
    assert_eq!(open(&target, &OpenHints::default()).unwrap().meta().center_freq, 10.5e9);
}

#[test]
fn auxi_alone_gives_the_frequency_of_an_sdr_recording() {
    let dir = TempDir::new("write-auxi");
    let sample_type: SampleType = "iq_i16".parse().unwrap();
    let source = write_wav(&dir.join("a.wav"), sample_type, 48_000, &signal(sample_type, 8), 1.0);
    let mut bytes = fs::read(&source).unwrap();
    let mut auxi = vec![0u8; AUXI_LEN];
    auxi[AUXI_CENTER..AUXI_CENTER + 4].copy_from_slice(&7_100_000u32.to_le_bytes());
    let mut chunk = b"auxi".to_vec();
    chunk.extend_from_slice(&(AUXI_LEN as u32).to_le_bytes());
    chunk.extend_from_slice(&auxi);
    bytes.splice(12..12, chunk);
    let riff_size = (bytes.len() - 8) as u32;
    bytes[4..8].copy_from_slice(&riff_size.to_le_bytes());
    fs::write(&source, &bytes).unwrap();
    assert_eq!(open(&source, &OpenHints::default()).unwrap().meta().center_freq, 7_100_000.0);
}

#[test]
fn exact_rate_from_argd_is_used_only_while_it_agrees_with_fmt() {
    let mut metadata = riff::Metadata {
        argd: Some((0.0, 2_399_999.75)),
        auxi: None,
    };
    assert_eq!(metadata.sample_rate_for(2_400_000.0), 2_399_999.75);
    metadata.argd = Some((0.0, 1_000.0));
    assert_eq!(metadata.sample_rate_for(2_400_000.0), 2_400_000.0);
}

#[test]
fn the_open_file_is_never_a_target() {
    let dir = TempDir::new("write-same");
    let sample_type: SampleType = "iq_i16".parse().unwrap();
    let source = write_wav(&dir.join("a.wav"), sample_type, 48_000, &signal(sample_type, 64), 1.0);
    let before = fs::read(&source).unwrap();
    let mut same = request(&source, OpenHints::default(), None, source.clone());
    assert!(matches!(run(&same), Err(WriteError::SameFile { .. })));
    same.target = dir.path().join(".").join("a.wav");
    assert!(matches!(run(&same), Err(WriteError::SameFile { .. })));
    assert_eq!(fs::read(&source).unwrap(), before);
    assert_eq!(names(&dir), ["a.wav"]);
}

#[test]
fn cancelling_leaves_no_file_and_the_old_target_untouched() {
    let dir = TempDir::new("write-cancel");
    let sample_type: SampleType = "iq_i16".parse().unwrap();
    let source = write_wav(&dir.join("a.wav"), sample_type, 48_000, &signal(sample_type, 64), 1.0);
    let target = dir.join("b.wav");
    fs::write(&target, b"keep me").unwrap();
    let request = request(&source, OpenHints::default(), None, target.clone());
    let result = save(&request, &mut |_, _| {}, &AtomicBool::new(true));
    assert!(matches!(result, Err(WriteError::Cancelled)));
    assert_eq!(fs::read(&target).unwrap(), b"keep me");
    assert_eq!(names(&dir), ["a.wav", "b.wav"]);
}

#[test]
fn a_source_shorter_than_its_metadata_fails_without_a_file() {
    let dir = TempDir::new("write-short");
    let sample_type: SampleType = "iq_i16".parse().unwrap();
    let source = write_wav(&dir.join("a.wav"), sample_type, 48_000, &signal(sample_type, 64), 1.0);
    let mut request = request(&source, OpenHints::default(), None, dir.join("b.wav"));
    request.meta.len_samples = 100;
    assert!(matches!(run(&request), Err(WriteError::SourceChanged { .. })));
    request.span = SampleSpan::between(90, 101);
    assert!(matches!(run(&request), Err(WriteError::OutOfRange)));
    assert_eq!(names(&dir), ["a.wav"]);
}

#[test]
fn progress_ends_at_the_total() {
    let dir = TempDir::new("write-progress");
    let sample_type: SampleType = "iq_i16".parse().unwrap();
    let source = write_wav(&dir.join("a.wav"), sample_type, 48_000, &signal(sample_type, 64), 1.0);
    let request = request(&source, OpenHints::default(), SampleSpan::between(4, 40), dir.join("b.wav"));
    let mut seen = Vec::new();
    save(&request, &mut |done, total| seen.push((done, total)), &AtomicBool::new(false)).unwrap();
    assert_eq!(seen.first(), Some(&(0, 36)));
    assert_eq!(seen.last(), Some(&(36, 36)));
}

/// A FLAC file of `bits`-bit integers, made with the encoder under test.
fn write_flac_fixture(path: &Path, channels: usize, bits: usize, frames: usize) -> Vec<i32> {
    let limit = 1i32 << (bits - 1);
    let samples: Vec<i32> = (0..frames * channels)
        .map(|n| ((n as i64 * 7919) % i64::from(2 * limit) - i64::from(limit)) as i32)
        .collect();
    let source = flacenc::source::MemSource::from_samples(&samples, channels, bits, 44_100);
    let config = flacenc::config::Encoder::default().into_verified().unwrap();
    let mut stream = flacenc::encode_with_fixed_block_size(&config, source, 1024).unwrap();
    stream.stream_info_mut().set_block_sizes(1024, 1024).unwrap();
    let mut sink = ByteSink::new();
    stream.write(&mut sink).unwrap();
    fs::write(path, sink.as_slice()).unwrap();
    samples
}

/// Decoded FLAC values as the integers they were stored as.
fn flac_ints(path: &Path, bits: u32) -> Vec<i32> {
    let hints = OpenHints {
        normalize: Some(crate::Normalize::None),
        ..Default::default()
    };
    let scale = f64::from(1u32 << (bits - 1));
    read_all(path, &hints)
        .into_iter()
        .map(|v| (f64::from(v) * scale).round() as i32)
        .collect()
}

#[test]
fn flac_is_re_encoded_without_loss() {
    let dir = TempDir::new("write-flac");
    for (channels, bits) in [(2, 16), (1, 24), (2, 8)] {
        let source = dir.join(&format!("a{channels}-{bits}.flac"));
        let samples = write_flac_fixture(&source, channels, bits, 5000);
        let target = dir.join(&format!("b{channels}-{bits}.flac"));
        let hints = OpenHints {
            center_freq: Some(14_074_000.5),
            ..Default::default()
        };
        let saved = run(&request(&source, hints, SampleSpan::between(1000, 4321), target.clone())).unwrap();
        assert_eq!(saved.container, "flac");

        let reopened = open(&target, &OpenHints::default()).unwrap();
        assert_eq!(reopened.meta().len_samples, 3321);
        assert_eq!(reopened.meta().center_freq, 14_074_000.5);
        assert_eq!(
            flac_ints(&target, bits as u32),
            samples[1000 * channels..4321 * channels],
            "{channels} channel(s), {bits} bit"
        );
    }
}

#[test]
fn flac_streaminfo_carries_the_md5_of_what_was_written() {
    let dir = TempDir::new("write-flac-md5");
    let source = dir.join("a.flac");
    write_flac_fixture(&source, 2, 16, 3000);
    let target = dir.join("b.flac");
    run(&request(&source, OpenHints::default(), None, target.clone())).unwrap();
    let original = fs::read(&source).unwrap();
    let written = fs::read(&target).unwrap();
    // Bytes 26 to 42 of STREAMINFO hold the MD5 of the decoded samples.
    assert_eq!(written[26..42], original[26..42]);
    assert_ne!(written[26..42], [0u8; 16]);
}


#[test]
fn an_existing_file_at_the_temporary_name_is_never_opened() {
    let dir = TempDir::new("write-collision");
    let sample_type: SampleType = "iq_i16".parse().unwrap();
    let source = write_wav(&dir.join("a.wav"), sample_type, 48_000, &signal(sample_type, 64), 1.0);
    let before = fs::read(&source).unwrap();
    let squatter = dir.join(&format!(".b.wav.{}-0.part", std::process::id()));
    fs::hard_link(&source, &squatter).unwrap();
    run(&request(&source, OpenHints::default(), None, dir.join("b.wav"))).unwrap();
    assert_eq!(fs::read(&source).unwrap(), before);
    assert_eq!(fs::read(&squatter).unwrap(), before);
    assert_eq!(data_bytes(&dir.join("b.wav")), data_bytes(&source));
}

#[test]
fn cancelling_after_the_last_block_still_keeps_the_old_target() {
    let dir = TempDir::new("write-late-cancel");
    let sample_type: SampleType = "iq_i16".parse().unwrap();
    let source = write_wav(&dir.join("a.wav"), sample_type, 48_000, &signal(sample_type, 64), 1.0);
    let target = dir.join("b.wav");
    fs::write(&target, b"keep me").unwrap();
    let cancel = AtomicBool::new(false);
    let request = request(&source, OpenHints::default(), None, target.clone());
    let result = save(
        &request,
        &mut |done, total| {
            if done == total {
                cancel.store(true, Ordering::Relaxed);
            }
        },
        &cancel,
    );
    assert!(matches!(result, Err(WriteError::Cancelled)));
    assert_eq!(fs::read(&target).unwrap(), b"keep me");
    assert_eq!(names(&dir), ["a.wav", "b.wav"]);
}

#[test]
fn a_source_replaced_since_it_was_opened_is_refused() {
    let dir = TempDir::new("write-replaced");
    let sample_type: SampleType = "iq_i16".parse().unwrap();
    let source = write_wav(&dir.join("a.wav"), sample_type, 48_000, &signal(sample_type, 64), 1.0);
    let request = request(&source, OpenHints::default(), SampleSpan::between(0, 10), dir.join("b.wav"));
    write_wav(&source, sample_type, 48_000, &signal(sample_type, 640), 1.0);
    assert!(matches!(run(&request), Err(WriteError::SourceChanged { .. })));
    assert_eq!(names(&dir), ["a.wav"]);
}

#[test]
fn chunks_after_data_are_not_copied_as_samples() {
    let dir = TempDir::new("write-trailing");
    let sample_type: SampleType = "iq_i16".parse().unwrap();
    let source = write_wav(&dir.join("a.wav"), sample_type, 48_000, &signal(sample_type, 64), 1.0);
    let mut bytes = fs::read(&source).unwrap();
    bytes.extend_from_slice(b"LIST\x04\x00\x00\x00junk");
    let riff_size = (bytes.len() - 8) as u32;
    bytes[4..8].copy_from_slice(&riff_size.to_le_bytes());
    fs::write(&source, &bytes).unwrap();
    let target = dir.join("b.wav");
    run(&request(&source, OpenHints::default(), None, target.clone())).unwrap();
    assert_eq!(data_bytes(&target).len(), 64 * 4);
}

#[test]
fn a_type_hint_on_a_decoded_wave_keeps_its_stored_layout() {
    let dir = TempDir::new("write-24-hint");
    let source = dir.join("a.wav");
    let data = write_wav24(&source, 2, 400);
    let hints = OpenHints {
        sample_type: Some("iq_i32".parse().unwrap()),
        ..Default::default()
    };
    let target = dir.join("b.wav");
    run(&request(&source, hints, SampleSpan::between(10, 30), target.clone())).unwrap();
    let bytes = fs::read(&target).unwrap();
    let chunks = riff::scan(&bytes).unwrap();
    assert_eq!(riff::parse_fmt(chunks.fmt).unwrap().bits, 24);
    assert_eq!(bytes[chunks.data_offset..chunks.data_offset + 120], data[60..180]);
}

#[test]
fn a_decoded_wave_reads_its_frequency_back() {
    let dir = TempDir::new("write-24-freq");
    let source = dir.join("a.wav");
    write_wav24(&source, 2, 400);
    let hints = OpenHints {
        center_freq: Some(144_800_000.5),
        ..Default::default()
    };
    let target = dir.join("b.wav");
    run(&request(&source, hints, None, target.clone())).unwrap();
    let reopened = open(&target, &OpenHints::default()).unwrap();
    assert_eq!(reopened.meta().center_freq, 144_800_000.5);
}

#[test]
fn a_header_that_cannot_state_the_byte_rate_is_refused() {
    let dir = TempDir::new("write-byte-rate");
    let source = dir.join("a.raw");
    fs::write(&source, vec![0u8; 800]).unwrap();
    let hints = OpenHints {
        raw: Some("iq_i32@600M".parse().unwrap()),
        ..Default::default()
    };
    let result = run(&request(&source, hints, None, dir.join("b.wav")));
    assert!(matches!(result, Err(WriteError::Unsupported { .. })), "{result:?}");
    assert_eq!(names(&dir), ["a.raw"]);
}

#[test]
fn flac_keeps_a_fractional_sample_rate() {
    let dir = TempDir::new("write-flac-rate");
    let source = dir.join("a.flac");
    write_flac_fixture(&source, 2, 16, 2000);
    let hints = OpenHints {
        sample_rate: Some(44_100.25),
        ..Default::default()
    };
    let target = dir.join("b.flac");
    run(&request(&source, hints, None, target.clone())).unwrap();
    assert_eq!(open(&target, &OpenHints::default()).unwrap().meta().sample_rate, 44_100.25);
}

#[test]
fn a_damaged_flac_frame_fails_instead_of_being_skipped() {
    let dir = TempDir::new("write-flac-damaged");
    let source = dir.join("a.flac");
    write_flac_fixture(&source, 2, 16, 5000);
    let mut bytes = fs::read(&source).unwrap();
    let middle = bytes.len() / 2;
    for byte in &mut bytes[middle..middle + 64] {
        *byte ^= 0x5A;
    }
    fs::write(&source, &bytes).unwrap();
    let result = run(&request(&source, OpenHints::default(), None, dir.join("b.flac")));
    assert!(result.is_err(), "{result:?}");
    assert_eq!(names(&dir), ["a.flac"]);
}
