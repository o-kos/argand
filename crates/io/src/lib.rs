//! Reading signal files into a lazy stream of samples.
//!
//! Container detection goes by content, never by extension: the same I/Q
//! capture turns up as `.wav`, `.iqw` and `.wavs`, and all three are RIFF.
//! RF64 and BW64, the 64-bit forms that captures past 4 GB need, are read
//! through the same path.
//!
//! Two readers cover everything argand claims to support. [`MmapSource`]
//! serves anything that is a flat array of interleaved fixed-width samples,
//! which is every WAVE layout in the format table plus headerless files;
//! [`DecodedSource`] handles FLAC and any WAVE variant the native path
//! declines.

pub mod convert;
pub mod decoder;
pub mod mmapped;
pub mod normalize;
pub mod riff;
pub mod spec;
#[cfg(any(test, feature = "testutil"))]
pub mod testutil;

use std::fs::File;
use std::path::{Path, PathBuf};

use argand_core::{SampleSource, SampleType, SignalMeta, SourceError};

pub use decoder::DecodedSource;
pub use mmapped::MmapSource;
pub use normalize::{Normalize, ParseNormalizeError};
pub use riff::{RiffError, WavLayout};
pub use spec::{ParseHzError, ParseRawSpecError, ParseTimeError, RawSpec, parse_hz, parse_time};

/// Everything the caller can say about a file that the file cannot say itself.
#[derive(Debug, Clone, Default)]
pub struct OpenHints {
    /// Read the file as a headerless sample array with this layout.
    pub raw: Option<RawSpec>,
    /// Override the detected sample type.
    pub sample_type: Option<SampleType>,
    /// Override the declared sample rate.
    pub sample_rate: Option<f64>,
    /// Centre of the frequency axis in Hz.
    pub center_freq: f64,
    /// Bytes to skip before the samples begin.
    pub byte_offset: u64,
    /// How to bring values onto the unit scale. `None` means "decide from the
    /// detected sample format".
    pub normalize: Option<Normalize>,
    pub gain_db: f32,
}

#[derive(Debug, thiserror::Error)]
pub enum IoError {
    #[error("cannot open {path}: {source}")]
    Open {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("{path} is empty")]
    Empty { path: PathBuf },
    #[error("cannot read {path} as wav: {source}")]
    Wav {
        path: PathBuf,
        #[source]
        source: RiffError,
    },
    #[error(
        "{path}: unrecognised container; pass --raw <type>[@<rate>] to read it as a headerless file"
    )]
    UnknownContainer { path: PathBuf },
    #[error("no sample rate for {path}: give it as --raw <type>@<rate> or --rate <hz>")]
    MissingRate { path: PathBuf },
    #[error("{path}: {source}")]
    Source {
        path: PathBuf,
        #[source]
        source: SourceError,
    },
}

/// Open a signal file, applying `hints` on top of whatever the file declares.
pub fn open(path: &Path, hints: &OpenHints) -> Result<Box<dyn SampleSource>, IoError> {
    let head = probe_head(path)?;

    if let Some(spec) = hints.raw {
        return open_raw(path, spec, hints);
    }

    if riff::is_wave(&head) {
        match riff::parse(&head) {
            Ok(layout) => return open_wave(path, layout, hints),
            Err(RiffError::Unsupported { .. }) => {
                // A valid wav in a layout the flat reader will not touch --
                // 24-bit, say. The decoder may still manage it.
                tracing::debug!("wav layout not natively supported, trying the decoder");
                return open_decoded(path, "wav", hints);
            }
            Err(source) => {
                return Err(IoError::Wav {
                    path: path.to_owned(),
                    source,
                });
            }
        }
    }

    if riff::is_flac(&head) {
        return open_decoded(path, "flac", hints);
    }

    Err(IoError::UnknownContainer {
        path: path.to_owned(),
    })
}

