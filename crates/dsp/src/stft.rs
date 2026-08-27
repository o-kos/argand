//! Short-time Fourier transform over a streamed signal.
//!
//! Two things here are not what a general-purpose audio spectrogram does, and
//! both matter for I/Q captures:
//!
//! * A complex signal goes into a complex FFT and the result is `fftshift`ed,
//!   producing a genuinely two-sided spectrum from -Fs/2 to +Fs/2. Feeding an
//!   interleaved `I, Q, I, Q` stream to a real transform instead -- which is
//!   the easy mistake -- yields a one-sided spectrum over a frequency axis
//!   that is wrong by a factor of two.
//! * Frames are folded into image columns as they are computed, so memory is
//!   bounded by the output size rather than by the length of the capture. A
//!   half-hour capture produces tens of thousands of frames; holding them all
//!   would cost more than the samples themselves.

use argand_core::{
    Colormap, Psd, SampleRange, SampleSource, SignalMeta, SourceError, SpectrogramImage,
    WaveformEnvelope, gradient_index,
};
use rayon::prelude::*;
use realfft::{RealFftPlanner, RealToComplex};
use rustfft::num_complex::Complex32;
use rustfft::{Fft, FftPlanner};
use std::sync::Arc;

use crate::waveform::EnvelopeBuilder;
use crate::window::{Window, WindowTable};

/// Samples pulled from the source per block. Large enough that the read cost
/// disappears against the transforms, small enough to stay cache-friendly.
#[cfg(not(test))]
const BLOCK_SAMPLES: usize = 1 << 20;
/// Small under test so that the block-boundary carry is actually exercised.
#[cfg(test)]
const BLOCK_SAMPLES: usize = 4096;

/// Magnitude floor, about -300 dB, so that silence cannot reach `-inf`.
const MAG_FLOOR: f32 = 1e-15;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StftConfig {
    pub fft_size: usize,
    pub hop: usize,
    pub window: Window,
}

impl StftConfig {
    /// Frames advance by a quarter of the transform, matching sgvr's default.
    pub fn new(fft_size: usize, window: Window) -> Self {
        Self {
            fft_size,
            hop: (fft_size / 4).max(1),
            window,
        }
    }

    pub fn overlap_percent(&self) -> f64 {
        if self.fft_size == 0 {
            return 0.0;
        }
        100.0 * (1.0 - self.hop as f64 / self.fft_size as f64)
    }
}

/// How several frames sharing one image column are combined.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reduce {
    /// Keep the strongest. Short bursts survive being scaled down.
    Max,
    /// Average. Smoother, but a brief signal can vanish into the floor.
    Mean,
}

pub const REDUCE_NAMES: [&str; 2] = ["max", "mean"];

/// What 0 dB on the colour scale means.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DbReference {
    /// The format's full scale. Two files can be compared directly.
    FullScale,
    /// The loudest bin in this file. Always uses the full colour range.
    Peak,
}

pub const DB_REFERENCE_NAMES: [&str; 2] = ["fs", "peak"];

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("unknown {what} `{name}`, expected one of: {options}")]
pub struct ParseEnumError {
    pub what: &'static str,
    pub name: String,
    pub options: String,
}

impl Reduce {
    pub const fn as_str(self) -> &'static str {
        match self {
            Reduce::Max => "max",
            Reduce::Mean => "mean",
        }
    }
}

impl std::str::FromStr for Reduce {
    type Err = ParseEnumError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "max" => Ok(Reduce::Max),
            "mean" | "avg" | "average" => Ok(Reduce::Mean),
            _ => Err(ParseEnumError {
                what: "reduce mode",
                name: s.to_string(),
                options: REDUCE_NAMES.join(", "),
            }),
        }
    }
}

impl std::fmt::Display for Reduce {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl DbReference {
    pub const fn as_str(self) -> &'static str {
        match self {
            DbReference::FullScale => "fs",
            DbReference::Peak => "peak",
        }
    }
}

impl std::str::FromStr for DbReference {
    type Err = ParseEnumError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "fs" | "full-scale" | "fullscale" => Ok(DbReference::FullScale),
            "peak" => Ok(DbReference::Peak),
            _ => Err(ParseEnumError {
                what: "dB reference",
                name: s.to_string(),
                options: DB_REFERENCE_NAMES.join(", "),
            }),
        }
    }
}

