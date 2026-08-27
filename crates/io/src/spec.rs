//! The compact `--raw` specification and the frequency literals used across
//! the CLI.

use std::fmt;
use std::str::FromStr;

use argand_core::{ParseSampleTypeError, SampleType};

/// How to read a headerless file: `<sample type>[@<rate>]`, as in
/// `iq_i16@24k` or `rl_f16x8@2.4M`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RawSpec {
    pub sample_type: SampleType,
    /// Absent when the rate is supplied separately via `--rate`.
    pub sample_rate: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ParseRawSpecError {
    #[error(transparent)]
    SampleType(#[from] ParseSampleTypeError),
    #[error(transparent)]
    Rate(#[from] ParseHzError),
    #[error("empty raw specification, expected <sample type>[@<rate>], e.g. iq_i16@24k")]
    Empty,
}

impl FromStr for RawSpec {
    type Err = ParseRawSpecError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        if s.is_empty() {
            return Err(ParseRawSpecError::Empty);
        }
        let (type_part, rate_part) = match s.split_once('@') {
            Some((t, r)) => (t, Some(r)),
            None => (s, None),
        };
        Ok(Self {
            sample_type: type_part.parse()?,
            sample_rate: rate_part.map(parse_hz).transpose()?,
        })
    }
}

impl fmt::Display for RawSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.sample_rate {
            Some(rate) => write!(f, "{}@{}", self.sample_type, rate),
            None => write!(f, "{}", self.sample_type),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error(
    "invalid frequency `{value}`, expected hertz with an optional k/M/G suffix, e.g. 24000, 24k, 2.4M"
)]
pub struct ParseHzError {
    pub value: String,
}

/// Parse a frequency literal.
///
/// A bare number is hertz. `k`, `M` and `G` scale it; case is ignored, since
/// nothing in this domain is measured in millihertz. A trailing `Hz` is
/// accepted so that `24kHz` works as typed.
pub fn parse_hz(s: &str) -> Result<f64, ParseHzError> {
    let err = || ParseHzError {
        value: s.to_string(),
    };

    let mut text = s.trim();
    if text.len() >= 2 && text[text.len() - 2..].eq_ignore_ascii_case("hz") {
        text = text[..text.len() - 2].trim_end();
    }

    let (number, scale) = match text.chars().last().ok_or_else(err)? {
        'k' | 'K' => (&text[..text.len() - 1], 1e3),
        'M' | 'm' => (&text[..text.len() - 1], 1e6),
        'G' | 'g' => (&text[..text.len() - 1], 1e9),
        _ => (text, 1.0),
    };

    let value: f64 = number.trim().parse().map_err(|_| err())?;
    if !value.is_finite() {
        return Err(err());
    }
    Ok(value * scale)
}

/// Parse a duration literal: `12.5`, `90s`, `1m30`, `01:30`, `1h02m03`.
pub fn parse_time(s: &str) -> Result<f64, ParseTimeError> {
    let err = || ParseTimeError {
        value: s.to_string(),
    };
    let text = s.trim().to_ascii_lowercase();
    if text.is_empty() {
        return Err(err());
    }

    // Colon form: [hh:]mm:ss(.fff)
    if text.contains(':') {
        let mut total = 0.0;
        for part in text.split(':') {
            let value: f64 = part.trim().parse().map_err(|_| err())?;
            if value < 0.0 {
                return Err(err());
            }
            total = total * 60.0 + value;
        }
        return Ok(total);
    }

    // Suffix form: 1h02m03.5, 90s, 250ms, or a bare number of seconds.
    if let Some(ms) = text.strip_suffix("ms") {
        return ms.parse::<f64>().map(|v| v / 1000.0).map_err(|_| err());
    }

    let mut total = 0.0;
    let mut number = String::new();
    let mut saw_unit = false;
    for ch in text.chars() {
        match ch {
            'h' | 'm' | 's' => {
                let value: f64 = number.parse().map_err(|_| err())?;
                total += value
                    * match ch {
                        'h' => 3600.0,
                        'm' => 60.0,
                        _ => 1.0,
                    };
                number.clear();
                saw_unit = true;
            }
            c if c.is_ascii_digit() || c == '.' => number.push(c),
            _ => return Err(err()),
        }
    }
    if !number.is_empty() {
        // Trailing digits after a unit are seconds: "1m30" is 90 seconds.
        total += number.parse::<f64>().map_err(|_| err())?;
    } else if !saw_unit {
        return Err(err());
    }

    if total.is_finite() && total >= 0.0 {
        Ok(total)
    } else {
        Err(err())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("invalid time `{value}`, expected seconds, 1m30, 01:30 or 250ms")]
pub struct ParseTimeError {
    pub value: String,
}

#[cfg(test)]
mod tests {
    include!("spec_tests.rs");
}
