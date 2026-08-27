//! `aspec` -- render a signal file's spectrum to a PNG.
//!
//! The work happens in `argand-core`, `argand-io` and `argand-dsp`; this
//! binary only parses arguments, drives progress and turns the core's RGBA
//! buffer into an image. That split is the point: a GPUI front end will
//! consume exactly the same outputs, so anything that leaks image handling
//! into the core here would leak toolkit types there.

mod cli;
mod render;
mod report;
mod text;

use std::io::IsTerminal;
use std::time::Instant;

use anyhow::{Context, Result, bail};
use argand_core::{SampleRange, SignalMeta};
use argand_dsp::{AnalysisRequest, StftConfig, analyze};
use argand_io::{Normalize, OpenHints};
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};

use crate::cli::Args;
use crate::render::{Layout, PlotInput};
use crate::report::Report;

fn main() -> Result<()> {
    let args = Args::parse();
    init_tracing(args.verbose);

    if let Err(e) = run(&args) {
        // clap already prints usage for argument errors; this covers the rest.
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
    Ok(())
}

fn run(args: &Args) -> Result<()> {
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

    let mut source = argand_io::open(&args.input, &hints)
        .with_context(|| format!("opening {}", args.input.display()))?;
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

    let progress = make_progress(args);
    let analysis = analyze(
        source.as_mut(),
        &AnalysisRequest {
            cfg,
            range,
            width: transform_w,
            height: transform_h,
            reduce: args.reduce,
            colormap: args.color_scheme,
            dynamic_range_db: args.dynamic_range,
            reference: args.reference,
            waveform_columns: layout.waveform_columns(),
        },
        &mut |done, total| {
            progress.set_length(total);
            progress.set_position(done);
        },
    )
    .context("computing the spectrum")?;
    progress.finish_and_clear();

    let effective_normalize = args
        .normalize
        .unwrap_or_else(|| Normalize::default_for(meta.sample_type.format));
    let mut report = Report::new(
        &meta,
        &analysis,
        &cfg,
        args.reduce,
        args.reference,
        args.dynamic_range,
        effective_normalize,
        args.gain,
        range.len as f64 / meta.sample_rate,
    );

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

    let output = args.output_path();
    canvas
        .save(&output)
        .with_context(|| format!("writing {}", output.display()))?;
    let bytes = std::fs::metadata(&output).map(|m| m.len()).unwrap_or(0);

    report = report.with_output(&output, width, height, bytes, args.panels.to_string());
    report.elapsed_seconds = started.elapsed().as_secs_f64();

    if args.json {
        println!("{}", report.to_json());
    } else if !args.quiet {
        println!("{}", output.display());
    }
    if !args.quiet {
        let mut stderr = std::io::stderr().lock();
        report.write_human(&mut stderr).ok();
    }

    Ok(())
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

fn make_progress(args: &Args) -> ProgressBar {
    if args.quiet || !std::io::stderr().is_terminal() {
        return ProgressBar::hidden();
    }
    let bar = ProgressBar::new(1);
    if let Ok(style) = ProgressStyle::with_template(
        "  stft  [{bar:28.cyan/blue}] {percent:>3}%  {pos}/{len} frames  eta {eta}",
    ) {
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