impl std::fmt::Display for DbReference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DspError {
    #[error("reading samples")]
    Source(#[from] SourceError),
    #[error("fft size must be a power of two of at least 2, got {0}")]
    BadFftSize(usize),
    #[error("hop must be at least 1")]
    BadHop,
    #[error("output size must be at least 1x1, got {width}x{height}")]
    BadOutputSize { width: usize, height: usize },
    #[error(
        "selection holds {samples} samples, which is fewer than the {fft_size}-point transform"
    )]
    TooShort { samples: u64, fft_size: usize },
}

/// Everything one pass over the signal produces.
pub struct Analysis {
    pub spectrogram: SpectrogramImage,
    pub psd: Psd,
    /// Time-domain envelope, present only when one was asked for.
    pub waveform: Option<WaveformEnvelope>,
    /// Largest absolute sample value seen, on the unit scale.
    pub time_peak: f32,
    pub frames: u64,
    /// Equivalent noise bandwidth of the window, in hertz.
    pub enbw_hz: f64,
}

/// What one pass over the signal should produce.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnalysisRequest {
    pub cfg: StftConfig,
    pub range: SampleRange,
    /// Spectrogram size in pixels, on the transform's own (time, frequency)
    /// axes rather than the image's.
    pub width: usize,
    pub height: usize,
    pub reduce: Reduce,
    pub colormap: Colormap,
    pub dynamic_range_db: f32,
    pub reference: DbReference,
    /// Columns of time-domain envelope to build, or `None` for no waveform.
    ///
    /// Matching this to `width` is what keeps a waveform panel aligned with
    /// the spectrogram it sits beside.
    pub waveform_columns: Option<usize>,
}

