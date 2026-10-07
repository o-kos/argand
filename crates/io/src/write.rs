//! Writing a capture, or a span of it, to a new file.
//!
//! The output keeps the source's own format. A WAVE or headerless source is
//! copied byte for byte into WAVE, so the stored values come out exactly as
//! they went in; FLAC is decoded and encoded again without loss. Nothing is
//! ever written over the source, and the target is replaced only once the new
//! file is complete: everything goes to a temporary file beside it first.

use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use argand_core::{SampleFormat, SampleSource, SampleSpan, SignalMeta, SourceError};
use flacenc::bitsink::ByteSink;
use flacenc::component::{BitRepr, StreamInfo};
use flacenc::error::Verify;
use flacenc::source::{Context, Fill, FrameBuf};

use crate::decoder::{DecodedSource, RATE_TAG, REFERENCE_TAG};
use crate::riff::{self, ARGD_ID, ARGD_LEN, ARGD_VERSION, AUXI_CENTER, AUXI_LEN, AUXI_RATE};
use crate::{OpenHints, RiffError};

/// Bytes moved per read and write, rounded down to whole samples.
const COPY_BYTES: usize = 4 << 20;

/// Largest RIFF size a 32-bit header can state, past which RF64 takes over.
const RIFF_LIMIT: u64 = u32::MAX as u64;

/// One file samples are saved from, as it was opened.
#[derive(Debug, Clone)]
pub struct SourceFile {
    /// The capture as it was resolved, including hints already applied.
    pub meta: SignalMeta,
    /// The hints it was opened with, which say where a headerless file's samples start.
    pub hints: OpenHints,
    /// The file as it was when it was opened, which it must still be.
    pub stamp: Option<SourceStamp>,
}

/// `len` samples of source `source` of a request, from its sample `start`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Segment {
    pub source: usize,
    pub start: u64,
    pub len: u64,
}

/// What to save and where.
///
/// The segments are written in order. Every source must store its samples the
/// same way ([`storage`]) at the rate `meta` states, and the first sets the
/// output format, while `meta` sets the rate and reference frequency written.
#[derive(Debug, Clone)]
pub struct SaveRequest {
    /// What the saved file states about itself, its sample rate and reference frequency.
    pub meta: SignalMeta,
    pub sources: Vec<SourceFile>,
    pub segments: Vec<Segment>,
    pub target: PathBuf,
    /// Files that must not be written over beyond the sources, such as the open file.
    pub protected: Vec<Protected>,
}

impl SaveRequest {
    /// Save `span` of one file, or the whole of it.
    pub fn span(source: SourceFile, span: Option<SampleSpan>, target: PathBuf) -> Self {
        let segment = match span {
            Some(span) => Segment {
                source: 0,
                start: span.start(),
                len: span.count(),
            },
            None => Segment {
                source: 0,
                start: 0,
                len: source.meta.len_samples,
            },
        };
        Self {
            meta: source.meta.clone(),
            sources: vec![source],
            segments: vec![segment],
            target,
            protected: Vec::new(),
        }
    }

    /// Join segments of several files, describing the result as the first one, or nothing without a file.
    pub fn joined(
        sources: Vec<SourceFile>,
        segments: Vec<Segment>,
        target: PathBuf,
    ) -> Option<Self> {
        let meta = sources.first()?.meta.clone();
        Some(Self {
            meta,
            sources,
            segments,
            target,
            protected: Vec::new(),
        })
    }

    fn total(&self) -> u64 {
        self.segments.iter().map(|segment| segment.len).sum()
    }
}

/// A file a save must not write over, known by its path and, when taken, its stamp.
#[derive(Debug, Clone)]
pub struct Protected {
    pub path: PathBuf,
    /// Recognises the file under another name, after a rename.
    pub stamp: Option<SourceStamp>,
}

/// How a source stores its samples, which decides whether two can share one file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Storage {
    /// Fixed-width samples, described by their `fmt ` tag, depth, channels and block size.
    Linear {
        format_tag: u16,
        bits: u16,
        channels: u16,
        block: u16,
        unscaled_float: bool,
    },
    /// FLAC at this bit depth and channel count.
    Flac { bits: u32, channels: usize },
}

/// How `source` stores its samples, read from the file as it is now.
pub fn storage(source: &SourceFile) -> Result<Storage, WriteError> {
    Ok(Opened::open(source)?.reader.storage())
}

/// What tells one version of a file from another without reading it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceStamp {
    len: u64,
    modified: Option<std::time::SystemTime>,
    identity: Option<(u64, u64)>,
}

impl SourceStamp {
    /// The stamp of the file now at `path`.
    pub fn of(path: &Path) -> std::io::Result<Self> {
        fs::metadata(path).map(|metadata| Self::from_metadata(&metadata))
    }

    fn of_file(file: &File) -> std::io::Result<Self> {
        file.metadata()
            .map(|metadata| Self::from_metadata(&metadata))
    }

    fn from_metadata(metadata: &fs::Metadata) -> Self {
        Self {
            len: metadata.len(),
            modified: metadata.modified().ok(),
            identity: identity(metadata),
        }
    }
}

