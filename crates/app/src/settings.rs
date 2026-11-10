//! Effective analysis settings, independent of the window and configuration file.

use argand_core::{Colormap, SampleRange, SignalMeta};
use argand_dsp::{Analysis, AnalysisRequest, DynamicRange, StftConfig, Window};
use serde::{Deserialize, Serialize, Serializer};

use crate::config::{Aggregation, Config};

pub const MAX_FFT_SIZE: usize = 1 << 20;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DisplayedRange {
    pub effective_db: f32,
    pub state: RangeState,
}

impl DisplayedRange {
    pub(crate) fn from_analysis(analysis: &Analysis) -> Self {
        Self {
            effective_db: analysis.dynamic_range.effective_db,
            state: RangeState::from_request(
                low_signal_recommendation(analysis),
                analysis.dynamic_range.requested,
            ),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum RangeState {
    Warned(f32),
    Corrected,
    Full,
}

impl RangeState {
    pub(crate) fn from_request(recommendation: Option<f32>, range: DynamicRange) -> Self {
        match (range, recommendation) {
            (DynamicRange::Default, Some(db)) => Self::Warned(db),
            (DynamicRange::Fixed(_), _) => Self::Corrected,
            (DynamicRange::Default | DynamicRange::Auto, _) => Self::Full,
        }
    }

    pub(crate) const fn next_range(self) -> Option<DynamicRange> {
        match self {
            Self::Warned(db) => Some(DynamicRange::Fixed(db)),
            Self::Corrected => Some(DynamicRange::Default),
            Self::Full => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Settings {
    pub fft_size: usize,
    #[serde(deserialize_with = "crate::config::parsed", serialize_with = "spelled")]
    pub window: Window,
    pub overlap: u8,
    pub aggregation: Aggregation,
    #[serde(deserialize_with = "crate::config::parsed", serialize_with = "spelled")]
    pub colormap: Colormap,
    #[serde(skip, default = "default_range")]
    pub dynamic_range: DynamicRange,
}

fn default_range() -> DynamicRange {
    DynamicRange::Default
}

fn spelled<T: std::fmt::Display, S: Serializer>(
    value: &T,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.collect_str(value)
}

impl Settings {
    /// What `session.toml` keeps of these settings, which leaves the range out.
    pub fn persisted(self) -> Self {
        Self {
            dynamic_range: default_range(),
            ..self
        }
    }

    pub fn minimap_colormap(self, displayed: Option<Self>) -> Colormap {
        displayed.unwrap_or(self).colormap
    }

    pub fn from_config(config: &Config) -> Self {
        Self {
            fft_size: config.stft.fft_size,
            window: config.stft.window,
            overlap: 75,
            aggregation: config.aggregation,
            colormap: config.color_scheme,
            dynamic_range: config.dynamic_range,
        }
    }

    pub fn restored(saved: Option<Self>, config: &Config) -> Self {
        let defaults = Self::from_config(&config.clone().repaired());
        saved
            .map(|settings| Self {
                dynamic_range: defaults.dynamic_range,
                ..settings
            })
            .filter(|settings| settings.validate(None).is_ok())
            .unwrap_or(defaults)
    }

    pub fn edited_numbers(self, overlap: &str, range: Option<&str>) -> Result<Self, String> {
        let overlap =
            crate::numbers::parse::<f32>(overlap).map_err(|_| "Enter a number for overlap")?;
        if !overlap.is_finite() || !(0.0..=95.0).contains(&overlap) || overlap.fract() != 0.0 {
            return Err(crate::numbers::text(
                "Overlap must be a whole number from 0 to 95%",
            ));
        }
        let dynamic_range = match range {
            Some(value) => DynamicRange::Fixed(
                crate::numbers::parse(value).map_err(|_| "Enter a number for range")?,
            ),
            None => self.dynamic_range,
        };
        let settings = Self {
            overlap: overlap as u8,
            dynamic_range,
            ..self
        };
        settings.validate(None)?;
        Ok(settings)
    }

    pub fn validate(self, samples: Option<u64>) -> Result<(), String> {
        if !self.fft_size.is_power_of_two() || !(2..=MAX_FFT_SIZE).contains(&self.fft_size) {
            return Err(crate::numbers::text(
                "FFT size must be a power of two from 2 to 1048576",
            ));
        }
        if self.overlap > 95 {
            return Err(crate::numbers::text("Overlap must be between 0 and 95%"));
        }
        if let DynamicRange::Fixed(value) = self.dynamic_range
            && (!value.is_finite() || value <= 0.0)
        {
            return Err("Display range must be finite and greater than zero".into());
        }
        if let Some(samples) = samples
            && samples < self.fft_size as u64
        {
            return Err(format!(
                "The signal has {} samples; choose a smaller FFT",
                crate::numbers::number(samples)
            ));
        }
        Ok(())
    }

    fn hop(self) -> usize {
        (self.fft_size / 100 * usize::from(100 - self.overlap)
            + self.fft_size % 100 * usize::from(100 - self.overlap) / 100)
            .max(1)
    }

    /// Percentages that round to the same sample hop describe the same picture.
    pub fn equivalent(self, other: Self) -> bool {
        Self {
            overlap: other.overlap,
            ..self
        } == other
            && self.hop() == other.hop()
    }

    pub fn analysis_request(
        self,
        meta: &SignalMeta,
        width: usize,
        height: usize,
    ) -> AnalysisRequest {
        AnalysisRequest {
            cfg: StftConfig {
                fft_size: self.fft_size,
                hop: self.hop(),
                window: self.window,
            },
            range: SampleRange::new(0, meta.len_samples),
            width,
            height,
            reduce: self.aggregation.reduce(),
            colormap: self.colormap,
            dynamic_range: self.dynamic_range,
            waveform_columns: Some(width),
        }
    }
}

#[cfg(test)]
#[path = "settings_test.rs"]
mod tests;

/// Warn only when an absolute scale hides a low-level signal in its lower half.
pub fn low_signal_recommendation(analysis: &argand_dsp::Analysis) -> Option<f32> {
    let peak = analysis
        .db
        .values
        .iter()
        .copied()
        .filter(|v| v.is_finite())
        .fold(f32::NEG_INFINITY, f32::max);
    visibility_advice(analysis.dynamic_range, peak, analysis.time_peak)
}

fn visibility_advice(
    range: argand_dsp::DynamicRangeResult,
    peak_db: f32,
    time_peak: f32,
) -> Option<f32> {
    (range.requested == DynamicRange::Default
        && time_peak.is_finite()
        && time_peak > 0.0
        && peak_db.is_finite()
        && peak_db <= -range.effective_db / 2.0)
        .then_some(range.recommended_db)
}

#[cfg(test)]
#[path = "settings_visibility_test.rs"]
mod visibility_tests;