/// Read enough of the file to identify it and parse a header.
fn probe_head(path: &Path) -> Result<Vec<u8>, IoError> {
    use std::io::Read;

    let mut file = File::open(path).map_err(|source| IoError::Open {
        path: path.to_owned(),
        source,
    })?;
    // Generous enough for a RIFF chunk list with metadata in front of `data`.
    let mut head = vec![0u8; 64 << 10];
    let mut filled = 0;
    while filled < head.len() {
        match file.read(&mut head[filled..]) {
            Ok(0) => break,
            Ok(n) => filled += n,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(source) => {
                return Err(IoError::Open {
                    path: path.to_owned(),
                    source,
                });
            }
        }
    }
    head.truncate(filled);
    if head.is_empty() {
        return Err(IoError::Empty {
            path: path.to_owned(),
        });
    }
    Ok(head)
}

fn open_wave(
    path: &Path,
    layout: WavLayout,
    hints: &OpenHints,
) -> Result<Box<dyn SampleSource>, IoError> {
    let sample_type = hints.sample_type.unwrap_or(layout.sample_type);
    let meta = SignalMeta {
        sample_rate: hints.sample_rate.unwrap_or(layout.sample_rate),
        center_freq: hints.center_freq,
        sample_type,
        len_samples: 0, // recomputed from the mapped length
        container: layout.container,
        divisor: 1.0, // replaced by the level scan
        source: path.to_owned(),
    };
    let normalize = hints
        .normalize
        .unwrap_or_else(|| Normalize::default_for(sample_type.format));

    MmapSource::new(
        path,
        meta,
        layout.data_offset,
        layout.declared_len.unwrap_or(usize::MAX),
        normalize,
        hints.gain_db,
    )
    .map(|s| Box::new(s) as Box<dyn SampleSource>)
    .map_err(|source| IoError::Source {
        path: path.to_owned(),
        source,
    })
}

fn open_raw(
    path: &Path,
    spec: RawSpec,
    hints: &OpenHints,
) -> Result<Box<dyn SampleSource>, IoError> {
    let sample_type = hints.sample_type.unwrap_or(spec.sample_type);
    let sample_rate =
        hints
            .sample_rate
            .or(spec.sample_rate)
            .ok_or_else(|| IoError::MissingRate {
                path: path.to_owned(),
            })?;

    let meta = SignalMeta {
        sample_rate,
        center_freq: hints.center_freq,
        sample_type,
        len_samples: 0,
        container: "raw",
        divisor: 1.0,
        source: path.to_owned(),
    };
    let normalize = hints
        .normalize
        .unwrap_or_else(|| Normalize::default_for(sample_type.format));

    MmapSource::new(
        path,
        meta,
        hints.byte_offset as usize,
        usize::MAX,
        normalize,
        hints.gain_db,
    )
    .map(|s| Box::new(s) as Box<dyn SampleSource>)
    .map_err(|source| IoError::Source {
        path: path.to_owned(),
        source,
    })
}

fn open_decoded(
    path: &Path,
    container: &'static str,
    hints: &OpenHints,
) -> Result<Box<dyn SampleSource>, IoError> {
    let wrap = |source| IoError::Source {
        path: path.to_owned(),
        source,
    };

    // The sample format is only known after probing, so a default normalize
    // mode needs one cheap open first.
    let normalize = match hints.normalize {
        Some(mode) => mode,
        None => {
            let probe = DecodedSource::open(
                path,
                container,
                hints.center_freq,
                hints.sample_rate,
                hints.sample_type,
                Normalize::None,
                0.0,
            )
            .map_err(wrap)?;
            Normalize::default_for(probe.meta().sample_type.format)
        }
    };

    DecodedSource::open(
        path,
        container,
        hints.center_freq,
        hints.sample_rate,
        hints.sample_type,
        normalize,
        hints.gain_db,
    )
    .map(|s| Box::new(s) as Box<dyn SampleSource>)
    .map_err(wrap)
}

#[cfg(test)]
mod tests {
    include!("lib_tests.rs");
}