/// Device and inode where the platform has them.
fn identity(metadata: &fs::Metadata) -> Option<(u64, u64)> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        Some((metadata.dev(), metadata.ino()))
    }
    #[cfg(not(unix))]
    {
        let _ = metadata;
        None
    }
}

/// A file written to completion.
#[derive(Debug, Clone, PartialEq)]
pub struct Saved {
    pub path: PathBuf,
    pub samples: u64,
    /// Container written, "wav", "rf64" or "flac".
    pub container: &'static str,
}

#[derive(Debug, thiserror::Error)]
pub enum WriteError {
    #[error("{path} is the open file and cannot be saved over")]
    SameFile { path: PathBuf },
    #[error("{path} is not a file name")]
    BadTarget { path: PathBuf },
    #[error("the samples to save lie outside the capture")]
    OutOfRange,
    #[error("cannot read {path}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("cannot read {path}")]
    Source {
        path: PathBuf,
        #[source]
        source: SourceError,
    },
    #[error("cannot read {path} as wav")]
    Wav {
        path: PathBuf,
        #[source]
        source: RiffError,
    },
    #[error("{path} changed while it was being saved")]
    SourceChanged { path: PathBuf },
    #[error("cannot save {path}: {reason}")]
    Unsupported { path: PathBuf, reason: String },
    #[error("cannot write {path}")]
    Write {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("cannot encode flac: {0}")]
    Flac(String),
    #[error("saving was cancelled")]
    Cancelled,
}

/// Save `request`, reporting `(samples done, samples total)` and stopping when `cancel` is set.
///
/// On any error, cancellation included, the temporary file is removed and an
/// existing target is left exactly as it was.
pub fn save(
    request: &SaveRequest,
    progress: &mut dyn FnMut(u64, u64),
    cancel: &AtomicBool,
) -> Result<Saved, WriteError> {
    save_with_limit(request, progress, cancel, RIFF_LIMIT)
}

pub(crate) fn save_with_limit(
    request: &SaveRequest,
    progress: &mut dyn FnMut(u64, u64),
    cancel: &AtomicBool,
    riff_limit: u64,
) -> Result<Saved, WriteError> {
    let target = &request.target;
    check_request(request)?;
    let mut opened = request
        .sources
        .iter()
        .map(Opened::open)
        .collect::<Result<Vec<_>, _>>()?;
    let first = common_storage(request, &opened)?;

    let meta = &request.meta;
    let total = request.total();
    let mut partial = Partial::create(target)?;
    let container = match first {
        Storage::Linear { .. } => {
            let mut copies = Vec::new();
            for open in &mut opened {
                if let Reader::Copy { input, layout } = &mut open.reader {
                    copies.push((input, &*layout));
                }
            }
            write_wave(
                &mut partial,
                request,
                &mut copies,
                riff_limit,
                progress,
                cancel,
            )?
        }
        Storage::Flac { bits, .. } => {
            let mut exacts: Vec<&mut Exact> = opened
                .iter_mut()
                .filter_map(|open| match &mut open.reader {
                    Reader::Decode(exact) => Some(exact),
                    Reader::Copy { .. } => None,
                })
                .collect();
            let mut decoding = Decoding {
                exacts: &mut exacts,
                request,
                bits,
            };
            match flac_header_rate(meta.sample_rate) {
                Some(rate) => {
                    write_flac(&mut partial, &mut decoding, rate, total, progress, cancel)?;
                    "flac"
                }
                None if bits % 8 == 0 => write_decoded_wave(
                    &mut partial,
                    &mut decoding,
                    total,
                    riff_limit,
                    progress,
                    cancel,
                )?,
                None => {
                    return Err(WriteError::Unsupported {
                        path: meta.source.clone(),
                        reason: format!(
                            "{bits} bit flac at {} Hz can be written neither as flac nor as wav of the same depth",
                            meta.sample_rate
                        ),
                    });
                }
            }
        }
    };
    // A target may have become a file being read or protected since the start, by a rename.
    if protected_target(request).is_some()
        || fs::metadata(target).is_ok_and(|now| {
            identity(&now).is_some() && opened.iter().any(|open| open.identity == identity(&now))
        })
    {
        return Err(WriteError::SameFile {
            path: target.clone(),
        });
    }
    partial.finish(target, cancel)?;
    Ok(Saved {
        path: target.clone(),
        samples: total,
        container,
    })
}

/// The protected file the target now is, recognised by its stamp's identity.
fn protected_target(request: &SaveRequest) -> Option<&Protected> {
    let now = identity(&fs::metadata(&request.target).ok()?)?;
    request.protected.iter().find(|protected| {
        protected
            .stamp
            .is_some_and(|stamp| stamp.identity == Some(now))
    })
}

