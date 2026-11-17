//! Whole-band interpolation with the unchanged odd-phase FIR.

#[cfg(test)]
use super::fir::PARTITION;
use super::fir::{DIRECT_PRODUCTS, Partitioned, ring_index};
use super::*;
use std::ops::Range;

pub(super) struct WholeExtractor {
    plan: ExtractPlan,
    len: u64,
    taken: u64,
    given: u64,
    needed: u64,
    phase: f64,
    engine: Convolution,
}

enum Convolution {
    Direct {
        support: Range<u64>,
        input: Vec<Complex32>,
    },
    Partitioned(Box<Partitioned>),
}

impl WholeExtractor {
    pub(super) fn new(
        plan: ExtractPlan,
        len: u64,
        support: Option<Range<u64>>,
    ) -> Result<Self, ExtractError> {
        let margin = plan.margin();
        let needed = len
            .checked_add(2 * margin)
            .filter(|&n| n.checked_mul(2).is_some_and(|n| i64::try_from(n).is_ok()))
            .ok_or(ExtractError::TooLong(len))?;
        let support = support.unwrap_or(0..needed);
        if support.start > support.end || support.end > needed {
            return Err(ExtractError::BadInputSupport);
        }
        // Each odd output uses at most this many support samples. This bound
        // includes real capture neighbours, not merely the selected span.
        let products =
            len.saturating_mul((support.end - support.start).min(plan.taps.len() as u64));
        let engine = if products <= DIRECT_PRODUCTS {
            Convolution::Direct {
                input: vec![Complex32::default(); (support.end - support.start) as usize],
                support,
            }
        } else {
            Convolution::Partitioned(Box::new(Partitioned::new(&plan.taps)))
        };
        Ok(Self {
            plan,
            len,
            taken: 0,
            given: 0,
            needed,
            phase: 0.0,
            engine,
        })
    }

