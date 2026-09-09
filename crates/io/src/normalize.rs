//! Level handling: how sample values get scaled on their way to the DSP.

use argand_core::SampleFormat;
use rayon::prelude::*;

use crate::convert::peak_abs;

/// Headroom applied to a measured peak so the loudest sample lands just below
/// full scale rather than exactly on it.
pub const AUTO_HEADROOM: f32 = 1.05;

/// Above this, scanning every sample costs a second full pass over the file,
/// so a spread of chunks is sampled instead.
const FULL_SCAN_LIMIT: usize = 512 << 20;

const SCAN_CHUNK_BYTES: usize = 16 << 20;

/// How to bring samples onto the [-1, 1] scale.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Normalize {
    /// Use the format's nominal full scale.
    None,
    /// Measure the peak and divide by it.
    Auto,
    /// Divide by an explicit value, in the file's own units.
    Factor(f32),
}

impl Normalize {
    /// What `--normalize` does when the user says nothing.
    ///
    /// Only the unnormalised float format needs it: every other format has a
    /// known full scale, and rescaling those would make two files
    /// incomparable to each other.
    pub fn default_for(format: SampleFormat) -> Self {
        match format {
            SampleFormat::F16x8 => Normalize::Auto,
            _ => Normalize::None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("invalid normalize value `{value}`, expected none, auto or a positive number")]
pub struct ParseNormalizeError {
    pub value: String,
}

impl std::str::FromStr for Normalize {
    type Err = ParseNormalizeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let trimmed = s.trim();
        match trimmed.to_ascii_lowercase().as_str() {
            "none" | "off" => Ok(Normalize::None),
            "auto" => Ok(Normalize::Auto),
            _ => match trimmed.parse::<f32>() {
                Ok(v) if v.is_finite() && v > 0.0 => Ok(Normalize::Factor(v)),
                _ => Err(ParseNormalizeError {
                    value: s.to_string(),
                }),
            },
        }
    }
}

/// Spelled the way `--normalize` takes it, so that a mode written down can be
/// read back by the same parser that read the command line.
impl std::fmt::Display for Normalize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => f.write_str("none"),
            Self::Auto => f.write_str("auto"),
            Self::Factor(divisor) => write!(f, "{divisor}"),
        }
    }
}

/// The divisor that brings raw values onto the unit scale.
///
/// `Auto` measures the file; anything else is a constant, so the scan is
/// skipped entirely.
pub fn resolve_divisor(mode: Normalize, format: SampleFormat, data: &[u8]) -> f32 {
    resolve_divisor_with_budget(mode, format, data, None)
}

/// Resolve levels using a bounded, evenly spread scan when a budget is supplied.
pub fn resolve_divisor_with_budget(
    mode: Normalize,
    format: SampleFormat,
    data: &[u8],
    scan_bytes: Option<usize>,
) -> f32 {
    match mode {
        Normalize::None => format.full_scale(),
        Normalize::Factor(v) => v,
        Normalize::Auto => {
            let peak = match scan_bytes {
                Some(budget) => bounded_peak(format, data, budget),
                None => measure_peak(format, data),
            } * AUTO_HEADROOM;
            if peak > 1e-9 {
                peak
            } else {
                format.full_scale()
            }
        }
    }
}

/// Largest absolute sample value in `data`, in the format's own units.
///
/// Exact for files up to [`FULL_SCAN_LIMIT`]; beyond that a spread of chunks is
/// sampled, which can miss an isolated peak.
fn measure_peak(format: SampleFormat, data: &[u8]) -> f32 {
    let width = format.bytes();
    if data.len() <= FULL_SCAN_LIMIT {
        return data
            .par_chunks(SCAN_CHUNK_BYTES - SCAN_CHUNK_BYTES % width)
            .map(|c| peak_abs(format, c))
            .reduce(|| 0.0f32, f32::max);
    }

    let chunks = (data.len() / SCAN_CHUNK_BYTES).clamp(2, 64);
    let stride = (data.len() - SCAN_CHUNK_BYTES) / (chunks - 1);
    (0..chunks)
        .into_par_iter()
        .map(|i| {
            let start = i * stride - (i * stride) % width; // stay on a value boundary
            let end = (start + SCAN_CHUNK_BYTES).min(data.len());
            peak_abs(format, &data[start..end])
        })
        .reduce(|| 0.0f32, f32::max)
}

fn bounded_peak(format: SampleFormat, data: &[u8], budget: usize) -> f32 {
    let width = format.bytes();
    let values = data.len() / width;
    let budget = (budget / width).min(values);
    if budget == 0 {
        return 0.0;
    }
    if budget == values {
        return data[..values * width]
            .par_chunks((1 << 20) / width * width)
            .map(|chunk| peak_abs(format, chunk))
            .reduce(|| 0.0, f32::max);
    }
    let chunks = budget.div_ceil((1 << 20) / width).min(64);
    let chunk = budget / chunks;
    (0..chunks)
        .into_par_iter()
        .map(|i| {
            let start = if chunks == 1 {
                0
            } else {
                i * (values - chunk) / (chunks - 1)
            };
            peak_abs(format, &data[start * width..(start + chunk) * width])
        })
        .reduce(|| 0.0, f32::max)
}

/// Turn decibels into a linear multiplier.
pub fn gain_factor(gain_db: f32) -> f32 {
    if gain_db == 0.0 {
        1.0
    } else {
        10f32.powf(gain_db / 20.0)
    }
}

#[cfg(test)]
mod tests {
    include!("normalize_tests.rs");
}
