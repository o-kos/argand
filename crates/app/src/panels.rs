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
mod tests {
    use super::*;

    const DEFAULT: MinimapSize = MinimapSize::Rem(3.0);

    #[test]
    fn a_rem_size_follows_the_font_and_not_the_window() {
        assert_eq!(waveform_height(600.0, 16.0, DEFAULT, 1.0), 48.0);
        assert_eq!(waveform_height(800.0, 16.0, DEFAULT, 1.0), 48.0);
        assert_eq!(waveform_height(600.0, 20.0, DEFAULT, 1.0), 60.0);
    }

    #[test]
    fn a_pixel_size_ignores_the_font() {
        let size = MinimapSize::Px(64.0);
        assert_eq!(waveform_height(600.0, 16.0, size, 1.0), 64.0);
        assert_eq!(waveform_height(600.0, 20.0, size, 1.0), 64.0);
    }

    #[test]
    fn a_small_panel_bounds_the_minimap() {
        assert_eq!(waveform_height(20.0, 16.0, DEFAULT, 1.0), 20.0);
        assert_eq!(waveform_height(-5.0, 16.0, DEFAULT, 1.0), 0.0);
    }

    #[test]
    fn the_minimap_stays_on_device_pixels_at_fractional_dpi() {
        for scale in [1.0, 1.25, 1.5, 2.0] {
            let height = waveform_height(316.0, 15.3, DEFAULT, scale);
            assert!((height * scale).fract().abs() < 1e-5);
        }
    }

    #[test]
    fn sizes_parse_with_either_unit_and_any_spacing() {
        for (text, size) in [
            ("4 rem", MinimapSize::Rem(4.0)),
            ("4.0rem", MinimapSize::Rem(4.0)),
            (" 2.5 REM ", MinimapSize::Rem(2.5)),
            ("64 px", MinimapSize::Px(64.0)),
            ("64px", MinimapSize::Px(64.0)),
            ("1e1 rem", MinimapSize::Rem(10.0)),
            ("+48 PX", MinimapSize::Px(48.0)),
        ] {
            assert_eq!(text.parse::<MinimapSize>(), Ok(size), "{text}");
        }
    }

    #[test]
    fn unusable_sizes_are_refused() {
        for text in [
            "", "4", "rem", "four rem", "4 em", "0 rem", "-4 rem", "nan rem", "inf rem", "21 rem",
            "8 px", "1000 px", "4 rem px", "4 remx",
        ] {
            assert!(text.parse::<MinimapSize>().is_err(), "{text}");
        }
    }

    #[test]
    fn a_size_reads_back_what_it_prints() {
        for size in [DEFAULT, MinimapSize::Rem(2.5), MinimapSize::Px(64.0)] {
            assert_eq!(size.to_string().parse::<MinimapSize>(), Ok(size));
        }
    }
}