    pub(super) const fn is_done(&self) -> bool {
        self.given == self.len
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
            self.take(
                Complex32::new(sample[0], if channels > 1 { sample[1] } else { 0.0 }),
                out,
            );
        }
    }

    pub(super) fn push_zeros(&mut self, count: u64, out: &mut Vec<[f32; 2]>) {
        for _ in 0..count.min(self.wanted()) {
            self.take(Complex32::default(), out);
        }
    }

    fn take(&mut self, value: Complex32, out: &mut Vec<[f32; 2]>) {
        if self.is_done() {
            return;
        }
        let margin = self.plan.margin();
        match &mut self.engine {
            Convolution::Direct { support, input } => {
                if support.contains(&self.taken) {
                    input[(self.taken - support.start) as usize] = value;
                }
            }
            Convolution::Partitioned(engine) => engine.take(value, self.taken),
        }
        self.taken += 1;
        match &mut self.engine {
            Convolution::Direct { .. } => {}
            Convolution::Partitioned(engine) => {
                if engine.fill != engine.partition() && self.taken != self.needed {
                    return;
                }
                let start = self.taken - engine.fill as u64;
                let intersects =
                    start < self.needed && start + 2 * engine.partition() as u64 > 2 * margin;
                engine.process(intersects);
            }
        }
        while self.given < self.len && 2 * margin + self.given < self.taken {
            let causal = 2 * margin + self.given;
            let native_at = margin + self.given;
            let (native, odd) = match &self.engine {
                Convolution::Direct { support, input } => {
                    let native = if support.contains(&native_at) {
                        input[(native_at - support.start) as usize]
                    } else {
                        Complex32::default()
                    };
                    let from = support
                        .start
                        .max((causal + 1).saturating_sub(self.plan.taps.len() as u64));
                    let to = support.end.min(causal + 1);
                    let mut sum = Complex32::default();
                    for at in from..to {
                        sum += input[(at - support.start) as usize]
                            * self.plan.taps[(causal - at) as usize];
                    }
                    (native, sum)
                }
                Convolution::Partitioned(engine) => (
                    engine.native[ring_index(native_at, engine.native.len())],
                    engine.output(0, causal),
                ),
            };
            for value in [native, odd] {
                let mixed = if self.plan.centre == 0.0 {
                    value
                } else {
                    let (sin, cos) = (std::f64::consts::TAU * self.phase).sin_cos();
                    self.phase =
                        (self.phase - self.plan.centre / (2.0 * self.plan.rate)).rem_euclid(1.0);
                    Complex32::new(
                        (f64::from(value.re) * cos - f64::from(value.im) * sin) as f32,
                        (f64::from(value.re) * sin + f64::from(value.im) * cos) as f32,
                    )
                };
                out.push([mixed.re, mixed.im]);
            }
            self.given += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_engines_match_direct_fir_across_partitions_and_partial_blocks() {
        let mut plan = ExtractPlan::for_export(48_000.0, -24_000.0, 23_999.5, false).unwrap();
        plan.taps = (0..38)
            .map(|n| ((n * 17 % 23) as f32 - 11.0) / 97.0)
            .collect();
        let len = PARTITION + 13;
        let input: Vec<_> = (0..len)
            .map(|n| Complex32::new((n * 13 % 101) as f32 / 101.0, (n * 7 % 59) as f32 / 59.0))
            .collect();
        let margin = plan.margin();
        let support = margin..margin + len as u64;
        let mut expected = Vec::new();
        let mut phase = 0.0f64;
        for n in 0..len {
            let mut odd = Complex32::default();
            for (k, &tap) in plan.taps.iter().enumerate() {
                let at = margin as i64 + n as i64 - k as i64;
                if at >= 0 && (at as usize) < len {
                    odd += input[at as usize] * tap;
                }
            }
            for value in [input[n], odd] {
                let (sin, cos) = (std::f64::consts::TAU * phase).sin_cos();
                phase = (phase - plan.centre / (2.0 * plan.rate)).rem_euclid(1.0);
                expected.push([
                    (f64::from(value.re) * cos - f64::from(value.im) * sin) as f32,
                    (f64::from(value.re) * sin + f64::from(value.im) * cos) as f32,
                ]);
            }
        }
        for partitioned in [false, true] {
            for chunk in [1, 137, PARTITION + 5] {
                let mut engine =
                    WholeExtractor::new(plan.clone(), len as u64, Some(support.clone())).unwrap();
                engine.engine = if partitioned {
                    Convolution::Partitioned(Box::new(Partitioned::new(&plan.taps)))
                } else {
                    Convolution::Direct {
                        support: support.clone(),
                        input: vec![Complex32::default(); len],
                    }
                };
                let mut out = Vec::new();
                engine.push_zeros(margin, &mut out);
                for block in input.chunks(chunk) {
                    let values: Vec<_> = block.iter().flat_map(|v| [v.re, v.im]).collect();
                    engine.push(&values, 2, &mut out);
                }
                engine.push_zeros(engine.wanted(), &mut out);
                assert!(engine.is_done());
                assert_output(&out, &expected, partitioned, chunk);
            }
        }
    }

    fn assert_output(out: &[[f32; 2]], expected: &[[f32; 2]], partitioned: bool, chunk: usize) {
        assert_eq!(out.len(), expected.len());
        for (n, (actual, expected)) in out.iter().zip(expected).enumerate() {
            for c in 0..2 {
                assert!(
                    (actual[c] - expected[c]).abs() < 2e-6,
                    "partitioned={partitioned}, chunk={chunk}, output={n}: {actual:?} vs {expected:?}"
                );
            }
            if n.is_multiple_of(2) {
                assert_eq!(actual.map(f32::to_bits), expected.map(f32::to_bits));
            }
        }
    }

    #[test]
    fn native_ring_coordinates_do_not_wrap_at_the_32_bit_boundary() {
        let len = PARTITION + 19;
        for at in [
            u32::MAX as u64 - 2,
            u32::MAX as u64,
            u32::MAX as u64 + 1,
            u32::MAX as u64 + 32_777,
        ] {
            assert_eq!(ring_index(at, len), (u128::from(at) % len as u128) as usize);
        }
        let at = u32::MAX as u64 + 1;
        assert_ne!(ring_index(at, len), (at as u32 as usize) % len);
    }

    #[test]
    fn multiple_filter_partitions_match_an_independent_sparse_fir() {
        let mut plan = ExtractPlan::for_export(48_000.0, -24_000.0, 24_000.0, false).unwrap();
        let taps = [
            (0, -0.13),
            (PARTITION - 1, 0.25),
            (PARTITION, 0.11),
            (PARTITION + 1, -0.17),
            (2 * PARTITION, 0.3),
            (2 * PARTITION + 37, 0.1),
        ];
        plan.taps = vec![0.0; 2 * PARTITION + 38];
        for &(at, value) in &taps {
            plan.taps[at] = value;
        }
        let len = 2 * PARTITION + 13;
        let input: Vec<_> = (0..len)
            .map(|n| {
                Complex32::new(
                    (((n * 71) ^ (n >> 5)) % 10007) as f32 / 10007.0 - 0.5,
                    (((n * 37) ^ (n >> 3)) % 5003) as f32 / 5003.0 - 0.5,
                )
            })
            .collect();
        let margin = plan.margin();
        let mut expected = Vec::new();
        for n in 0..len {
            let mut odd = Complex32::default();
            for &(k, tap) in &taps {
                let at = margin as i64 + n as i64 - k as i64;
                if at >= 0 && (at as usize) < len {
                    odd += input[at as usize] * tap;
                }
            }
            expected.push([input[n].re, input[n].im]);
            expected.push([odd.re, odd.im]);
        }
        for chunk in [1, 137, PARTITION + 5] {
            let mut engine = WholeExtractor::new(plan.clone(), len as u64, None).unwrap();
            assert!(matches!(engine.engine, Convolution::Partitioned(_)));
            let mut out = Vec::new();
            engine.push_zeros(margin, &mut out);
            for block in input.chunks(chunk) {
                let values: Vec<_> = block.iter().flat_map(|v| [v.re, v.im]).collect();
                engine.push(&values, 2, &mut out);
            }
            engine.push_zeros(engine.wanted(), &mut out);
            assert!(engine.is_done());
            assert_output(&out, &expected, true, chunk);
        }
    }

    #[test]
    fn support_bound_selects_direct_without_ignoring_capture_neighbours() {
        let plan = ExtractPlan::for_export(1000.0, -500.0, 500.0, false).unwrap();
        let margin = plan.margin();
        let short = WholeExtractor::new(plan.clone(), 100, Some(margin..margin + 1)).unwrap();
        assert!(matches!(short.engine, Convolution::Direct { .. }));
        let neighbours = WholeExtractor::new(plan.clone(), 100, None).unwrap();
        assert!(matches!(neighbours.engine, Convolution::Partitioned(_)));
        assert!(matches!(
            WholeExtractor::new(plan, 100, Some(0..u64::MAX)),
            Err(ExtractError::BadInputSupport)
        ));
    }
}