/// Refuse a request that would write over one of its sources or read past one.
fn check_request(request: &SaveRequest) -> Result<(), WriteError> {
    if let Some(path) = request
        .sources
        .iter()
        .map(|source| &source.meta.source)
        .chain(request.protected.iter().map(|protected| &protected.path))
        .find(|path| same_file(path, &request.target))
    {
        return Err(WriteError::SameFile { path: path.clone() });
    }
    if let Some(protected) = protected_target(request) {
        return Err(WriteError::SameFile {
            path: protected.path.clone(),
        });
    }
    let fits = |segment: &Segment| {
        request.sources.get(segment.source).is_some_and(|source| {
            segment.len > 0
                && segment
                    .start
                    .checked_add(segment.len)
                    .is_some_and(|end| end <= source.meta.len_samples)
        })
    };
    if request.segments.is_empty() || !request.segments.iter().all(fits) {
        return Err(WriteError::OutOfRange);
    }
    Ok(())
}

/// The storage every source shares, refusing one stored differently or at another rate.
fn common_storage(request: &SaveRequest, opened: &[Opened]) -> Result<Storage, WriteError> {
    let first = opened[0].reader.storage();
    let rate = request.meta.sample_rate;
    match request
        .sources
        .iter()
        .zip(opened)
        .find(|(source, open)| open.reader.storage() != first || source.meta.sample_rate != rate)
    {
        Some((odd, _)) => Err(WriteError::Unsupported {
            path: odd.meta.source.clone(),
            reason: "its samples are stored differently from the capture it is saved with".into(),
        }),
        None => Ok(first),
    }
}

/// A source opened for saving, checked to be the file that was opened as the capture.
struct Opened {
    reader: Reader,
    identity: Option<(u64, u64)>,
}

/// How one source's samples reach the new file.
enum Reader {
    /// Byte copy through the handle the header was read from.
    Copy { input: File, layout: CopyLayout },
    /// Exact integers from a strict decoder.
    Decode(Exact),
}

/// A strict decoder whose values go back to integers of `bits`.
struct Exact {
    decoder: DecodedSource,
    bits: u32,
}

/// Where the source's samples lie and the `fmt ` body that describes them.
struct CopyLayout {
    fmt: Vec<u8>,
    data_offset: u64,
    /// Bytes per sample, which is per I/Q pair for a complex capture.
    block: u64,
}

/// Bytes read to identify a container and parse its header.
const HEAD_BYTES: u64 = 64 << 10;

impl Opened {
    fn open(source: &SourceFile) -> Result<Self, WriteError> {
        let meta = &source.meta;
        let path = &meta.source;
        let read_error = |error| WriteError::Read {
            path: path.clone(),
            source: error,
        };
        let mut input = File::open(path).map_err(read_error)?;
        let stamp = SourceStamp::of_file(&input).map_err(read_error)?;
        if source.stamp.is_some_and(|expected| expected != stamp) {
            return Err(WriteError::SourceChanged { path: path.clone() });
        }
        let reader = Reader::open(source, &mut input)?;
        let reader = match reader {
            Some(layout) => Reader::Copy { input, layout },
            None => Reader::Decode(Exact::open(meta)?),
        };
        Ok(Self {
            reader,
            identity: stamp.identity,
        })
    }
}

impl Reader {
    /// The copy layout of a linear source, or nothing for one that must be decoded.
    fn open(source: &SourceFile, input: &mut File) -> Result<Option<CopyLayout>, WriteError> {
        let meta = &source.meta;
        let path = &meta.source;
        let read_error = |error| WriteError::Read {
            path: path.clone(),
            source: error,
        };
        let file_len = input.metadata().map_err(read_error)?.len();

        if source.hints.raw.is_some() {
            let layout = CopyLayout {
                fmt: synthesized_fmt(meta)?,
                data_offset: source.hints.byte_offset,
                block: meta.sample_type.bytes_per_sample() as u64,
            };
            let available = file_len.saturating_sub(layout.data_offset);
            layout.check_length(meta, available)?;
            return Ok(Some(layout));
        }

        let mut head = Vec::new();
        Read::by_ref(input)
            .take(HEAD_BYTES)
            .read_to_end(&mut head)
            .map_err(read_error)?;

        if riff::is_wave(&head) {
            let wav = |error| WriteError::Wav {
                path: path.clone(),
                source: error,
            };
            let chunks = riff::scan(&head).map_err(wav)?;
            let layout = CopyLayout::for_wave(source, &head, &chunks)?;
            let available = file_len.saturating_sub(layout.data_offset);
            let declared = chunks.declared_len.map_or(available, |len| len as u64);
            layout.check_length(meta, declared.min(available))?;
            return Ok(Some(layout));
        }

        if riff::is_flac(&head) {
            return Ok(None);
        }

        Err(WriteError::Unsupported {
            path: path.clone(),
            reason: "unrecognised container".into(),
        })
    }

    fn storage(&self) -> Storage {
        match self {
            Reader::Copy { layout, .. } => layout.storage(),
            Reader::Decode(exact) => Storage::Flac {
                bits: exact.bits,
                channels: exact.decoder.meta().channels(),
            },
        }
    }
}

