//! RIFF/WAVE header parsing.
//!
//! Covers every layout argand needs, including two that general-purpose
//! decoders miss:
//!
//! * The CoolEdit "16x8" extension marks a float32 file whose samples are
//!   *not* scaled to [-1, 1]. It claims `audioFormat = 1` (integer PCM) with
//!   32 bits per sample and is distinguishable only by a 20-byte `fmt `
//!   chunk carrying a known magic word.
//! * RF64 and BW64 carry the real sizes in a `ds64` chunk, because RIFF's
//!   32-bit length fields stop at 4 GB. That ceiling is close for radio work:
//!   an I/Q capture at 2.4 MS/s of int16 crosses it in seven and a half
//!   minutes. A reader that ignores `ds64` does not fail on such a file, it
//!   silently stops partway through.

use argand_core::{Domain, SampleFormat, SampleType};

/// `fmt ` extension word that marks non-normalised float32 samples.
pub const F16X8_MAGIC: u32 = 0x0001_0002;

/// A 32-bit size field that means "look in `ds64`", or just "unknown" in a
/// plain RIFF file a streaming recorder never got to finalise.
const SIZE_UNKNOWN: u32 = 0xFFFF_FFFF;

/// Smallest useful `ds64`: riffSize, dataSize, sampleCount, tableLength.
const DS64_MIN_LEN: usize = 28;

const WAVE_FORMAT_PCM: u16 = 1;
const WAVE_FORMAT_IEEE_FLOAT: u16 = 3;
const WAVE_FORMAT_EXTENSIBLE: u16 = 0xFFFE;

/// Where the samples live in a WAVE file and how to read them.
///
/// Parsing only ever sees the head of the file, so the data length stays as
/// declared; resolving it against the real file length is [`data_len`].
///
/// [`data_len`]: WavLayout::data_len
#[derive(Debug, Clone, PartialEq)]
pub struct WavLayout {
    pub sample_type: SampleType,
    pub sample_rate: f64,
    pub data_offset: usize,
    /// Length of the sample data, from `ds64` when present and from the
    /// `data` header otherwise. `None` when neither gave a usable answer, as
    /// with a capture whose writer was killed before it could finalise.
    pub declared_len: Option<usize>,
    /// Which of the three WAVE flavours this was, for the report.
    pub container: &'static str,
}

impl WavLayout {
    /// Bytes of sample data actually present in a file of `file_len` bytes.
    ///
    /// Takes the smaller of the declared and available lengths, so a stale or
    /// missing header cannot send the reader past the end, and drops any
    /// partial trailing frame.
    pub fn data_len(&self, file_len: usize) -> usize {
        let available = file_len.saturating_sub(self.data_offset);
        let len = self.declared_len.unwrap_or(usize::MAX).min(available);
        len - len % self.sample_type.bytes_per_sample()
    }

