//! What the run tells you afterwards.
//!
//! One section per file: a header naming it, its facts indented under it, and
//! every value printed once. What was measured in the signal belongs to the
//! input; what the picture shows, and the transform that drew it, belong to
//! the render.
//!
//! Levels are dBFS, which is the scale the range decision is made on and the
//! one that compares between files. The file's own units are what you check a
//! capture chain against, so `-v` prints them beside the decibels.

use std::io::Write;
use std::path::Path;

use argand_core::{SignalMeta, format_bytes, format_duration, format_hz, format_samples};
use argand_dsp::{Analysis, AnalysisRequest};
use argand_io::Normalize;
use serde::Serialize;

/// How much of a file's report a run asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Detail {
    /// One line for a file in a batch, so that a listing stays scannable.
    Compact,
    /// What was measured and what was drawn, each said once.
    Default,
    /// Everything the default leaves out: the settings that produced the
    /// render, the file's own units, and the render's full path.
    Verbose,
}

#[derive(Debug, Clone, Serialize)]
pub struct Level {
    /// Value on the [-1, 1] scale the transform worked with.
    pub unit: f32,
    /// The same value in the units actually stored in the file.
    pub absolute: f32,
    pub dbfs: f32,
}

impl Level {
    fn from_unit(unit: f32, divisor: f32) -> Self {
        Self {
            unit,
            absolute: unit * divisor,
            dbfs: 20.0 * unit.max(1e-15).log10(),
        }
    }