impl CopyLayout {
    /// The layout of a WAVE source, keeping its own `fmt ` unless a hint reinterprets its bytes.
    fn for_wave(
        source: &SourceFile,
        head: &[u8],
        chunks: &riff::Chunks<'_>,
    ) -> Result<Self, WriteError> {
        let meta = &source.meta;
        let path = &meta.source;
        let data_offset = chunks.data_offset as u64;
        let wav = |source| WriteError::Wav {
            path: path.clone(),
            source,
        };
        // Only the native reader honours a sample type hint, the decoder reads the stored layout.
        let native = !matches!(riff::parse(head), Err(RiffError::Unsupported { .. }));
        if native && source.hints.sample_type.is_some() {
            return Ok(Self {
                fmt: synthesized_fmt(meta)?,
                data_offset,
                block: meta.sample_type.bytes_per_sample() as u64,
            });
        }
        let fmt = riff::parse_fmt(chunks.fmt).map_err(wav)?;
        if !fmt.is_linear() {
            return Err(WriteError::Unsupported {
                path: path.clone(),
                reason: format!(
                    "wav format tag {} with {} bit samples is not a plain sample array",
                    fmt.format_tag, fmt.bits
                ),
            });
        }
        let rate = wave_rate(meta)?;
        let mut body = chunks.fmt.to_vec();
        body[4..8].copy_from_slice(&rate.to_le_bytes());
        body[8..12].copy_from_slice(&byte_rate(meta, rate, fmt.block_align)?.to_le_bytes());
        Ok(Self {
            fmt: body,
            data_offset,
            block: u64::from(fmt.block_align),
        })
    }

    fn storage(&self) -> Storage {
        let fmt = riff::parse_fmt(&self.fmt);
        Storage::Linear {
            format_tag: fmt.as_ref().map_or(0, riff::FmtChunk::effective_tag),
            bits: fmt.as_ref().map_or(0, |fmt| fmt.bits),
            channels: fmt.as_ref().map_or(0, |fmt| fmt.channels),
            block: self.block as u16,
            unscaled_float: fmt.as_ref().is_ok_and(riff::FmtChunk::is_f16x8),
        }
    }

    /// Refuse a source whose sample data no longer matches the capture that was opened.
    fn check_length(&self, meta: &SignalMeta, data_len: u64) -> Result<(), WriteError> {
        if data_len / self.block == meta.len_samples {
            Ok(())
        } else {
            Err(WriteError::SourceChanged {
                path: meta.source.clone(),
            })
        }
    }
}

/// The capture's rate as a WAVE `fmt ` can state it.
fn wave_rate(meta: &SignalMeta) -> Result<u32, WriteError> {
    let rate = meta.sample_rate.round();
    if rate >= 1.0 && rate <= f64::from(u32::MAX) {
        Ok(rate as u32)
    } else {
        Err(WriteError::Unsupported {
            path: meta.source.clone(),
            reason: format!(
                "a sample rate of {} Hz does not fit a wav header",
                meta.sample_rate
            ),
        })
    }
}

/// `nAvgBytesPerSec`, refused when the header cannot hold it.
fn byte_rate(meta: &SignalMeta, rate: u32, block_align: u16) -> Result<u32, WriteError> {
    rate.checked_mul(u32::from(block_align))
        .ok_or_else(|| WriteError::Unsupported {
            path: meta.source.clone(),
            reason: format!(
                "a sample rate of {} Hz is too high for a wav header at this sample size",
                meta.sample_rate
            ),
        })
}

/// A `fmt ` body for the capture's effective sample type.
fn synthesized_fmt(meta: &SignalMeta) -> Result<Vec<u8>, WriteError> {
    let rate = wave_rate(meta)?;
    let format = meta.sample_type.format;
    let channels = meta.channels() as u16;
    let bits = (format.bytes() * 8) as u16;
    let block_align = channels * bits / 8;
    let tag: u16 = match format {
        SampleFormat::F32 => 3,
        _ => 1,
    };
    let mut body = Vec::with_capacity(20);
    body.extend_from_slice(&tag.to_le_bytes());
    body.extend_from_slice(&channels.to_le_bytes());
    body.extend_from_slice(&rate.to_le_bytes());
    body.extend_from_slice(&byte_rate(meta, rate, block_align)?.to_le_bytes());
    body.extend_from_slice(&block_align.to_le_bytes());
    body.extend_from_slice(&bits.to_le_bytes());
    if format == SampleFormat::F16x8 {
        body.extend_from_slice(&riff::F16X8_MAGIC.to_le_bytes());
    }
    Ok(body)
}

/// Whether a capture is written as WAVE whatever its own container, which decides the file's extension.
pub fn writes_as_wave(meta: &SignalMeta, hints: &OpenHints) -> bool {
    hints.raw.is_some()
        || (meta.container == "flac" && flac_header_rate(meta.sample_rate).is_none())
}