/// Run the transform over the requested range and render every view of it.
///
/// The spectrogram, the averaged spectrum and the time-domain envelope all
/// come from the same streamed blocks, so asking for all three costs one pass
/// over the file rather than three.
pub fn analyze(
    src: &mut dyn SampleSource,
    request: &AnalysisRequest,
    progress: &mut dyn FnMut(u64, u64),
) -> Result<Analysis, DspError> {
    let AnalysisRequest {
        cfg,
        range,
        width: out_width,
        height: out_height,
        reduce,
        colormap,
        dynamic_range_db,
        reference,
        waveform_columns,
    } = *request;
    let cfg = &cfg;

    if cfg.fft_size < 2 || !cfg.fft_size.is_power_of_two() {
        return Err(DspError::BadFftSize(cfg.fft_size));
    }
    if cfg.hop == 0 {
        return Err(DspError::BadHop);
    }
    if out_width == 0 || out_height == 0 {
        return Err(DspError::BadOutputSize {
            width: out_width,
            height: out_height,
        });
    }

    let meta = src.meta().clone();
    let range = range.clamped_to(meta.len_samples);
    if range.len < cfg.fft_size as u64 {
        return Err(DspError::TooShort {
            samples: range.len,
            fft_size: cfg.fft_size,
        });
    }

    let total_frames = (range.len - cfg.fft_size as u64) / cfg.hop as u64 + 1;
    let plan = Plan::new(cfg, &meta, out_height);
    let bins = plan.bins;

    let mut columns = ColumnStore::new(out_width, out_height, reduce);
    let mut power = vec![0.0f64; bins];
    let mut time_peak = 0.0f32;

    let channels = meta.channels();
    let mut envelope = waveform_columns
        .filter(|c| *c > 0)
        .map(|c| EnvelopeBuilder::new(c, channels, range.len));
    let mut buf = vec![0.0f32; BLOCK_SAMPLES.max(cfg.fft_size) * channels];
    let capacity = buf.len() / channels;
    let mut filled = 0usize;
    let mut remaining = range.len;
    let mut frame_base = 0u64;
    // Range index of `buf[0]`, and how much of the range the envelope has seen.
    let mut buf_start = 0u64;
    let mut folded = 0u64;

    src.seek(range.start)?;
    progress(0, total_frames);

    loop {
        // Top up the block, keeping whatever overlap the last pass left.
        while filled < capacity && remaining > 0 {
            let want = ((capacity - filled) as u64).min(remaining) as usize;
            let got = src.read(&mut buf[filled * channels..(filled + want) * channels])?;
            if got == 0 {
                remaining = 0;
                break;
            }
            let samples = got / channels;
            filled += samples;
            remaining -= samples as u64;
        }

        // Fold before the frame check: the last block can be shorter than one
        // transform and still hold samples the strip has to show.
        if let Some(builder) = envelope.as_mut() {
            let available = buf_start + filled as u64;
            if available > folded {
                let from = (folded - buf_start) as usize * channels;
                builder.fold(&buf[from..filled * channels], folded);
                folded = available;
            }
        }

        if filled < cfg.fft_size {
            break;
        }

        let frames_here =
            ((filled - cfg.fft_size) / cfg.hop + 1).min((total_frames - frame_base) as usize);
        if frames_here == 0 {
            break;
        }

        let partial = (0..frames_here)
            .into_par_iter()
            .fold(
                || Partial::new(bins, cfg.fft_size),
                |mut acc, k| {
                    let start = k * cfg.hop * channels;
                    let frame = &buf[start..start + cfg.fft_size * channels];
                    plan.frame(
                        frame,
                        &mut acc,
                        column_of(frame_base + k as u64, total_frames, out_width),
                    );
                    acc
                },
            )
            .reduce(|| Partial::new(bins, cfg.fft_size), Partial::merge);

        columns.absorb(&partial, out_height);
        for (slot, add) in power.iter_mut().zip(partial.power.iter()) {
            *slot += add;
        }
        time_peak = time_peak.max(partial.time_peak);

        frame_base += frames_here as u64;
        progress(frame_base, total_frames);

        // Carry the overlapping tail into the next block.
        let consumed = frames_here * cfg.hop;
        buf.copy_within(consumed * channels..filled * channels, 0);
        filled -= consumed;
        buf_start += consumed as u64;

        if frame_base >= total_frames || (remaining == 0 && filled < cfg.fft_size) {
            break;
        }
    }

    // The frame loop stops at the last whole transform, which can leave up to
    // one hop unread. The strip spans the same time axis as the spectrogram,
    // so those samples are read rather than left to a borrowed column.
    if let Some(builder) = envelope.as_mut() {
        while remaining > 0 {
            let want = remaining.min(capacity as u64) as usize;
            let got = src.read(&mut buf[..want * channels])?;
            if got == 0 {
                break;
            }
            let samples = got / channels;
            builder.fold(&buf[..samples * channels], folded);
            folded += samples as u64;
            remaining -= samples as u64;
        }
    }

    let frames = frame_base.max(1);
    let db = columns.finish();
    let observed_max = db
        .iter()
        .copied()
        .filter(|v| v.is_finite())
        .fold(f32::NEG_INFINITY, f32::max);

    let db_max = match reference {
        DbReference::FullScale => 0.0,
        DbReference::Peak => {
            if observed_max.is_finite() {
                observed_max
            } else {
                0.0
            }
        }
    };
    let db_min = db_max - dynamic_range_db;

    let spectrogram = render(
        &DbGrid {
            values: &db,
            width: out_width,
            height: out_height,
        },
        Shading {
            colormap,
            db_min,
            db_max,
        },
        &meta,
        &range,
    );

    let psd = Psd {
        freqs_hz: (0..bins).map(|i| plan.bin_freq(i, &meta)).collect(),
        db: power
            .iter()
            .map(|p| 10.0 * (p / frames as f64).max(1e-30).log10() as f32)
            .collect(),
        segments: frames,
    };

    Ok(Analysis {
        waveform: envelope.map(|b| {
            b.finish(
                range.start as f64 / meta.sample_rate,
                range.end() as f64 / meta.sample_rate,
            )
        }),
        spectrogram,
        psd,
        time_peak,
        frames: frame_base,
        enbw_hz: plan.window.enbw_hz(meta.sample_rate),
    })
}

/// Which image column a frame lands in.
fn column_of(frame: u64, total_frames: u64, width: usize) -> usize {
    if total_frames <= 1 {
        return 0;
    }
    ((frame * width as u64) / total_frames).min(width as u64 - 1) as usize
}

/// The transform, window and scaling for one configuration.
struct Plan {
    window: WindowTable,
    complex: Option<Arc<dyn Fft<f32>>>,
    real: Option<Arc<dyn RealToComplex<f32>>>,
    fft_size: usize,
    bins: usize,
    is_iq: bool,
    /// Divides the raw transform output so a full-scale tone reads 0 dBFS.
    amplitude_scale: f32,
    out_height: usize,
}

