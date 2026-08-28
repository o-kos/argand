//! `aspec` -- render a signal file's spectrum to a PNG.
//!
//! The work happens in `argand-core`, `argand-io` and `argand-dsp`; this
//! binary only parses arguments, drives progress and turns the core's RGBA
//! buffer into an image. That split is the point: a GPUI front end will
//! consume exactly the same outputs, so anything that leaks image handling
//! into the core here would leak toolkit types there.

mod cli;
mod inputs;
mod mask;
mod render;
mod report;
mod text;

use std::io::{IsTerminal, Write};
use std::path::Path;
use std::time::Instant;

use anyhow::{Context, Result, bail};
use argand_core::{SampleRange, SignalMeta, format_duration};
use argand_dsp::{AnalysisRequest, StftConfig, analyze};
use argand_io::{Normalize, OpenHints};
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};

use crate::cli::Args;
use crate::render::{Layout, PlotInput};
use crate::report::{Report, Scaling};

fn main() -> Result<()> {
    let args = Args::parse();
    init_tracing(args.verbose);

    match run(&args) {
        Ok(0) => Ok(()),
        // A batch has already said which files failed and why.
        Ok(_) => std::process::exit(1),
        Err(e) => {
            // clap already prints usage for argument errors; this covers the rest.
            eprintln!("error: {e:#}");
            std::process::exit(1);
        }
    }
}

/// Resolve the inputs, process each one, and report how many failed.
///
/// A single file keeps the exit path it always had: its error travels out of
/// here and is printed once. A batch prints each failure as it happens and
/// carries on, because one unreadable capture is no reason to skip the rest.
fn run(args: &Args) -> Result<usize> {
    let started = Instant::now();
    let files = inputs::resolve(&args.inputs)?;

    let total = files.len();
    let batch = total > 1;
    let reporting = Reporting::new(args, batch, total);
    if batch && args.output.is_some() {
        bail!(
            "--output names one PNG but {total} files resolved; \
             drop -o to write <input>.png beside each input"
        );
    }

    let mut failed = 0;
    for (i, file) in files.iter().enumerate() {
        let index = i + 1;
        match process(args, file, index, total) {
            Ok(report) => write_report(&report, &reporting, index),
            Err(e) if batch => {
                failed += 1;
                // Errors survive --quiet: a silent failure is worse than noise.
                eprintln!("[{index}/{total}] {}  error: {e:#}", name_of(file));
            }
            Err(e) => return Err(e),
        }
    }

    if batch && !args.quiet {
        eprintln!(
            "  processed {total} · {} succeeded · {failed} failed · {}",
            total - failed,
            format_duration(started.elapsed().as_secs_f64())
        );
    }
    Ok(failed)
}

/// Analyse one file and write its PNG.
fn process(args: &Args, input: &Path, index: usize, total: usize) -> Result<Report> {
    let started = Instant::now();

    let hints = OpenHints {
        raw: args.raw,
        sample_type: args.sample_type,
        sample_rate: args.rate,
        center_freq: args.center,
        byte_offset: args.offset,
        normalize: args.normalize,
        gain_db: args.gain,
    };

    let mut source = argand_io::open(input, &hints)?;
    let meta = source.meta().clone();
    let range = resolve_range(&meta, args)?;

    let cfg = StftConfig {
        fft_size: args.fft_size,
        hop: args.hop.unwrap_or((args.fft_size / 4).max(1)),
        window: args.window_type,
    };

    let (width, height) = args.image_size;
    let layout = Layout::compute(width, height, args.panels, args.orientation);
    let (transform_w, transform_h) = layout.transform_size();
    if transform_w == 0 || transform_h == 0 {
        bail!("image {width}x{height} leaves no room for the plot; try a larger --image-size");
    }

    // Built once and handed to both the transform and the report, so the
    // report cannot end up describing settings the transform never used.
    let request = AnalysisRequest {
        cfg,
        range,
        width: transform_w,
        height: transform_h,
        reduce: args.reduce,
        colormap: args.color_scheme,
        dynamic_range_db: args.dynamic_range,
        reference: args.reference,
        waveform_columns: layout.waveform_columns(),
    };

    let progress = make_progress(args, index, total);
    let analysis = analyze(source.as_mut(), &request, &mut |done, total| {
        progress.set_length(total);
        progress.set_position(done);
    })
    .context("computing the spectrum")?;
    progress.finish_and_clear();

    let scaling = Scaling {
        normalize: args
            .normalize
            .unwrap_or_else(|| Normalize::default_for(meta.sample_type.format)),
        gain_db: args.gain,
    };
    let mut report = Report::new(&meta, &analysis, &request, scaling);

    let canvas = render::render(
        &layout,
        &PlotInput {
            analysis: &analysis,
            title: &report.plot_title(),
            footer: &report.plot_footer(),
            colormap: args.color_scheme,
            waveform_full_scale: render::waveform_full_scale(analysis.time_peak, args.reference),
        },
    );

    let output = args.output_path(input);
    canvas
        .save(&output)
        .with_context(|| format!("writing {}", output.display()))?;
    let bytes = std::fs::metadata(&output).map(|m| m.len()).unwrap_or(0);

    report = report.with_output(&output, width, height, bytes, args.panels.to_string());
    report.elapsed_seconds = started.elapsed().as_secs_f64();
    Ok(report)
}

