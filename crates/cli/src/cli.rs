//! Command line surface.
//!
//! Short flags deliberately match the sgvr CLI (`-f -w -c -i -d`) so that
//! muscle memory carries between the two tools.

use std::path::{Path, PathBuf};

use argand_core::{Colormap, SampleType};
use argand_dsp::{DbReference, Reduce, Window};
use argand_io::{Normalize, RawSpec, parse_hz, parse_time};
use clap::{ArgAction, Parser};

use crate::render::{
    Orientation, Panels, orientation_help, panels_help, panels_overview, vertical_orientation_alias,
};

#[derive(Parser, Debug)]
#[command(
    name = "aspec",
    version,
    about = "Render a signal file's spectrum to a PNG",
    long_about = "Render a signal file's spectrogram to a PNG, with a waveform strip above it.\n\n\
                  The container is detected from the file's content, not its extension, so \
                  .wav, .iqw and .wavs captures all work. A file with no header needs --raw.\n\n\
                  Several inputs may be given, each an exact path or a filename mask. Every \
                  option applies to every file, and a file that fails does not stop the rest.",
    after_help = extended_help()
)]
pub struct Args {
    /// Signal files to analyse: exact paths or filename masks
    #[arg(required = true, value_name = "INPUT")]
    pub inputs: Vec<PathBuf>,

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

    /// Start of the analysed span: 12.5, 1m30, 01:30
    #[arg(long, value_name = "TIME", value_parser = time)]
    pub start: Option<f64>,

    /// Length of the analysed span
    #[arg(long, value_name = "TIME", value_parser = time)]
    pub duration: Option<f64>,

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

    /// Output PNG, for a single input only (default: <input>.png)
    #[arg(short = 'o', long, value_name = "PNG")]
    pub output: Option<PathBuf>,

    /// Image size as WxH
    #[arg(short = 'i', long, value_name = "WxH", default_value = "2048x512", value_parser = image_size)]
    pub image_size: (u32, u32),

    /// Panels drawn beside the spectrogram, which is always present
    #[arg(
        long,
        value_name = "P",
        default_value_t = Panels::WAVEFORM,
        help = panels_help()
    )]
    pub panels: Panels,

    /// Time axis direction
    #[arg(
        long,
        value_name = "DIR",
        default_value_t = Orientation::Horizontal,
        help = orientation_help()
    )]
    pub orientation: Orientation,

    /// Print a machine-readable report on stdout
    #[arg(long)]
    pub json: bool,

    /// FFT size, a power of two
    #[arg(short = 'f', long, value_name = "N", default_value_t = 2048)]
    pub fft_size: usize,

    /// Frame advance in samples (default: fft-size / 4)
    #[arg(long, value_name = "N")]
    pub hop: Option<usize>,

    /// Window function
    #[arg(short = 'w', long, value_name = "W", default_value = "hann")]
    pub window_type: Window,

    /// Colour scheme
    #[arg(short = 'c', long, value_name = "C", default_value = "oceanic")]
    pub color_scheme: Colormap,

    /// Dynamic range below the reference level, in dB
    #[arg(short = 'd', long, value_name = "DB", default_value_t = 110.0)]
    pub dynamic_range: f32,

    /// How frames sharing a column are combined
    #[arg(long, value_name = "R", default_value = "max")]
    pub reduce: Reduce,

    /// What 0 dB means: fs (format full scale) or peak (this file's loudest bin)
    #[arg(long = "ref", value_name = "R", default_value = "fs")]
    pub reference: DbReference,

    /// Suppress the progress bar and report
    #[arg(short = 'q', long, conflicts_with = "verbose")]
    pub quiet: bool,

    /// Log more; repeat for trace level
    #[arg(short = 'v', long, action = ArgAction::Count)]
    pub verbose: u8,
}

fn extended_help() -> String {
    let panels = panels_overview();
    let vertical = vertical_orientation_alias();
    let all_panels = Panels::ALL;
    format!(
        concat!(
            "INPUTS:\n    ",
            "An input is an exact path or a mask over the filenames of one directory:\n    ",
            "* for any run, ? for one character, [0-9] and [!0-9] for a set.\n\n",
            "    Quote a mask so the shell passes it through. Masks are not recursive:\n",
            "    ** and masks in a directory component are refused, and a mask that\n",
            "    matches nothing is an error. Matches are sorted and de-duplicated.\n\n",
            "    With more than one file, -o is refused and each PNG is written beside\n",
            "    its input; the report shrinks to one line per file naming the render\n",
            "    as *.png, or the full block under -v, and a summary follows on stderr.\n",
            "    Output paths reach stdout only when stdout is a pipe or a file, since\n",
            "    on a terminal the line above already says where the render went.\n\n",
            "PANELS:\n    ",
            "The spectrogram is always drawn; --panels selects what joins it.\n    ",
            "{panels}\n\n",
            "    The waveform strip is a min/max envelope scaled to the --ref level,\n",
            "    so a burst shorter than one pixel column still shows.\n\n",
            "SAMPLE TYPES:\n    ",
            "rl_u8, rl_i16, rl_i32, rl_f32, rl_f16x8 (real)\n    ",
            "iq_u8, iq_i16, iq_i32, iq_f32, iq_f16x8 (complex, I/Q interleaved)\n\n",
            "    f32 is scaled to [-1, 1]; f16x8 is float32 at arbitrary scale.\n\n",
            "EXAMPLES:\n    ",
            "aspec capture.iqw -o spec.png\n    ",
            "aspec dump.bin --raw iq_i16@24k --center 12.579M\n    ",
            "aspec quiet.wav --normalize auto --ref peak\n    ",
            "aspec long.iqw --start 5m --duration 30s --orientation {vertical}\n    ",
            "aspec capture.iqw --panels {all_panels}\n    ",
            "aspec '*.iqw' --center 12.579M\n    ",
            "aspec /data/'[0-9]*.wav' --raw iq_i16@24k -q",
        ),
        panels = panels,
        vertical = vertical,
        all_panels = all_panels
    )
}

impl Args {
    /// Where `input`'s PNG goes when `--output` was not given.
    pub fn output_path(&self, input: &Path) -> PathBuf {
        self.output.clone().unwrap_or_else(|| {
            let mut name = input.as_os_str().to_owned();
            name.push(".png");
            PathBuf::from(name)
        })
    }
}

fn hz(s: &str) -> Result<f64, String> {
    parse_hz(s).map_err(|e| e.to_string())
}

fn time(s: &str) -> Result<f64, String> {
    parse_time(s).map_err(|e| e.to_string())
}

fn image_size(s: &str) -> Result<(u32, u32), String> {
    let (w, h) = s
        .trim()
        .split_once(['x', 'X', '*'])
        .ok_or_else(|| format!("invalid image size `{s}`, expected WxH such as 2048x512"))?;
    let parse = |part: &str, what: &str| -> Result<u32, String> {
        part.trim()
            .parse::<u32>()
            .ok()
            .filter(|v| *v > 0)
            .ok_or_else(|| format!("invalid image {what} `{part}`, expected a positive number"))
    };
    Ok((parse(w, "width")?, parse(h, "height")?))
}

#[cfg(test)]
mod tests {
    include!("cli_tests.rs");
}