    fn from_db(db: f32, divisor: f32) -> Self {
        let unit = 10f32.powf(db / 20.0);
        Self {
            unit,
            absolute: unit * divisor,
            dbfs: db,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PeakBin {
    pub bin: usize,
    pub offset_hz: f64,
    pub freq_hz: f64,
    #[serde(flatten)]
    pub level: Level,
}

#[derive(Debug, Clone, Serialize)]
pub struct StftReport {
    pub fft_size: usize,
    pub hop: usize,
    pub window: String,
    pub overlap_percent: f64,
    pub frames: u64,
    pub enbw_hz: f64,
    pub reduce: String,
    pub dynamic_range_mode: String,
    pub dynamic_range_db: f32,
    pub recommended_dynamic_range_db: f32,
    pub db_min: f32,
    pub db_max: f32,
}

#[derive(Debug, Clone, Serialize)]
pub struct OutputReport {
    pub path: String,
    pub width: u32,
    pub height: u32,
    pub bytes: u64,
    /// Panels drawn beside the spectrogram, as `--panels` spelled them.
    pub panels: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Report {
    pub file: String,
    pub container: String,
    pub sample_type: String,
    pub sample_rate: f64,
    pub center_freq: f64,
    pub samples: u64,
    pub duration_seconds: f64,
    pub analysed_seconds: f64,
    pub normalize: String,
    pub divisor: f32,
    pub gain_db: f32,
    pub stft: StftReport,
    pub peak_sample: Level,
    pub peak_bin: Option<PeakBin>,
    pub floor: Option<Level>,
    pub output: Option<OutputReport>,
    pub elapsed_seconds: f64,
}

/// How sample values were scaled before the transform saw them.
///
/// The mode here is the one actually applied, after any format default was
/// resolved, which is what the report has to describe.
#[derive(Debug, Clone, Copy)]
pub struct Scaling {
    pub normalize: Normalize,
    pub gain_db: f32,
}

impl Report {
    /// Describe a finished analysis.
    ///
    /// The settings come from the `request` that produced `analysis` rather
    /// than being passed again alongside it. Reading them twice from the
    /// command line is what would let the report describe a transform that
    /// never ran.
    pub fn new(
        meta: &SignalMeta,
        analysis: &Analysis,
        request: &AnalysisRequest,
        scaling: Scaling,
    ) -> Self {
        let image = &analysis.spectrogram;
        let divisor = meta.divisor;
        let cfg = &request.cfg;
        let analysed_seconds = request.range.len as f64 / meta.sample_rate;

        let peak_bin = analysis.psd.peak(meta.center_freq).map(|p| PeakBin {
            bin: p.bin,
            offset_hz: p.offset_hz,
            freq_hz: p.freq_hz,
            level: Level::from_db(p.db, divisor),
        });

        Self {
            file: meta.source.display().to_string(),
            container: meta.container.to_string(),
            sample_type: meta.sample_type.to_string(),
            sample_rate: meta.sample_rate,
            center_freq: meta.center_freq,
            samples: meta.len_samples,
            duration_seconds: meta.duration_seconds(),
            analysed_seconds,
            normalize: describe_normalize(scaling.normalize),
            divisor,
            gain_db: scaling.gain_db,
            stft: StftReport {
                fft_size: cfg.fft_size,
                hop: cfg.hop,
                window: cfg.window.to_string(),
                overlap_percent: cfg.overlap_percent(),
                frames: analysis.frames,
                enbw_hz: analysis.enbw_hz,
                reduce: request.reduce.to_string(),
                dynamic_range_mode: analysis.dynamic_range.requested.mode().to_string(),
                dynamic_range_db: analysis.dynamic_range.effective_db,
                recommended_dynamic_range_db: analysis.dynamic_range.recommended_db,
                db_min: image.db_min,
                db_max: image.db_max,
            },
            peak_sample: Level::from_unit(analysis.time_peak, divisor),
            peak_bin,
            floor: analysis
                .psd
                .floor_db()
                .map(|db| Level::from_db(db, divisor)),
            output: None,
            elapsed_seconds: 0.0,
        }
    }

    pub fn with_output(
        mut self,
        path: &Path,
        width: u32,
        height: u32,
        bytes: u64,
        panels: String,
    ) -> Self {
        self.output = Some(OutputReport {
            path: path.display().to_string(),
            width,
            height,
            bytes,
            panels,
        });
        self
    }

    /// Text the foot of the image carries.
    ///
    /// The scale reference and an optional recommendation stay adjacent so the
    /// action reads as a direct response to the range it follows.
    ///
    /// The unit is `dBFS`, without a `per bin`: the transform divides by the
    /// window's coherent gain, not by any bandwidth. A full-scale tone on a bin
    /// centre therefore reads 0 dBFS at any transform size, and one between
    /// bins reads under it by the window's scalloping loss rather than by
    /// anything to do with the bin's width. Only noise moves with the
    /// bandwidth, which is why that bandwidth is named in the same field -- and
    /// named as `ENBW`, since how much noise a bin answers to is the window's
    /// to decide and only a rectangular one leaves it at the bin spacing.
    pub fn plot_footer(&self) -> String {
        format!(
            "fft {} · {} · hop {} ({:.0}% overlap) · {} · {}",
            self.stft.fft_size,
            self.stft.window,
            self.stft.hop,
            self.stft.overlap_percent,
            self.stft.reduce,
            self.plot_scale_footer(),
        )
    }

    /// Scale information retained when the complete footer does not fit.
    pub fn plot_scale_footer(&self) -> String {
        let reference = match self.stft.dynamic_range_mode.as_str() {
            "default" => "full scale".to_string(),
            _ => format!("peak ({:.1} dBFS)", self.stft.db_max),
        };
        let suggestion = self
            .range_suggestion()
            .map_or_else(String::new, |value| format!(" ({value})"));
        format!(
            "{} dB below {}{} · dBFS, ENBW {}",
            self.stft.dynamic_range_db,
            reference,
            suggestion,
            format_hz(self.stft.enbw_hz),
        )
    }

    /// The input's own name, which is what identifies it once it is drawn.
    fn file_name(&self) -> String {
        file_name_of(&self.file)
    }

    /// Text the header of the image carries.
    pub fn plot_title(&self) -> String {
        let name = self.file_name();
        format!(
            "{name} · {} · {} · {} · {}",
            self.container,
            self.sample_type,
            format_hz(self.sample_rate),
            format_duration(self.analysed_seconds)
        )
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|e| format!("{{\"error\":\"{e}\"}}"))
    }

    /// One line per file, which is what a batch prints instead of the block.
    ///
    /// It names the same fields in the same order and units as the block and
    /// drops what a listing cannot afford: the bin's frequency, the floor, and
    /// the render's pixel size, which is the same for every file in the run.
    pub fn write_compact(
        &self,
        out: &mut impl Write,
        index: usize,
        total: usize,
    ) -> std::io::Result<()> {
        write!(
            out,
            "[{index}/{total}] {}: {}, {}",
            self.file_name(),
            self.source_fields(Detail::Compact),
            self.level_fields(Detail::Compact)
        )?;
        if let Some(advice) = self.range_advice() {
            write!(out, ", {advice}")?;
        }
        let Some(output) = &self.output else {
            return writeln!(out);
        };
        writeln!(
            out,
            " → {}, {}, {}",
            self.output_label(output),
            format_bytes(output.bytes),
            format_duration(self.elapsed_seconds)
        )
    }

    /// Where the render went, with the input's own path folded into a `*`.
    ///
    /// A batch always writes `<input>.png`, so spelling the whole path out
    /// again buries the part of the line that says anything.
    fn output_label(&self, output: &OutputReport) -> String {
        match output.path.strip_prefix(&self.file) {
            Some(suffix) => format!("*{suffix}"),
            None => output.path.clone(),
        }
    }

    /// The block printed to stderr when the run finishes.
    ///
    /// Two sections, each headed by the name of the file it describes: what
    /// the signal measured, then what the picture shows.
    pub fn write_block(&self, out: &mut impl Write, detail: Detail) -> std::io::Result<()> {
        writeln!(out, "{}:", self.file_name())?;
        writeln!(out, "  {}", self.source_fields(detail))?;
        if detail == Detail::Verbose {
            writeln!(
                out,
                "  normalize {}, gain {:+.1} dB",
                self.normalize, self.gain_db
            )?;
        }
        writeln!(out, "  {}", self.level_fields(detail))?;

        let Some(output) = &self.output else {
            return Ok(());
        };
        writeln!(out, "{}:", self.render_header(output, detail))?;
        writeln!(out, "  {}", self.transform_fields(detail))?;
        writeln!(
            out,
            "  {}×{}, {}, {}",
            output.width,
            output.height,
            format_bytes(output.bytes),
            format_duration(self.elapsed_seconds)
        )
    }

    /// How the file was read.
    fn source_fields(&self, detail: Detail) -> String {
        let mut fields = format!(
            "{} {}, {}, {}",
            self.container,
            self.sample_type,
            format_hz(self.sample_rate),
            format_duration(self.duration_seconds)
        );
        if (self.analysed_seconds - self.duration_seconds).abs() > 1e-6 {
            fields.push_str(&format!(
                ", analysed {}",
                format_duration(self.analysed_seconds)
            ));
        }
        if detail != Detail::Verbose {
            return fields;
        }
        fields.push_str(&format!(", {}", format_samples(self.samples)));
        // The divisor is one property of the file, so it belongs here rather
        // than hung off each of the levels below.
        if let Some(scale) = self.full_scale() {
            fields.push_str(&format!(", full scale {scale}"));
        }
        fields
    }

    /// What the signal measured, on the scale the range decision is made on.
    fn level_fields(&self, detail: Detail) -> String {
        let mut fields = format!("peak {}", self.level(&self.peak_sample, detail));
        if let Some(peak) = &self.peak_bin {
            fields.push_str(&format!(", bin {}", self.level(&peak.level, detail)));
            fields.push_str(&self.bin_frequency(peak, detail));
        }
        if let Some(floor) = &self.floor
            && detail != Detail::Compact
        {
            fields.push_str(&format!(", floor {}", self.level(floor, detail)));
        }
        fields.push_str(" dBFS");
        fields
    }

    /// Where the peak bin sits, which a compact line has no room for.
    fn bin_frequency(&self, peak: &PeakBin, detail: Detail) -> String {
        if detail == Detail::Compact {
            return String::new();
        }
        let offset = signed_hz(peak.offset_hz);
        if self.center_freq == 0.0 {
            return format!(" @ {offset}");
        }
        format!(" @ {offset} ({})", format_hz(peak.freq_hz))
    }

    /// A level as the report gives it: decibels, with the file's own units
    /// beside them once `-v` asks for that detail.
    fn level(&self, level: &Level, detail: Detail) -> String {
        if detail != Detail::Verbose {
            return format!("{:+.1}", level.dbfs);
        }
        format!("{:+.1} ({})", level.dbfs, absolute(level.absolute))
    }

    /// What one unit of level is worth in the file's own counts.
    ///
    /// The test is the divisor's size, not the sample format: an integer
    /// format brings its own full scale, and `--normalize` measures one for a
    /// float capture that was never scaled to `[-1, 1]`. Either is worth
    /// naming. A divisor near one is not a count at all, and printing
    /// `full scale 1` would say nothing.
    fn full_scale(&self) -> Option<String> {
        (self.divisor >= 128.0).then(|| format!("{:.0}", self.divisor))
    }

    /// The transform that drew the picture, and the window it drew it in.
    ///
    /// The two fields `-v` adds sit either side of the range, so the line is
    /// built whole rather than appended to in four steps.
    fn transform_fields(&self, detail: Detail) -> String {
        let verbose = detail == Detail::Verbose;
        let reduce = if verbose {
            format!(", reduce {}", self.stft.reduce)
        } else {
            String::new()
        };
        let mode = if verbose {
            format!(" ({})", self.stft.dynamic_range_mode)
        } else {
            String::new()
        };
        let advice = self.range_advice().map_or_else(String::new, |advice| {
            format!(", {advice} to fit the drawn range")
        });
        format!(
            "fft {}, {}, hop {}, {} frames{reduce}, range {} dB{mode}{advice}",
            self.stft.fft_size,
            self.stft.window,
            self.stft.hop,
            self.stft.frames,
            self.stft.dynamic_range_db
        )
    }

    /// What the render section is headed by.
    ///
    /// A render written beside its input is identified by its own name, which
    /// lines the two headers up and can be copied whole. One sent elsewhere by
    /// `-o` is findable only by its full path, and `-v` always spells it out.
    ///
    /// That path is made absolute rather than repeated as it was typed: a
    /// relative `-o spec.png` against an input in another directory would
    /// otherwise print a bare name, which reads exactly like the render that
    /// does sit beside its input.
    fn render_header(&self, output: &OutputReport, detail: Detail) -> String {
        let path = Path::new(&output.path);
        let beside_input = path.parent() == Path::new(&self.file).parent();
        if detail != Detail::Verbose && beside_input {
            return file_name_of(&output.path);
        }
        std::path::absolute(path).map_or_else(
            |_| output.path.clone(),
            |absolute| absolute.display().to_string(),
        )
    }
}

impl Report {
    fn suggested_range_db(&self) -> Option<f32> {
        if self.stft.dynamic_range_mode == "auto" {
            return None;
        }
        if self.stft.dynamic_range_db - self.stft.recommended_dynamic_range_db < 10.0 {
            return None;
        }
        Some(self.stft.recommended_dynamic_range_db)
    }

    /// Suggest the measured range when the selected one is wider by at least
    /// one whole 10 dB recommendation step.
    ///
    /// This is the image footer's spelling, where the width is measured and
    /// fitted; the console has room for a verb.
    pub fn range_suggestion(&self) -> Option<String> {
        self.suggested_range_db()
            .map(|range| format!("sugg -d {range:.0}"))
    }

    /// The same suggestion as the console report spells it.
    fn range_advice(&self) -> Option<String> {
        self.suggested_range_db()
            .map(|range| format!("try -d {range:.0}"))
    }
}

/// Level in the units the file stores, sized to the format.
///
/// A weak bin can be a fraction of one count, so the precision follows the
/// magnitude: printing `0` for a real measurement is worse than printing
/// nothing.
fn absolute(value: f32) -> String {
    let value = value.abs();
    let digits = if value >= 1000.0 {
        0
    } else if value >= 10.0 {
        1
    } else if value >= 0.01 {
        3
    } else {
        6
    };
    format!("{value:.digits$}")
}

/// The name a section header carries, which is a file's own name and not the
/// directory the caller happened to type.
fn file_name_of(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string())
}

fn describe_normalize(mode: Normalize) -> String {
    match mode {
        Normalize::None => "none".to_string(),
        Normalize::Auto => "auto".to_string(),
        Normalize::Factor(v) => format!("{v}"),
    }
}

fn signed_hz(hz: f64) -> String {
    if hz >= 0.0 {
        format!("+{}", format_hz(hz))
    } else {
        format_hz(hz)
    }
}

#[cfg(test)]
mod tests {
    include!("report_tests.rs");
}
