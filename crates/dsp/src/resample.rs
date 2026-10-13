//! Bounded fractional-rate reconstruction of a filtered complex baseband.

use std::collections::VecDeque;

use crate::extract::{ExtractError, bessel_i0, sinc};

const PHASES: usize = 1024;
const OUTPUT_BATCH: usize = 4096;

/// A fractional-delay Kaiser-windowed sinc bank, with 80 dB rejection.
pub struct Plan {
    input_rate: f64,
    output_rate: f64,
    half: usize,
    kernels: Vec<Vec<f32>>,
}

impl Plan {
    /// Reconstruct a centred band of `width` hertz at `output_rate`.
    pub fn new(input_rate: f64, output_rate: f64, width: f64) -> Result<Self, ExtractError> {
        for rate in [input_rate, output_rate] {
            if !rate.is_finite() || rate <= 0.0 {
                return Err(ExtractError::BadRate(rate));
            }
        }
        if !width.is_finite() || width <= 0.0 || width > input_rate || width >= output_rate {
            return Err(ExtractError::OutsideCapture {
                low: 0.0,
                high: width,
            });
        }
        let available = input_rate.min(output_rate);
        let transition = if width == input_rate {
            (output_rate - width) / 2.0
        } else {
            (available - width) / 2.0
        };
        let estimate =
            ((80.0 - 7.95) * input_rate / (2.285 * std::f64::consts::TAU * transition)).ceil();
        if estimate > 8192.0 {
            return Err(ExtractError::TooNarrow {
                width: transition,
                narrowest: input_rate / 1024.0,
            });
        }
        let half = (estimate as usize).div_ceil(2).max(64);
        let cutoff = ((width + transition) / 2.0).min(input_rate / 2.0);
        let fraction = 2.0 * cutoff / input_rate;
        let norm = bessel_i0(7.85726);
        let kernels = (0..=PHASES)
            .map(|phase| kernel(half, phase as f64 / PHASES as f64, fraction, norm))
            .collect();
        Ok(Self {
            input_rate,
            output_rate,
            half,
            kernels,
        })
    }

    /// Input samples needed on either side of an output coordinate.
    pub fn margin(&self) -> u64 {
        self.half as u64 + 1
    }
}

fn kernel(half: usize, phase: f64, fraction: f64, norm: f64) -> Vec<f32> {
    let raw: Vec<f64> = (0..=2 * half)
        .map(|tap| {
            let offset = tap as f64 - half as f64 - phase;
            let ratio = offset / (half as f64 + 1.0);
            let window = bessel_i0(7.85726 * (1.0 - ratio * ratio).max(0.0).sqrt()) / norm;
            fraction * sinc(fraction * offset) * window
        })
        .collect();
    let sum: f64 = raw.iter().sum();
    raw.into_iter().map(|value| (value / sum) as f32).collect()
}

/// Output coordinates within an intermediate stream, expressed without accumulated phase error.
pub struct Span {
    pub offset: f64,
    pub samples: u64,
}

/// Streaming reconstruction, retaining only the neighbouring sinc samples.
pub struct Stream {
    plan: Plan,
    span: Span,
    buffer: VecDeque<[f32; 2]>,
    start: i64,
    taken: i64,
    given: u64,
}

impl Stream {
    pub fn new(plan: Plan, span: Span) -> Result<Self, ExtractError> {
        let end = span.offset + span.samples as f64 * plan.input_rate / plan.output_rate;
        if !span.offset.is_finite()
            || span.offset < 0.0
            || !end.is_finite()
            || end > (1u64 << 52) as f64
        {
            return Err(ExtractError::TooLong(span.samples));
        }
        Ok(Self {
            plan,
            span,
            buffer: VecDeque::new(),
            start: 0,
            taken: 0,
            given: 0,
        })
    }

    /// Consume part of the input, producing at most 4096 outputs per call.
    /// Call again with the unconsumed suffix until all input has been consumed.
    pub fn push(&mut self, input: &[[f32; 2]], out: &mut Vec<[f32; 2]>) -> usize {
        self.emit(false, out);
        for (index, &sample) in input.iter().enumerate() {
            if self.is_done() {
                return input.len();
            }
            if out.len() >= OUTPUT_BATCH {
                return index;
            }
            self.buffer.push_back(sample);
            self.taken += 1;
            self.emit(false, out);
            self.trim();
        }
        input.len()
    }

    /// Emit at most 4096 remaining outputs, padding only beyond the supplied stream.
    pub fn finish(&mut self, out: &mut Vec<[f32; 2]>) {
        self.emit(true, out);
    }

    pub fn is_done(&self) -> bool {
        self.given == self.span.samples
    }

    fn trim(&mut self) {
        let keep = self.position().floor() as i64 - self.plan.half as i64;
        while self.start < keep && !self.buffer.is_empty() {
            self.buffer.pop_front();
            self.start += 1;
        }
    }

    fn position(&self) -> f64 {
        self.span.offset + self.given as f64 * self.plan.input_rate / self.plan.output_rate
    }

    fn emit(&mut self, finish: bool, out: &mut Vec<[f32; 2]>) {
        while self.given < self.span.samples && out.len() < OUTPUT_BATCH {
            let position = self.position();
            let centre = position.floor() as i64;
            if !finish && centre + self.plan.half as i64 >= self.taken {
                break;
            }
            let phase = position.fract() * PHASES as f64;
            let index = (phase.floor() as usize).min(PHASES - 1);
            let blend = (phase - index as f64) as f32;
            let mut value = [0.0f64; 2];
            for tap in 0..=2 * self.plan.half {
                let at = centre - self.plan.half as i64 + tap as i64;
                let Some(sample) = at
                    .checked_sub(self.start)
                    .and_then(|n| usize::try_from(n).ok())
                    .and_then(|n| self.buffer.get(n))
                else {
                    continue;
                };
                let a = self.plan.kernels[index][tap];
                let b = self.plan.kernels[index + 1][tap];
                let weight = f64::from(a + blend * (b - a));
                for channel in 0..2 {
                    value[channel] += f64::from(sample[channel]) * weight;
                }
            }
            out.push(value.map(|v| v as f32));
            self.given += 1;
            self.trim();
        }
    }
}

#[cfg(test)]
mod tests {
    include!("resample_tests.rs");
}
