use super::*;
use crate::riff;
use crate::testutil::{TempDir, all_sample_types, iq_tone, write_raw, write_wav};
use argand_core::{Domain, SampleFormat};

fn open_wav(dir: &TempDir, name: &str, st: SampleType, values: &[f32], scale: f32) -> MmapSource {
    open_wav_with(dir, name, st, values, scale, Normalize::default_for(st.format))
}

fn open_wav_with(
    dir: &TempDir,
    name: &str,
    st: SampleType,
    values: &[f32],
    scale: f32,
    normalize: Normalize,
) -> MmapSource {
    let path = write_wav(&dir.join(name), st, 24_000, values, scale);
    let bytes = std::fs::read(&path).unwrap();
    let layout = riff::parse(&bytes).unwrap();
    assert_eq!(layout.sample_type, st);

    let meta = SignalMeta {
        sample_rate: layout.sample_rate,
        center_freq: 0.0,
        sample_type: st,
        len_samples: 0,
        container: "wav",
        divisor: 1.0,
        source: path.clone(),
    };
    MmapSource::new(
        &path,
        meta,
        layout.data_offset,
        layout.declared_len.unwrap_or(usize::MAX),
        normalize,
        0.0,
    )
    .unwrap()
}

fn read_all(src: &mut MmapSource) -> Vec<f32> {
    let mut out = Vec::new();
    let mut buf = [0.0f32; 8];
    loop {
        let n = src.read(&mut buf).unwrap();
        if n == 0 {
            return out;
        }
        out.extend_from_slice(&buf[..n]);
    }
}

#[test]
fn round_trips_every_sample_type() {
    let dir = TempDir::new("mmap-roundtrip");
    let values = [0.0f32, 0.5, -0.5, 0.25, -0.25, 0.75];

    for st in all_sample_types() {
        // Level handling is exercised separately; here only the byte-level
        // decode is under test, so nothing is rescaled.
        let mut src = open_wav_with(
            &dir,
            &format!("{st}.wav"),
            st,
            &values,
            1.0,
            Normalize::None,
        );
        assert_eq!(src.meta().sample_type, st, "{st}");
        assert_eq!(
            src.meta().len_samples,
            (values.len() / st.channels()) as u64,
            "{st}"
        );

        let got = read_all(&mut src);
        assert_eq!(got.len(), values.len(), "{st}");
        // u8 has only 8 bits to work with, so allow a coarse tolerance.
        let tolerance = if st.format == SampleFormat::U8 { 0.01 } else { 1e-4 };
        for (got, want) in got.iter().zip(values.iter()) {
            assert!((got - want).abs() < tolerance, "{st}: {got} vs {want}");
        }
    }
}

#[test]
fn length_counts_iq_pairs_once() {
    let dir = TempDir::new("mmap-len");
    let st = SampleType::new(Domain::Iq, SampleFormat::I16);
    let values = iq_tone(100, 24_000.0, 1000.0, 0.5);
    let src = open_wav(&dir, "iq.wav", st, &values, 1.0);
    assert_eq!(src.meta().len_samples, 100);
    assert_eq!(src.meta().duration_seconds(), 100.0 / 24_000.0);
}

#[test]
fn reads_never_split_an_iq_pair() {
    let dir = TempDir::new("mmap-pairs");
    let st = SampleType::new(Domain::Iq, SampleFormat::I16);
    let values: Vec<f32> = (0..20).map(|i| i as f32 / 32.0).collect();
    let mut src = open_wav(&dir, "pairs.wav", st, &values, 1.0);

    // An odd-length buffer must come back with an even count.
    let mut buf = [0.0f32; 5];
    let n = src.read(&mut buf).unwrap();
    assert_eq!(n, 4);
    assert!((buf[0] - 0.0).abs() < 1e-4);
    assert!((buf[3] - 3.0 / 32.0).abs() < 1e-4);

    // A single-slot buffer cannot hold a pair at all.
    let mut one = [0.0f32; 1];
    assert_eq!(src.read(&mut one).unwrap(), 0);
}

#[test]
fn seek_repositions_and_rejects_past_the_end() {
    let dir = TempDir::new("mmap-seek");
    let st = SampleType::new(Domain::Real, SampleFormat::I16);
    let values: Vec<f32> = (0..10).map(|i| i as f32 / 16.0).collect();
    let mut src = open_wav(&dir, "seek.wav", st, &values, 1.0);

    src.seek(7).unwrap();
    let tail = read_all(&mut src);
    assert_eq!(tail.len(), 3);
    assert!((tail[0] - 7.0 / 16.0).abs() < 1e-4);

    // Seeking exactly to the end is legal and yields nothing.
    src.seek(10).unwrap();
    assert!(read_all(&mut src).is_empty());

    assert!(matches!(
        src.seek(11).unwrap_err(),
        SourceError::SeekOutOfRange {
            requested: 11,
            total: 10
        }
    ));
}