impl Plan {
    fn new(cfg: &StftConfig, meta: &SignalMeta, out_height: usize) -> Self {
        let window = WindowTable::new(cfg.window, cfg.fft_size);
        let is_iq = meta.is_iq();
        let bins = if is_iq {
            cfg.fft_size
        } else {
            cfg.fft_size / 2 + 1
        };

        let (complex, real) = if is_iq {
            (
                Some(FftPlanner::<f32>::new().plan_fft_forward(cfg.fft_size)),
                None,
            )
        } else {
            (
                None,
                Some(RealFftPlanner::<f32>::new().plan_fft_forward(cfg.fft_size)),
            )
        };

        Self {
            complex,
            real,
            fft_size: cfg.fft_size,
            bins,
            is_iq,
            amplitude_scale: 1.0 / (cfg.fft_size as f32 * window.coherent_gain),
            out_height,
            window,
        }
    }

    /// Frequency of bin `i` in hertz.
    fn bin_freq(&self, i: usize, meta: &SignalMeta) -> f64 {
        let step = meta.sample_rate / self.fft_size as f64;
        if self.is_iq {
            // After the shift, bin 0 sits at -Fs/2.
            meta.center_freq + (i as f64 - self.fft_size as f64 / 2.0) * step
        } else {
            meta.center_freq + i as f64 * step
        }
    }

    /// Transform one frame and fold the result into `acc`.
    fn frame(&self, samples: &[f32], acc: &mut Partial, column: usize) {
        acc.time_peak = samples
            .iter()
            .fold(acc.time_peak, |m, v| if v.abs() > m { v.abs() } else { m });

        let mags = &mut acc.mags;
        if self.is_iq {
            let fft = self.complex.as_ref().expect("complex plan");
            acc.complex.clear();
            acc.complex.extend(
                samples
                    .chunks_exact(2)
                    .zip(self.window.coefficients.iter())
                    .map(|(iq, &w)| Complex32::new(iq[0] * w, iq[1] * w)),
            );
            fft.process(&mut acc.complex);

            // fftshift: negative frequencies first.
            let half = self.fft_size / 2;
            for (i, slot) in mags.iter_mut().enumerate().take(self.fft_size) {
                let src = if i < half { i + half } else { i - half };
                *slot = acc.complex[src].norm() * self.amplitude_scale;
            }
        } else {
            let fft = self.real.as_ref().expect("real plan");
            for (dst, (s, &w)) in acc
                .real_in
                .iter_mut()
                .zip(samples.iter().zip(self.window.coefficients.iter()))
            {
                *dst = s * w;
            }
            // The transform only fails on a size mismatch, which cannot
            // happen: the buffers come from the same plan.
            let _ = fft.process(&mut acc.real_in, &mut acc.real_out);

            let last = self.bins - 1;
            for (i, slot) in mags.iter_mut().enumerate().take(self.bins) {
                // A real tone splits its energy between the positive and
                // negative bin; the one-sided view has to double it back,
                // except at DC and Nyquist, which are not mirrored.
                let mirror = if i == 0 || i == last { 1.0 } else { 2.0 };
                *slot = acc.real_out[i].norm() * self.amplitude_scale * mirror;
            }
        }

        for (slot, mag) in acc.power.iter_mut().zip(mags.iter().take(self.bins)) {
            let m = *mag as f64;
            *slot += m * m;
        }

        // Fold bins down to image rows, low frequency first.
        let row_base = acc.rows.len();
        acc.rows
            .resize(row_base + self.out_height, f32::NEG_INFINITY);
        for y in 0..self.out_height {
            let lo = y * self.bins / self.out_height;
            let hi = (((y + 1) * self.bins) / self.out_height)
                .max(lo + 1)
                .min(self.bins);
            let peak = mags[lo..hi].iter().fold(0.0f32, |m, v| m.max(*v));
            acc.rows[row_base + y] = 20.0 * peak.max(MAG_FLOOR).log10();
        }
        acc.cols.push(column as u32);
    }
}

/// Per-thread accumulator: scratch buffers plus the frames it has finished.
struct Partial {
    cols: Vec<u32>,
    /// `cols.len() * out_height` dB values, low frequency first.
    rows: Vec<f32>,
    power: Vec<f64>,
    time_peak: f32,
    mags: Vec<f32>,
    complex: Vec<Complex32>,
    real_in: Vec<f32>,
    real_out: Vec<Complex32>,
}

