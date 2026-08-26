use super::*;

/// Assemble a WAVE file from a raw `fmt ` body plus `data` bytes.
fn wave(fmt_body: &[u8], data: &[u8], extra_chunks: &[(&[u8; 4], Vec<u8>)]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&0u32.to_le_bytes()); // patched below
    out.extend_from_slice(b"WAVE");

    let chunk = |id: &[u8; 4], body: &[u8], out: &mut Vec<u8>| {
        out.extend_from_slice(id);
        out.extend_from_slice(&(body.len() as u32).to_le_bytes());
        out.extend_from_slice(body);
        if body.len() % 2 == 1 {
            out.push(0); // word alignment pad
        }
    };

    chunk(b"fmt ", fmt_body, &mut out);
    for (id, body) in extra_chunks {
        chunk(id, body, &mut out);
    }
    chunk(b"data", data, &mut out);

    let riff_size = (out.len() - 8) as u32;
    out[4..8].copy_from_slice(&riff_size.to_le_bytes());
    out
}

fn fmt_body(tag: u16, channels: u16, rate: u32, bits: u16, ext: Option<u32>) -> Vec<u8> {
    let block_align = channels * bits / 8;
    let mut body = Vec::new();
    body.extend_from_slice(&tag.to_le_bytes());
    body.extend_from_slice(&channels.to_le_bytes());
    body.extend_from_slice(&rate.to_le_bytes());
    body.extend_from_slice(&(rate * block_align as u32).to_le_bytes());
    body.extend_from_slice(&block_align.to_le_bytes());
    body.extend_from_slice(&bits.to_le_bytes());
    if let Some(word) = ext {
        body.extend_from_slice(&word.to_le_bytes());
    }
    body
}

fn parse_layout(tag: u16, channels: u16, bits: u16, ext: Option<u32>) -> WavLayout {
    let data = vec![0u8; 64];
    let file = wave(&fmt_body(tag, channels, 24_000, bits, ext), &data, &[]);
    parse(&file).expect("layout should parse")
}

#[test]
fn detects_every_supported_wav_layout() {
    let cases: [(u16, u16, Option<u32>, SampleFormat); 5] = [
        (WAVE_FORMAT_PCM, 8, None, SampleFormat::U8),
        (WAVE_FORMAT_PCM, 16, None, SampleFormat::I16),
        (WAVE_FORMAT_PCM, 32, None, SampleFormat::I32),
        (WAVE_FORMAT_IEEE_FLOAT, 32, None, SampleFormat::F32),
        (WAVE_FORMAT_PCM, 32, Some(F16X8_MAGIC), SampleFormat::F16x8),
    ];

    for (tag, bits, ext, want) in cases {
        for (channels, domain) in [(1, Domain::Real), (2, Domain::Iq)] {
            let layout = parse_layout(tag, channels, bits, ext);
            assert_eq!(
                layout.sample_type,
                SampleType::new(domain, want),
                "tag {tag}, {bits} bit, {channels} ch"
            );
            assert_eq!(layout.sample_rate, 24_000.0);
        }
    }
}

#[test]
fn a_20_byte_fmt_without_the_magic_is_not_f16x8() {
    // Same shape, wrong magic: this must stay plain 32-bit integer PCM.
    let layout = parse_layout(WAVE_FORMAT_PCM, 1, 32, Some(0xDEAD_BEEF));
    assert_eq!(layout.sample_type.format, SampleFormat::I32);
}

#[test]
fn extensible_reads_the_format_tag_from_the_guid() {
    let mut body = fmt_body(WAVE_FORMAT_EXTENSIBLE, 2, 48_000, 32, None);
    body.extend_from_slice(&22u16.to_le_bytes()); // cbSize
    body.extend_from_slice(&32u16.to_le_bytes()); // valid bits
    body.extend_from_slice(&3u32.to_le_bytes()); // channel mask
    body.extend_from_slice(&WAVE_FORMAT_IEEE_FLOAT.to_le_bytes());
    body.extend_from_slice(&[0u8; 14]); // rest of the GUID

    let file = wave(&body, &[0u8; 32], &[]);
    let layout = parse(&file).unwrap();
    assert_eq!(
        layout.sample_type,
        SampleType::new(Domain::Iq, SampleFormat::F32)
    );
}

