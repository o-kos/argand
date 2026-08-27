//! Spectral analysis for argand.
//!
//! Consumes the [`SampleSource`] trait from `argand-core` and produces the
//! render view-models defined there. It knows nothing about file formats and
//! nothing about any GUI toolkit.
//!
//! [`SampleSource`]: argand_core::SampleSource

pub mod stft;
pub mod waveform;
pub mod window;

pub use stft::{
    Analysis, AnalysisRequest, DB_REFERENCE_NAMES, DbReference, DspError, ParseEnumError,
    REDUCE_NAMES, Reduce, StftConfig, analyze,
};
pub use waveform::EnvelopeBuilder;
pub use window::{ParseWindowError, WINDOW_NAMES, Window, WindowTable};
