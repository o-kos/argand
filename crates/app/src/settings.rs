//! Effective analysis settings, independent of the window and configuration file.

use argand_core::{Colormap, SampleRange, SignalMeta};
use argand_dsp::{AnalysisRequest, DynamicRange, StftConfig, Window};
use serde::{Deserialize, Serialize, Serializer};

use crate::config::{Aggregation, Config};

pub const MAX_FFT_SIZE: usize = 1 << 20;

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
        let overlap = overlap
            .parse::<f32>()
            .map_err(|_| "Enter a number for overlap")?;
        if !overlap.is_finite() || !(0.0..=95.0).contains(&overlap) || overlap.fract() != 0.0 {
            return Err("Overlap must be a whole number from 0 to 95%".into());
        }
        let dynamic_range = match range {
            Some(value) => {
                DynamicRange::Fixed(value.parse().map_err(|_| "Enter a number for range")?)
            }
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
            return Err("FFT size must be a power of two from 2 to 1048576".into());
        }
        if self.overlap > 95 {
            return Err("Overlap must be between 0 and 95%".into());
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
                "The signal has {samples} samples; choose a smaller FFT"
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
mod tests {
    use super::*;

    #[test]
    fn confirming_numeric_edits_validates_both_fields_without_losing_either() {
        let settings = Settings::from_config(&Config::default());
        let edited = settings.edited_numbers("50", Some("47.5")).unwrap();
        assert_eq!(edited.overlap, 50);
        assert_eq!(edited.dynamic_range, DynamicRange::Fixed(47.5));
        for overlap in ["", "nan", "75.5", "96"] {
            assert!(settings.edited_numbers(overlap, Some("47.5")).is_err());
        }
        for range in ["", "0", "-1", "nan", "inf"] {
            assert!(settings.edited_numbers("50", Some(range)).is_err());
        }
        assert_eq!(
            settings.edited_numbers("50.0", None).unwrap().dynamic_range,
            DynamicRange::Default
        );
    }

    #[test]
    fn saved_choices_round_trip_and_invalid_values_fall_back() {
        let config = Config::default();
        let settings = Settings {
            fft_size: 1024,
            window: Window::BlackmanHarris,
            overlap: 50,
            aggregation: Aggregation::MeanPower,
            colormap: Colormap::Inferno,
            dynamic_range: DynamicRange::Fixed(47.5),
        };
        let text = toml::to_string(&settings).unwrap();
        let saved = toml::from_str(&text).unwrap();
        assert!(!text.contains("dynamic_range"));
        assert_eq!(
            Settings::restored(Some(saved), &config),
            Settings {
                dynamic_range: config.dynamic_range,
                ..settings
            }
        );
        let legacy = format!("{text}dynamic_range = '43'\n");
        let saved = toml::from_str(&legacy).unwrap();
        let mut configured = config.clone();
        configured.dynamic_range = DynamicRange::Auto;
        assert_eq!(
            Settings::restored(Some(saved), &configured).dynamic_range,
            DynamicRange::Auto
        );
        assert_eq!(
            Settings::restored(
                Some(Settings {
                    fft_size: 3,
                    ..settings
                }),
                &config
            ),
            Settings::from_config(&config)
        );
        assert!(settings.validate(Some(32)).is_err());
        assert!(
            Settings {
                overlap: 100,
                ..settings
            }
            .validate(None)
            .is_err()
        );
        assert!(
            Settings {
                dynamic_range: DynamicRange::Fixed(f32::NAN),
                ..settings
            }
            .validate(None)
            .is_err()
        );
    }
    #[test]
    fn overlap_aliases_have_identical_requests_and_can_acknowledge_the_display() {
        let base = Settings {
            fft_size: 4,
            ..Settings::from_config(&Config::default())
        };
        for overlap in [75, 87, 90, 95] {
            let selected = Settings { overlap, ..base };
            assert!(selected.equivalent(base));
            assert_eq!(selected.hop(), 1);
        }
        assert!(!Settings { overlap: 0, ..base }.equivalent(base));
        assert!(
            !Settings {
                colormap: Colormap::Inferno,
                ..base
            }
            .equivalent(base)
        );
    }

    #[test]
    fn configuration_defaults_obey_the_same_limits_as_saved_and_interactive_settings() {
        let mut config = Config::default();
        config.stft.fft_size = usize::MAX / 2 + 1;
        config.color_scheme = Colormap::Inferno;
        let settings = Settings::restored(None, &config);
        assert!(settings.validate(None).is_ok());
        assert_eq!(settings.fft_size, 2048);
        assert_eq!(settings.colormap, Colormap::Inferno);
    }
}

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
mod visibility_tests {
    use super::*;
    #[test]
    fn a_narrow_recommendation_does_not_make_a_normal_signal_a_warning() {
        let range = argand_dsp::DynamicRangeResult {
            requested: DynamicRange::Default,
            effective_db: 110.0,
            recommended_db: 30.0,
        };
        assert_eq!(visibility_advice(range, -6.0, 0.9), None);
        assert_eq!(visibility_advice(range, -60.0, 0.002), Some(30.0));
        assert_eq!(visibility_advice(range, -180.0, 0.0), None);
        assert_eq!(visibility_advice(range, f32::NEG_INFINITY, 0.01), None);
        for requested in [DynamicRange::Fixed(110.0), DynamicRange::Auto] {
            assert_eq!(
                visibility_advice(
                    argand_dsp::DynamicRangeResult { requested, ..range },
                    -60.0,
                    0.002
                ),
                None
            );
        }
    }
}
