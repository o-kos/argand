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
    Analysis, AnalysisRequest, DB_FLOOR, DEFAULT_DYNAMIC_RANGE_DB, DspError, DynamicRange,
    DynamicRangeResult, MAX_RECOMMENDED_RANGE_DB, MIN_RECOMMENDED_RANGE_DB, ParseDynamicRangeError,
    ParseEnumError, REDUCE_NAMES, Reduce, Shading, StftConfig, analyze, shade,
};
pub use waveform::EnvelopeBuilder;
pub use window::{ParseWindowError, WINDOW_NAMES, Window, WindowTable};
