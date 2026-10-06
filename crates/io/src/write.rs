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

/// What to save and where.
#[derive(Debug, Clone)]
pub struct SaveRequest {
    /// The open capture as it was resolved, including hints already applied.
    pub meta: SignalMeta,
    /// The hints it was opened with, which say where a headerless file's samples start.
    pub hints: OpenHints,
    /// The samples to keep, or the whole capture when absent.
    pub span: Option<SampleSpan>,
    pub target: PathBuf,
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
    let source = &request.meta.source;
    let target = &request.target;
    if same_file(source, target) {
        return Err(WriteError::SameFile {
            path: target.clone(),
        });
    }
    let total = request.meta.len_samples;
    let span = match request.span {
        Some(span) => span.within(total).filter(|kept| *kept == span),
        None => SampleSpan::between(0, total),
    }
    .ok_or(WriteError::OutOfRange)?;

    let mut input = File::open(source).map_err(|error| WriteError::Read {
        path: source.clone(),
        source: error,
    })?;
    let plan = Plan::for_request(request, &mut input)?;
    let mut partial = Partial::create(target)?;
    let container = match plan {
        Plan::Copy(layout) => {
            let copy = ByteRange {
                source,
                input: &mut input,
                layout,
                span,
            };
            write_wave(
                &mut partial,
                &request.meta,
                copy,
                riff_limit,
                progress,
                cancel,
            )?
        }
        Plan::Flac { bits } => {
            write_flac(&mut partial, bits, &request.meta, span, progress, cancel)?;
            "flac"
        }
    };
    partial.finish(target, cancel)?;
    Ok(Saved {
        path: target.clone(),
        samples: span.count(),
        container,
    })
}

