//! Extracting a frequency band as a complex baseband signal at a lower rate.
//!
//! The band centre is moved to 0 Hz by a numerically controlled oscillator,
//! a windowed-sinc low-pass keeps the band, and a polyphase decimator by an
//! integer `D` computes only the outputs it keeps. The filter's delay is
//! compensated, so output `n` is the band at input sample `n·D` of the span.
//!
//! Everything here is a pure stream over blocks the caller reads: no I/O, no
//! threads, and the state is bounded by the filter length.

use thiserror::Error;

/// Stopband attenuation of the low-pass filter, in decibels.
const STOPBAND_DB: f64 = 80.0;

/// The narrowest band, as a fraction of the sample rate, which bounds the filter length.
pub const NARROWEST_BAND: f64 = 1.0 / 65_536.0;

/// The part of the decimated rate left to the filter's transition.
const TRANSITION: f64 = 1.25;

/// The narrowest guard against a real capture's mirror, as a fraction of the sample rate.
///
/// A band closer than this to 0 Hz or the Nyquist rate keeps the part of its mirror
/// that falls within the guard, since a real signal cannot be told from its mirror there.
const MIRROR_GUARD: f64 = 1.0 / 2048.0;

/// A complex band this close to the whole capture, as a fraction of the sample rate, is kept unfiltered.
const WHOLE_BAND: f64 = 1.0 / 1024.0;

#[derive(Debug, Error, PartialEq)]
pub enum ExtractError {
    #[error("the sample rate {0} Hz is not a positive finite number")]
    BadRate(f64),
    #[error("the band {low} to {high} Hz is not inside the capture's band")]
    OutsideCapture { low: f64, high: f64 },
    #[error("the band is {width} Hz wide, narrower than the {narrowest} Hz that can be saved")]
    TooNarrow { width: f64, narrowest: f64 },
    #[error("a span of {0} samples is too long to extract")]
    TooLong(u64),
}

/// How a band is extracted: the shift, the filter and the decimation.
#[derive(Debug, Clone)]
pub struct ExtractPlan {
    rate: f64,
    centre: f64,
    decimation: u64,
    /// Symmetric low-pass taps of odd length, summing to the gain.
    taps: Vec<f32>,
    real: bool,
}

impl ExtractPlan {
    /// Check that `low..high` hertz can be extracted from a capture at `rate`, without building the filter.
    ///
    /// The edges are relative to the capture's baseband 0 Hz. A complex capture
    /// spans `-rate/2..rate/2` and a real one `0..rate/2`.
    pub fn check(rate: f64, low: f64, high: f64, real: bool) -> Result<(), ExtractError> {
        if !(rate.is_finite() && rate > 0.0) {
            return Err(ExtractError::BadRate(rate));
        }
        let nyquist = rate / 2.0;
        let floor = if real { 0.0 } else { -nyquist };
        let inside = low.is_finite() && high.is_finite() && low >= floor && high <= nyquist;
        if !inside || low >= high {
            return Err(ExtractError::OutsideCapture { low, high });
        }
        let width = high - low;
        let narrowest = rate * NARROWEST_BAND;
        if width < narrowest {
            return Err(ExtractError::TooNarrow { width, narrowest });
        }
        Ok(())
    }

    /// Plan the extraction of `low..high` hertz from a capture at `rate`, as [`Self::check`] allows.
    pub fn new(rate: f64, low: f64, high: f64, real: bool) -> Result<Self, ExtractError> {
        Self::check(rate, low, high, real)?;
        let width = high - low;
        let decimation = ((rate / (TRANSITION * width)).floor() as u64).max(1);
        let pass = width / 2.0;
        // An analytic signal has the real tone's amplitude, twice its positive half.
        let gain = if real { 2.0 } else { 1.0 };
        let taps = match transition(rate, low, high, decimation, real) {
            Some(transition) => low_pass(rate, pass, pass + transition, gain),
            None => vec![gain as f32],
        };
        Ok(Self {
            rate,
            centre: (low + high) / 2.0,
            decimation,
            taps,
            real,
        })
    }

