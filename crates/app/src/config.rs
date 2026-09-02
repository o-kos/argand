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

use std::path::{Path, PathBuf};

use serde::Deserialize;

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
#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub theme: Theme,
    pub panels: Panels,
}

/// How the window divides between the views that will fill it.
///
/// Fractions of the window rather than pixels, so the split survives a resize
/// and a display change. The panels themselves arrive with later milestones;
/// what this milestone settles is that their proportions are configured here.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Panels {
    /// Share of the height the waveform strip takes, in `0.0..1.0`.
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
        match toml::from_str(text) {
            Ok(config) => {
                tracing::info!(path = %path.display(), "configuration loaded");
                config
            }
            Err(error) => {
                tracing::warn!(path = %path.display(), %error, "malformed configuration, using defaults");
                Self::default()
            }
        }
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