#[test]
fn skips_unknown_and_odd_sized_chunks() {
    let extras: Vec<(&[u8; 4], Vec<u8>)> = vec![
        (b"fact", 0u32.to_le_bytes().to_vec()),
        (b"LIST", vec![b'x'; 7]), // odd length, needs a pad byte
    ];
    let file = wave(
        &fmt_body(WAVE_FORMAT_PCM, 2, 24_000, 16, None),
        &[0u8; 40],
        &extras,
    );
    let layout = parse(&file).unwrap();
    assert_eq!(layout.data_len(file.len()), 40);
    assert_eq!(layout.len_samples(file.len()), 10);
    assert_eq!(&file[layout.data_offset..layout.data_offset + 40], &[0u8; 40]);
}

#[test]
fn a_zero_or_oversized_data_header_falls_back_to_the_file_length() {
    let mut file = wave(
        &fmt_body(WAVE_FORMAT_PCM, 2, 24_000, 16, None),
        &[7u8; 40],
        &[],
    );
    let data_size_at = file.len() - 44;

    file[data_size_at..data_size_at + 4].copy_from_slice(&0u32.to_le_bytes());
    let layout = parse(&file).unwrap();
    assert_eq!(layout.declared_len, None);
    assert_eq!(layout.data_len(file.len()), 40);

    file[data_size_at..data_size_at + 4].copy_from_slice(&999_999u32.to_le_bytes());
    let layout = parse(&file).unwrap();
    assert_eq!(layout.declared_len, Some(999_999));
    assert_eq!(layout.data_len(file.len()), 40);
}

#[test]
fn a_declared_length_survives_being_parsed_from_the_head_alone() {
    // The reader only maps the first pages before parsing, so the declared
    // length must not be clamped to whatever the parser happened to see.
    let file = wave(
        &fmt_body(WAVE_FORMAT_PCM, 2, 24_000, 16, None),
        &[0u8; 4000],
        &[],
    );
    let head = &file[..512];
    let layout = parse(head).unwrap();
    assert_eq!(layout.declared_len, Some(4000));
    assert_eq!(layout.len_samples(file.len()), 1000);
}

#[test]
fn a_partial_trailing_frame_is_dropped() {
    // 41 bytes of iq_i16 is ten whole frames plus one stray byte.
    let file = wave(
        &fmt_body(WAVE_FORMAT_PCM, 2, 24_000, 16, None),
        &[0u8; 41],
        &[],
    );
    let layout = parse(&file).unwrap();
    assert_eq!(layout.data_len(file.len()), 40);
    assert_eq!(layout.len_samples(file.len()), 10);
}

#[test]
fn rejects_files_it_cannot_describe() {
    assert_eq!(parse(b"not a wave at all").unwrap_err(), RiffError::NotWave);

    // Three channels is neither real nor I/Q.
    let file = wave(&fmt_body(WAVE_FORMAT_PCM, 3, 24_000, 16, None), &[0; 6], &[]);
    assert!(matches!(
        parse(&file).unwrap_err(),
        RiffError::Unsupported { channels: 3, .. }
    ));

    // 24-bit is a valid wav, just not one argand claims to read.
    let file = wave(&fmt_body(WAVE_FORMAT_PCM, 1, 24_000, 24, None), &[0; 6], &[]);
    assert!(matches!(
        parse(&file).unwrap_err(),
        RiffError::Unsupported { bits: 24, .. }
    ));

    let mut headerless = wave(&fmt_body(WAVE_FORMAT_PCM, 1, 24_000, 16, None), &[0; 6], &[]);
    headerless.truncate(20);
    assert!(parse(&headerless).is_err());
}

/// Wrap a chunk list in a 64-bit WAVE header with a `ds64` in front.
fn wave64bit(magic: &[u8; 4], fmt: &[u8], data: &[u8], ds64_data_len: Option<u64>) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(magic);
    out.extend_from_slice(&u32::MAX.to_le_bytes()); // riff size lives in ds64
    out.extend_from_slice(b"WAVE");

    if let Some(len) = ds64_data_len {
        let mut ds64 = Vec::new();
        ds64.extend_from_slice(&0u64.to_le_bytes()); // riffSize
        ds64.extend_from_slice(&len.to_le_bytes()); // dataSize
        ds64.extend_from_slice(&(len / 4).to_le_bytes()); // sampleCount
        ds64.extend_from_slice(&0u32.to_le_bytes()); // tableLength
        out.extend_from_slice(b"ds64");
        out.extend_from_slice(&(ds64.len() as u32).to_le_bytes());
        out.extend_from_slice(&ds64);
    }

    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&(fmt.len() as u32).to_le_bytes());
    out.extend_from_slice(fmt);
    out.extend_from_slice(b"data");
    // The 32-bit field is the sentinel; the real length came from ds64.
    out.extend_from_slice(&u32::MAX.to_le_bytes());
    out.extend_from_slice(data);
    out
}

