//! Domain types shared by every part of argand.
//!
//! This crate knows about signals, sample formats and render outputs. It does
//! not know about file containers, transforms or any GUI toolkit -- that
//! separation is what keeps the render seam replaceable.

pub mod colormap;
pub mod fmt;
pub mod sample;
pub mod signal;
pub mod view;

pub use colormap::{COLORMAP_NAMES, Colormap, GRADIENT_SIZE, Gradient, gradient_index};
pub use fmt::{format_bytes, format_duration, format_hz, format_samples};
pub use sample::{Domain, ParseSampleTypeError, SAMPLE_TYPE_TOKENS, SampleFormat, SampleType};
pub use signal::{SampleRange, SampleSource, SignalMeta, SourceError};
pub use view::{Psd, SpectrogramImage, SpectrumPeak, WaveformEnvelope};