fn write_wave(
    partial: &mut Partial,
    request: &SaveRequest,
    copies: &mut [(&mut File, &CopyLayout)],
    riff_limit: u64,
    progress: &mut dyn FnMut(u64, u64),
    cancel: &AtomicBool,
) -> Result<&'static str, WriteError> {
    let total = request.total();
    let block = copies[0].1.block;
    let data_len = total.checked_mul(block).ok_or(WriteError::OutOfRange)?;
    let meta = &request.meta;
    let container =
        write_wave_header(partial, meta, &copies[0].1.fmt, data_len, total, riff_limit)?;
    let mut done = 0;
    progress(0, total);
    for segment in &request.segments {
        let (input, layout) = &mut copies[segment.source];
        let path = &request.sources[segment.source].meta.source;
        copy_samples(
            partial,
            input,
            layout,
            path,
            *segment,
            &mut |copied| {
                progress(done + copied, total);
            },
            cancel,
        )?;
        done += segment.len;
    }
    if data_len % 2 == 1 {
        partial.write(&[0])?;
    }
    Ok(container)
}

/// Write the chunks before the samples, choosing RF64 when RIFF cannot state the size.
fn write_wave_header(
    partial: &mut Partial,
    meta: &SignalMeta,
    fmt: &[u8],
    data_len: u64,
    samples: u64,
    riff_limit: u64,
) -> Result<&'static str, WriteError> {
    let mut chunks: Vec<(&[u8; 4], Vec<u8>)> = vec![(b"fmt ", fmt.to_vec())];
    if let Some(auxi) = auxi_body(meta) {
        chunks.push((b"auxi", auxi));
    }
    chunks.push((ARGD_ID, argd_body(meta)));

    let chunk_bytes: u64 = chunks
        .iter()
        .map(|(_, body)| 8 + padded(body.len() as u64))
        .sum();
    let data_bytes = 8 + padded(data_len);
    let plain_size = 4 + chunk_bytes + data_bytes;
    let rf64 = plain_size > riff_limit;
    // Only the native reader understands RF64, so a layout it declines could never be opened again.
    let native = riff::parse_fmt(fmt).is_ok_and(|fmt| fmt.sample_type().is_ok());
    if rf64 && !native {
        return Err(WriteError::Unsupported {
            path: meta.source.clone(),
            reason: "a wav this large in this sample format could not be opened again".into(),
        });
    }

    let mut header = Vec::new();
    if rf64 {
        let riff_size = plain_size + 8 + 28;
        header.extend_from_slice(b"RF64");
        header.extend_from_slice(&u32::MAX.to_le_bytes());
        header.extend_from_slice(b"WAVE");
        header.extend_from_slice(b"ds64");
        header.extend_from_slice(&28u32.to_le_bytes());
        header.extend_from_slice(&riff_size.to_le_bytes());
        header.extend_from_slice(&data_len.to_le_bytes());
        header.extend_from_slice(&samples.to_le_bytes());
        header.extend_from_slice(&0u32.to_le_bytes());
    } else {
        header.extend_from_slice(b"RIFF");
        header.extend_from_slice(&(plain_size as u32).to_le_bytes());
        header.extend_from_slice(b"WAVE");
    }
    for (id, body) in &chunks {
        header.extend_from_slice(*id);
        header.extend_from_slice(&(body.len() as u32).to_le_bytes());
        header.extend_from_slice(body);
        if body.len() % 2 == 1 {
            header.push(0);
        }
    }
    header.extend_from_slice(b"data");
    let data_field = if rf64 { u32::MAX } else { data_len as u32 };
    header.extend_from_slice(&data_field.to_le_bytes());
    partial.write(&header)?;
    Ok(if rf64 { "rf64" } else { "wav" })
}

fn padded(len: u64) -> u64 {
    len + (len & 1)
}

/// The `auxi` body SDR software reads, written only when the frequency fits its field.
fn auxi_body(meta: &SignalMeta) -> Option<Vec<u8>> {
    let freq = meta.center_freq.round();
    if !(freq >= 1.0 && freq <= f64::from(u32::MAX)) {
        return None;
    }
    let rate = meta.sample_rate.round().clamp(0.0, f64::from(u32::MAX)) as u32;
    let mut body = vec![0u8; AUXI_LEN];
    body[AUXI_CENTER..AUXI_CENTER + 4].copy_from_slice(&(freq as u32).to_le_bytes());
    body[AUXI_RATE..AUXI_RATE + 4].copy_from_slice(&rate.to_le_bytes());
    Some(body)
}

fn argd_body(meta: &SignalMeta) -> Vec<u8> {
    let mut body = Vec::with_capacity(ARGD_LEN);
    body.extend_from_slice(&ARGD_VERSION.to_le_bytes());
    body.extend_from_slice(&meta.center_freq.to_le_bytes());
    body.extend_from_slice(&meta.sample_rate.to_le_bytes());
    body
}

fn copy_samples(
    partial: &mut Partial,
    input: &mut File,
    layout: &CopyLayout,
    path: &Path,
    segment: Segment,
    copied: &mut dyn FnMut(u64),
    cancel: &AtomicBool,
) -> Result<(), WriteError> {
    let read_error = |error| WriteError::Read {
        path: path.to_owned(),
        source: error,
    };
    let block = layout.block;
    let start = segment
        .start
        .checked_mul(block)
        .and_then(|bytes| bytes.checked_add(layout.data_offset))
        .ok_or(WriteError::OutOfRange)?;
    input.seek(SeekFrom::Start(start)).map_err(read_error)?;

    let per_chunk = (COPY_BYTES as u64 / block).max(1);
    let mut buf = vec![0u8; (per_chunk.min(segment.len) * block) as usize];
    let mut done = 0u64;
    while done < segment.len {
        if cancel.load(Ordering::Relaxed) {
            return Err(WriteError::Cancelled);
        }
        let samples = per_chunk.min(segment.len - done);
        let bytes = &mut buf[..(samples * block) as usize];
        input.read_exact(bytes).map_err(|error| {
            if error.kind() == std::io::ErrorKind::UnexpectedEof {
                WriteError::SourceChanged {
                    path: path.to_owned(),
                }
            } else {
                read_error(error)
            }
        })?;
        partial.write(bytes)?;
        done += samples;
        copied(done);
    }
    Ok(())
}

