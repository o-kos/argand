//! Finite two-phase export FIR, independent of the conversion interpolator.

#[cfg(test)]
use super::fir::PARTITION;
use super::fir::{DIRECT_PRODUCTS, Partitioned};
use super::*;
use std::ops::Range;

pub(super) struct ExportExtractor {
    plan: ExtractPlan,
    taken: u64,
    given: u64,
    total: u64,
    needed: u64,
    phase: f64,
    step: f64,
    engine: Convolution,
}

enum Convolution {
    Direct {
        support: Range<u64>,
        input: Vec<Complex32>,
    },
    Partitioned(Box<Partitioned>),
}

impl ExportExtractor {
    pub(super) fn new(
        plan: ExtractPlan,
        len: u64,
        support: Option<Range<u64>>,
    ) -> Result<Self, ExtractError> {
        let margin = plan.margin();
        let full = len
            .checked_add(2 * margin + 1)
            .filter(|&n| n.checked_mul(2).is_some_and(|n| i64::try_from(n).is_ok()))
            .ok_or(ExtractError::TooLong(len))?;
        let total = plan.output_len(len);
        let needed = if total == 0 {
            0
        } else {
            (total - 1) * plan.decimation / 2 + 2 * margin + 1
        };
        let support = support.unwrap_or(0..needed);
        if support.start > support.end || support.end > full - 1 {
            return Err(ExtractError::BadInputSupport);
        }
        let support = support.start.min(needed)..support.end.min(needed);
        let products =
            total.saturating_mul((support.end - support.start).min(plan.taps.len() as u64));
        let engine = if products <= DIRECT_PRODUCTS {
            Convolution::Direct {
                input: vec![Complex32::default(); (support.end - support.start) as usize],
                support,
            }
        } else {
            let mut phases = vec![plan.taps.as_slice()];
            if !plan.half_taps.is_empty() {
                phases.push(&plan.half_taps);
            }
            Convolution::Partitioned(Box::new(Partitioned::polyphase(&phases)))
        };
        let step = -plan.centre / plan.rate;
        Ok(Self {
            phase: (-(margin as f64) * step).rem_euclid(1.0),
            step,
            plan,
            taken: 0,
            given: 0,
            total,
            needed,
            engine,
        })
    }

    pub(super) const fn is_done(&self) -> bool {
        self.given == self.total
    }
    pub(super) fn wanted(&self) -> u64 {
        if self.is_done() {
            0
        } else {
            self.needed - self.taken
        }
    }

    pub(super) fn push(&mut self, input: &[f32], channels: usize, out: &mut Vec<[f32; 2]>) {
        for sample in input.chunks_exact(channels.max(1)) {
            self.take([sample[0], if channels > 1 { sample[1] } else { 0.0 }], out);
        }
    }

    pub(super) fn push_zeros(&mut self, count: u64, out: &mut Vec<[f32; 2]>) {
        for _ in 0..count.min(self.wanted()) {
            self.take([0.0; 2], out);
        }
    }

    fn take(&mut self, [i, q]: [f32; 2], out: &mut Vec<[f32; 2]>) {
        if self.is_done() {
            return;
        }
        let (sin, cos) = (std::f64::consts::TAU * self.phase).sin_cos();
        self.phase = (self.phase + self.step).rem_euclid(1.0);
        let (i, q) = (f64::from(i), f64::from(q));
        let value = Complex32::new((i * cos - q * sin) as f32, (i * sin + q * cos) as f32);
        match &mut self.engine {
            Convolution::Direct { support, input } => {
                if support.contains(&self.taken) {
                    input[(self.taken - support.start) as usize] = value;
                }
            }
            Convolution::Partitioned(engine) => engine.take(value, self.taken),
        }
        self.taken += 1;
        if let Convolution::Partitioned(engine) = &mut self.engine {
            if engine.fill != engine.partition() && self.taken != self.needed {
                return;
            }
            let start = self.taken - engine.fill as u64;
            let intersects = start < self.needed
                && start + 2 * engine.partition() as u64 > 2 * self.plan.margin();
            engine.process(intersects);
        }
        while self.given < self.total {
            let q = self.given * self.plan.decimation;
            let at = 2 * self.plan.margin() + q / 2;
            if at >= self.taken {
                break;
            }
            let phase = (q % 2) as usize;
            let value = match &self.engine {
                Convolution::Direct { support, input } => {
                    let taps = if phase == 0 {
                        &self.plan.taps
                    } else {
                        &self.plan.half_taps
                    };
                    let from = support
                        .start
                        .max((at + 1).saturating_sub(taps.len() as u64));
                    let to = support.end.min(at + 1);
                    let mut sum = Complex32::default();
                    for index in from..to {
                        sum +=
                            input[(index - support.start) as usize] * taps[(at - index) as usize];
                    }
                    sum
                }
                Convolution::Partitioned(engine) => engine.output(phase, at),
            };
            out.push([value.re, value.im]);
            self.given += 1;
        }
    }
}

#[cfg(test)]
#[path = "extract_polyphase_tests.rs"]
mod tests;
