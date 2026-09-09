//! The command line: a file to open, and how to read it.
//!
//! The seven options are `aspec`'s own, spelled the same way and parsed by the
//! same code, because they are exactly the fields of [`OpenHints`] and a
//! headerless capture is unopenable without them. Nothing that decides how the
//! picture is drawn belongs here: that is `argand.toml`'s, and a window has
//! somewhere to change it.

use std::path::PathBuf;

use argand_core::SampleType;
use argand_io::{Normalize, OpenHints, RawSpec, parse_hz};
use clap::Parser;

use crate::document::Origin;

#[derive(Debug, Parser)]
#[command(
    name = "argand",
    version,
    about = "A signal editor and analyzer for I/Q captures",
    long_about = "Open a signal file for viewing and analysis.\n\n\
                  The container is detected from the file's content, not its extension, so \
                  .wav, .iqw and .wavs captures all work. A file with no header needs --raw.\n\n\
                  Started with no file, argand opens an empty window."
)]
pub struct Args {
    /// Signal file to open
    #[arg(value_name = "FILE")]
    pub input: Option<PathBuf>,

    /// Read as a headerless file: <type>[@<rate>], e.g. iq_i16@24k
    #[arg(long, value_name = "SPEC")]
    pub raw: Option<RawSpec>,

    /// Override the detected sample type
    #[arg(short = 't', long, value_name = "TYPE")]
    pub sample_type: Option<SampleType>,

    /// Override the sample rate: 24000, 24k, 2.4M
    #[arg(short = 'r', long, value_name = "HZ", value_parser = hz)]
    pub rate: Option<f64>,

    /// Centre frequency for the frequency axis
    ///
    /// Hyphens are allowed through because a negative offset with a unit
    /// suffix (`--center -1M`) is a value, not a flag.
    #[arg(long, value_name = "HZ", value_parser = hz, default_value = "0", allow_hyphen_values = true)]
    pub center: f64,

    /// Bytes to skip before the samples begin
    #[arg(long, value_name = "BYTES", default_value_t = 0)]
    pub offset: u64,

    /// Level scaling: none, auto, or a divisor. Defaults to auto for f16x8
    #[arg(short = 'n', long, value_name = "MODE")]
    pub normalize: Option<Normalize>,

    /// Extra gain applied after normalization
    ///
    /// Attenuating is as ordinary as boosting, so `-g -6` has to reach the
    /// value parser rather than look like a flag.
    #[arg(
        short = 'g',
        long,
        value_name = "DB",
        default_value_t = 0.0,
        allow_hyphen_values = true
    )]
    pub gain: f32,
}

impl Args {
    /// The file to open on start-up, with everything the command line said
    /// about how to read it.
    pub fn origin(&self) -> Option<Origin> {
        Some(Origin {
            path: self.input.clone()?,
            hints: OpenHints {
                level_scan_bytes: None,
                raw: self.raw,
                sample_type: self.sample_type,
                sample_rate: self.rate,
                center_freq: self.center,
                byte_offset: self.offset,
                normalize: self.normalize,
                gain_db: self.gain,
            },
        })
    }
}

/// A frequency literal, as `aspec` reads one.
fn hz(s: &str) -> Result<f64, String> {
    parse_hz(s).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    include!("cli_tests.rs");
}