impl Exact {
    /// Open the source again, refusing it if it is no longer the capture that was opened.
    fn open(meta: &SignalMeta) -> Result<Self, WriteError> {
        let path = &meta.source;
        let decoder = DecodedSource::open_exact(path, meta.container).map_err(|source| {
            WriteError::Source {
                path: path.clone(),
                source,
            }
        })?;
        let found = decoder.meta();
        if found.len_samples != meta.len_samples || found.channels() != meta.channels() {
            return Err(WriteError::SourceChanged { path: path.clone() });
        }
        match decoder.bits_per_sample() {
            Some(bits @ 4..=24) => Ok(Self { decoder, bits }),
            Some(bits) => Err(WriteError::Unsupported {
                path: path.clone(),
                reason: format!("{bits} bit flac cannot be re-encoded exactly"),
            }),
            None => Err(WriteError::Unsupported {
                path: path.clone(),
                reason: "the flac stream does not state its bit depth".into(),
            }),
        }
    }
}

/// The segments of a request read as exact integers through one strict decoder per source.
struct Decoding<'a, 'b> {
    exacts: &'a mut [&'b mut Exact],
    request: &'a SaveRequest,
    bits: u32,
}

impl Decoding<'_, '_> {
    /// Hand every segment to `sink` as interleaved integers, `block` samples at a time across segments.
    fn each_block(
        &mut self,
        block: usize,
        progress: &mut dyn FnMut(u64, u64),
        cancel: &AtomicBool,
        sink: &mut dyn FnMut(&[i32]) -> Result<(), WriteError>,
    ) -> Result<(), WriteError> {
        let channels = self.request.sources[0].meta.channels();
        let scale = f64::from(1u32 << (self.bits - 1));
        let mut values = vec![0.0f32; block * channels];
        let mut ints: Vec<i32> = Vec::with_capacity(block * channels);
        let total = self.request.total();
        let mut done = 0u64;
        progress(0, total);
        for segment in &self.request.segments {
            let path = &self.request.sources[segment.source].meta.source;
            let source_error = |error| WriteError::Source {
                path: path.clone(),
                source: error,
            };
            let decoder = &mut self.exacts[segment.source].decoder;
            decoder.seek(segment.start).map_err(source_error)?;
            let mut left = segment.len;
            while left > 0 {
                if cancel.load(Ordering::Relaxed) {
                    return Err(WriteError::Cancelled);
                }
                let room = block - ints.len() / channels;
                let want = (room as u64).min(left) as usize * channels;
                read_exactly(decoder, &mut values[..want], path)?;
                for value in &values[..want] {
                    ints.push(exact_integer(*value, scale, self.bits)?);
                }
                left -= (want / channels) as u64;
                done += (want / channels) as u64;
                if ints.len() == block * channels {
                    sink(&ints)?;
                    ints.clear();
                    progress(done, total);
                }
            }
        }
        if !ints.is_empty() {
            sink(&ints)?;
        }
        progress(done, total);
        Ok(())
    }
}

/// Fill `values` from `decoder`, refusing a stream that ends first.
fn read_exactly(
    decoder: &mut DecodedSource,
    values: &mut [f32],
    path: &Path,
) -> Result<(), WriteError> {
    let mut got = 0;
    while got < values.len() {
        let n = decoder
            .read_plain(&mut values[got..])
            .map_err(|error| WriteError::Source {
                path: path.to_owned(),
                source: error,
            })?;
        if n == 0 {
            return Err(WriteError::SourceChanged {
                path: path.to_owned(),
            });
        }
        got += n;
    }
    Ok(())
}

/// A decoded value as the integer it was stored as, refused when it is not one.
fn exact_integer(value: f32, scale: f64, bits: u32) -> Result<i32, WriteError> {
    let exact = f64::from(value) * scale;
    let rounded = exact.round();
    if (exact - rounded).abs() > 1e-3 {
        return Err(WriteError::Flac(format!(
            "decoded value {value} is not a {bits} bit integer"
        )));
    }
    Ok(rounded as i32)
}

