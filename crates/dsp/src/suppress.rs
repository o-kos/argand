//! Suppressing a frequency band of a capture, keeping its length and everything outside the band.
//!
//! The output is `x[n] − w[n]·b[n] + f[n]·σ[n]·v[n]`, where `b` is the band-pass
//! of the input, `w` a time taper (1 across a band suppressed over the whole
//! capture), `σ` the noise level of the frequencies beside the band and `v`
//! unit white noise through the complementary band-pass. The band-pass passes
//! the band whole with its transitions outside it, so the band's content gives
//! way to noise as loud as its neighbours' and frequencies past the transitions
//! are left as they were. The residue keeps `1 − w·|H|` of the input, so the
//! fill's filter has the power gain `2|H| − |H|²` and its taper `f` the power
//! `2w − w²`, which keeps a noise background level through both transitions.
//! Both are applied by overlap-save FFT convolution with their delay
//! compensated, so output `n` is input `n`.
//!
//! [`NoiseFloor`] measures `σ` beforehand, block by block over the region, as
//! the median power of the neighbouring frequencies, so tones there do not
//! raise it and a level that changes over time is followed.
//!
//! Like `extract`, everything here is a pure stream over blocks the caller
//! reads, with no I/O and no threads.

use std::sync::Arc;

use rustfft::num_complex::Complex32;
use rustfft::{Fft, FftPlanner};

use crate::extract::{ExtractError, LARGEST_FFT, NARROWEST_TRANSITION, check_band, low_pass};

/// The shortest fade, which keeps a short filter or a whole band suppressed in time from clicking.
pub const SHORTEST_TAPER: u64 = 256;

/// The longest transform the noise floor is measured with.
const NOISE_FFT: usize = 4096;
/// The fewest samples the noise floor is measured over.
const SHORTEST_NOISE_FFT: usize = 64;
/// The most blocks the noise floor follows over time.
const NOISE_BLOCKS: u64 = 256;
/// Segments measured in each block.
const SEGMENTS_PER_BLOCK: u64 = 8;
/// Bins left between a band's transition and the noise measured beside it, past a Hann window's leakage of a tone.
const GUARD_BINS: f64 = 8.0;
/// The fewest bins measured on each side of a band.
const MEASURED_BINS: f64 = 32.0;