impl Partial {
    fn new(bins: usize, fft_size: usize) -> Self {
        Self {
            cols: Vec::new(),
            rows: Vec::new(),
            power: vec![0.0; bins],
            time_peak: 0.0,
            mags: vec![0.0; bins],
            complex: Vec::new(),
            real_in: vec![0.0; fft_size],
            real_out: vec![Complex32::new(0.0, 0.0); fft_size / 2 + 1],
        }
    }

    fn merge(mut self, other: Self) -> Self {
        self.cols.extend_from_slice(&other.cols);
        self.rows.extend_from_slice(&other.rows);
        for (slot, add) in self.power.iter_mut().zip(other.power.iter()) {
            *slot += add;
        }
        self.time_peak = self.time_peak.max(other.time_peak);
        self
    }
}

/// The image-sized dB accumulator that frames are folded into.
struct ColumnStore {
    width: usize,
    height: usize,
    reduce: Reduce,
    values: Vec<f32>,
    counts: Vec<u32>,
}

impl ColumnStore {
    fn new(width: usize, height: usize, reduce: Reduce) -> Self {
        Self {
            width,
            height,
            reduce,
            values: vec![f32::NEG_INFINITY; width * height],
            counts: vec![0; width],
        }
    }

    fn absorb(&mut self, partial: &Partial, height: usize) {
        for (frame, &col) in partial.cols.iter().enumerate() {
            let col = col as usize;
            if col >= self.width {
                continue;
            }
            let src = &partial.rows[frame * height..(frame + 1) * height];
            let dst = &mut self.values[col * self.height..(col + 1) * self.height];
            match self.reduce {
                Reduce::Max => {
                    for (d, s) in dst.iter_mut().zip(src) {
                        if *s > *d {
                            *d = *s;
                        }
                    }
                }
                Reduce::Mean => {
                    for (d, s) in dst.iter_mut().zip(src) {
                        *d = if d.is_finite() { *d + *s } else { *s };
                    }
                }
            }
            self.counts[col] += 1;
        }
    }

    /// Column-major dB values, low frequency first within each column.
    fn finish(mut self) -> Vec<f32> {
        if matches!(self.reduce, Reduce::Mean) {
            for col in 0..self.width {
                let n = self.counts[col];
                if n > 1 {
                    for v in &mut self.values[col * self.height..(col + 1) * self.height] {
                        *v /= n as f32;
                    }
                }
            }
        }
        // A column no frame landed in borrows its neighbour rather than
        // showing as a black stripe.
        for col in 0..self.width {
            if self.counts[col] == 0 && col > 0 {
                let (left, right) = self.values.split_at_mut(col * self.height);
                right[..self.height]
                    .copy_from_slice(&left[(col - 1) * self.height..col * self.height]);
            }
        }
        self.values
    }
}

/// Decibel values and the shape they are laid out in.
///
/// The slice is column-major: `values[x * height + bin]`. Carrying the two
/// dimensions next to the slice is what keeps that readable at the one place
/// it is indexed.
struct DbGrid<'a> {
    values: &'a [f32],
    width: usize,
    height: usize,
}

/// How a decibel value becomes a colour.
#[derive(Clone, Copy)]
struct Shading {
    colormap: Colormap,
    db_min: f32,
    db_max: f32,
}

fn render(
    grid: &DbGrid<'_>,
    shading: Shading,
    meta: &SignalMeta,
    range: &SampleRange,
) -> SpectrogramImage {
    let DbGrid {
        values,
        width,
        height,
    } = *grid;
    let Shading {
        colormap,
        db_min,
        db_max,
    } = shading;

    let gradient = colormap.gradient();
    let span = (db_max - db_min).max(1e-6);
    let mut image = SpectrogramImage::new(width, height);

    for x in 0..width {
        for y in 0..height {
            // Row 0 is the top of the image, which is the highest frequency.
            let value = values[x * height + (height - 1 - y)];
            let normalized = if value.is_finite() {
                (value - db_min) / span
            } else {
                0.0
            };
            image.put(x, y, gradient[gradient_index(normalized)]);
        }
    }

    let (f0, f1) = meta.frequency_span();
    image.t0 = range.start as f64 / meta.sample_rate;
    image.t1 = range.end() as f64 / meta.sample_rate;
    image.f0 = f0;
    image.f1 = f1;
    image.db_min = db_min;
    image.db_max = db_max;
    image
}

#[cfg(test)]
mod tests {
    include!("stft_tests.rs");
}