fn write_flac(
    partial: &mut Partial,
    decoding: &mut Decoding<'_, '_>,
    rate: usize,
    total: u64,
    progress: &mut dyn FnMut(u64, u64),
    cancel: &AtomicBool,
) -> Result<(), WriteError> {
    let flac = |error: &dyn std::fmt::Display| WriteError::Flac(error.to_string());
    let meta = &decoding.request.meta;
    if total >= 1 << 36 {
        return Err(WriteError::Unsupported {
            path: meta.source.clone(),
            reason: format!("{total} samples do not fit a flac header"),
        });
    }
    let channels = meta.channels();
    let bits = decoding.bits as usize;
    let config = flacenc::config::Encoder::default()
        .into_verified()
        .map_err(|(_, error)| flac(&error))?;
    let block = config.block_size;
    let mut info = StreamInfo::new(rate, channels, bits).map_err(|error| flac(&error))?;

    let mut header = Vec::new();
    header.extend_from_slice(b"fLaC");
    header.extend_from_slice(&[0, 0, 0, 34]);
    let info_offset = header.len() as u64;
    header.extend_from_slice(&[0u8; 34]);
    let comment = vorbis_comment(meta.center_freq, meta.sample_rate);
    header.push(0x80 | 4);
    header.extend_from_slice(&(comment.len() as u32).to_be_bytes()[1..]);
    header.extend_from_slice(&comment);
    partial.write(&header)?;

    let mut fill = (
        FrameBuf::with_size(channels, block).map_err(|error| flac(&error))?,
        Context::new(bits, channels),
    );
    let mut sink = ByteSink::new();
    decoding.each_block(block, progress, cancel, &mut |ints| {
        fill.fill_interleaved(ints).map_err(|error| flac(&error))?;
        let number = fill
            .1
            .current_frame_number()
            .ok_or_else(|| WriteError::Flac("no frame was filled".into()))?;
        let frame = flacenc::encode_fixed_size_frame(&config, &fill.0, number, &info)
            .map_err(|error| flac(&error))?;
        info.update_frame_info(&frame);
        sink.clear();
        frame.write(&mut sink).map_err(|error| flac(&error))?;
        partial.write(sink.as_slice())
    })?;

    info.set_md5_digest(&fill.1.md5_digest());
    info.set_total_samples(total as usize);
    info.set_block_sizes(block, block)
        .map_err(|error| flac(&error))?;
    sink.clear();
    info.write(&mut sink).map_err(|error| flac(&error))?;
    partial.write_at(info_offset, sink.as_slice())
}

/// A FLAC source the encoder cannot state the rate of, written as WAVE of the same whole-byte depth.
fn write_decoded_wave(
    partial: &mut Partial,
    decoding: &mut Decoding<'_, '_>,
    total: u64,
    riff_limit: u64,
    progress: &mut dyn FnMut(u64, u64),
    cancel: &AtomicBool,
) -> Result<&'static str, WriteError> {
    let meta = &decoding.request.meta;
    let width = decoding.bits / 8;
    let channels = meta.channels() as u16;
    let block_align = channels * width as u16;
    let rate = wave_rate(meta)?;
    let mut fmt = Vec::with_capacity(16);
    fmt.extend_from_slice(&1u16.to_le_bytes());
    fmt.extend_from_slice(&channels.to_le_bytes());
    fmt.extend_from_slice(&rate.to_le_bytes());
    fmt.extend_from_slice(&byte_rate(meta, rate, block_align)?.to_le_bytes());
    fmt.extend_from_slice(&block_align.to_le_bytes());
    fmt.extend_from_slice(&((width * 8) as u16).to_le_bytes());

    let data_len = total
        .checked_mul(u64::from(block_align))
        .ok_or(WriteError::OutOfRange)?;
    let container = write_wave_header(partial, meta, &fmt, data_len, total, riff_limit)?;
    let mut bytes = Vec::new();
    decoding.each_block(COPY_BLOCK, progress, cancel, &mut |ints| {
        bytes.clear();
        for &value in ints {
            // Eight-bit WAVE is offset binary, wider samples are signed.
            if width == 1 {
                bytes.push((value + 128) as u8);
            } else {
                bytes.extend_from_slice(&value.to_le_bytes()[..width as usize]);
            }
        }
        partial.write(&bytes)
    })?;
    if data_len % 2 == 1 {
        partial.write(&[0])?;
    }
    Ok(container)
}

/// Samples decoded per block when a FLAC source is written as WAVE.
const COPY_BLOCK: usize = 65536;

/// The rate STREAMINFO and every frame header can state, within a hertz of the exact one.
///
/// The exact rate travels in the Vorbis comment, so the header only has to be close.
fn flac_header_rate(rate: f64) -> Option<usize> {
    [rate.round(), rate.floor(), rate.ceil()]
        .into_iter()
        .find(|candidate| (candidate - rate).abs() < 1.0 && flac_states(*candidate))
        .map(|candidate| candidate as usize)
}

/// Whether the encoder in use accepts `rate` and its frame headers can carry it.
fn flac_states(rate: f64) -> bool {
    (1.0..=96_000.0).contains(&rate) && (rate <= 65_535.0 || rate % 10.0 == 0.0)
}

