//! Fixture helpers: scratch directories and synthetic signal files.
//!
//! Enabled for this crate's own tests and, via the `testutil` feature, for the
//! dsp and cli test suites. Writing the fixtures rather than committing them
//! keeps 172 MB captures out of the repository and lets every sample format be
//! covered, including the two the real fixtures happen not to use.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use argand_core::{SampleFormat, SampleType};

use crate::riff::F16X8_MAGIC;

static COUNTER: AtomicU32 = AtomicU32::new(0);

/// A scratch directory removed when it goes out of scope.
pub struct TempDir {
    path: PathBuf,
}

impl TempDir {
    pub fn new(label: &str) -> Self {
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("argand-{label}-{}-{unique}", std::process::id()));
        std::fs::create_dir_all(&path).expect("create scratch dir");
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn join(&self, name: &str) -> PathBuf {
        self.path.join(name)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// Encode unit-scale values into a sample format's own representation.
pub fn encode(format: SampleFormat, values: &[f32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(values.len() * format.bytes());
    for &v in values {
        match format {
            SampleFormat::U8 => out.push(((v * 128.0).round() as i32 + 128).clamp(0, 255) as u8),
            SampleFormat::I16 => {
                let q = (v * 32768.0).round().clamp(-32768.0, 32767.0) as i16;
                out.extend_from_slice(&q.to_le_bytes());
            }
            SampleFormat::I32 => {
                let q = (v as f64 * 2147483648.0)
                    .round()
                    .clamp(-2147483648.0, 2147483647.0) as i32;
                out.extend_from_slice(&q.to_le_bytes());
            }
            SampleFormat::F32 | SampleFormat::F16x8 => {
                out.extend_from_slice(&v.to_le_bytes());
            }
        }
    }
    out
}

/// Write a WAVE file in the layout argand expects for `sample_type`.
///
/// `values` are interleaved unit-scale samples: `I, Q, I, Q, ...` for an I/Q
/// type, one value per sample for a real one. `scale` multiplies them before
/// encoding, which is how an unnormalised `f16x8` fixture is produced.
pub fn write_wav(
    path: &Path,
    sample_type: SampleType,
    sample_rate: u32,
    values: &[f32],
    scale: f32,
) -> PathBuf {
    let format = sample_type.format;
    let channels = sample_type.channels() as u16;
    let bits = (format.bytes() * 8) as u16;
    let block_align = channels * bits / 8;

    let scaled: Vec<f32> = values.iter().map(|v| v * scale).collect();
    let data = encode(format, &scaled);

    // The 16x8 extension is what separates an unnormalised float file from an
    // ordinary 32-bit integer one: same format tag, same width, four extra
    // bytes in `fmt `.
    let ext = matches!(format, SampleFormat::F16x8).then_some(F16X8_MAGIC);
    let tag: u16 = match format {
        SampleFormat::F32 => 3,
        _ => 1,
    };

    let mut fmt = Vec::new();
    fmt.extend_from_slice(&tag.to_le_bytes());
    fmt.extend_from_slice(&channels.to_le_bytes());
    fmt.extend_from_slice(&sample_rate.to_le_bytes());
    fmt.extend_from_slice(&(sample_rate * block_align as u32).to_le_bytes());
    fmt.extend_from_slice(&block_align.to_le_bytes());
    fmt.extend_from_slice(&bits.to_le_bytes());
    if let Some(word) = ext {
        fmt.extend_from_slice(&word.to_le_bytes());
    }

    let mut file = Vec::new();
    file.extend_from_slice(b"RIFF");
    file.extend_from_slice(&0u32.to_le_bytes());
    file.extend_from_slice(b"WAVE");
    file.extend_from_slice(b"fmt ");
    file.extend_from_slice(&(fmt.len() as u32).to_le_bytes());
    file.extend_from_slice(&fmt);
    file.extend_from_slice(b"data");
    file.extend_from_slice(&(data.len() as u32).to_le_bytes());
    file.extend_from_slice(&data);
    let riff_size = (file.len() - 8) as u32;
    file[4..8].copy_from_slice(&riff_size.to_le_bytes());

    std::fs::File::create(path)
        .and_then(|mut f| f.write_all(&file))
        .expect("write wav fixture");
    path.to_owned()
}

/// Write a headerless file holding the same values.
pub fn write_raw(path: &Path, format: SampleFormat, values: &[f32], scale: f32) -> PathBuf {
    let scaled: Vec<f32> = values.iter().map(|v| v * scale).collect();
    std::fs::File::create(path)
        .and_then(|mut f| f.write_all(&encode(format, &scaled)))
        .expect("write raw fixture");
    path.to_owned()
}

/// A complex tone at `freq_hz`, interleaved as `I, Q, ...`.
pub fn iq_tone(len: usize, sample_rate: f64, freq_hz: f64, amplitude: f32) -> Vec<f32> {
    let mut out = Vec::with_capacity(len * 2);
    for n in 0..len {
        let phase = std::f64::consts::TAU * freq_hz * n as f64 / sample_rate;
        out.push(amplitude * phase.cos() as f32);
        out.push(amplitude * phase.sin() as f32);
    }
    out
}

/// A real tone at `freq_hz`.
pub fn real_tone(len: usize, sample_rate: f64, freq_hz: f64, amplitude: f32) -> Vec<f32> {
    (0..len)
        .map(|n| {
            let phase = std::f64::consts::TAU * freq_hz * n as f64 / sample_rate;
            amplitude * phase.cos() as f32
        })
        .collect()
}

/// Every sample type, for tests that must cover the whole matrix.
pub fn all_sample_types() -> Vec<SampleType> {
    argand_core::SAMPLE_TYPE_TOKENS
        .iter()
        .map(|t| t.parse().expect("known token"))
        .collect()
}