#[test]
fn rf64_and_bw64_take_their_length_from_ds64() {
    for (magic, want_container) in [(b"RF64", "rf64"), (b"BW64", "bw64")] {
        let data = vec![0u8; 400];
        let file = wave64bit(
            magic,
            &fmt_body(WAVE_FORMAT_PCM, 2, 24_000, 16, None),
            &data,
            Some(8_000_000_000),
        );
        let layout = parse(&file).unwrap();

        assert_eq!(layout.container, want_container);
        assert_eq!(
            layout.sample_type,
            SampleType::new(Domain::Iq, SampleFormat::I16)
        );
        // Eight gigabytes, far past what the 32-bit field could have said.
        assert_eq!(layout.declared_len, Some(8_000_000_000));
        assert_eq!(layout.len_samples(8_000_000_100), 2_000_000_000);
    }
}

#[test]
fn a_64_bit_header_without_ds64_is_an_error_not_a_truncation() {
    let file = wave64bit(
        b"RF64",
        &fmt_body(WAVE_FORMAT_PCM, 2, 24_000, 16, None),
        &[0u8; 40],
        None,
    );
    assert!(matches!(
        parse(&file).unwrap_err(),
        RiffError::MissingDs64 { container: "rf64" }
    ));
}

#[test]
fn a_short_ds64_is_rejected() {
    let mut file = wave64bit(
        b"RF64",
        &fmt_body(WAVE_FORMAT_PCM, 2, 24_000, 16, None),
        &[0u8; 40],
        Some(1000),
    );
    // Shrink the ds64 body to below the mandatory 28 bytes.
    let at = 12 + 4;
    file[at..at + 4].copy_from_slice(&8u32.to_le_bytes());
    assert!(matches!(
        parse(&file).unwrap_err(),
        RiffError::Truncated { what: "ds64 chunk", .. }
    ));
}

#[test]
fn the_sentinel_length_means_unknown_in_a_plain_riff_too() {
    // A recorder killed mid-capture leaves 0xFFFFFFFF behind. Taking it at
    // face value caps the read at four gigabytes, silently.
    let mut file = wave(
        &fmt_body(WAVE_FORMAT_PCM, 2, 24_000, 16, None),
        &[7u8; 40],
        &[],
    );
    let data_size_at = file.len() - 44;
    file[data_size_at..data_size_at + 4].copy_from_slice(&u32::MAX.to_le_bytes());

    let layout = parse(&file).unwrap();
    assert_eq!(layout.declared_len, None, "the sentinel is not a length");
    assert_eq!(layout.data_len(file.len()), 40);

    // On a file that really is past four gigabytes, the whole of it is read.
    let five_gb = 5 * 1024 * 1024 * 1024;
    assert_eq!(
        layout.len_samples(five_gb),
        ((five_gb - layout.data_offset) / 4) as u64
    );
}

#[test]
fn a_declared_length_is_never_capped_at_four_gigabytes() {
    let file = wave(&fmt_body(WAVE_FORMAT_PCM, 2, 24_000, 16, None), &[0u8; 40], &[]);
    let layout = WavLayout {
        declared_len: Some(6_000_000_000),
        ..parse(&file).unwrap()
    };
    assert_eq!(layout.data_len(7_000_000_000), 6_000_000_000);
    assert_eq!(layout.len_samples(7_000_000_000), 1_500_000_000);
}

#[test]
fn container_probes_do_not_overlap() {
    let wav = wave(&fmt_body(WAVE_FORMAT_PCM, 1, 8000, 16, None), &[0; 4], &[]);
    assert!(is_wave(&wav));
    assert!(!is_flac(&wav));
    assert!(is_wave(b"RF64\xff\xff\xff\xffWAVE"));
    assert!(is_wave(b"BW64\xff\xff\xff\xffWAVE"));
    assert!(!is_wave(b"RF64\xff\xff\xff\xffAVI "));
    assert!(is_flac(b"fLaC\x00\x00\x00\x22"));
    assert!(!is_wave(b"fLaC\x00\x00\x00\x22"));
    assert!(!is_wave(b"RIF"));
    assert!(!is_flac(b"fLa"));
}