/// A VORBIS_COMMENT body carrying the reference frequency and the exact sample rate.
fn vorbis_comment(freq: f64, rate: f64) -> Vec<u8> {
    let vendor = b"argand";
    let entries = [
        format!("{REFERENCE_TAG}={freq}"),
        format!("{RATE_TAG}={rate}"),
    ];
    let mut body = Vec::new();
    body.extend_from_slice(&(vendor.len() as u32).to_le_bytes());
    body.extend_from_slice(vendor);
    body.extend_from_slice(&(entries.len() as u32).to_le_bytes());
    for entry in &entries {
        body.extend_from_slice(&(entry.len() as u32).to_le_bytes());
        body.extend_from_slice(entry.as_bytes());
    }
    body
}

/// Whether two paths name the same file, including through links.
fn same_file(a: &Path, b: &Path) -> bool {
    #[cfg(unix)]
    if let (Ok(x), Ok(y)) = (fs::metadata(a), fs::metadata(b)) {
        use std::os::unix::fs::MetadataExt;
        return x.dev() == y.dev() && x.ino() == y.ino();
    }
    match (canonical(a), canonical(b)) {
        (Some(x), Some(y)) => x == y,
        _ => a == b,
    }
}

/// The canonical path, or the canonical parent plus the name for a file not yet there.
fn canonical(path: &Path) -> Option<PathBuf> {
    path.canonicalize().ok().or_else(|| {
        let parent = match path.parent() {
            Some(parent) if !parent.as_os_str().is_empty() => parent,
            _ => Path::new("."),
        };
        Some(parent.canonicalize().ok()?.join(path.file_name()?))
    })
}

/// How many temporary names are tried before giving up.
const TEMPORARY_ATTEMPTS: u32 = 100;

/// A temporary file beside the target, removed unless it is renamed into place.
struct Partial {
    path: PathBuf,
    file: Option<File>,
    renamed: bool,
}

impl Partial {
    fn create(target: &Path) -> Result<Self, WriteError> {
        if target.file_name().is_none() {
            return Err(WriteError::BadTarget {
                path: target.to_owned(),
            });
        }
        let mut attempt = 0u32;
        loop {
            // Short whatever the target is called, so a name at the length limit still has room.
            let path =
                target.with_file_name(format!(".argand-{}-{attempt}.part", std::process::id()));
            // A new file only, so an existing path, a link to the source among them, is never opened.
            match File::options().write(true).create_new(true).open(&path) {
                Ok(file) => {
                    return Ok(Self {
                        path,
                        file: Some(file),
                        renamed: false,
                    });
                }
                Err(error)
                    if error.kind() == std::io::ErrorKind::AlreadyExists
                        && attempt < TEMPORARY_ATTEMPTS =>
                {
                    attempt += 1;
                }
                Err(source) => return Err(WriteError::Write { path, source }),
            }
        }
    }

    fn write(&mut self, bytes: &[u8]) -> Result<(), WriteError> {
        let result = match self.file.as_mut() {
            Some(file) => file.write_all(bytes),
            None => Err(std::io::ErrorKind::BrokenPipe.into()),
        };
        result.map_err(|source| self.error(source))
    }

    fn write_at(&mut self, offset: u64, bytes: &[u8]) -> Result<(), WriteError> {
        let result = match self.file.as_mut() {
            Some(file) => file
                .seek(SeekFrom::Start(offset))
                .and_then(|_| file.write_all(bytes))
                .and_then(|_| file.seek(SeekFrom::End(0)).map(drop)),
            None => Err(std::io::ErrorKind::BrokenPipe.into()),
        };
        result.map_err(|source| self.error(source))
    }

    fn error(&self, source: std::io::Error) -> WriteError {
        WriteError::Write {
            path: self.path.clone(),
            source,
        }
    }

    /// Flush to disk, close and move the finished file over the target.
    ///
    /// Cancelling is possible until the rename, which is the point of no return.
    fn finish(mut self, target: &Path, cancel: &AtomicBool) -> Result<(), WriteError> {
        let synced = match self.file.take() {
            Some(file) => file.sync_all(),
            None => Err(std::io::ErrorKind::BrokenPipe.into()),
        };
        synced.map_err(|source| self.error(source))?;
        if cancel.load(Ordering::Relaxed) {
            return Err(WriteError::Cancelled);
        }
        fs::rename(&self.path, target).map_err(|source| WriteError::Write {
            path: target.to_owned(),
            source,
        })?;
        self.renamed = true;
        sync_directory(target);
        Ok(())
    }
}

impl Drop for Partial {
    fn drop(&mut self) {
        self.file = None;
        if !self.renamed
            && let Err(error) = fs::remove_file(&self.path)
        {
            tracing::warn!(path = %self.path.display(), %error, "temporary file left behind");
        }
    }
}

/// Make the rename itself durable where the platform allows it.
fn sync_directory(target: &Path) {
    #[cfg(unix)]
    {
        let parent = match target.parent() {
            Some(parent) if !parent.as_os_str().is_empty() => parent,
            _ => Path::new("."),
        };
        if let Err(error) = File::open(parent).and_then(|dir| dir.sync_all()) {
            tracing::warn!(path = %parent.display(), %error, "cannot sync the folder of a saved file");
        }
    }
    #[cfg(not(unix))]
    let _ = target;
}

#[cfg(test)]
mod tests {
    include!("write_tests.rs");
}
