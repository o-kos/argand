//! `argand.toml`: the settings a person writes.
//!
//! This file belongs to whoever edits it. The application only ever reads it,
//! so comments, ordering and anything it does not understand survive untouched.
//! What the application itself remembers goes in [`crate::session`] instead.
//!
//! Nothing here can stop the program starting. A file that is missing,
//! unreadable or malformed produces the defaults and a line in the log, because
//! an editor that refuses to open until its own configuration is repaired is
//! worse than one that opens with the settings it shipped with.

use std::fmt::Display;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use argand_core::Colormap;
use argand_dsp::{DynamicRange, Window};
use serde::{Deserialize, Deserializer};

/// The name a person looks for, beside the binary or in the configuration
/// directory.
pub const FILE_NAME: &str = "argand.toml";

/// How the window is painted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    Dark,
    Light,
}

/// Everything `argand.toml` can set.
///
/// Every field has a default, so a file that sets one value is a complete file.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub theme: Theme,
    /// Colours the spectrogram is shaded with, by the same names `aspec` takes.
    #[serde(deserialize_with = "parsed")]
    pub color_scheme: Colormap,
    /// How the colour scale's decibel window is chosen: `default`, `auto`, or a
    /// number of decibels below the measured peak.
    #[serde(deserialize_with = "dynamic_range")]
    pub dynamic_range: DynamicRange,
    pub stft: Stft,
    pub panels: Panels,
}

/// What the application does when nobody has said otherwise.
///
/// Written out rather than derived, because two of these types have no default
/// of their own and because this list is the answer to "what does argand do out
/// of the box" -- it should be readable in one place.
impl Default for Config {
    fn default() -> Self {
        Self {
            theme: Theme::default(),
            color_scheme: Colormap::Oceanic,
            dynamic_range: DynamicRange::Default,
            stft: Stft::default(),
            panels: Panels::default(),
        }
    }
}

/// Transform defaults, which a later milestone will let a person override per
/// file without editing this.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Stft {
    /// Points per transform. A power of two of at least two.
    pub fft_size: usize,
    /// Window function, by the same names `aspec` takes.
    #[serde(deserialize_with = "parsed")]
    pub window: Window,
}

impl Default for Stft {
    fn default() -> Self {
        Self {
            fft_size: 2048,
            window: Window::Hann,
        }
    }
}

/// Read a value written the way it is spelled on the command line.
///
/// The names come from the same `FromStr` the CLI parses with, so a colour
/// scheme or a window function is spelled once for both front ends and cannot
/// drift between them.
fn parsed<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: FromStr,
    T::Err: Display,
{
    let text = String::deserialize(deserializer)?;
    text.parse().map_err(serde::de::Error::custom)
}

/// The colour range, which needs one word the shared parser does not take.
///
/// `DynamicRange` prints `default` but does not parse it: on the command line
/// that choice is spelled by leaving `-d` out, and a file has no absent value
/// once the key is written. Accepting the word it prints is what lets a person
/// write back what the report showed them.
fn dynamic_range<'de, D>(deserializer: D) -> Result<DynamicRange, D::Error>
where
    D: Deserializer<'de>,
{
    let text = String::deserialize(deserializer)?;
    if text.trim().eq_ignore_ascii_case("default") {
        return Ok(DynamicRange::Default);
    }
    text.parse().map_err(serde::de::Error::custom)
}

/// How the window divides between the views that will fill it.
///
/// Fractions of the window rather than pixels, so the split survives a resize
/// and a display change. The panels themselves arrive with later milestones;
/// what this milestone settles is that their proportions are configured here.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Panels {
    /// Share of the height the waveform strip takes.
    ///
    /// Bounded well inside `0..1`: a strip taking none of the window or all of
    /// it is not a layout, it is a missing panel.
    pub waveform_fraction: f32,
}

impl Default for Panels {
    fn default() -> Self {
        Self {
            waveform_fraction: 0.2,
        }
    }
}

impl Config {
    /// Read the configuration, falling back to defaults for every failure.
    ///
    /// The search order is the binary's own directory first, then the
    /// platform's configuration directory: a copy carried beside the executable
    /// is what makes the application portable, and it should win over whatever
    /// the host happens to hold.
    pub fn load(candidates: &[PathBuf]) -> Self {
        for path in candidates {
            match std::fs::read_to_string(path) {
                Ok(text) => return Self::parse(&text, path),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(error) => {
                    tracing::warn!(path = %path.display(), %error, "cannot read configuration, using defaults");
                    return Self::default();
                }
            }
        }
        tracing::debug!("no {FILE_NAME} found, using defaults");
        Self::default()
    }

    /// Parse one file's text, logging and falling back on a malformed one.
    fn parse(text: &str, path: &Path) -> Self {
        match toml::from_str::<Self>(text) {
            Ok(config) => {
                let config = config.repaired();
                tracing::info!(path = %path.display(), ?config, "configuration loaded");
                config
            }
            Err(error) => {
                tracing::warn!(path = %path.display(), %error, "malformed configuration, using defaults");
                Self::default()
            }
        }
    }

    /// Replace values that parsed but cannot be used, one log line each.
    ///
    /// These are the constraints the type system does not carry: a transform
    /// size the FFT will refuse, a fraction outside the window. A person who
    /// wrote one of them wants the rest of their file, not the defaults for all
    /// of it, so each bad value is replaced on its own.
    fn repaired(mut self) -> Self {
        let default = Self::default();
        if !self.stft.fft_size.is_power_of_two() || self.stft.fft_size < 2 {
            tracing::warn!(
                found = self.stft.fft_size,
                using = default.stft.fft_size,
                "fft size must be a power of two of at least two"
            );
            self.stft.fft_size = default.stft.fft_size;
        }
        if !(0.05..=0.9).contains(&self.panels.waveform_fraction) {
            tracing::warn!(
                found = self.panels.waveform_fraction,
                using = default.panels.waveform_fraction,
                "the waveform share must leave room for the spectrogram"
            );
            self.panels.waveform_fraction = default.panels.waveform_fraction;
        }
        self
    }

    /// Where `argand.toml` is looked for, in the order it is looked for.
    pub fn search_path() -> Vec<PathBuf> {
        let mut paths = Vec::new();
        if let Ok(exe) = std::env::current_exe()
            && let Some(dir) = exe.parent()
        {
            paths.push(dir.join(FILE_NAME));
        }
        if let Some(dir) = dirs::config_dir() {
            paths.push(dir.join("argand").join(FILE_NAME));
        }
        paths
    }
}

#[cfg(test)]
mod tests {
    include!("config_tests.rs");
}
