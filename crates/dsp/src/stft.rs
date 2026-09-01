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

/// Magnitude floor, so that silence cannot reach `-inf`.
const MAG_FLOOR: f32 = 1e-15;
/// Power floor for the averaged spectrum, which is the same floor in power.
const POWER_FLOOR: f64 = 1e-30;

/// The lowest decibel any transform here can produce.
///
/// Both floors above come out at this level: silence is clamped rather than
/// left to reach `-inf`, so no axis this crate feeds can print a value below
/// it. A renderer reserving room for decibel labels can bound them with this
/// instead of guessing at one.
pub const DB_FLOOR: f32 = -300.0;

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

/// The sliding window of samples the frame loop transforms.
///
/// Reading, envelope folding and carrying the overlap all move the same few
/// indices in step, so they belong together rather than as loose locals in the
/// middle of the loop.
struct Block<'a> {
    src: &'a mut dyn SampleSource,
    buf: Vec<f32>,
    channels: usize,
    /// Samples the buffer holds when full.
    capacity: usize,
    /// Samples currently in the buffer.
    filled: usize,
    /// Samples of the range not yet read.
    remaining: u64,
    /// Range index of `buf[0]`.
    start: u64,
    /// How much of the range the envelope has already seen.
    folded: u64,
}

impl<'a> Block<'a> {
    fn new(src: &'a mut dyn SampleSource, fft_size: usize, channels: usize, len: u64) -> Self {
        let buf = vec![0.0f32; BLOCK_SAMPLES.max(fft_size) * channels];
        let capacity = buf.len() / channels;
        Self {
            src,
            buf,
            channels,
            capacity,
            filled: 0,
            remaining: len,
            start: 0,
            folded: 0,
        }
    }

    /// Top up the buffer, keeping whatever overlap the last pass left.
    fn fill(&mut self) -> Result<(), DspError> {
        while self.filled < self.capacity && self.remaining > 0 {
            let want = ((self.capacity - self.filled) as u64).min(self.remaining) as usize;
            let from = self.filled * self.channels;
            let to = (self.filled + want) * self.channels;
            let got = self.src.read(&mut self.buf[from..to])?;
            if got == 0 {
                self.remaining = 0;
                break;
            }
            let samples = got / self.channels;
            self.filled += samples;
            self.remaining -= samples as u64;
        }
        Ok(())
    }

    /// Give the envelope whatever the buffer holds that it has not seen.
    fn fold_into(&mut self, builder: &mut EnvelopeBuilder) {
        let available = self.start + self.filled as u64;
        if available <= self.folded {
            return;
        }
        let from = (self.folded - self.start) as usize * self.channels;
        builder.fold(&self.buf[from..self.filled * self.channels], self.folded);
        self.folded = available;
    }

    /// Drop the samples the frames consumed, sliding the overlap to the front.
    fn carry(&mut self, consumed: usize) {
        self.buf
            .copy_within(consumed * self.channels..self.filled * self.channels, 0);
        self.filled -= consumed;
        self.start += consumed as u64;
    }

    /// Read the rest of the range for the envelope alone.
    ///
    /// The frame loop stops at the last whole transform, which can leave up to
    /// one hop unread. The strip spans the same time axis as the spectrogram,
    /// so those samples are read rather than left to a borrowed column.
    fn drain_into(&mut self, builder: &mut EnvelopeBuilder) -> Result<(), DspError> {
        while self.remaining > 0 {
            let want = self.remaining.min(self.capacity as u64) as usize;
            let got = self.src.read(&mut self.buf[..want * self.channels])?;
            if got == 0 {
                break;
            }
            let samples = got / self.channels;
            builder.fold(&self.buf[..samples * self.channels], self.folded);
            self.folded += samples as u64;
            self.remaining -= samples as u64;
        }
        Ok(())
    }
}

/// Close the envelope over the span that was analysed, if one was built.
fn finish_envelope(
    envelope: Option<EnvelopeBuilder>,
    range: SampleRange,
    sample_rate: f64,
) -> Option<WaveformEnvelope> {
    envelope.map(|b| {
        b.finish(
            range.start as f64 / sample_rate,
            range.end() as f64 / sample_rate,
        )
    })
}

/// The spectrum averaged over every frame, in dB against full scale.
///
/// The floor keeps a silent bin out of `log10(0)`; `frames` is at least one,
/// so a capture that produced no frame still divides safely.
fn averaged_spectrum(plan: &Plan, meta: &SignalMeta, power: &[f64], frames: u64) -> Psd {
    Psd {
        freqs_hz: (0..plan.bins).map(|i| plan.bin_freq(i, meta)).collect(),
        db: power
            .iter()
            .map(|p| 10.0 * (p / frames as f64).max(POWER_FLOOR).log10() as f32)
            .collect(),
        segments: frames,
    }
}

/// Reject a request the transform cannot run before any of it is set up.
fn check_request(cfg: &StftConfig, out_width: usize, out_height: usize) -> Result<(), DspError> {
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
    Ok(())
}

