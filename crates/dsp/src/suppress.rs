//! Suppressing a frequency band of a capture, keeping its length and everything outside the band.
//!
//! The output is `x[n] − w[n]·b[n]`, where `b` is the band-pass of the input
//! and `w` a time taper, 1 across a band suppressed over the whole capture.
//! The band-pass passes the band whole with its transitions outside it, so
//! the band falls by the filter's stopband attenuation and frequencies past
//! the transitions are left as they were. It is applied by overlap-save FFT
//! convolution with its delay compensated, so output `n` is input `n`.
//!
//! Like `extract`, everything here is a pure stream over blocks the caller
//! reads, with no I/O and no threads.

use std::sync::Arc;

use rustfft::num_complex::Complex32;
use rustfft::{Fft, FftPlanner};

use crate::extract::{ExtractError, LARGEST_FFT, NARROWEST_TRANSITION, check_band, low_pass};

/// The shortest fade, which keeps a short filter or a whole band suppressed in time from clicking.
pub const SHORTEST_TAPER: u64 = 256;

/// How a band is suppressed, with the band-pass that finds it.
#[derive(Debug, Clone)]
pub struct SuppressPlan {
    /// Band-pass taps, an odd number of them centred on the middle one.
    taps: Vec<Complex32>,
}

impl SuppressPlan {
    /// Check that `low..high` hertz can be suppressed in a capture at `rate`, without building the filter.
    ///
    /// The edges are relative to the capture's baseband 0 Hz. A complex capture
    /// spans `-rate/2..rate/2` and a real one `0..rate/2`.
    pub fn check(rate: f64, low: f64, high: f64, real: bool) -> Result<(), ExtractError> {
        check_band(rate, low, high, real)
    }

    /// Plan the suppression of `low..high` hertz in a capture at `rate`, as [`Self::check`] allows.
    pub fn new(rate: f64, low: f64, high: f64, real: bool) -> Result<Self, ExtractError> {
        Self::check(rate, low, high, real)?;
        let taps = if real {
            real_band_pass(rate, low, high)
        } else {
            complex_band_pass(rate, low, high)
        };
        Ok(Self { taps })
    }

    /// Input samples the filter reads on each side of what it suppresses.
    pub fn margin(&self) -> u64 {
        (self.taps.len() / 2) as u64
    }

    /// Samples over which a suppression limited in time fades in and out, the filter's own time resolution or more.
    pub fn taper(&self) -> u64 {
        self.margin().max(SHORTEST_TAPER)
    }

    pub fn taps(&self) -> &[Complex32] {
        &self.taps
    }
}

/// The transition beside a band of `width` hertz, a quarter of it within `floor..room`.
fn transition(rate: f64, width: f64, room: f64) -> f64 {
    let narrowest = rate * NARROWEST_TRANSITION;
    (width / 4.0).min(room).max(narrowest)
}

/// A complex band-pass, the low-pass of half the band modulated to its centre.
fn complex_band_pass(rate: f64, low: f64, high: f64) -> Vec<Complex32> {
    let width = high - low;
    // The transitions on both sides of the band share what lies outside it.
    let room = (rate - width) / 2.0;
    if room < rate * NARROWEST_TRANSITION {
        return vec![Complex32::new(1.0, 0.0)];
    }
    let transition = transition(rate, width, room);
    let lp = low_pass(rate, width / 2.0, width / 2.0 + transition, 1.0);
    let centre = (low + high) / 2.0;
    let half = (lp.len() / 2) as f64;
    lp.iter()
        .enumerate()
        .map(|(k, &tap)| {
            let turn = std::f64::consts::TAU * centre * (k as f64 - half) / rate;
            let (sin, cos) = turn.sin_cos();
            let tap = f64::from(tap);
            Complex32::new((tap * cos) as f32, (tap * sin) as f32)
        })
        .collect()
}

/// A real band-pass, the low-pass to `high` less the low-pass to `low`, both with their transitions outside the band.
///
/// A band from 0 Hz needs no second term, and a band to Fs/2 makes the first a
/// unit impulse, so no transition runs past the capture's edge.
fn real_band_pass(rate: f64, low: f64, high: f64) -> Vec<Complex32> {
    let narrowest = rate * NARROWEST_TRANSITION;
    let nyquist = rate / 2.0;
    let from_zero = low < narrowest;
    let to_nyquist = nyquist - high < narrowest;
    let room = match (from_zero, to_nyquist) {
        (true, true) => return vec![Complex32::new(1.0, 0.0)],
        (true, false) => nyquist - high,
        (false, true) => low,
        (false, false) => low.min(nyquist - high),
    };
    let transition = transition(rate, high - low, room);
    let upper = if to_nyquist {
        vec![1.0]
    } else {
        low_pass(rate, high, high + transition, 1.0)
    };
    let lower = if from_zero {
        Vec::new()
    } else {
        low_pass(rate, low - transition, low, 1.0)
    };
    let len = upper.len().max(lower.len());
    let mut taps = vec![Complex32::default(); len];
    for (term, sign) in [(&upper, 1.0f32), (&lower, -1.0)] {
        let offset = (len - term.len()) / 2;
        for (tap, &value) in taps[offset..].iter_mut().zip(term.iter()) {
            tap.re += sign * value;
        }
    }
    taps
}

