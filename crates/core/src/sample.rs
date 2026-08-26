use std::fmt;

use serde::Serialize;

/// Storage format of one scalar sample value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SampleFormat {
    /// Unsigned 8-bit, offset binary (silence at 128).
    U8,
    /// Signed 16-bit little-endian.
    I16,
    /// Signed 32-bit little-endian.
    I32,
    /// IEEE float32 already scaled to [-1, 1].
    F32,
    /// IEEE float32 at arbitrary scale (CoolEdit "16x8" WAV extension).
    F16x8,
}

impl SampleFormat {
    /// Bytes occupied by one scalar value.
    pub const fn bytes(self) -> usize {
        match self {
            SampleFormat::U8 => 1,
            SampleFormat::I16 => 2,
            SampleFormat::I32 | SampleFormat::F32 | SampleFormat::F16x8 => 4,
        }
    }

    /// Value that maps to 0 dBFS. Float formats are nominally unit scale;
    /// `F16x8` may exceed it, which is what `Normalize::Auto` is for.
    pub const fn full_scale(self) -> f32 {
        match self {
            SampleFormat::U8 => 128.0,
            SampleFormat::I16 => 32768.0,
            SampleFormat::I32 => 2147483648.0,
            SampleFormat::F32 | SampleFormat::F16x8 => 1.0,
        }
    }

    /// Whether the report should print absolute values as integer counts.
    pub const fn is_integer(self) -> bool {
        matches!(
            self,
            SampleFormat::U8 | SampleFormat::I16 | SampleFormat::I32
        )
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            SampleFormat::U8 => "u8",
            SampleFormat::I16 => "i16",
            SampleFormat::I32 => "i32",
            SampleFormat::F32 => "f32",
            SampleFormat::F16x8 => "f16x8",
        }
    }
}

/// Whether samples are real or complex (I/Q interleaved).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Domain {
    Real,
    Iq,
}

impl Domain {
    /// Scalar values per sample: 1 for real, 2 for interleaved I/Q.
    pub const fn channels(self) -> usize {
        match self {
            Domain::Real => 1,
            Domain::Iq => 2,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Domain::Real => "rl",
            Domain::Iq => "iq",
        }
    }
}

/// Sample format plus domain, written as one token: `iq_i16`, `rl_f16x8`, ...
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct SampleType {
    pub format: SampleFormat,
    pub domain: Domain,
}

/// Every accepted token, in the order shown by error messages and `--help`.
pub const SAMPLE_TYPE_TOKENS: [&str; 10] = [
    "rl_u8", "rl_i16", "rl_i32", "rl_f32", "rl_f16x8", "iq_u8", "iq_i16", "iq_i32", "iq_f32",
    "iq_f16x8",
];

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("unknown sample type `{token}`, expected one of: {}", SAMPLE_TYPE_TOKENS.join(", "))]
pub struct ParseSampleTypeError {
    pub token: String,
}

impl SampleType {
    pub const fn new(domain: Domain, format: SampleFormat) -> Self {
        Self { format, domain }
    }

    /// Scalar values per sample.
    pub const fn channels(self) -> usize {
        self.domain.channels()
    }

    /// Bytes per sample, both components included for I/Q.
    pub const fn bytes_per_sample(self) -> usize {
        self.format.bytes() * self.domain.channels()
    }

    pub const fn full_scale(self) -> f32 {
        self.format.full_scale()
    }

    pub const fn is_iq(self) -> bool {
        matches!(self.domain, Domain::Iq)
    }
}

impl std::str::FromStr for SampleType {
    type Err = ParseSampleTypeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let err = || ParseSampleTypeError {
            token: s.to_string(),
        };
        let lower = s.trim().to_ascii_lowercase();
        let (prefix, rest) = lower.split_once('_').ok_or_else(err)?;

        let domain = match prefix {
            "rl" => Domain::Real,
            "iq" => Domain::Iq,
            _ => return Err(err()),
        };
        let format = match rest {
            "u8" => SampleFormat::U8,
            "i16" => SampleFormat::I16,
            "i32" => SampleFormat::I32,
            "f32" => SampleFormat::F32,
            "f16x8" => SampleFormat::F16x8,
            _ => return Err(err()),
        };
        Ok(Self { format, domain })
    }
}

impl fmt::Display for SampleType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}_{}", self.domain.as_str(), self.format.as_str())
    }
}

#[cfg(test)]
mod tests {
    include!("sample_tests.rs");
}
