use super::*;

use std::sync::atomic::AtomicBool;

use argand_core::{Domain, SampleType};

use crate::testutil::{TempDir, all_sample_types, encode, iq_tone, real_tone, write_raw, write_wav};
use crate::{RawSpec, open};

fn source_file(path: &Path, hints: OpenHints) -> SourceFile {
    let meta = open(path, &hints).expect("open source").meta().clone();
    SourceFile {
        meta,
        hints,
        stamp: SourceStamp::of(path).ok(),
    }
}

fn request(path: &Path, hints: OpenHints, span: Option<SampleSpan>, target: PathBuf) -> SaveRequest {
    SaveRequest::span(source_file(path, hints), span, target)
}

fn run(request: &SaveRequest) -> Result<Saved, WriteError> {
    save(request, &mut |_, _| {}, &AtomicBool::new(false))
}

/// The stored bytes of a WAVE file's samples.
fn data_bytes(path: &Path) -> Vec<u8> {
    let bytes = fs::read(path).expect("read output");
    let chunks = riff::scan(&bytes).expect("scan output");
    let available = bytes.len() - chunks.data_offset;
    let len = chunks.declared_len.map_or(available, |len| len.min(available));
    bytes[chunks.data_offset..chunks.data_offset + len].to_vec()
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
    request.sources[0].meta.len_samples = 100;
    assert!(matches!(run(&request), Err(WriteError::SourceChanged { .. })));
    request.segments = vec![Segment {
        source: 0,
        start: 90,
        len: 11,
    }];
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
    let squatter = dir.join(&format!(".argand-{}-0.part", std::process::id()));
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
    let mut request = request(&source, OpenHints::default(), SampleSpan::between(0, 10), dir.join("b.wav"));
    write_wav(&source, sample_type, 48_000, &signal(sample_type, 640), 1.0);
    assert!(matches!(run(&request), Err(WriteError::SourceChanged { .. })));
    request.sources[0].stamp = None;
    assert!(matches!(run(&request), Err(WriteError::SourceChanged { .. })), "the length alone");
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

/// Byte offsets of the frames in a FLAC file, found by their sync code.
fn flac_frames(bytes: &[u8]) -> Vec<usize> {
    (42..bytes.len() - 1)
        .filter(|&i| bytes[i] == 0xFF && bytes[i + 1] == 0xF8)
        .collect()
}

#[test]
fn a_damaged_frame_inside_a_short_selection_fails() {
    let dir = TempDir::new("write-flac-gap");
    let source = dir.join("a.flac");
    write_flac_fixture(&source, 2, 16, 5000);
    let mut request = request(&source, OpenHints::default(), SampleSpan::between(0, 1500), dir.join("b.flac"));
    request.sources[0].stamp = None;
    let mut bytes = fs::read(&source).unwrap();
    let frames = flac_frames(&bytes);
    let second = frames[1] + (frames[2] - frames[1]) / 2;
    bytes[second] ^= 0xFF;
    fs::write(&source, &bytes).unwrap();
    let result = run(&request);
    assert!(result.is_err(), "{result:?}");
    assert_eq!(names(&dir), ["a.flac"]);
}

#[test]
fn a_flac_selection_starts_on_its_own_sample() {
    let dir = TempDir::new("write-flac-start");
    let source = dir.join("a.flac");
    let samples = write_flac_fixture(&source, 2, 16, 3000);
    for (start, end) in [(15, 25), (1023, 1030), (1024, 2049)] {
        let target = dir.join(&format!("b{start}.flac"));
        run(&request(&source, OpenHints::default(), SampleSpan::between(start, end), target.clone())).unwrap();
        assert_eq!(
            flac_ints(&target, 16),
            samples[start as usize * 2..end as usize * 2],
            "[{start}, {end})"
        );
    }
}

#[test]
fn flac_at_a_rate_the_encoder_cannot_state_is_saved_as_wave() {
    let dir = TempDir::new("write-flac-wave");
    for bits in [16usize, 24, 8] {
        let source = dir.join(&format!("a{bits}.flac"));
        let samples = write_flac_fixture(&source, 2, bits, 3000);
        let hints = OpenHints {
            sample_rate: Some(192_000.0),
            center_freq: Some(7_074_000.0),
            ..Default::default()
        };
        let opened = request(&source, hints.clone(), SampleSpan::between(100, 2100), dir.join(&format!("b{bits}.wav")));
        assert!(writes_as_wave(&opened.sources[0].meta, &hints));
        let saved = run(&opened).unwrap();
        assert_eq!(saved.container, "wav");
        let reopened = open(&saved.path, &OpenHints::default()).unwrap();
        assert_eq!(reopened.meta().sample_rate, 192_000.0);
        assert_eq!(reopened.meta().center_freq, 7_074_000.0);
        let stored = data_bytes(&saved.path);
        let width = bits.div_ceil(8);
        let decoded: Vec<i32> = stored
            .chunks_exact(width)
            .map(|c| match width {
                1 => i32::from(c[0]) - 128,
                2 => i32::from(i16::from_le_bytes([c[0], c[1]])),
                _ => i32::from_le_bytes([0, c[0], c[1], c[2]]) >> 8,
            })
            .collect();
        assert_eq!(decoded, samples[200..4200], "{bits} bit");
    }
}

#[test]
fn a_fractional_rate_states_a_nearby_header_rate_and_keeps_the_exact_one() {
    assert_eq!(flac_header_rate(88_200.75), Some(88_200));
    assert_eq!(flac_header_rate(44_100.25), Some(44_100));
    assert_eq!(flac_header_rate(70_001.0), None);
    assert_eq!(flac_header_rate(2_400_000.0), None);
    let dir = TempDir::new("write-flac-882");
    let source = dir.join("a.flac");
    write_flac_fixture(&source, 1, 16, 2000);
    let hints = OpenHints {
        sample_rate: Some(88_200.75),
        ..Default::default()
    };
    let target = dir.join("b.flac");
    let saved = run(&request(&source, hints, None, target.clone())).unwrap();
    assert_eq!(saved.container, "flac");
    assert_eq!(open(&target, &OpenHints::default()).unwrap().meta().sample_rate, 88_200.75);
}

#[test]
fn a_flac_source_replaced_since_it_was_opened_is_refused() {
    let dir = TempDir::new("write-flac-replaced");
    let source = dir.join("a.flac");
    write_flac_fixture(&source, 2, 16, 5000);
    let mut request = request(&source, OpenHints::default(), SampleSpan::between(0, 100), dir.join("b.flac"));
    write_flac_fixture(&source, 2, 16, 6000);
    assert!(matches!(run(&request), Err(WriteError::SourceChanged { .. })));
    request.sources[0].stamp = None;
    assert!(matches!(run(&request), Err(WriteError::SourceChanged { .. })), "the length alone");
    assert_eq!(names(&dir), ["a.flac"]);
}

#[test]
fn headerless_and_unstateable_flac_captures_are_named_as_wave() {
    let mut meta = SignalMeta {
        sample_rate: 48_000.0,
        center_freq: 0.0,
        sample_type: "iq_i16".parse().unwrap(),
        len_samples: 10,
        container: "flac",
        divisor: 1.0,
        source: PathBuf::from("a.flac"),
    };
    assert!(!writes_as_wave(&meta, &OpenHints::default()));
    meta.sample_rate = 2_400_000.0;
    assert!(writes_as_wave(&meta, &OpenHints::default()));
    meta.container = "wav";
    assert!(!writes_as_wave(&meta, &OpenHints::default()));
    let raw = OpenHints {
        raw: Some("iq_i16@1M".parse().unwrap()),
        ..Default::default()
    };
    assert!(writes_as_wave(&meta, &raw));
}

#[test]
fn a_damaged_first_frame_fails_instead_of_starting_later() {
    let dir = TempDir::new("write-flac-first");
    let source = dir.join("a.flac");
    write_flac_fixture(&source, 2, 16, 5000);
    let mut request = request(&source, OpenHints::default(), SampleSpan::between(0, 10), dir.join("b.flac"));
    request.sources[0].stamp = None;
    let mut bytes = fs::read(&source).unwrap();
    let frames = flac_frames(&bytes);
    bytes[frames[0] + (frames[1] - frames[0]) / 2] ^= 0xFF;
    fs::write(&source, &bytes).unwrap();
    let result = run(&request);
    assert!(result.is_err(), "{result:?}");
    assert_eq!(names(&dir), ["a.flac"]);
}

#[test]
fn an_unknown_length_flac_is_counted_strictly_and_saved_whole() {
    let dir = TempDir::new("write-flac-unknown");
    let source = dir.join("a.flac");
    let mut data = include_bytes!("../tests/fixtures/levels.flac").to_vec();
    data[21] &= 0xf0;
    data[22..26].fill(0);
    fs::write(&source, &data).unwrap();
    let target = dir.join("b.flac");
    let saved = run(&request(&source, OpenHints::default(), None, target.clone())).unwrap();
    assert_eq!(saved.samples, 16384);
    assert_eq!(open(&target, &OpenHints::default()).unwrap().meta().len_samples, 16384);
}

#[test]
fn a_large_wave_the_native_reader_declines_is_refused() {
    let dir = TempDir::new("write-24-rf64");
    let source = dir.join("a.wav");
    write_wav24(&source, 2, 400);
    let request = request(&source, OpenHints::default(), None, dir.join("b.wav"));
    let result = save_with_limit(&request, &mut |_, _| {}, &AtomicBool::new(false), 1000);
    assert!(matches!(result, Err(WriteError::Unsupported { .. })), "{result:?}");
    assert_eq!(names(&dir), ["a.wav"]);
}

#[test]
fn flac_of_an_odd_depth_at_an_unstateable_rate_is_refused() {
    let dir = TempDir::new("write-flac-12");
    let source = dir.join("a.flac");
    write_flac_fixture(&source, 1, 12, 2000);
    let hints = OpenHints {
        sample_rate: Some(192_000.0),
        ..Default::default()
    };
    let result = run(&request(&source, hints, None, dir.join("b.wav")));
    assert!(matches!(result, Err(WriteError::Unsupported { .. })), "{result:?}");
    assert_eq!(names(&dir), ["a.flac"]);
}

#[test]
fn a_rotated_source_is_neither_read_nor_saved_over() {
    let dir = TempDir::new("write-rotated");
    let sample_type: SampleType = "iq_i16".parse().unwrap();
    let source = write_wav(&dir.join("a.wav"), sample_type, 48_000, &signal(sample_type, 64), 1.0);
    let original = fs::read(&source).unwrap();
    let archive = dir.join("archive.wav");
    let request = request(&source, OpenHints::default(), None, archive.clone());
    fs::rename(&source, &archive).unwrap();
    write_wav(&source, sample_type, 48_000, &real_tone(128, 48_000.0, 100.0, 0.25), 1.0);
    assert_eq!(fs::metadata(&source).unwrap().len(), original.len() as u64);
    let result = run(&request);
    assert!(result.is_err(), "{result:?}");
    assert_eq!(fs::read(&archive).unwrap(), original);
}

#[test]
fn a_target_name_at_the_length_limit_still_saves() {
    let dir = TempDir::new("write-long");
    let sample_type: SampleType = "iq_i16".parse().unwrap();
    let source = write_wav(&dir.join("a.wav"), sample_type, 48_000, &signal(sample_type, 64), 1.0);
    let target = dir.join(&format!("{}.wav", "x".repeat(251)));
    run(&request(&source, OpenHints::default(), None, target.clone())).unwrap();
    assert_eq!(data_bytes(&target), data_bytes(&source));
}

fn segment(source: usize, start: u64, len: u64) -> Segment {
    Segment { source, start, len }
}

#[test]
fn segments_of_one_and_two_wave_files_are_joined_in_order() {
    let dir = TempDir::new("write-segments");
    let sample_type: SampleType = "iq_i16".parse().unwrap();
    let a = write_wav(&dir.join("a.wav"), sample_type, 48_000, &signal(sample_type, 100), 1.0);
    let b = write_raw(&dir.join("b.raw"), sample_type.format, &signal(sample_type, 50), 0.5);
    let raw = OpenHints {
        raw: Some("iq_i16@48k".parse().unwrap()),
        ..Default::default()
    };
    let request = SaveRequest::joined(
        vec![source_file(&a, OpenHints::default()), source_file(&b, raw)],
        vec![segment(0, 60, 40), segment(1, 10, 5), segment(0, 0, 3)],
        dir.join("c.wav"),
    ).unwrap();
    let saved = run(&request).unwrap();
    assert_eq!(saved.samples, 48);
    let first = data_bytes(&a);
    let second = fs::read(&b).unwrap();
    let mut expected = first[60 * 4..100 * 4].to_vec();
    expected.extend_from_slice(&second[10 * 4..15 * 4]);
    expected.extend_from_slice(&first[..3 * 4]);
    assert_eq!(data_bytes(&saved.path), expected);
}

#[test]
fn sources_stored_differently_or_at_another_rate_are_refused() {
    let dir = TempDir::new("write-mismatch");
    let i16: SampleType = "iq_i16".parse().unwrap();
    let f32: SampleType = "iq_f32".parse().unwrap();
    let a = write_wav(&dir.join("a.wav"), i16, 48_000, &signal(i16, 10), 1.0);
    let b = write_wav(&dir.join("b.wav"), f32, 48_000, &signal(f32, 10), 1.0);
    let c = write_wav(&dir.join("c.wav"), i16, 96_000, &signal(i16, 10), 1.0);
    for other in [&b, &c] {
        let request = SaveRequest::joined(
            vec![source_file(&a, OpenHints::default()), source_file(other, OpenHints::default())],
            vec![segment(0, 0, 5), segment(1, 0, 5)],
            dir.join("out.wav"),
        ).unwrap();
        assert!(matches!(run(&request), Err(WriteError::Unsupported { .. })));
    }
    assert!(!dir.join("out.wav").exists());
}

#[test]
fn storage_tells_a_raw_file_from_a_wider_wave_of_the_same_type() {
    let dir = TempDir::new("write-storage");
    let i16: SampleType = "iq_i16".parse().unwrap();
    let wav = write_wav(&dir.join("a.wav"), i16, 48_000, &signal(i16, 10), 1.0);
    let raw = write_raw(&dir.join("b.raw"), i16.format, &signal(i16, 10), 1.0);
    let hints = OpenHints {
        raw: Some("iq_i16@48k".parse().unwrap()),
        ..Default::default()
    };
    assert_eq!(
        storage(&source_file(&wav, OpenHints::default())).unwrap(),
        storage(&source_file(&raw, hints)).unwrap()
    );
    let wide = dir.join("c.wav");
    write_wav24(&wide, 2, 10);
    let raw32 = write_raw(&dir.join("d.raw"), SampleFormat::I32, &signal(i16, 10), 1.0);
    let hints32 = OpenHints {
        raw: Some("iq_i32@96k".parse().unwrap()),
        ..Default::default()
    };
    assert_ne!(
        storage(&source_file(&wide, OpenHints::default())).unwrap(),
        storage(&source_file(&raw32, hints32)).unwrap()
    );
}

#[test]
fn flac_segments_from_two_files_cross_block_boundaries_exactly() {
    let dir = TempDir::new("write-flac-segments");
    let a = dir.join("a.flac");
    let b = dir.join("b.flac");
    let first = write_flac_fixture(&a, 2, 16, 9000);
    let second = write_flac_fixture(&b, 2, 16, 3000);
    let request = SaveRequest::joined(
        vec![source_file(&a, OpenHints::default()), source_file(&b, OpenHints::default())],
        vec![segment(0, 4000, 4500), segment(1, 1, 2999), segment(0, 17, 100)],
        dir.join("c.flac"),
    ).unwrap();
    let saved = run(&request).unwrap();
    assert_eq!(saved.container, "flac");
    let mut expected = first[8000..17000].to_vec();
    expected.extend_from_slice(&second[2..6000]);
    expected.extend_from_slice(&first[34..234]);
    assert_eq!(flac_ints(&saved.path, 16), expected);
    let hints = OpenHints {
        sample_rate: Some(192_000.0),
        ..Default::default()
    };
    let wave = SaveRequest::joined(
        vec![source_file(&a, hints.clone()), source_file(&b, hints)],
        vec![segment(1, 5, 10), segment(0, 0, 10)],
        dir.join("d.wav"),
    ).unwrap();
    let saved = run(&wave).unwrap();
    let stored: Vec<i32> = data_bytes(&saved.path)
        .chunks_exact(2)
        .map(|c| i32::from(i16::from_le_bytes([c[0], c[1]])))
        .collect();
    let mut expected = second[10..30].to_vec();
    expected.extend_from_slice(&first[..20]);
    assert_eq!(stored, expected);
}

#[test]
fn no_source_of_a_request_can_be_its_target() {
    let dir = TempDir::new("write-second-source");
    let i16: SampleType = "iq_i16".parse().unwrap();
    let a = write_wav(&dir.join("a.wav"), i16, 48_000, &signal(i16, 10), 1.0);
    let b = write_wav(&dir.join("b.wav"), i16, 48_000, &signal(i16, 10), 1.0);
    let before = fs::read(&b).unwrap();
    let request = SaveRequest::joined(
        vec![source_file(&a, OpenHints::default()), source_file(&b, OpenHints::default())],
        vec![segment(0, 0, 5), segment(1, 0, 5)],
        b.clone(),
    ).unwrap();
    assert!(matches!(run(&request), Err(WriteError::SameFile { .. })));
    assert_eq!(fs::read(&b).unwrap(), before);
}

#[test]
fn a_protected_file_is_never_a_target_and_the_output_states_the_request_meta() {
    let dir = TempDir::new("write-protected");
    let i16: SampleType = "iq_i16".parse().unwrap();
    let a = write_wav(&dir.join("a.wav"), i16, 48_000, &signal(i16, 10), 1.0);
    let b = write_wav(&dir.join("b.wav"), i16, 48_000, &signal(i16, 10), 1.0);
    let mut request = SaveRequest::span(source_file(&b, OpenHints::default()), None, a.clone());
    request.protected = vec![Protected {
        path: a.clone(),
        stamp: None,
    }];
    let before = fs::read(&a).unwrap();
    assert!(matches!(run(&request), Err(WriteError::SameFile { .. })));
    assert_eq!(fs::read(&a).unwrap(), before);
    request.protected.clear();
    request.target = dir.join("c.wav");
    request.meta.center_freq = 145_000_000.0;
    run(&request).unwrap();
    let reopened = open(&dir.join("c.wav"), &OpenHints::default()).unwrap();
    assert_eq!(reopened.meta().center_freq, 145_000_000.0);
}

// Windows has no file identity without the file ID of #199.
#[cfg(unix)]
#[test]
fn a_protected_file_renamed_since_is_still_recognised() {
    let dir = TempDir::new("write-protected-renamed");
    let i16: SampleType = "iq_i16".parse().unwrap();
    let a = write_wav(&dir.join("a.wav"), i16, 48_000, &signal(i16, 10), 1.0);
    let b = write_wav(&dir.join("b.wav"), i16, 48_000, &signal(i16, 10), 1.0);
    let stamp = SourceStamp::of(&a).ok();
    let archive = dir.join("archive.wav");
    fs::rename(&a, &archive).unwrap();
    let before = fs::read(&archive).unwrap();
    let mut request = SaveRequest::span(source_file(&b, OpenHints::default()), None, archive.clone());
    request.protected = vec![Protected { path: a, stamp }];
    assert!(matches!(run(&request), Err(WriteError::SameFile { .. })));
    assert_eq!(fs::read(&archive).unwrap(), before);
}

#[test]
fn opening_stamps_what_it_read_for_every_reader() {
    let dir = TempDir::new("write-open-stamped");
    let i16: SampleType = "iq_i16".parse().unwrap();
    let wav = write_wav(&dir.join("a.wav"), i16, 48_000, &signal(i16, 10), 1.0);
    let raw = write_raw(&dir.join("b.raw"), i16.format, &signal(i16, 10), 1.0);
    let flac = dir.join("c.flac");
    write_flac_fixture(&flac, 2, 16, 3000);
    let raw_hints = OpenHints {
        raw: Some("iq_i16@48k".parse().unwrap()),
        ..Default::default()
    };
    for (path, hints) in [(&wav, OpenHints::default()), (&raw, raw_hints), (&flac, OpenHints::default())] {
        let (_, stamp) = crate::open_stamped(path, &hints).unwrap();
        assert_eq!(stamp, SourceStamp::of(path).ok(), "{}", path.display());
    }
}

fn replacing(path: &Path) -> Option<Replacing> {
    Some(Replacing {
        path: path.to_owned(),
        stamp: SourceStamp::of(path).unwrap(),
    })
}

#[test]
fn a_save_may_replace_the_file_it_reads_once_staged() {
    let dir = TempDir::new("write-replace");
    let i16: SampleType = "iq_i16".parse().unwrap();
    let source = write_wav(&dir.join("a.wav"), i16, 48_000, &signal(i16, 100), 1.0);
    let original = data_bytes(&source);
    let mut request = request(&source, OpenHints::default(), SampleSpan::between(10, 30), source.clone());
    assert!(matches!(run(&request), Err(WriteError::SameFile { .. })), "not without replacing");
    request.replacing = replacing(&source);
    let staged = stage(&request, &mut |_, _| {}, &AtomicBool::new(false)).unwrap();
    assert_eq!(data_bytes(&source), original, "nothing replaced before the commit");
    let saved = staged.commit().unwrap();
    assert_eq!(saved.samples, 20);
    assert_eq!(data_bytes(&source), original[10 * 4..30 * 4]);
    assert_eq!(names(&dir), ["a.wav"]);
}

#[test]
fn a_replaced_file_changed_before_the_commit_is_kept() {
    let dir = TempDir::new("write-replace-changed");
    let i16: SampleType = "iq_i16".parse().unwrap();
    let source = write_wav(&dir.join("a.wav"), i16, 48_000, &signal(i16, 100), 1.0);
    let mut request = request(&source, OpenHints::default(), SampleSpan::between(0, 10), source.clone());
    request.replacing = replacing(&source);
    let staged = stage(&request, &mut |_, _| {}, &AtomicBool::new(false)).unwrap();
    write_wav(&source, i16, 48_000, &signal(i16, 200), 1.0);
    let changed = fs::read(&source).unwrap();
    let refused = staged.commit().unwrap_err();
    assert!(matches!(refused.error, WriteError::SourceChanged { .. }));
    assert_eq!(fs::read(&source).unwrap(), changed);
    let kept = refused.keep_beside();
    assert_eq!(kept, dir.join("a.unsaved-1.wav"));
    assert_eq!(data_bytes(&kept).len(), 10 * 4, "the written samples are kept beside it");
    assert_eq!(names(&dir), ["a.unsaved-1.wav", "a.wav"]);
}

#[test]
fn a_dropped_stage_leaves_no_file() {
    let dir = TempDir::new("write-stage-drop");
    let i16: SampleType = "iq_i16".parse().unwrap();
    let source = write_wav(&dir.join("a.wav"), i16, 48_000, &signal(i16, 100), 1.0);
    let request = request(&source, OpenHints::default(), None, dir.join("b.wav"));
    let staged = stage(&request, &mut |_, _| {}, &AtomicBool::new(false)).unwrap();
    assert_eq!(names(&dir).len(), 2, "the temporary file exists while staged");
    drop(staged);
    assert_eq!(names(&dir), ["a.wav"]);
}

#[test]
fn a_headerless_capture_saved_over_itself_keeps_its_preamble() {
    let dir = TempDir::new("write-headerless");
    let i16: SampleType = "iq_i16".parse().unwrap();
    let source = dir.join("a.bin");
    let mut bytes = b"PREAMBLE".to_vec();
    bytes.extend(encode(i16.format, &signal(i16, 50)));
    fs::write(&source, &bytes).unwrap();
    let hints = OpenHints {
        raw: Some("iq_i16@48k".parse().unwrap()),
        byte_offset: 8,
        ..Default::default()
    };
    let mut request = request(&source, hints, SampleSpan::between(5, 15), source.clone());
    request.replacing = replacing(&source);
    request.output = Output::Headerless { preamble: 8 };
    let saved = run(&request).unwrap();
    assert_eq!(saved.container, "raw");
    let mut expected = b"PREAMBLE".to_vec();
    expected.extend_from_slice(&bytes[8 + 5 * 4..8 + 15 * 4]);
    assert_eq!(fs::read(&source).unwrap(), expected);
}

#[test]
fn flac_cannot_be_written_without_a_container() {
    let dir = TempDir::new("write-headerless-flac");
    let source = dir.join("a.flac");
    write_flac_fixture(&source, 2, 16, 3000);
    let mut request = request(&source, OpenHints::default(), None, dir.join("b.raw"));
    request.output = Output::Headerless { preamble: 0 };
    assert!(matches!(run(&request), Err(WriteError::Unsupported { .. })));
    assert_eq!(names(&dir), ["a.flac"]);
}

#[test]
fn kept_edits_never_take_a_name_another_file_has() {
    let dir = TempDir::new("write-keep-free");
    let i16: SampleType = "iq_i16".parse().unwrap();
    let source = write_wav(&dir.join("a.wav"), i16, 48_000, &signal(i16, 100), 1.0);
    fs::write(dir.join("a.unsaved-1.wav"), b"someone else's").unwrap();
    let mut request = request(&source, OpenHints::default(), SampleSpan::between(0, 10), source.clone());
    request.replacing = replacing(&source);
    let staged = stage(&request, &mut |_, _| {}, &AtomicBool::new(false)).unwrap();
    write_wav(&source, i16, 48_000, &signal(i16, 200), 1.0);
    let kept = staged.commit().unwrap_err().keep_beside();
    assert_eq!(kept, dir.join("a.unsaved-2.wav"));
    assert_eq!(fs::read(dir.join("a.unsaved-1.wav")).unwrap(), b"someone else's");
    assert_eq!(data_bytes(&kept).len(), 10 * 4);
    assert_eq!(names(&dir), ["a.unsaved-1.wav", "a.unsaved-2.wav", "a.wav"]);
}

#[test]
fn the_saved_stamp_is_the_written_file_as_it_was_closed() {
    let dir = TempDir::new("write-saved-stamp");
    let i16: SampleType = "iq_i16".parse().unwrap();
    let source = write_wav(&dir.join("a.wav"), i16, 48_000, &signal(i16, 100), 1.0);
    let target = dir.join("b.wav");
    let saved = run(&request(&source, OpenHints::default(), None, target.clone())).unwrap();
    assert_eq!(saved.stamp, SourceStamp::of(&target).ok());
}

#[test]
fn a_kiwi_recording_is_saved_as_one_data_chunk() {
    let dir = TempDir::new("write-kiwi");
    let values = iq_tone(1300, 12_000.0, 1_000.0, 0.5);
    let source = crate::testutil::write_kiwi_wav(&dir.join("iq.wav"), 12_000, &values, 512);
    let stored = encode(SampleFormat::I16, &values);
    let target = dir.join("cut.wav");
    let span = SampleSpan::between(300, 1100);
    let saved = run(&request(&source, OpenHints::default(), span, target.clone())).unwrap();
    assert_eq!(saved.samples, 800);
    assert_eq!(data_bytes(&target), stored[300 * 4..1100 * 4]);
    let bytes = fs::read(&target).unwrap();
    assert!(!bytes.windows(4).any(|id| id == b"kiwi"));
    assert_eq!(read_all(&target, &OpenHints::default()), read_all(&source, &OpenHints::default())[600..2200]);

    let whole = dir.join("whole.wav");
    run(&request(&source, OpenHints::default(), None, whole.clone())).unwrap();
    assert_eq!(data_bytes(&whole), stored);
}

#[test]
fn a_long_kiwi_recording_is_copied_in_large_reads() {
    let dir = TempDir::new("write-kiwi-long");
    let values: Vec<f32> = (0..3_000_000).map(|n| (n % 2001) as f32 / 2001.0 - 0.5).collect();
    let source = crate::testutil::write_kiwi_wav(&dir.join("iq.wav"), 12_000, &values, 512);
    let stored = encode(SampleFormat::I16, &values);
    let target = dir.join("cut.wav");
    let span = SampleSpan::between(100, 1_400_000);
    let saved = run(&request(&source, OpenHints::default(), span, target.clone())).unwrap();
    assert_eq!(saved.samples, 1_399_900);
    assert_eq!(data_bytes(&target), stored[100 * 4..1_400_000 * 4]);
}

#[test]
fn a_declined_layout_with_several_data_chunks_saves_what_it_opens() {
    let dir = TempDir::new("write-24-chunks");
    let source = dir.join("a.wav");
    let data = write_wav24(&source, 2, 400);
    let mut bytes = fs::read(&source).unwrap();
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&(60u32).to_le_bytes());
    bytes.extend_from_slice(&[1; 60]);
    let riff_size = (bytes.len() - 8) as u32;
    bytes[4..8].copy_from_slice(&riff_size.to_le_bytes());
    fs::write(&source, &bytes).unwrap();
    let target = dir.join("b.wav");
    let saved = run(&request(&source, OpenHints::default(), None, target.clone())).unwrap();
    assert_eq!(saved.samples, open(&source, &OpenHints::default()).unwrap().meta().len_samples);
    assert_eq!(data_bytes(&target), data);
}
