//! Physical axis order and screen mapping, independent of the window toolkit.

use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    #[default]
    Horizontal,
    Vertical,
}

impl Mode {
    pub fn vertical(self) -> bool {
        self == Self::Vertical
    }

    pub fn toggled(self) -> Self {
        match self {
            Self::Horizontal => Self::Vertical,
            Self::Vertical => Self::Horizontal,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Horizontal => "Horizontal",
            Self::Vertical => "Vertical",
        }
    }

    /// Convert screen lengths into time columns and frequency rows (and back).
    pub fn axes<T>(self, x: T, y: T) -> (T, T) {
        match self {
            Self::Horizontal => (x, y),
            Self::Vertical => (y, x),
        }
    }

    /// Time from the start, frequency from the top of an ordinary spectral image.
    pub fn fractions(self, x: f64, y: f64) -> (f64, f64) {
        match self {
            Self::Horizontal => (x, y),
            Self::Vertical => (y, 1. - x),
        }
    }

    /// Map an ordinary normalized spectral rectangle to the selected orientation.
    pub fn rect(self, [x, y, width, height]: [f32; 4]) -> [f32; 4] {
        match self {
            Self::Horizontal => [x, y, width, height],
            Self::Vertical => [1. - y - height, x, height, width],
        }
    }
}

#[cfg(test)]
#[path = "orientation_test.rs"]
mod tests;