#[test]
fn auto_normalize_lifts_an_unnormalised_float_file() {
    let dir = TempDir::new("mmap-norm");
    let st = SampleType::new(Domain::Real, SampleFormat::F16x8);
    let values = [0.0f32, 0.5, -1.0, 0.25];

    // Written 4000x too hot, as an unnormalised capture would be.
    let mut src = open_wav(&dir, "hot.wav", st, &values, 4000.0);
    assert!((src.divisor() - 4000.0 * crate::normalize::AUTO_HEADROOM).abs() < 1e-2);

    let got = read_all(&mut src);
    let peak = got.iter().fold(0.0f32, |m, v| m.max(v.abs()));
    assert!(peak < 1.0 && peak > 0.9, "peak came back as {peak}");
}

#[test]
fn gain_scales_on_top_of_normalization() {
    let dir = TempDir::new("mmap-gain");
    let st = SampleType::new(Domain::Real, SampleFormat::I16);
    let path = write_wav(&dir.join("gain.wav"), st, 24_000, &[0.25f32], 1.0);
    let bytes = std::fs::read(&path).unwrap();
    let layout = riff::parse(&bytes).unwrap();

    let meta = SignalMeta {
        sample_rate: layout.sample_rate,
        center_freq: 0.0,
        sample_type: st,
        len_samples: 0,
        container: "wav",
        divisor: 1.0,
        source: path.clone(),
    };
    let mut src = MmapSource::new(
        &path,
        meta,
        layout.data_offset,
        layout.declared_len.unwrap_or(usize::MAX),
        Normalize::None,
        6.0206, // +6 dB is a factor of two
    )
    .unwrap();

    let got = read_all(&mut src);
    assert!((got[0] - 0.5).abs() < 1e-3, "{got:?}");
}

#[test]
fn a_headerless_file_is_read_from_an_offset() {
    let dir = TempDir::new("mmap-raw");
    let values = [0.5f32, -0.5, 0.25, -0.25];
    let path = write_raw(&dir.join("dump.bin"), SampleFormat::I16, &values, 1.0);

    // Prepend a fake 6-byte header and check --offset skips exactly it.
    let mut with_header = vec![0xAAu8; 6];
    with_header.extend_from_slice(&std::fs::read(&path).unwrap());
    std::fs::write(&path, &with_header).unwrap();

    let st = SampleType::new(Domain::Real, SampleFormat::I16);
    let meta = SignalMeta {
        sample_rate: 24_000.0,
        center_freq: 0.0,
        sample_type: st,
        len_samples: 0,
        container: "raw",
        divisor: 1.0,
        source: path.clone(),
    };
    let mut src =
        MmapSource::new(&path, meta, 6, usize::MAX, Normalize::None, 0.0).unwrap();
    assert_eq!(src.meta().len_samples, 4);

    let got = read_all(&mut src);
    for (got, want) in got.iter().zip(values.iter()) {
        assert!((got - want).abs() < 1e-4, "{got} vs {want}");
    }
}

#[test]
fn data_survives_the_pages_behind_it_being_released() {
    // Consumed pages are handed back to the kernel as the reader advances.
    // They are clean, so re-reading must fault them back in unchanged --
    // this is the one way that optimisation could go wrong.
    let dir = TempDir::new("mmap-release");
    let st = SampleType::new(Domain::Real, SampleFormat::I16);
    let values: Vec<f32> = (0..300_000).map(|i| ((i % 2000) as f32 / 2000.0) - 0.5).collect();
    let mut src = open_wav_with(&dir, "long.wav", st, &values, 1.0, Normalize::None);

    let first = read_all(&mut src);
    assert_eq!(first.len(), values.len());

    src.seek(0).unwrap();
    let second = read_all(&mut src);
    assert_eq!(first, second, "a second pass must read the same samples");

    // And a partial re-read from the middle, after the head has moved on.
    src.seek(0).unwrap();
    let mut buf = [0.0f32; 16];
    src.seek(250_000).unwrap();
    let n = src.read(&mut buf).unwrap();
    assert_eq!(n, 16);
    for (i, got) in buf.iter().enumerate() {
        assert!((got - values[250_000 + i]).abs() < 1e-4, "at {i}: {got}");
    }
}