/// The decibel range the colours span, as `(min, max)`.
///
/// Full scale pins the top at 0 dBFS so two files are comparable. `Peak` pins
/// it to this file's loudest bin, falling back to full scale when nothing
/// finite was measured.
fn db_window(db: &[f32], reference: DbReference, dynamic_range_db: f32) -> (f32, f32) {
    let observed_max = db
        .iter()
        .copied()
        .filter(|v| v.is_finite())
        .fold(f32::NEG_INFINITY, f32::max);

    let db_max = match reference {
        DbReference::Peak if observed_max.is_finite() => observed_max,
        DbReference::Peak | DbReference::FullScale => 0.0,
    };
    (db_max - dynamic_range_db, db_max)
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

    check_request(cfg, out_width, out_height)?;

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

    let mut store = ColumnStore::new(out_width, out_height, reduce);
    let columns = Columns::new(total_frames, out_width);
    let mut power = vec![0.0f64; bins];
    let mut time_peak = 0.0f32;

    let channels = meta.channels();
    let mut envelope = waveform_columns
        .filter(|c| *c > 0)
        .map(|c| EnvelopeBuilder::new(c, channels, range.len));
    let mut frame_base = 0u64;

    src.seek(range.start)?;
    progress(0, total_frames);
    let mut block = Block::new(src, cfg.fft_size, channels, range.len);

    loop {
        block.fill()?;

        // Fold before the frame check: the last block can be shorter than one
        // transform and still hold samples the strip has to show.
        if let Some(builder) = envelope.as_mut() {
            block.fold_into(builder);
        }

        if block.filled < cfg.fft_size {
            break;
        }

        let frames_here =
            ((block.filled - cfg.fft_size) / cfg.hop + 1).min((total_frames - frame_base) as usize);
        if frames_here == 0 {
            break;
        }

        let partial = plan.transform_block(
            &block.buf,
            cfg.hop,
            channels,
            frames_here,
            frame_base,
            columns,
        );

        store.absorb(&partial, out_height);
        for (slot, add) in power.iter_mut().zip(partial.power.iter()) {
            *slot += add;
        }
        time_peak = time_peak.max(partial.time_peak);

        frame_base += frames_here as u64;
        progress(frame_base, total_frames);

        block.carry(frames_here * cfg.hop);

        if frame_base >= total_frames || (block.remaining == 0 && block.filled < cfg.fft_size) {
            break;
        }
    }

    if let Some(builder) = envelope.as_mut() {
        block.drain_into(builder)?;
    }

    let frames = frame_base.max(1);
    let db = store.finish();
    let (db_min, db_max) = db_window(&db, reference, dynamic_range_db);

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

    let psd = averaged_spectrum(&plan, &meta, &power, frames);

    Ok(Analysis {
        waveform: finish_envelope(envelope, range, meta.sample_rate),
        spectrogram,
        psd,
        time_peak,
        frames: frame_base,
        enbw_hz: plan.window.enbw_hz(meta.sample_rate),
    })
}

/// Maps a frame index onto the image column it lands in.
#[derive(Clone, Copy)]
struct Columns {
    total_frames: u64,
    width: usize,
}

impl Columns {
    const fn new(total_frames: u64, width: usize) -> Self {
        Self {
            total_frames,
            width,
        }
    }

    fn of(self, frame: u64) -> usize {
        if self.total_frames <= 1 {
            return 0;
        }
        ((frame * self.width as u64) / self.total_frames).min(self.width as u64 - 1) as usize
    }
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

    /// Transform every whole frame the block holds, in parallel.
    fn transform_block(
        &self,
        buf: &[f32],
        hop: usize,
        channels: usize,
        frames: usize,
        frame_base: u64,
        columns: Columns,
    ) -> Partial {
        (0..frames)
            .into_par_iter()
            .fold(
                || Partial::new(self.bins, self.fft_size),
                |mut acc, k| {
                    let start = k * hop * channels;
                    let frame = &buf[start..start + self.fft_size * channels];
                    self.frame(frame, &mut acc, columns.of(frame_base + k as u64));
                    acc
                },
            )
            .reduce(|| Partial::new(self.bins, self.fft_size), Partial::merge)
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
            fold_column(dst, src, self.reduce);
            self.counts[col] += 1;
        }
    }

    /// Divide each averaged column by the number of frames that landed in it.
    fn average_columns(&mut self) {
        for col in 0..self.width {
            let n = self.counts[col];
            if n <= 1 {
                continue;
            }
            for v in &mut self.values[col * self.height..(col + 1) * self.height] {
                *v /= n as f32;
            }
        }
    }

    /// Column-major dB values, low frequency first within each column.
    fn finish(mut self) -> Vec<f32> {
        if matches!(self.reduce, Reduce::Mean) {
            self.average_columns();
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

/// Fold one frame's rows into the column they share.
///
/// `Max` keeps the loudest value seen; `Mean` accumulates, and `finish`
/// divides by the count afterwards. A non-finite accumulator means the column
/// is still empty, so the first value replaces it rather than adding to it.
fn fold_column(dst: &mut [f32], src: &[f32], reduce: Reduce) {
    match reduce {
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
