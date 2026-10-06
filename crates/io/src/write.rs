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

use crate::decoder::{DecodedSource, REFERENCE_TAG};
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

    let plan = Plan::for_request(request)?;
    let mut partial = Partial::create(target)?;
    let container = match &plan {
        Plan::Copy {
            fmt,
            data_offset,
            block,
        } => {
            let copy = ByteRange {
                source,
                data_offset: *data_offset,
                block: *block,
                span,
            };
            write_wave(
                &mut partial,
                fmt,
                &request.meta,
                &copy,
                riff_limit,
                progress,
                cancel,
            )?
        }
        Plan::Flac { bits } => {
            write_flac(&mut partial, *bits, &request.meta, span, progress, cancel)?;
            "flac"
        }
    };
    partial.finish(target)?;
    Ok(Saved {
        path: target.clone(),
        samples: span.count(),
        container,
    })
}

/// How the source's samples reach the new file.
enum Plan {
    /// Copy `block` bytes per sample from `data_offset`, under this `fmt ` body.
    Copy {
        fmt: Vec<u8>,
        data_offset: u64,
        block: u64,
    },
    /// Decode and encode again at this bit depth.
    Flac { bits: u32 },
}

impl Plan {
    fn for_request(request: &SaveRequest) -> Result<Self, WriteError> {
        let meta = &request.meta;
        let path = &meta.source;
        let rate = wave_rate(meta)?;
        let block = meta.sample_type.bytes_per_sample() as u64;

        if request.hints.raw.is_some() {
            return Ok(Plan::Copy {
                fmt: synthesized_fmt(meta, rate),
                data_offset: request.hints.byte_offset,
                block,
            });
        }

        let head = crate::probe_head(path).map_err(|error| match error {
            crate::IoError::Open { source, .. } => WriteError::Read {
                path: path.clone(),
                source,
            },
            _ => WriteError::SourceChanged { path: path.clone() },
        })?;

        if riff::is_wave(&head) {
            let wav = |source| WriteError::Wav {
                path: path.clone(),
                source,
            };
            let chunks = riff::scan(&head).map_err(wav)?;
            let data_offset = chunks.data_offset as u64;
            if request.hints.sample_type.is_some() {
                return Ok(Plan::Copy {
                    fmt: synthesized_fmt(meta, rate),
                    data_offset,
                    block,
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
            let mut body = chunks.fmt.to_vec();
            let byte_rate = rate.saturating_mul(u32::from(fmt.block_align));
            body[4..8].copy_from_slice(&rate.to_le_bytes());
            body[8..12].copy_from_slice(&byte_rate.to_le_bytes());
            return Ok(Plan::Copy {
                fmt: body,
                data_offset,
                block: u64::from(fmt.block_align),
            });
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

/// A `fmt ` body for the capture's effective sample type.
fn synthesized_fmt(meta: &SignalMeta, rate: u32) -> Vec<u8> {
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
    body.extend_from_slice(&rate.saturating_mul(u32::from(block_align)).to_le_bytes());
    body.extend_from_slice(&block_align.to_le_bytes());
    body.extend_from_slice(&bits.to_le_bytes());
    if format == SampleFormat::F16x8 {
        body.extend_from_slice(&riff::F16X8_MAGIC.to_le_bytes());
    }
    body
}

/// A byte range of the source to copy.
struct ByteRange<'a> {
    source: &'a Path,
    data_offset: u64,
    block: u64,
    span: SampleSpan,
}

fn write_wave(
    partial: &mut Partial,
    fmt: &[u8],
    meta: &SignalMeta,
    copy: &ByteRange<'_>,
    riff_limit: u64,
    progress: &mut dyn FnMut(u64, u64),
    cancel: &AtomicBool,
) -> Result<&'static str, WriteError> {
    let data_len = copy
        .span
        .count()
        .checked_mul(copy.block)
        .ok_or(WriteError::OutOfRange)?;
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

    copy_samples(partial, copy, progress, cancel)?;
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
    copy: &ByteRange<'_>,
    progress: &mut dyn FnMut(u64, u64),
    cancel: &AtomicBool,
) -> Result<(), WriteError> {
    let read_error = |source| WriteError::Read {
        path: copy.source.to_owned(),
        source,
    };
    let mut input = File::open(copy.source).map_err(read_error)?;
    let start = copy
        .span
        .start()
        .checked_mul(copy.block)
        .and_then(|bytes| bytes.checked_add(copy.data_offset))
        .ok_or(WriteError::OutOfRange)?;
    input.seek(SeekFrom::Start(start)).map_err(read_error)?;

    let per_chunk = (COPY_BYTES as u64 / copy.block).max(1);
    let mut buf = vec![0u8; (per_chunk * copy.block) as usize];
    let total = copy.span.count();
    let mut done = 0u64;
    progress(0, total);
    while done < total {
        if cancel.load(Ordering::Relaxed) {
            return Err(WriteError::Cancelled);
        }
        let samples = per_chunk.min(total - done);
        let bytes = &mut buf[..(samples * copy.block) as usize];
        input.read_exact(bytes).map_err(|source| {
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
    DecodedSource::reopen(&plain, 0.0).map_err(|source| WriteError::Source {
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

    let config = flacenc::config::Encoder::default()
        .into_verified()
        .map_err(|(_, error)| flac(&error))?;
    let block = config.block_size;
    let mut info =
        StreamInfo::new(rate as usize, channels, bits as usize).map_err(|error| flac(&error))?;

    let mut header = Vec::new();
    header.extend_from_slice(b"fLaC");
    header.extend_from_slice(&[0, 0, 0, 34]);
    let info_offset = header.len() as u64;
    header.extend_from_slice(&[0u8; 34]);
    let comment = vorbis_comment(meta.center_freq);
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

/// A VORBIS_COMMENT body carrying the reference frequency.
fn vorbis_comment(freq: f64) -> Vec<u8> {
    let vendor = b"argand";
    let entry = format!("{REFERENCE_TAG}={freq}");
    let mut body = Vec::new();
    body.extend_from_slice(&(vendor.len() as u32).to_le_bytes());
    body.extend_from_slice(vendor);
    body.extend_from_slice(&1u32.to_le_bytes());
    body.extend_from_slice(&(entry.len() as u32).to_le_bytes());
    body.extend_from_slice(entry.as_bytes());
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
        let mut temporary = std::ffi::OsString::from(".");
        temporary.push(name);
        temporary.push(format!(".{}.part", std::process::id()));
        let path = target.with_file_name(temporary);
        let file = File::create(&path).map_err(|source| WriteError::Write {
            path: path.clone(),
            source,
        })?;
        Ok(Self {
            path,
            file: Some(file),
            renamed: false,
        })
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
    fn finish(mut self, target: &Path) -> Result<(), WriteError> {
        let synced = match self.file.take() {
            Some(file) => file.sync_all(),
            None => Err(std::io::ErrorKind::BrokenPipe.into()),
        };
        synced.map_err(|source| self.error(source))?;
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
        if !self.renamed {
            let _ = fs::remove_file(&self.path);
        }
    }
}

/// Make the rename itself durable where the platform allows it.
fn sync_directory(target: &Path) {
    #[cfg(unix)]
    if let Some(parent) = target.parent().filter(|p| !p.as_os_str().is_empty()) {
        let _ = File::open(parent).and_then(|dir| dir.sync_all());
    }
    #[cfg(not(unix))]
    let _ = target;
}

#[cfg(test)]
mod tests {
    include!("write_tests.rs");
}
