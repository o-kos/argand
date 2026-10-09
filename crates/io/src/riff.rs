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
//! * KiwiSDR recorders split the samples over many `data` chunks, each after
//!   a `kiwi` chunk with the GNSS time of its block. A reader that stops at
//!   the first `data` chunk sees half a second of a recording that lasts
//!   minutes; [`WavLayout::runs`] collects them all.

use std::io::{BufReader, Read, Seek, SeekFrom};

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
    /// Physical frequency of baseband 0 Hz, from `argd` or `auxi`.
    pub reference_freq: Option<f64>,
    /// End of the RIFF body as its header states it, where later `data` chunks are looked for.
    pub riff_end: Option<u64>,
}

impl WavLayout {
    /// Every run of sample bytes in the file, read from `input` past the first `data` chunk.
    pub fn runs<R: Read + Seek>(&self, input: R, file_len: u64) -> std::io::Result<Vec<DataRun>> {
        runs(
            self.container,
            self.data_offset,
            self.declared_len,
            self.riff_end,
            input,
            file_len,
        )
    }

    /// Bytes of sample data of the first `data` chunk actually present in a file of `file_len` bytes.
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
    let chunks = scan(bytes)?;
    let fmt = parse_fmt(chunks.fmt)?;
    let declared_rate = fmt.sample_rate as f64;
    Ok(WavLayout {
        sample_type: fmt.sample_type()?,
        sample_rate: chunks.metadata.sample_rate_for(declared_rate),
        data_offset: chunks.data_offset,
        declared_len: chunks.declared_len,
        container: chunks.container,
        reference_freq: chunks.metadata.reference_freq(),
        riff_end: chunks.riff_end,
    })
}

/// One stretch of sample bytes, the body of a `data` chunk or what the file holds of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DataRun {
    pub offset: u64,
    pub len: u64,
}

/// The first `data` chunk and, in a plain RIFF, every later one, clipped to the file.
fn runs<R: Read + Seek>(
    container: &str,
    data_offset: usize,
    declared_len: Option<usize>,
    riff_end: Option<u64>,
    input: R,
    file_len: u64,
) -> std::io::Result<Vec<DataRun>> {
    let offset = data_offset as u64;
    let available = file_len.saturating_sub(offset);
    let first = DataRun {
        offset,
        len: declared_len.map_or(available, |len| (len as u64).min(available)),
    };
    let end = riff_end.unwrap_or(file_len).min(file_len);
    let Some(declared) = declared_len.filter(|_| container == "wav") else {
        return Ok(vec![first]);
    };
    let declared = declared as u64;
    let next = offset + declared + (declared & 1);
    if next >= end {
        return Ok(vec![first]);
    }
    let mut out = vec![first];
    out.extend(later_data(input, next, end)?);
    Ok(out)
}

/// The `data` chunks from `pos` to `end`, stopping at a chunk that runs past `end`.
fn later_data<R: Read + Seek>(input: R, mut pos: u64, end: u64) -> std::io::Result<Vec<DataRun>> {
    let mut input = BufReader::with_capacity(64 << 10, input);
    input.seek(SeekFrom::Start(pos))?;
    let mut runs = Vec::new();
    let mut header = [0u8; 8];
    while pos + 8 <= end {
        input.read_exact(&mut header)?;
        let size = u64::from(u32::from_le_bytes([
            header[4], header[5], header[6], header[7],
        ]));
        let body = pos + 8;
        let fits = size <= end - body;
        if &header[0..4] == b"data" {
            let len = size.min(end - body);
            if len > 0 {
                runs.push(DataRun { offset: body, len });
            }
        }
        if !fits {
            break;
        }
        let step = size + (size & 1);
        input.seek_relative(step as i64)?;
        pos = body + step;
    }
    Ok(runs)
}

/// The chunks a WAVE file is read and copied by, whatever its sample layout.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Chunks<'a> {
    pub container: &'static str,
    /// Body of `fmt `, exactly as stored.
    pub fmt: &'a [u8],
    pub data_offset: usize,
    pub declared_len: Option<usize>,
    pub riff_end: Option<u64>,
    pub metadata: Metadata,
}

impl Chunks<'_> {
    /// Every run of sample bytes in the file, read from `input` past the first `data` chunk.
    pub fn runs<R: Read + Seek>(&self, input: R, file_len: u64) -> std::io::Result<Vec<DataRun>> {
        runs(
            self.container,
            self.data_offset,
            self.declared_len,
            self.riff_end,
            input,
            file_len,
        )
    }
}

/// What the `argd` and `auxi` chunks say about the capture.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct Metadata {
    /// Reference frequency and exact sample rate from `argd`.
    pub argd: Option<(f64, f64)>,
    /// Centre frequency from `auxi`, in whole hertz.
    pub auxi: Option<u32>,
}

impl Metadata {
    /// The physical frequency of baseband 0 Hz, preferring the exact value.
    pub fn reference_freq(&self) -> Option<f64> {
        self.argd
            .map(|(freq, _)| freq)
            .or_else(|| self.auxi.map(f64::from))
    }

    /// The exact rate from `argd` while it still agrees with the `fmt ` rate.
    pub fn sample_rate_for(&self, declared: f64) -> f64 {
        match self.argd {
            Some((_, exact)) if (exact - declared).abs() < 1.0 => exact,
            _ => declared,
        }
    }
}