/// How the source's samples reach the new file.
enum Plan {
    Copy(CopyLayout),
    /// Decode and encode again at this bit depth.
    Flac {
        bits: u32,
    },
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

impl Plan {
    fn for_request(request: &SaveRequest, input: &mut File) -> Result<Self, WriteError> {
        let meta = &request.meta;
        let path = &meta.source;
        let read_error = |source| WriteError::Read {
            path: path.clone(),
            source,
        };
        let file_len = input.metadata().map_err(read_error)?.len();

        if request.hints.raw.is_some() {
            let layout = CopyLayout {
                fmt: synthesized_fmt(meta)?,
                data_offset: request.hints.byte_offset,
                block: meta.sample_type.bytes_per_sample() as u64,
            };
            let available = file_len.saturating_sub(layout.data_offset);
            layout.check_length(meta, available)?;
            return Ok(Plan::Copy(layout));
        }

        let mut head = Vec::new();
        Read::by_ref(input)
            .take(HEAD_BYTES)
            .read_to_end(&mut head)
            .map_err(read_error)?;

        if riff::is_wave(&head) {
            let wav = |source| WriteError::Wav {
                path: path.clone(),
                source,
            };
            let chunks = riff::scan(&head).map_err(wav)?;
            let layout = CopyLayout::for_wave(request, &head, &chunks)?;
            let available = file_len.saturating_sub(layout.data_offset);
            let declared = chunks.declared_len.map_or(available, |len| len as u64);
            layout.check_length(meta, declared.min(available))?;
            return Ok(Plan::Copy(layout));
        }

        if riff::is_flac(&head) {
            return Ok(Plan::Flac {
                bits: flac_bits(meta)?,
            });
        }

        Err(WriteError::Unsupported {
            path: path.clone(),
            reason: "unrecognised container".into(),
        })
    }
}

impl CopyLayout {
    /// The layout of a WAVE source, keeping its own `fmt ` unless a hint reinterprets its bytes.
    fn for_wave(
        request: &SaveRequest,
        head: &[u8],
        chunks: &riff::Chunks<'_>,
    ) -> Result<Self, WriteError> {
        let meta = &request.meta;
        let path = &meta.source;
        let data_offset = chunks.data_offset as u64;
        let wav = |source| WriteError::Wav {
            path: path.clone(),
            source,
        };
        // Only the native reader honours a sample type hint, the decoder reads the stored layout.
        let native = !matches!(riff::parse(head), Err(RiffError::Unsupported { .. }));
        if native && request.hints.sample_type.is_some() {
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

/// A span of the source to copy, read through the handle its header was read from.
struct ByteRange<'a> {
    source: &'a Path,
    input: &'a mut File,
    layout: CopyLayout,
    span: SampleSpan,
}

fn write_wave(
    partial: &mut Partial,
    meta: &SignalMeta,
    mut copy: ByteRange<'_>,
    riff_limit: u64,
    progress: &mut dyn FnMut(u64, u64),
    cancel: &AtomicBool,
) -> Result<&'static str, WriteError> {
    let data_len = copy
        .span
        .count()
        .checked_mul(copy.layout.block)
        .ok_or(WriteError::OutOfRange)?;
    let mut chunks: Vec<(&[u8; 4], Vec<u8>)> = vec![(b"fmt ", copy.layout.fmt.clone())];
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
        header.extend_from_slice(&copy.span.count().to_le_bytes());
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

    copy_samples(partial, &mut copy, progress, cancel)?;
    if data_len % 2 == 1 {
        partial.write(&[0])?;
    }
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
    copy: &mut ByteRange<'_>,
    progress: &mut dyn FnMut(u64, u64),
    cancel: &AtomicBool,
) -> Result<(), WriteError> {
    let read_error = |source| WriteError::Read {
        path: copy.source.to_owned(),
        source,
    };
    let block = copy.layout.block;
    let start = copy
        .span
        .start()
        .checked_mul(block)
        .and_then(|bytes| bytes.checked_add(copy.layout.data_offset))
        .ok_or(WriteError::OutOfRange)?;
    copy.input
        .seek(SeekFrom::Start(start))
        .map_err(read_error)?;

    let per_chunk = (COPY_BYTES as u64 / block).max(1);
    let mut buf = vec![0u8; (per_chunk * block) as usize];
    let total = copy.span.count();
    let mut done = 0u64;
    progress(0, total);
    while done < total {
        if cancel.load(Ordering::Relaxed) {
            return Err(WriteError::Cancelled);
        }
        let samples = per_chunk.min(total - done);
        let bytes = &mut buf[..(samples * block) as usize];
        copy.input.read_exact(bytes).map_err(|source| {
            if source.kind() == std::io::ErrorKind::UnexpectedEof {
                WriteError::SourceChanged {
                    path: copy.source.to_owned(),
                }
            } else {
                read_error(source)
            }
        })?;
        partial.write(bytes)?;
        done += samples;
        progress(done, total);
    }
    Ok(())
}

/// The bit depth a FLAC source is encoded at again.
fn flac_bits(meta: &SignalMeta) -> Result<u32, WriteError> {
    let decoder = plain_decoder(meta)?;
    match decoder.bits_per_sample() {
        Some(bits @ 4..=24) => Ok(bits),
        other => Err(WriteError::Unsupported {
            path: meta.source.clone(),
            reason: match other {
                Some(bits) => format!("{bits} bit flac cannot be re-encoded exactly"),
                None => "the flac stream does not state its bit depth".into(),
            },
        }),
    }
}

/// A decoder that hands back codec values without normalization or gain.
fn plain_decoder(meta: &SignalMeta) -> Result<DecodedSource, WriteError> {
    let plain = SignalMeta {
        divisor: 1.0,
        ..meta.clone()
    };
    DecodedSource::reopen(&plain, 0.0)
        .map(DecodedSource::strict)
        .map_err(|source| WriteError::Source {
            path: meta.source.clone(),
            source,
        })
}

fn write_flac(
    partial: &mut Partial,
    bits: u32,
    meta: &SignalMeta,
    span: SampleSpan,
    progress: &mut dyn FnMut(u64, u64),
    cancel: &AtomicBool,
) -> Result<(), WriteError> {
    let flac = |error: &dyn std::fmt::Display| WriteError::Flac(error.to_string());
    let source_error = |source| WriteError::Source {
        path: meta.source.clone(),
        source,
    };
    let channels = meta.channels();
    let rate = flac_rate(meta, span)?;

    let config = flacenc::config::Encoder::default()
        .into_verified()
        .map_err(|(_, error)| flac(&error))?;
    let block = config.block_size;
    let mut info = StreamInfo::new(rate, channels, bits as usize).map_err(|error| flac(&error))?;

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

    let mut decoder = plain_decoder(meta)?;
    decoder.seek(span.start()).map_err(source_error)?;
    let mut fill = (
        FrameBuf::with_size(channels, block).map_err(|error| flac(&error))?,
        Context::new(bits as usize, channels),
    );
    let scale = f64::from(1u32 << (bits - 1));
    let mut values = vec![0.0f32; block * channels];
    let mut ints = vec![0i32; block * channels];
    let mut sink = ByteSink::new();
    let total = span.count();
    let mut done = 0u64;
    progress(0, total);
    while done < total {
        if cancel.load(Ordering::Relaxed) {
            return Err(WriteError::Cancelled);
        }
        let samples = (block as u64).min(total - done) as usize;
        let want = samples * channels;
        let mut got = 0;
        while got < want {
            let n = decoder
                .read_plain(&mut values[got..want])
                .map_err(source_error)?;
            if n == 0 {
                return Err(WriteError::SourceChanged {
                    path: meta.source.clone(),
                });
            }
            got += n;
        }
        for (int, value) in ints.iter_mut().zip(&values[..want]) {
            let exact = f64::from(*value) * scale;
            let rounded = exact.round();
            if (exact - rounded).abs() > 1e-3 {
                return Err(WriteError::Flac(format!(
                    "decoded value {value} is not a {bits} bit integer"
                )));
            }
            *int = rounded as i32;
        }
        fill.fill_interleaved(&ints[..want])
            .map_err(|error| flac(&error))?;
        let number = fill
            .1
            .current_frame_number()
            .ok_or_else(|| WriteError::Flac("no frame was filled".into()))?;
        let frame = flacenc::encode_fixed_size_frame(&config, &fill.0, number, &info)
            .map_err(|error| flac(&error))?;
        info.update_frame_info(&frame);
        sink.clear();
        frame.write(&mut sink).map_err(|error| flac(&error))?;
        partial.write(sink.as_slice())?;
        done += samples as u64;
        progress(done, total);
    }

    info.set_md5_digest(&fill.1.md5_digest());
    info.set_total_samples(total as usize);
    info.set_block_sizes(block, block)
        .map_err(|error| flac(&error))?;
    sink.clear();
    info.write(&mut sink).map_err(|error| flac(&error))?;
    partial.write_at(info_offset, sink.as_slice())
}

/// The rate STREAMINFO states, refusing captures its fields cannot describe.
fn flac_rate(meta: &SignalMeta, span: SampleSpan) -> Result<usize, WriteError> {
    if span.count() >= 1 << 36 {
        return Err(WriteError::Unsupported {
            path: meta.source.clone(),
            reason: format!("{} samples do not fit a flac header", span.count()),
        });
    }
    let rate = meta.sample_rate.round();
    if !(1.0..=f64::from((1u32 << 20) - 1)).contains(&rate) {
        return Err(WriteError::Unsupported {
            path: meta.source.clone(),
            reason: format!(
                "a sample rate of {} Hz does not fit a flac header",
                meta.sample_rate
            ),
        });
    }
    Ok(rate as usize)
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
        let name = target.file_name().ok_or_else(|| WriteError::BadTarget {
            path: target.to_owned(),
        })?;
        let mut attempt = 0u32;
        loop {
            let mut temporary = std::ffi::OsString::from(".");
            temporary.push(name);
            temporary.push(format!(".{}-{attempt}.part", std::process::id()));
            let path = target.with_file_name(temporary);
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