/// How a band is suppressed, with the band-pass that finds it.
#[derive(Debug, Clone)]
pub struct SuppressPlan {
    /// Band-pass taps, an odd number of them centred on the middle one.
    taps: Vec<Complex32>,
    rate: f64,
    real: bool,
    /// The band and the transition beside it, none when the band is the whole capture.
    edges: Option<(f64, f64, f64)>,
    /// The band-pass the fill's noise takes, which with the residue of the band keeps the noise power level.
    noise_taps: Vec<Complex32>,
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
        let (taps, transition) = if real {
            real_band_pass(rate, low, high)
        } else {
            complex_band_pass(rate, low, high)
        };
        let noise_taps = complementary(&taps, real);
        Ok(Self {
            taps,
            rate,
            real,
            edges: transition.map(|transition| (low, high, transition)),
            noise_taps,
        })
    }

    /// The frequencies beside the band whose noise fills it, for a transform of `bins` bins.
    ///
    /// They start a few bins past the band's transitions, so a tone at the band's
    /// edge does not leak into them, and stay inside the capture.
    fn neighbours(&self, bins: usize) -> Vec<(f64, f64)> {
        let Some((low, high, transition)) = self.edges else {
            return Vec::new();
        };
        let resolution = self.rate / bins as f64;
        let guard = transition + GUARD_BINS * resolution;
        let extent = (high - low).max(MEASURED_BINS * resolution);
        let floor = if self.real { 0.0 } else { -self.rate / 2.0 };
        [
            (low - guard - extent, low - guard),
            (high + guard, high + guard + extent),
        ]
        .into_iter()
        .map(|(from, to)| (from.max(floor), to.min(self.rate / 2.0)))
        .filter(|(from, to)| from < to)
        .collect()
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

/// A complex band-pass, the low-pass of half the band modulated to its centre, with its transition.
fn complex_band_pass(rate: f64, low: f64, high: f64) -> (Vec<Complex32>, Option<f64>) {
    let width = high - low;
    // The transitions on both sides of the band share what lies outside it.
    let room = (rate - width) / 2.0;
    if room < rate * NARROWEST_TRANSITION {
        return (vec![Complex32::new(1.0, 0.0)], None);
    }
    let transition = transition(rate, width, room);
    let lp = low_pass(rate, width / 2.0, width / 2.0 + transition, 1.0);
    let centre = (low + high) / 2.0;
    let half = (lp.len() / 2) as f64;
    let taps = lp
        .iter()
        .enumerate()
        .map(|(k, &tap)| {
            let turn = std::f64::consts::TAU * centre * (k as f64 - half) / rate;
            let (sin, cos) = turn.sin_cos();
            let tap = f64::from(tap);
            Complex32::new((tap * cos) as f32, (tap * sin) as f32)
        })
        .collect();
    (taps, Some(transition))
}

/// A real band-pass, the low-pass to `high` less the low-pass to `low`, both with their transitions outside the band.
///
/// A band from 0 Hz needs no second term, and a band to Fs/2 makes the first a
/// unit impulse, so no transition runs past the capture's edge.
fn real_band_pass(rate: f64, low: f64, high: f64) -> (Vec<Complex32>, Option<f64>) {
    let narrowest = rate * NARROWEST_TRANSITION;
    let nyquist = rate / 2.0;
    let from_zero = low < narrowest;
    let to_nyquist = nyquist - high < narrowest;
    let room = match (from_zero, to_nyquist) {
        (true, true) => return (vec![Complex32::new(1.0, 0.0)], None),
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
    (taps, Some(transition))
}

/// The band-pass whose power gain is `2|H| − |H|²` where `taps` has gain `|H|`, with the same delay.
///
/// The band's residue passes with `1 − |H|`, so the residue's power and the
/// fill's add up to the noise power through the transitions as well.
fn complementary(taps: &[Complex32], real: bool) -> Vec<Complex32> {
    let len = taps.len();
    if len == 1 {
        return taps.to_vec();
    }
    let size = (4 * len).next_power_of_two();
    let mut planner = FftPlanner::new();
    let mut spectrum = vec![Complex32::default(); size];
    spectrum[..len].copy_from_slice(taps);
    planner.plan_fft_forward(size).process(&mut spectrum);
    let centre = (len / 2) as f64;
    for (k, bin) in spectrum.iter_mut().enumerate() {
        let gain = f64::from(bin.norm());
        let power = (2.0 * gain - gain * gain).max(0.0);
        let turn = -std::f64::consts::TAU * k as f64 * centre / size as f64;
        let (sin, cos) = turn.sin_cos();
        let magnitude = power.sqrt() / size as f64;
        *bin = Complex32::new((magnitude * cos) as f32, (magnitude * sin) as f32);
    }
    planner.plan_fft_inverse(size).process(&mut spectrum);
    let beta = 0.1102 * (80.0 - 8.7);
    let norm = crate::extract::bessel_i0(beta);
    spectrum[..len]
        .iter()
        .enumerate()
        .map(|(k, &tap)| {
            let ratio = (k as f64 - centre) / centre;
            let window =
                crate::extract::bessel_i0(beta * (1.0 - ratio * ratio).max(0.0).sqrt()) / norm;
            let tap = tap * window as f32;
            if real {
                Complex32::new(tap.re, 0.0)
            } else {
                tap
            }
        })
        .collect()
}

/// The noise level beside a band, measured over a region block by block.
///
/// The caller reads the segments [`NoiseFloor::segments`] names and pushes
/// each, then takes the levels with [`NoiseFloor::finish`].
pub struct NoiseFloor {
    size: usize,
    fft: Option<Arc<dyn Fft<f32>>>,
    window: Vec<f32>,
    /// The window's power, which makes a bin of white noise its variance.
    power: f32,
    /// The transform bins that lie beside the band.
    bins: Vec<usize>,
    /// Summed bin powers of each block, bin by bin.
    sums: Vec<Vec<f32>>,
    counts: Vec<u32>,
    block_len: u64,
    len: u64,
    work: Vec<Complex32>,
    scratch: Vec<Complex32>,
}

impl NoiseFloor {
    /// Prepare to measure the neighbours of `plan`'s band over a region of `len` samples.
    pub fn new(plan: &SuppressPlan, len: u64) -> Self {
        let longest = usize::try_from(len).unwrap_or(usize::MAX).min(NOISE_FFT);
        let size = if longest >= SHORTEST_NOISE_FFT {
            1 << longest.ilog2()
        } else {
            0
        };
        let neighbours = plan.neighbours(size.max(1));
        let bins: Vec<usize> = (0..size)
            .filter(|&k| {
                let frequency = bin_frequency(k, size, plan.rate, plan.real);
                frequency.is_some_and(|frequency| {
                    neighbours
                        .iter()
                        .any(|&(from, to)| (from..=to).contains(&frequency))
                })
            })
            .collect();
        let size = if bins.is_empty() { 0 } else { size };
        let blocks = match size {
            0 => 0,
            _ => (len / (SEGMENTS_PER_BLOCK * size as u64)).clamp(1, NOISE_BLOCKS),
        };
        let fft = (size > 0).then(|| FftPlanner::new().plan_fft_forward(size));
        let window: Vec<f32> = (0..size)
            .map(|n| {
                let phase = std::f64::consts::TAU * n as f64 / size as f64;
                (0.5 - 0.5 * phase.cos()) as f32
            })
            .collect();
        let power = window.iter().map(|w| w * w).sum();
        let scratch =
            vec![Complex32::default(); fft.as_ref().map_or(0, |fft| fft.get_inplace_scratch_len())];
        Self {
            size,
            fft,
            window,
            power,
            sums: vec![vec![0.0; bins.len()]; blocks as usize],
            counts: vec![0; blocks as usize],
            bins,
            block_len: len.div_ceil(blocks.max(1)),
            len,
            work: vec![Complex32::default(); size],
            scratch,
        }
    }

    /// Samples in each segment to read.
    pub fn segment_len(&self) -> usize {
        self.size
    }

    /// Where each segment to read starts in the region, a few spread over every block.
    pub fn segments(&self) -> Vec<u64> {
        let size = self.size as u64;
        if size == 0 {
            return Vec::new();
        }
        let mut starts = Vec::new();
        for block in 0..self.counts.len() as u64 {
            let first = (block * self.block_len).min(self.len - size);
            let last = ((block + 1) * self.block_len)
                .min(self.len)
                .saturating_sub(size)
                .max(first);
            let count = ((last - first) / size + 1).min(SEGMENTS_PER_BLOCK);
            for segment in 0..count {
                let step = (last - first) / count.saturating_sub(1).max(1);
                starts.push(first + segment * step);
            }
        }
        starts
    }

    /// Measure the segment starting at region sample `start`, interleaved with `channels` values each.
    pub fn push(&mut self, start: u64, input: &[f32], channels: usize) {
        let Some(fft) = &self.fft else {
            return;
        };
        if input.len() < self.size * channels {
            return;
        }
        for (n, (work, sample)) in self
            .work
            .iter_mut()
            .zip(input.chunks_exact(channels.max(1)))
            .enumerate()
        {
            let q = if channels > 1 { sample[1] } else { 0.0 };
            *work = Complex32::new(sample[0], q) * self.window[n];
        }
        fft.process_with_scratch(&mut self.work, &mut self.scratch);
        let block =
            ((start + self.size as u64 / 2) / self.block_len).min(self.counts.len() as u64 - 1);
        let block = block as usize;
        for (sum, &bin) in self.sums[block].iter_mut().zip(&self.bins) {
            *sum += self.work[bin].norm_sqr() / self.power;
        }
        self.counts[block] += 1;
    }

    /// The noise level of every block, none when nothing beside the band could be measured.
    pub fn finish(self) -> Option<NoiseFill> {
        let mut levels: Vec<Option<f32>> = self
            .sums
            .iter()
            .zip(&self.counts)
            .map(|(sums, &count)| (count > 0).then(|| block_level(sums, count)))
            .collect();
        // A block with no segment of its own takes the nearest measured one's level.
        let measured: Vec<(usize, f32)> = levels
            .iter()
            .enumerate()
            .filter_map(|(index, level)| level.map(|level| (index, level)))
            .collect();
        if measured.is_empty() {
            return None;
        }
        for (index, level) in levels.iter_mut().enumerate() {
            if level.is_none() {
                let nearest = measured.iter().min_by_key(|(at, _)| at.abs_diff(index));
                *level = nearest.map(|&(_, level)| level);
            }
        }
        Some(NoiseFill {
            levels: levels.into_iter().flatten().collect(),
            block_len: self.block_len,
        })
    }
}

/// The frequency of transform bin `k`, none for the mirror half of a real capture's transform.
fn bin_frequency(k: usize, size: usize, rate: f64, real: bool) -> Option<f64> {
    let resolution = rate / size as f64;
    match (real, k <= size / 2) {
        (_, true) => Some(k as f64 * resolution),
        (false, false) => Some((k as f64 - size as f64) * resolution),
        (true, false) => None,
    }
}

/// The noise amplitude of a block from its summed bin powers over `count` segments.
///
/// The median keeps tones beside the band from raising it, and the factor turns
/// the median of an average of `count` exponential powers into their mean.
fn block_level(sums: &[f32], count: u32) -> f32 {
    let mut powers: Vec<f32> = sums.iter().map(|sum| sum / count as f32).collect();
    let middle = powers.len() / 2;
    let (_, median, _) = powers.select_nth_unstable_by(middle, f32::total_cmp);
    let segments = count as f32;
    (*median * segments / (segments - 1.0 / 3.0)).sqrt()
}

/// The noise level over a region, block by block, that fills a suppressed band.
#[derive(Debug, Clone, PartialEq)]
pub struct NoiseFill {
    /// The noise amplitude of each block.
    pub levels: Vec<f32>,
    pub block_len: u64,
}

impl NoiseFill {
    /// The amplitude at region sample `n`, interpolated between the blocks' centres.
    fn at(&self, n: u64) -> f32 {
        let position = n as f64 / self.block_len as f64 - 0.5;
        let last = self.levels.len() - 1;
        if position <= 0.0 || last == 0 {
            return self.levels[0];
        }
        let below = (position.floor() as usize).min(last);
        let above = (below + 1).min(last);
        let fraction = (position - below as f64).min(1.0) as f32;
        self.levels[below] + (self.levels[above] - self.levels[below]) * fraction
    }
}

/// Gaussian noise from a fixed seed, so a suppression computes the same samples every time.
struct Noise {
    state: u64,
    spare: Option<f32>,
}

impl Noise {
    const fn new(seed: u64) -> Self {
        Self {
            state: seed,
            spare: None,
        }
    }

    /// A uniform value in `(0, 1]`, by SplitMix64.
    fn uniform(&mut self) -> f64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        ((z >> 11) + 1) as f64 / (1u64 << 53) as f64
    }

    /// A standard normal value, by Box and Muller.
    fn gaussian(&mut self) -> f32 {
        if let Some(spare) = self.spare.take() {
            return spare;
        }
        let radius = (-2.0 * self.uniform().ln()).sqrt();
        let (sin, cos) = (std::f64::consts::TAU * self.uniform()).sin_cos();
        self.spare = Some((radius * sin) as f32);
        (radius * cos) as f32
    }
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
    /// The fill's filter, as `spectrum` is, while the band is filled with noise.
    noise_spectrum: Vec<Complex32>,
    noise_taps: Vec<Complex32>,
    /// Input samples, the last `taps - 1` of the previous block first.
    block: Vec<Complex32>,
    /// Unit noise, kept as `block` is, while the band is filled with noise.
    noise_block: Vec<Complex32>,
    /// How much of `block` holds samples.
    fill: usize,
    work: Vec<Complex32>,
    noise_work: Vec<Complex32>,
    noise: Noise,
    /// The noise level of the band's neighbours, none to leave the band empty.
    level: Option<NoiseFill>,
    real: bool,
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
            noise_spectrum: Vec::new(),
            noise_taps: plan.noise_taps.clone(),
            block: vec![Complex32::default(); size],
            noise_block: Vec::new(),
            fill: 0,
            work: vec![Complex32::default(); size],
            noise_work: Vec::new(),
            noise: Noise::new(0x5EED_A26A_7D00_0001),
            level: None,
            real: plan.real,
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

    /// Fill the band with noise as loud as `level` says its neighbours are.
    #[must_use]
    pub fn filled_with(mut self, level: NoiseFill) -> Self {
        let size = self.block.len();
        let scale = 1.0 / size as f32;
        let mut spectrum = vec![Complex32::default(); size];
        for (bin, &tap) in spectrum.iter_mut().zip(&self.noise_taps) {
            *bin = tap * scale;
        }
        self.forward
            .process_with_scratch(&mut spectrum, &mut self.scratch);
        self.noise_spectrum = spectrum;
        self.noise_block = vec![Complex32::default(); size];
        self.noise_work = vec![Complex32::default(); size];
        self.level = Some(level);
        self
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
        if self.level.is_some() {
            self.noise_block[self.fill] = self.unit_noise();
        }
        self.fill += 1;
        self.taken += 1;
        let last = self.taken == self.total + 2 * self.margin;
        if self.fill == self.block.len() || last {
            self.convolve(out);
        }
    }

    /// Noise of unit power, real for a real capture.
    fn unit_noise(&mut self) -> Complex32 {
        if self.real {
            return Complex32::new(self.noise.gaussian(), 0.0);
        }
        let scale = std::f32::consts::FRAC_1_SQRT_2;
        Complex32::new(self.noise.gaussian() * scale, self.noise.gaussian() * scale)
    }

    /// Filter the block and give the outputs due within it, keeping its tail for the next.
    fn convolve(&mut self, out: &mut Vec<[f32; 2]>) {
        band_pass(
            &self.block,
            self.fill,
            &mut self.work,
            &self.spectrum,
            (&self.forward, &self.inverse),
            &mut self.scratch,
        );
        if self.level.is_some() {
            band_pass(
                &self.noise_block,
                self.fill,
                &mut self.noise_work,
                &self.noise_spectrum,
                (&self.forward, &self.inverse),
                &mut self.scratch,
            );
        }
        // Output n is the band-pass at input n + 2·margin, centred on input n + margin.
        while !self.is_done() {
            let at = self.given + 2 * self.margin;
            if at >= self.start + self.fill as u64 {
                break;
            }
            let index = (at - self.start) as usize;
            let input = self.block[(at - self.margin - self.start) as usize];
            let weight = self.weight(self.given);
            let mut value = input - self.work[index] * weight;
            if let Some(level) = &self.level {
                // The residue keeps 1 − w of the band, so the fill takes the power that leaves.
                let fill = (2.0 * weight - weight * weight).max(0.0).sqrt();
                value += self.noise_work[index] * level.at(self.given) * fill;
            }
            // Rounding in the transform leaves a real capture a trace of imaginary part.
            out.push([value.re, if self.real { 0.0 } else { value.im }]);
            self.given += 1;
        }
        let kept = (self.taps - 1).min(self.fill);
        self.block.copy_within(self.fill - kept..self.fill, 0);
        if self.level.is_some() {
            self.noise_block.copy_within(self.fill - kept..self.fill, 0);
        }
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

/// Filter the first `fill` samples of `block` into `work`, the samples past it read as zeros.
fn band_pass(
    block: &[Complex32],
    fill: usize,
    work: &mut [Complex32],
    spectrum: &[Complex32],
    (forward, inverse): (&Arc<dyn Fft<f32>>, &Arc<dyn Fft<f32>>),
    scratch: &mut [Complex32],
) {
    work.copy_from_slice(block);
    // Samples past the fill are stale, and zeros there change no output before it.
    work[fill..].fill(Complex32::default());
    forward.process_with_scratch(work, scratch);
    for (bin, filter) in work.iter_mut().zip(spectrum) {
        *bin *= filter;
    }
    inverse.process_with_scratch(work, scratch);
}

#[cfg(test)]
mod tests {
    include!("suppress_tests.rs");
}