    pub const fn decimation(&self) -> u64 {
        self.decimation
    }

    /// The rate of the extracted signal.
    pub fn output_rate(&self) -> f64 {
        self.rate / self.decimation as f64
    }

    /// The band centre relative to the capture's baseband 0 Hz, which becomes the output's 0 Hz.
    pub const fn centre(&self) -> f64 {
        self.centre
    }

    pub const fn is_real(&self) -> bool {
        self.real
    }

    /// Input samples the filter reads on each side of the span.
    pub fn margin(&self) -> u64 {
        (self.taps.len() / 2) as u64
    }

    /// Output samples a span of `len` input samples gives.
    pub fn output_len(&self, len: u64) -> u64 {
        len.div_ceil(self.decimation)
    }

    pub fn taps(&self) -> &[f32] {
        &self.taps
    }
}

/// The width of the filter's transition, or none for a complex band that is nearly the whole capture.
///
/// It is the narrowest of three limits. What aliases when decimating must land
/// beyond the stopband. The stopband must stay below the Nyquist rate, and it
/// takes half of what lies outside the band so the rest is stopped. A real
/// capture's mirror must lie beyond it too, down to a guard near 0 Hz and Fs/2.
fn transition(rate: f64, low: f64, high: f64, decimation: u64, real: bool) -> Option<f64> {
    let width = high - low;
    let outside = rate - width;
    if !real && outside < rate * WHOLE_BAND {
        return None;
    }
    let aliasing = if decimation == 1 {
        width * (TRANSITION - 1.0)
    } else {
        rate / decimation as f64 - width
    };
    let wrapping = outside / 4.0;
    let mirror = if real {
        let gap = 2.0 * low.min(rate / 2.0 - high);
        gap.max(rate * MIRROR_GUARD)
    } else {
        f64::INFINITY
    };
    Some(aliasing.min(wrapping).min(mirror))
}

/// A Kaiser-windowed sinc passing `pass` hertz and stopping from `stop`, with DC gain `gain`.
fn low_pass(rate: f64, pass: f64, stop: f64, gain: f64) -> Vec<f32> {
    let cutoff = (pass + stop) / 2.0;
    if cutoff >= rate / 2.0 {
        return vec![gain as f32];
    }
    let transition = 2.0 * std::f64::consts::PI * (stop - pass) / rate;
    let estimate = ((STOPBAND_DB - 7.95) / (2.285 * transition)).ceil() as usize + 1;
    let half = estimate / 2;
    let len = 2 * half + 1;
    let beta = 0.1102 * (STOPBAND_DB - 8.7);
    let norm = bessel_i0(beta);
    let fraction = 2.0 * cutoff / rate;
    let raw: Vec<f64> = (0..len)
        .map(|k| {
            let offset = k as f64 - half as f64;
            let ratio = offset / half.max(1) as f64;
            let window = bessel_i0(beta * (1.0 - ratio * ratio).max(0.0).sqrt()) / norm;
            fraction * sinc(fraction * offset) * window
        })
        .collect();
    let sum: f64 = raw.iter().sum();
    raw.iter().map(|tap| (tap * gain / sum) as f32).collect()
}

fn sinc(x: f64) -> f64 {
    if x == 0.0 {
        1.0
    } else {
        let arg = std::f64::consts::PI * x;
        arg.sin() / arg
    }
}

/// The modified Bessel function of the first kind and order zero, by its series.
fn bessel_i0(x: f64) -> f64 {
    let quarter = x * x / 4.0;
    let mut term = 1.0;
    let mut sum = 1.0;
    for k in 1..200 {
        term *= quarter / (k * k) as f64;
        sum += term;
        if term < sum * 1e-17 {
            break;
        }
    }
    sum
}