    pub fn len_samples(&self, file_len: usize) -> u64 {
        (self.data_len(file_len) / self.sample_type.bytes_per_sample()) as u64
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RiffError {
    #[error("not a RIFF/WAVE file")]
    NotWave,
    #[error("truncated {what} at offset {offset}")]
    Truncated { what: &'static str, offset: usize },
    #[error("`fmt ` chunk not found")]
    NoFmt,
    #[error("`data` chunk not found")]
    NoData,
    #[error("{container} file has no `ds64` chunk, so its real length is unknown")]
    MissingDs64 { container: &'static str },
    #[error("declared data length of {bytes} bytes does not fit in this platform's address space")]
    TooLarge { bytes: u64 },
    #[error("unsupported wav layout: format tag {format_tag}, {bits} bit, {channels} channel(s)")]
    Unsupported {
        format_tag: u16,
        bits: u16,
        channels: u16,
    },
}

/// Quick check used to decide whether to even try the WAVE reader.
///
/// All three flavours share the RIFF shape and differ only in the leading
/// magic: `RIFF` is the classic 32-bit form, `RF64` (EBU Tech 3306) and
/// `BW64` (ITU-R BS.2088) are its 64-bit successors.
pub fn is_wave(bytes: &[u8]) -> bool {
    bytes.len() >= 12
        && matches!(&bytes[0..4], b"RIFF" | b"RF64" | b"BW64")
        && &bytes[8..12] == b"WAVE"
}

/// Name for the leading magic, or `None` if it is not a WAVE file at all.
fn container_of(bytes: &[u8]) -> Option<&'static str> {
    match bytes.get(0..4)? {
        b"RIFF" => Some("wav"),
        b"RF64" => Some("rf64"),
        b"BW64" => Some("bw64"),
        _ => None,
    }
}

pub fn is_flac(bytes: &[u8]) -> bool {
    bytes.len() >= 4 && &bytes[0..4] == b"fLaC"
}

/// Walk the chunk list and work out the sample layout.
pub fn parse(bytes: &[u8]) -> Result<WavLayout, RiffError> {
    if !is_wave(bytes) {
        return Err(RiffError::NotWave);
    }
    let container = container_of(bytes).ok_or(RiffError::NotWave)?;
    let is_64bit = container != "wav";

    let mut fmt: Option<FmtChunk> = None;
    let mut data: Option<(usize, Option<usize>)> = None;
    let mut ds64_data_len: Option<usize> = None;
    let mut pos = 12;

    while pos + 8 <= bytes.len() {
        let id = &bytes[pos..pos + 4];
        let raw_size = u32::from_le_bytes([
            bytes[pos + 4],
            bytes[pos + 5],
            bytes[pos + 6],
            bytes[pos + 7],
        ]);
        let size = raw_size as usize;
        let body = pos + 8;

        match id {
            b"fmt " => {
                let body_bytes = slice(bytes, body, size, "fmt chunk")?;
                fmt = Some(parse_fmt(body_bytes)?);
            }
            b"ds64" => {
                let body_bytes = slice(bytes, body, size, "ds64 chunk")?;
                ds64_data_len = Some(parse_ds64(body_bytes)?);
            }
            b"data" => {
                // A finalised RIFF states the length here. RF64 parks the
                // sentinel and puts the real one in `ds64`; a recorder that
                // died mid-capture leaves zero or the sentinel behind.
                let from_header = (raw_size != 0 && raw_size != SIZE_UNKNOWN).then_some(size);
                data = Some((body, ds64_data_len.or(from_header)));
                break;
            }
            _ => {}
        }

        // Chunks are word-aligned: an odd size is followed by a pad byte.
        let step = size + (size & 1);
        match body.checked_add(step) {
            Some(next) if next > pos => pos = next,
            // An oversized or sentinel length on some other chunk: there is
            // nothing sane left to walk to.
            _ => break,
        }
    }

    let fmt = fmt.ok_or(RiffError::NoFmt)?;
    let (data_offset, declared_len) = data.ok_or(RiffError::NoData)?;
    if is_64bit && ds64_data_len.is_none() {
        return Err(RiffError::MissingDs64 { container });
    }

    Ok(WavLayout {
        sample_type: fmt.sample_type()?,
        sample_rate: fmt.sample_rate as f64,
        data_offset,
        declared_len,
        container,
    })
}

fn slice<'a>(
    bytes: &'a [u8],
    offset: usize,
    len: usize,
    what: &'static str,
) -> Result<&'a [u8], RiffError> {
    let end = offset
        .checked_add(len)
        .ok_or(RiffError::Truncated { what, offset })?;
    bytes
        .get(offset..end)
        .ok_or(RiffError::Truncated { what, offset })
}

/// Read the 64-bit data length out of a `ds64` chunk.
///
/// The chunk also carries the RIFF size, a sample count and a table of sizes
/// for any other oversized chunk; only the data length matters here, since
/// everything else argand reads is small by construction.
fn parse_ds64(body: &[u8]) -> Result<usize, RiffError> {
    if body.len() < DS64_MIN_LEN {
        return Err(RiffError::Truncated {
            what: "ds64 chunk",
            offset: 0,
        });
    }
    let data_size = u64::from_le_bytes(body[8..16].try_into().expect("8 bytes"));
    usize::try_from(data_size).map_err(|_| RiffError::TooLarge { bytes: data_size })
}

#[derive(Debug, Clone, Copy)]
struct FmtChunk {
    format_tag: u16,
    channels: u16,
    sample_rate: u32,
    bits: u16,
    /// First extension word, when `fmt ` is exactly 20 bytes.
    ext_word: Option<u32>,
    /// Sub-format tag from a WAVE_FORMAT_EXTENSIBLE GUID.
    sub_format: Option<u16>,
}

fn parse_fmt(body: &[u8]) -> Result<FmtChunk, RiffError> {
    if body.len() < 16 {
        return Err(RiffError::Truncated {
            what: "fmt chunk",
            offset: 0,
        });
    }
    let u16_at = |i: usize| u16::from_le_bytes([body[i], body[i + 1]]);
    let u32_at = |i: usize| u32::from_le_bytes([body[i], body[i + 1], body[i + 2], body[i + 3]]);

    Ok(FmtChunk {
        format_tag: u16_at(0),
        channels: u16_at(2),
        sample_rate: u32_at(4),
        bits: u16_at(14),
        ext_word: (body.len() == 20).then(|| u32_at(16)),
        // Extensible layout: cbSize, validBits, channelMask, then a 16-byte
        // GUID whose first two bytes are the real format tag.
        sub_format: (body.len() >= 40).then(|| u16_at(24)),
    })
}

impl FmtChunk {
    fn sample_type(&self) -> Result<SampleType, RiffError> {
        let domain = match self.channels {
            1 => Domain::Real,
            2 => Domain::Iq,
            _ => return Err(self.unsupported()),
        };

        let effective_tag = if self.format_tag == WAVE_FORMAT_EXTENSIBLE {
            self.sub_format.ok_or_else(|| self.unsupported())?
        } else {
            self.format_tag
        };

        // Checked before the plain integer path: a 16x8 file is an integer
        // PCM file as far as the format tag is concerned.
        if effective_tag == WAVE_FORMAT_PCM && self.bits == 32 && self.ext_word == Some(F16X8_MAGIC)
        {
            return Ok(SampleType::new(domain, SampleFormat::F16x8));
        }

        let format = match (effective_tag, self.bits) {
            (WAVE_FORMAT_PCM, 8) => SampleFormat::U8,
            (WAVE_FORMAT_PCM, 16) => SampleFormat::I16,
            (WAVE_FORMAT_PCM, 32) => SampleFormat::I32,
            (WAVE_FORMAT_IEEE_FLOAT, 32) => SampleFormat::F32,
            _ => return Err(self.unsupported()),
        };
        Ok(SampleType::new(domain, format))
    }

    fn unsupported(&self) -> RiffError {
        RiffError::Unsupported {
            format_tag: self.format_tag,
            bits: self.bits,
            channels: self.channels,
        }
    }
}

#[cfg(test)]
mod tests {
    include!("riff_tests.rs");
}
