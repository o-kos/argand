//! Window functions and the two numbers that make their levels meaningful.
//!
//! The windows are *periodic* (`n / N`), not symmetric (`n / (N-1)`). A
//! symmetric window is the right choice for filter design and the wrong one
//! for an STFT: it makes overlapping frames sum unevenly and shifts the
//! coherent gain, so calibrated dBFS readings come out slightly low.

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Window {
    Hann,
    Hamming,
    BlackmanHarris,
    Rect,
}

pub const WINDOW_NAMES: [&str; 4] = ["hann", "hamming", "blackman-harris", "rect"];

impl Window {
    pub const fn as_str(self) -> &'static str {
        match self {
            Window::Hann => "hann",
            Window::Hamming => "hamming",
            Window::BlackmanHarris => "blackman-harris",
            Window::Rect => "rect",
        }
    }

    /// Sample the window over `len` points.
    pub fn coefficients(self, len: usize) -> Vec<f32> {
        if len == 0 {
            return Vec::new();
        }
        let n = len as f64;
        (0..len)
            .map(|i| {
                let t = std::f64::consts::TAU * i as f64 / n;
                let v = match self {
                    Window::Rect => 1.0,
                    Window::Hann => 0.5 - 0.5 * t.cos(),
                    Window::Hamming => 0.54 - 0.46 * t.cos(),
                    Window::BlackmanHarris => {
                        0.35875 - 0.48829 * t.cos() + 0.14128 * (2.0 * t).cos()
                            - 0.01168 * (3.0 * t).cos()
                    }
                };
                v as f32
            })
            .collect()
    }
}

/// A window together with the corrections its shape implies.
#[derive(Debug, Clone)]
pub struct WindowTable {
    pub kind: Window,
    pub coefficients: Vec<f32>,
    /// Mean coefficient. A tone's peak bin is attenuated by this much, so
    /// dividing it back out is what puts a full-scale tone at 0 dBFS.
    pub coherent_gain: f32,
    /// Equivalent noise bandwidth in bins: how much wider than one bin the
    /// window's response is to broadband noise.
    pub enbw_bins: f32,
}

impl WindowTable {
    pub fn new(kind: Window, len: usize) -> Self {
        let coefficients = kind.coefficients(len);
        let sum: f64 = coefficients.iter().map(|&v| v as f64).sum();
        let sum_sq: f64 = coefficients.iter().map(|&v| (v as f64) * (v as f64)).sum();

        let coherent_gain = if len == 0 {
            1.0
        } else {
            (sum / len as f64) as f32
        };
        let enbw_bins = if sum > 0.0 {
            (len as f64 * sum_sq / (sum * sum)) as f32
        } else {
            1.0
        };

        Self {
            kind,
            coefficients,
            coherent_gain,
            enbw_bins,
        }
    }

    pub fn len(&self) -> usize {
        self.coefficients.len()
    }

    pub fn is_empty(&self) -> bool {
        self.coefficients.is_empty()
    }

    /// Equivalent noise bandwidth in hertz.
    pub fn enbw_hz(&self, sample_rate: f64) -> f64 {
        if self.is_empty() {
            return 0.0;
        }
        self.enbw_bins as f64 * sample_rate / self.len() as f64
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("unknown window `{name}`, expected one of: {}", WINDOW_NAMES.join(", "))]
pub struct ParseWindowError {
    pub name: String,
}

impl std::str::FromStr for Window {
    type Err = ParseWindowError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "hann" | "hanning" => Ok(Window::Hann),
            "hamming" => Ok(Window::Hamming),
            "blackman-harris" | "blackmanharris" | "bh" => Ok(Window::BlackmanHarris),
            "rect" | "rectangular" | "none" => Ok(Window::Rect),
            _ => Err(ParseWindowError {
                name: s.to_string(),
            }),
        }
    }
}

impl fmt::Display for Window {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    include!("window_tests.rs");
}