/// The extraction of one span, fed its input in order.
///
/// Input starts `margin` samples before the span and ends `margin` samples
/// after it; the caller feeds zeros where the capture has no samples there.
pub struct Extractor {
    plan: ExtractPlan,
    /// Phase of the oscillator in cycles, at the next input sample.
    phase: f64,
    /// Cycles the oscillator turns per input sample.
    step: f64,
    /// Each mixed sample twice, `len` apart, so the newest `len` are one contiguous slice.
    history: Vec<[f32; 2]>,
    /// Where the next mixed sample goes in `history`.
    head: usize,
    /// Input samples taken, the margin before the span included.
    taken: u64,
    /// Outputs given so far.
    given: u64,
    /// Outputs the span gives.
    total: u64,
}

impl Extractor {
    /// Start extracting a span of `len` input samples, refused when its input cannot be counted.
    pub fn new(plan: ExtractPlan, len: u64) -> Result<Self, ExtractError> {
        let step = -plan.centre / plan.rate;
        let margin = plan.margin();
        let taps = plan.taps.len();
        let total = plan.output_len(len);
        // The last output is due once the span and both margins are in.
        if len.checked_add(2 * margin + 1).is_none() {
            return Err(ExtractError::TooLong(len));
        }
        Ok(Self {
            // The oscillator is at zero phase at the span's first sample.
            phase: (-(margin as f64) * step).rem_euclid(1.0),
            step,
            history: vec![[0.0; 2]; 2 * taps],
            head: 0,
            taken: 0,
            given: 0,
            total,
            plan,
        })
    }

    /// Whether every output of the span has been given.
    pub const fn is_done(&self) -> bool {
        self.given >= self.total
    }

    /// Input samples still wanted, the margin after the span included.
    pub fn wanted(&self) -> u64 {
        if self.is_done() {
            return 0;
        }
        self.due(self.total - 1) - self.taken
    }

    /// Input samples taken when output `n` can be computed.
    fn due(&self, n: u64) -> u64 {
        n * self.plan.decimation + 2 * self.plan.margin() + 1
    }

    /// Feed interleaved samples, `channels` values each, giving outputs into `out`.
    pub fn push(&mut self, input: &[f32], channels: usize, out: &mut Vec<[f32; 2]>) {
        for sample in input.chunks_exact(channels.max(1)) {
            let value = [sample[0], if channels > 1 { sample[1] } else { 0.0 }];
            self.take(value, out);
        }
    }

    /// Feed `count` zero samples, for input the capture does not have.
    pub fn push_zeros(&mut self, count: u64, out: &mut Vec<[f32; 2]>) {
        for _ in 0..count {
            self.take([0.0; 2], out);
        }
    }

    fn take(&mut self, [i, q]: [f32; 2], out: &mut Vec<[f32; 2]>) {
        if self.is_done() {
            return;
        }
        let (sin, cos) = (std::f64::consts::TAU * self.phase).sin_cos();
        self.phase = (self.phase + self.step).rem_euclid(1.0);
        let (i, q, cos, sin) = (f64::from(i), f64::from(q), cos, sin);
        let mixed = [(i * cos - q * sin) as f32, (i * sin + q * cos) as f32];
        let len = self.plan.taps.len();
        self.history[self.head] = mixed;
        self.history[self.head + len] = mixed;
        self.head = (self.head + 1) % len;
        self.taken += 1;
        if self.taken == self.due(self.given) {
            out.push(self.filtered());
            self.given += 1;
        }
    }

    /// The filter over the newest `len` mixed samples, oldest first.
    fn filtered(&self) -> [f32; 2] {
        let len = self.plan.taps.len();
        let window = &self.history[self.head..self.head + len];
        // A long filter sums in f64, so rounding stays below its stopband.
        let (mut i, mut q) = (0.0f64, 0.0f64);
        for (tap, sample) in self.plan.taps.iter().zip(window) {
            i += f64::from(tap * sample[0]);
            q += f64::from(tap * sample[1]);
        }
        [i as f32, q as f32]
    }
}

#[cfg(test)]
mod tests {
    include!("extract_tests.rs");
}
