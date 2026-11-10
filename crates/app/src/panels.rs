//! Toolkit-independent vertical panel layout.

use std::fmt;
use std::str::FromStr;

/// The minimap's fixed size across the panel, in font-relative or logical pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MinimapSize {
    /// A multiple of the interface font size.
    Rem(f32),
    /// Logical pixels, independent of the font.
    Px(f32),
}

impl MinimapSize {
    /// The smallest and largest size in each unit.
    const REM: (f32, f32) = (1.0, 20.0);
    const PX: (f32, f32) = (16.0, 320.0);

    /// The size in logical pixels for an interface font of `rem` pixels.
    pub fn logical(self, rem: f32) -> f32 {
        match self {
            Self::Rem(value) => value * rem,
            Self::Px(value) => value,
        }
    }
}

impl Default for MinimapSize {
    fn default() -> Self {
        Self::Rem(3.0)
    }
}

impl fmt::Display for MinimapSize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Rem(value) => write!(f, "{value} rem"),
            Self::Px(value) => write!(f, "{value} px"),
        }
    }
}

impl FromStr for MinimapSize {
    type Err = String;

    /// A number and its unit, `rem` or `px`, with or without a space between.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let text = text.trim();
        let lower = text.to_ascii_lowercase();
        let (number, unit) = ["rem", "px"]
            .into_iter()
            .find_map(|unit| lower.strip_suffix(unit).map(|number| (number, unit)))
            .ok_or_else(|| format!("{text:?} ends in no unit, write rem or px"))?;
        let value: f32 = number
            .trim()
            .parse()
            .map_err(|_| format!("{:?} is not a number", number.trim()))?;
        let (size, (low, high)) = match unit {
            "rem" => (Self::Rem(value), Self::REM),
            _ => (Self::Px(value), Self::PX),
        };
        if !(low..=high).contains(&value) {
            return Err(format!("{size} is outside {low} to {high} {unit}"));
        }
        Ok(size)
    }
}

/// The minimap's size on device pixels, never more than the panel holds.
pub fn waveform_height(total: f32, rem: f32, size: MinimapSize, scale: f32) -> f32 {
    let height = size.logical(rem).min(total.max(0.0));
    (height * scale).round() / scale
}

#[cfg(test)]
#[path = "panels_test.rs"]
mod tests;