/// Walk the chunk list up to `data` without interpreting `fmt `.
pub(crate) fn scan(bytes: &[u8]) -> Result<Chunks<'_>, RiffError> {
    if !is_wave(bytes) {
        return Err(RiffError::NotWave);
    }
    let container = container_of(bytes).ok_or(RiffError::NotWave)?;
    let is_64bit = container != "wav";

    let mut fmt: Option<&[u8]> = None;
    let mut data: Option<(usize, Option<usize>)> = None;
    let mut ds64_data_len: Option<usize> = None;
    let mut metadata = Metadata::default();
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
                fmt = Some(slice(bytes, body, size, "fmt chunk")?);
            }
            b"ds64" => {
                let body_bytes = slice(bytes, body, size, "ds64 chunk")?;
                ds64_data_len = Some(parse_ds64(body_bytes)?);
            }
            b"argd" => {
                if let Ok(body_bytes) = slice(bytes, body, size, "argd chunk") {
                    metadata.argd = parse_argd(body_bytes);
                }
            }
            b"auxi" => {
                if let Ok(body_bytes) = slice(bytes, body, size, "auxi chunk") {
                    metadata.auxi = parse_auxi(body_bytes);
                }
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

    let riff_size = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
    let riff_end = (!is_64bit && riff_size != 0 && riff_size != SIZE_UNKNOWN)
        .then(|| u64::from(riff_size) + 8);

    Ok(Chunks {
        container,
        fmt,
        data_offset,
        declared_len,
        riff_end,
        metadata,
    })
}

/// Argand's own chunk: version, reference frequency and sample rate.
pub(crate) const ARGD_ID: &[u8; 4] = b"argd";
pub(crate) const ARGD_VERSION: u32 = 1;
pub(crate) const ARGD_LEN: usize = 20;

/// Size of the `auxi` body SDR#, HDSDR and SDRuno write.
pub(crate) const AUXI_LEN: usize = 164;
/// Offset of `CenterFreq`, after the start and stop `SYSTEMTIME`s.
pub(crate) const AUXI_CENTER: usize = 32;
/// Offset of `ADFrequency`, the sample rate.
pub(crate) const AUXI_RATE: usize = 36;

fn parse_argd(body: &[u8]) -> Option<(f64, f64)> {
    if body.len() < ARGD_LEN {
        return None;
    }
    let version = u32::from_le_bytes(body[0..4].try_into().ok()?);
    let freq = f64::from_le_bytes(body[4..12].try_into().ok()?);
    let rate = f64::from_le_bytes(body[12..20].try_into().ok()?);
    (version == ARGD_VERSION && freq.is_finite() && rate.is_finite() && rate > 0.0)
        .then_some((freq, rate))
}

fn parse_auxi(body: &[u8]) -> Option<u32> {
    let field = body.get(AUXI_CENTER..AUXI_CENTER + 4)?;
    let freq = u32::from_le_bytes(field.try_into().ok()?);
    (freq > 0).then_some(freq)
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
pub(crate) struct FmtChunk {
    pub format_tag: u16,
    pub channels: u16,
    pub sample_rate: u32,
    pub block_align: u16,
    pub bits: u16,
    /// First extension word, when `fmt ` is exactly 20 bytes.
    ext_word: Option<u32>,
    /// Sub-format tag from a WAVE_FORMAT_EXTENSIBLE GUID.
    sub_format: Option<u16>,
}

pub(crate) fn parse_fmt(body: &[u8]) -> Result<FmtChunk, RiffError> {
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
        block_align: u16_at(12),
        bits: u16_at(14),
        ext_word: (body.len() == 20).then(|| u32_at(16)),
        // Extensible layout: cbSize, validBits, channelMask, then a 16-byte
        // GUID whose first two bytes are the real format tag.
        sub_format: (body.len() >= 40).then(|| u16_at(24)),
    })
}

impl FmtChunk {
    /// The format tag after looking inside `WAVE_FORMAT_EXTENSIBLE`.
    pub(crate) fn effective_tag(&self) -> u16 {
        if self.format_tag == WAVE_FORMAT_EXTENSIBLE {
            self.sub_format.unwrap_or(self.format_tag)
        } else {
            self.format_tag
        }
    }

    /// Whether this is the 16x8 layout of unscaled 32-bit floats.
    pub(crate) fn is_f16x8(&self) -> bool {
        self.ext_word == Some(F16X8_MAGIC)
    }

    /// Whether every sample is a fixed number of bytes, so a byte range is a span of samples.
    pub(crate) fn is_linear(&self) -> bool {
        let tag = if self.format_tag == WAVE_FORMAT_EXTENSIBLE {
            self.sub_format
        } else {
            Some(self.format_tag)
        };
        let width = usize::from(self.bits).div_ceil(8);
        matches!(tag, Some(WAVE_FORMAT_PCM | WAVE_FORMAT_IEEE_FLOAT))
            && matches!(self.channels, 1 | 2)
            && width > 0
            && usize::from(self.block_align) == usize::from(self.channels) * width
    }

    pub(crate) fn sample_type(&self) -> Result<SampleType, RiffError> {
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
