//! Shared partitioned FIR banks with one input-spectrum history.

use super::*;

pub(super) const PARTITION: usize = 32_768;
pub(super) const DIRECT_PRODUCTS: u64 = 1 << 20;

pub(super) fn ring_index(at: u64, len: usize) -> usize {
    (at % len as u64) as usize
}

struct Bank {
    filters: Vec<Vec<Complex32>>,
    overlap: Vec<Complex32>,
    output: Vec<Complex32>,
}

pub(super) struct Partitioned {
    forward: Arc<dyn Fft<f32>>,
    inverse: Arc<dyn Fft<f32>>,
    banks: Vec<Bank>,
    history: Vec<Vec<Complex32>>,
    slot: usize,
    partition: usize,
    block: Vec<Complex32>,
    pub(super) fill: usize,
    work: Vec<Complex32>,
    scratch: Vec<Complex32>,
    pub(super) native: Vec<Complex32>,
}

impl Partitioned {
    pub(super) fn new(taps: &[f32]) -> Self {
        Self::banks(&[taps], Some(taps.len() / 2))
    }

    pub(super) fn polyphase(phases: &[&[f32]]) -> Self {
        Self::banks(phases, None)
    }

    fn banks(phases: &[&[f32]], native_margin: Option<usize>) -> Self {
        let longest = phases.iter().map(|taps| taps.len()).max().unwrap_or(1);
        let partition = longest.next_power_of_two().clamp(256, PARTITION);
        let size = 2 * partition;
        let native_len = native_margin.map_or(0, |margin| margin + partition + 1);
        let count = phases
            .iter()
            .map(|taps| taps.len().div_ceil(partition))
            .max()
            .unwrap_or(1);
        let mut planner = FftPlanner::new();
        let forward = planner.plan_fft_forward(size);
        let inverse = planner.plan_fft_inverse(size);
        let mut scratch = vec![
            Complex32::default();
            forward
                .get_inplace_scratch_len()
                .max(inverse.get_inplace_scratch_len())
        ];
        let banks = phases
            .iter()
            .map(|taps| {
                let filters = taps
                    .chunks(partition)
                    .map(|taps| {
                        let mut spectrum = vec![Complex32::default(); size];
                        for (bin, &tap) in spectrum.iter_mut().zip(taps) {
                            bin.re = tap / size as f32;
                        }
                        forward.process_with_scratch(&mut spectrum, &mut scratch);
                        spectrum
                    })
                    .collect();
                Bank {
                    filters,
                    overlap: vec![Complex32::default(); partition],
                    output: vec![Complex32::default(); partition],
                }
            })
            .collect();
        Self {
            forward,
            inverse,
            banks,
            history: vec![vec![Complex32::default(); size]; count],
            slot: 0,
            partition,
            block: vec![Complex32::default(); size],
            fill: 0,
            work: vec![Complex32::default(); size],
            scratch,
            native: vec![Complex32::default(); native_len],
        }
    }

    pub(super) fn partition(&self) -> usize {
        self.partition
    }

    pub(super) fn take(&mut self, value: Complex32, at: u64) {
        self.block[self.fill] = value;
        if !self.native.is_empty() {
            let index = ring_index(at, self.native.len());
            self.native[index] = value;
        }
        self.fill += 1;
    }

    pub(super) fn output(&self, phase: usize, at: u64) -> Complex32 {
        self.banks[phase].output[ring_index(at, self.partition)]
    }

    pub(super) fn process(&mut self, wanted: bool) {
        self.block[self.fill..].fill(Complex32::default());
        self.history[self.slot].copy_from_slice(&self.block);
        self.forward
            .process_with_scratch(&mut self.history[self.slot], &mut self.scratch);
        if wanted {
            self.convolve_banks();
        } else {
            for bank in &mut self.banks {
                bank.overlap.fill(Complex32::default());
            }
        }
        self.slot = (self.slot + 1) % self.history.len();
        self.fill = 0;
    }
    fn convolve_banks(&mut self) {
        for bank in &mut self.banks {
            self.work.fill(Complex32::default());
            for (partition, filter) in bank.filters.iter().enumerate() {
                let input = &self.history
                    [(self.slot + self.history.len() - partition) % self.history.len()];
                for ((sum, &input), &filter) in self.work.iter_mut().zip(input).zip(filter) {
                    *sum += input * filter;
                }
            }
            self.inverse
                .process_with_scratch(&mut self.work, &mut self.scratch);
            for (out, (&value, &overlap)) in bank
                .output
                .iter_mut()
                .zip(self.work.iter().zip(&bank.overlap))
            {
                *out = value + overlap;
            }
            bank.overlap.copy_from_slice(&self.work[self.partition..]);
        }
    }
}