/// What a finished file puts on stdout, which is the machine-readable stream.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum StdoutLine {
    /// The whole report, for whatever is parsing it.
    Json,
    /// Only the render's path, so the run can be piped.
    Path,
    Nothing,
}

/// What a finished file puts on stderr, which is the stream a person reads.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum StderrBlock {
    /// One line per file, for a batch.
    Compact,
    /// The full multi-line block.
    Human,
    Nothing,
}

/// How the run reports itself, decided once and used for every file.
struct Reporting {
    /// More than one file resolved, so the block shrinks to a line.
    batch: bool,
    total: usize,
    stdout: StdoutLine,
    stderr: StderrBlock,
}

/// Pick the stdout stream's content.
///
/// `--json` is a request for machine output and outranks `--quiet`; anything
/// else on stdout is suppressed by either `--quiet` or a caller that does not
/// want paths echoed.
fn stdout_line(args: &Args, echo_paths: bool) -> StdoutLine {
    if args.json {
        return StdoutLine::Json;
    }
    if args.quiet || !echo_paths {
        return StdoutLine::Nothing;
    }
    StdoutLine::Path
}

/// Pick the stderr stream's content.
///
/// `--quiet` silences it outright. A batch shrinks the block to one line per
/// file unless `-v` asks for the block back.
fn stderr_block(args: &Args, batch: bool) -> StderrBlock {
    if args.quiet {
        return StderrBlock::Nothing;
    }
    if batch && args.verbose == 0 {
        return StderrBlock::Compact;
    }
    StderrBlock::Human
}

impl Reporting {
    /// Settle both output decisions up front, so that printing a file is a
    /// lookup rather than a re-derivation.
    ///
    /// The two streams are decided independently, which is the point: reading
    /// either rule out of interleaved conditions is what made the previous
    /// shape hard to trust.
    fn new(args: &Args, batch: bool, total: usize) -> Self {
        // A batch on a terminal already says where every render went, so
        // echoing the path on stdout only doubles the lines and reads like a
        // file the mask swept up. Piped, those paths are the point of stdout.
        let echo_paths = !batch || !std::io::stdout().is_terminal();

        Self {
            batch,
            total,
            stdout: stdout_line(args, echo_paths),
            stderr: stderr_block(args, batch),
        }
    }
}

/// Say what one finished file produced, in whichever mode was asked for.
fn write_report(report: &Report, reporting: &Reporting, index: usize) {
    match reporting.stdout {
        StdoutLine::Json => println!("{}", report.to_json()),
        StdoutLine::Path => {
            if let Some(output) = &report.output {
                println!("{}", output.path);
            }
        }
        StdoutLine::Nothing => {}
    }

    let mut stderr = std::io::stderr().lock();
    match reporting.stderr {
        StderrBlock::Compact => {
            report
                .write_compact(&mut stderr, index, reporting.total)
                .ok();
        }
        StderrBlock::Human => {
            let follows_another_block = reporting.batch && index > 1;
            if follows_another_block {
                writeln!(stderr).ok();
            }
            report.write_human(&mut stderr).ok();
        }
        StderrBlock::Nothing => {}
    }
}

fn name_of(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

/// Turn `--start` and `--duration` into a sample span.
fn resolve_range(meta: &SignalMeta, args: &Args) -> Result<SampleRange> {
    if meta.sample_rate <= 0.0 {
        bail!("sample rate is {}; pass --rate", meta.sample_rate);
    }
    if meta.len_samples == 0 {
        bail!("{} holds no samples", meta.source.display());
    }

    let to_samples = |seconds: f64| (seconds * meta.sample_rate).round().max(0.0) as u64;
    let start = args.start.map(to_samples).unwrap_or(0);
    if start >= meta.len_samples {
        bail!(
            "--start is past the end of the signal ({} long)",
            argand_core::format_duration(meta.duration_seconds())
        );
    }

    let len = args
        .duration
        .map(to_samples)
        .unwrap_or(meta.len_samples - start);
    Ok(SampleRange::new(start, len).clamped_to(meta.len_samples))
}

fn make_progress(args: &Args, index: usize, total: usize) -> ProgressBar {
    if args.quiet || !std::io::stderr().is_terminal() {
        return ProgressBar::hidden();
    }
    // A batch of one long capture still has to show that it is alive, so the
    // bar stays and only gains the counter telling you how far along it is.
    let counter = if total > 1 {
        format!("[{index}/{total}] ")
    } else {
        String::new()
    };
    let bar = ProgressBar::new(1);
    if let Ok(style) = ProgressStyle::with_template(&format!(
        "  {counter}stft  [{{bar:28.cyan/blue}}] {{percent:>3}}%  {{pos}}/{{len}} frames  eta {{eta}}"
    )) {
        bar.set_style(style.progress_chars("##-"));
    }
    bar
}

fn init_tracing(verbosity: u8) {
    let level = match verbosity {
        0 => "warn",
        1 => "info",
        2 => "debug",
        _ => "trace",
    };
    let filter = tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        tracing_subscriber::EnvFilter::new(format!(
            "aspec={level},argand_io={level},argand_dsp={level}"
        ))
    });
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .try_init();
}

#[cfg(test)]
mod tests {
    include!("main_tests.rs");
}