/// The suppression of one region, fed its input in order.
///
/// Input starts `margin` samples before the region and ends `margin` samples
/// after it, and the caller feeds zeros where the capture has no samples there.
pub struct Suppressor {
    forward: Arc<dyn Fft<f32>>,
    inverse: Arc<dyn Fft<f32>>,
    /// The filter's spectrum at the FFT size, scaled for the inverse transform.
    spectrum: Vec<Complex32>,
    /// Input samples, the last `taps - 1` of the previous block first.
    block: Vec<Complex32>,
    /// How much of `block` holds samples.
    fill: usize,
    work: Vec<Complex32>,
    scratch: Vec<Complex32>,
    taps: usize,
    margin: u64,
    /// Input index, counted from the margin before the region, of `block[0]`.
    start: u64,
    /// Input samples taken, the margin before the region included.
    taken: u64,
    /// Outputs given so far.
    given: u64,
    /// Outputs the region gives.
    total: u64,
    rise: u64,
    fall: u64,
}

impl Suppressor {
    /// Start suppressing a region of `len` samples, fading in over its first `rise` and out over its last `fall`.
    pub fn new(plan: &SuppressPlan, len: u64, rise: u64, fall: u64) -> Result<Self, ExtractError> {
        let taps = plan.taps.len();
        let margin = plan.margin();
        if len.checked_add(2 * margin).is_none() || rise.saturating_add(fall) > len {
            return Err(ExtractError::TooLong(len));
        }
        let size = (4 * taps)
            .next_power_of_two()
            .clamp(1024, LARGEST_FFT.max(2 * taps));
        let mut planner = FftPlanner::new();
        let forward = planner.plan_fft_forward(size);
        let inverse = planner.plan_fft_inverse(size);
        let scale = 1.0 / size as f32;
        let mut spectrum = vec![Complex32::default(); size];
        for (bin, &tap) in spectrum.iter_mut().zip(&plan.taps) {
            *bin = tap * scale;
        }
        let length = forward
            .get_inplace_scratch_len()
            .max(inverse.get_inplace_scratch_len());
        let mut scratch = vec![Complex32::default(); length];
        forward.process_with_scratch(&mut spectrum, &mut scratch);
        Ok(Self {
            forward,
            inverse,
            spectrum,
            block: vec![Complex32::default(); size],
            fill: 0,
            work: vec![Complex32::default(); size],
            scratch,
            taps,
            margin,
            start: 0,
            taken: 0,
            given: 0,
            total: len,
            rise,
            fall,
        })
    }

    /// Whether every output of the region has been given.
    pub const fn is_done(&self) -> bool {
        self.given >= self.total
    }

    /// Input samples still wanted, the margin after the region included.
    pub fn wanted(&self) -> u64 {
        if self.is_done() {
            return 0;
        }
        self.total + 2 * self.margin - self.taken
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
        self.block[self.fill] = Complex32::new(i, q);
        self.fill += 1;
        self.taken += 1;
        let last = self.taken == self.total + 2 * self.margin;
        if self.fill == self.block.len() || last {
            self.convolve(out);
        }
    }

    /// Filter the block and give the outputs due within it, keeping its tail for the next.
    fn convolve(&mut self, out: &mut Vec<[f32; 2]>) {
        self.work.copy_from_slice(&self.block);
        // Samples past the fill are stale, and zeros there change no output before it.
        self.work[self.fill..].fill(Complex32::default());
        self.forward
            .process_with_scratch(&mut self.work, &mut self.scratch);
        for (bin, filter) in self.work.iter_mut().zip(&self.spectrum) {
            *bin *= filter;
        }
        self.inverse
            .process_with_scratch(&mut self.work, &mut self.scratch);
        // Output n is the band-pass at input n + 2·margin, centred on input n + margin.
        while !self.is_done() {
            let at = self.given + 2 * self.margin;
            if at >= self.start + self.fill as u64 {
                break;
            }
            let filtered = self.work[(at - self.start) as usize];
            let input = self.block[(at - self.margin - self.start) as usize];
            let weight = self.weight(self.given);
            let value = input - filtered * weight;
            out.push([value.re, value.im]);
            self.given += 1;
        }
        let kept = (self.taps - 1).min(self.fill);
        self.block.copy_within(self.fill - kept..self.fill, 0);
        self.start += (self.fill - kept) as u64;
        self.fill = kept;
    }

    /// The taper at output `n`, a raised cosine over the first `rise` and the last `fall` outputs.
    fn weight(&self, n: u64) -> f32 {
        let ramp = |step: u64, steps: u64| {
            let fraction = (step + 1) as f64 / (steps + 1) as f64;
            (0.5 - 0.5 * (std::f64::consts::PI * fraction).cos()) as f32
        };
        if n < self.rise {
            return ramp(n, self.rise);
        }
        let from_end = self.total - 1 - n;
        if from_end < self.fall {
            return ramp(from_end, self.fall);
        }
        1.0
    }
}

#[cfg(test)]
mod tests {
    include!("suppress_tests.rs");
}
