use std::fmt;

use hsl::HSL;
use serde::Serialize;

/// Number of entries in a baked gradient.
pub const GRADIENT_SIZE: usize = 256;

pub type Gradient = [[u8; 3]; GRADIENT_SIZE];

/// Colour ramps for magnitude-to-pixel mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Colormap {
    Oceanic,
    Grayscale,
    Inferno,
    Viridis,
    Synthwave,
    Sunset,
}

pub const COLORMAP_NAMES: [&str; 6] = [
    "oceanic",
    "grayscale",
    "inferno",
    "viridis",
    "synthwave",
    "sunset",
];

impl Colormap {
    /// Anchor colours, dark (weakest) to bright (strongest).
    pub const fn stops(self) -> &'static [u32] {
        match self {
            Colormap::Oceanic => &[0x01041B, 0x072E69, 0x4DA4D5, 0xDCF3FF],
            Colormap::Grayscale => &[0x000000, 0x888888, 0xFFFFFF],
            Colormap::Inferno => &[0x000004, 0x3B0F70, 0xAC255E, 0xF98E09, 0xFCFD21],
            Colormap::Viridis => &[0x440154, 0x3B528B, 0x21918C, 0x5EC962, 0xFDE725],
            Colormap::Synthwave => &[0x0D0221, 0x2D134B, 0xA537FD, 0x00F6FF],
            Colormap::Sunset => &[0x3C031C, 0x9C1521, 0xFD6A02, 0xFEC812],
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Colormap::Oceanic => "oceanic",
            Colormap::Grayscale => "grayscale",
            Colormap::Inferno => "inferno",
            Colormap::Viridis => "viridis",
            Colormap::Synthwave => "synthwave",
            Colormap::Sunset => "sunset",
        }
    }

    /// Bake the ramp into a lookup table.
    ///
    /// Interpolation runs in HSL and takes the short way round the hue circle,
    /// which keeps ramps like Synthwave from washing out through grey.
    pub fn gradient(self) -> Gradient {
        let stops: Vec<HSL> = self
            .stops()
            .iter()
            .map(|&rgb| HSL::from_rgb(&unpack(rgb)))
            .collect();

        let mut gradient = [[0u8; 3]; GRADIENT_SIZE];
        if stops.len() == 1 {
            let (r, g, b) = stops[0].to_rgb();
            gradient.fill([r, g, b]);
            return gradient;
        }

        let segments = stops.len() - 1;
        for (i, slot) in gradient.iter_mut().enumerate() {
            let progress = i as f64 / (GRADIENT_SIZE - 1) as f64;
            let scaled = progress * segments as f64;
            let (index, t) = if progress >= 1.0 {
                (segments - 1, 1.0)
            } else {
                (scaled.floor() as usize, scaled.fract())
            };

            let (start, end) = (stops[index], stops[index + 1]);
            let s = start.s + (end.s - start.s) * t;
            let l = start.l + (end.l - start.l) * t;

            let mut h_start = start.h;
            let diff = end.h - h_start;
            if diff.abs() > 180.0 {
                h_start += if diff > 0.0 { 360.0 } else { -360.0 };
            }
            let h = (h_start + (end.h - h_start) * t).rem_euclid(360.0);

            let (r, g, b) = HSL { h, s, l }.to_rgb();
            *slot = [r, g, b];
        }
        gradient
    }
}

const fn unpack(rgb: u32) -> [u8; 3] {
    [
        ((rgb >> 16) & 0xFF) as u8,
        ((rgb >> 8) & 0xFF) as u8,
        (rgb & 0xFF) as u8,
    ]
}

/// Index into a baked gradient for a value already normalised to [0, 1].
pub fn gradient_index(normalized: f32) -> usize {
    let scaled = normalized.clamp(0.0, 1.0) * (GRADIENT_SIZE - 1) as f32;
    (scaled.round() as usize).min(GRADIENT_SIZE - 1)
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("unknown colormap `{name}`, expected one of: {}", COLORMAP_NAMES.join(", "))]
pub struct ParseColormapError {
    pub name: String,
}

impl std::str::FromStr for Colormap {
    type Err = ParseColormapError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "oceanic" => Ok(Colormap::Oceanic),
            "grayscale" | "greyscale" => Ok(Colormap::Grayscale),
            "inferno" => Ok(Colormap::Inferno),
            "viridis" => Ok(Colormap::Viridis),
            "synthwave" => Ok(Colormap::Synthwave),
            "sunset" => Ok(Colormap::Sunset),
            _ => Err(ParseColormapError {
                name: s.to_string(),
            }),
        }
    }
}

impl fmt::Display for Colormap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    include!("colormap_tests.rs");
}
