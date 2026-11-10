//! Bounded source summaries and exact piece-boundary queries in centered stored units.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use anyhow::{Result, ensure};
use argand_core::Adjustment;
use argand_edit::Capture;
use argand_io::write::SourceFile;

const CELLS: usize = 4096;
const CACHE_ENTRIES: usize = 128;
const BLOCK_VALUES: usize = 65536;

#[derive(Clone, Copy, Debug, Default)]
struct Aggregate {
    energy: f64,
    peak: f64,
    scalar: f64,
    invalid: bool,
}

impl Aggregate {
    fn add(&mut self, other: Self) {
        self.energy += other.energy;
        self.peak = self.peak.max(other.peak);
        self.scalar = self.scalar.max(other.scalar);
        self.invalid |= other.invalid;
    }

    fn fold(&mut self, values: &[f32], channels: usize) {
        for frame in values.chunks_exact(channels) {
            let mut energy = 0.;
            for &value in frame {
                let value = f64::from(value);
                self.invalid |= !value.is_finite();
                energy += value * value;
                self.scalar = self.scalar.max(value.abs());
            }
            self.energy += energy;
            self.peak = self.peak.max(energy.sqrt());
        }
    }
}

/// At most 8192 32-byte nodes, retained with one source's minimap.
#[derive(Clone)]
pub struct Index {
    total: u64,
    channels: usize,
    cells: usize,
    tree: Vec<Aggregate>,
    pub factor: Option<f64>,
}

impl Index {
    pub fn new(total: u64, channels: usize, factor: Option<f64>) -> Self {
        let cells = total.min(CELLS as u64).max(1) as usize;
        let cells = cells.next_power_of_two();
        Self {
            total,
            channels,
            cells,
            tree: vec![Aggregate::default(); cells * 2],
            factor,
        }
    }

    pub fn fold(&mut self, values: &[f32], first: u64) {
        let mut offset = 0;
        let frames = values.len() / self.channels;
        while offset < frames {
            let at = first + offset as u64;
            if at >= self.total {
                break;
            }
            let cell = (u128::from(at) * self.cells as u128 / u128::from(self.total)) as usize;
            let count = (self.edge(cell + 1) - at).min((frames - offset) as u64) as usize;
            self.tree[self.cells + cell].fold(
                &values[offset * self.channels..(offset + count) * self.channels],
                self.channels,
            );
            offset += count;
        }
    }

    pub fn finish(mut self) -> Self {
        for at in (1..self.cells).rev() {
            let mut value = self.tree[at * 2];
            value.add(self.tree[at * 2 + 1]);
            self.tree[at] = value;
        }
        self
    }

    fn edge(&self, cell: usize) -> u64 {
        (u128::from(self.total) * cell as u128).div_ceil(self.cells as u128) as u64
    }

    fn partition(&self, predicate: impl Fn(u64) -> bool) -> usize {
        let mut left = 0;
        let mut right = self.cells + 1;
        while left < right {
            let middle = left + (right - left) / 2;
            if predicate(self.edge(middle)) {
                left = middle + 1;
            } else {
                right = middle;
            }
        }
        left
    }

    fn complete(&self, mut first: usize, mut end: usize) -> Aggregate {
        let mut value = Aggregate::default();
        first += self.cells;
        end += self.cells;
        while first < end {
            if first % 2 == 1 {
                value.add(self.tree[first]);
                first += 1;
            }
            if end % 2 == 1 {
                end -= 1;
                value.add(self.tree[end]);
            }
            first /= 2;
            end /= 2;
        }
        value
    }
}

struct Boundary {
    index: std::sync::Weak<Index>,
    first: u64,
    end: u64,
    value: Aggregate,
}

#[derive(Default)]
pub struct Cache(Mutex<VecDeque<Boundary>>, #[cfg(test)] ReadCounter);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Metrics {
    pub peak: f64,
    pub rms: f64,
}

pub struct Request {
    pub capture: Capture,
    pub sources: Vec<Option<SourceFile>>,
    pub indices: Vec<Option<Arc<Index>>>,
    pub adjustment: Adjustment,
}

/// Only partial source cells require IO; full cells merge through the tree.
pub fn measure(request: &Request, cache: &Cache, active: &dyn Fn() -> bool) -> Result<Metrics> {
    ensure!(!request.capture.is_empty(), "empty capture");
    crate::extraction::unchanged(&request.sources)?;
    let mut energy = 0.;
    let mut peak: f64 = 0.;
    let gain = request.adjustment.gain().abs();
    for piece in request.capture.pieces() {
        ensure!(active(), "statistics cancelled");
        let id = piece.source.0 as usize;
        let file = request
            .sources
            .get(id)
            .and_then(Option::as_ref)
            .ok_or_else(|| anyhow::anyhow!("missing source"))?;
        let index = request
            .indices
            .get(id)
            .and_then(Option::as_ref)
            .ok_or_else(|| anyhow::anyhow!("source statistics pending"))?;
        let factor = file
            .encoding
            .map(|encoding| encoding.factor)
            .or(index.factor)
            .filter(|factor| factor.is_finite() && *factor > 0.)
            .ok_or_else(|| anyhow::anyhow!("unknown sample units"))?;
        let end = piece
            .start
            .checked_add(piece.len)
            .filter(|end| *end <= index.total)
            .ok_or_else(|| anyhow::anyhow!("invalid source span"))?;
        let first_cell = index.partition(|edge| edge < piece.start);
        let end_cell = index.partition(|edge| edge <= end).saturating_sub(1);
        let mut value = index.complete(first_cell.min(end_cell), end_cell);
        let middle_start = index.edge(first_cell.min(end_cell));
        if piece.start < middle_start {
            value.add(boundary(
                file,
                index,
                piece.start,
                middle_start.min(end),
                cache,
                active,
            )?);
        }
        let tail = index.edge(end_cell).max(piece.start);
        if tail < end {
            value.add(boundary(file, index, tail, end, cache, active)?);
        }
        ensure!(
            !value.invalid && ((value.scalar * gain) as f32).is_finite(),
            "nonfinite adjusted samples"
        );
        let scale = factor * gain;
        peak = peak.max(value.peak * scale);
        energy += value.energy * scale * scale;
    }
    crate::extraction::unchanged(&request.sources)?;
    let rms = (energy / request.capture.len() as f64).sqrt();
    ensure!(
        peak.is_finite() && rms.is_finite(),
        "unrepresentable amplitude statistics"
    );
    Ok(Metrics { peak, rms })
}

fn boundary(
    file: &SourceFile,
    index: &Arc<Index>,
    first: u64,
    end: u64,
    cache: &Cache,
    active: &dyn Fn() -> bool,
) -> Result<Aggregate> {
    if let Some(value) = cache
        .0
        .lock()
        .map_err(|_| anyhow::anyhow!("statistics cache unavailable"))?
        .iter()
        .find(|entry| {
            entry.index.ptr_eq(&Arc::downgrade(index)) && entry.first == first && entry.end == end
        })
        .map(|entry| entry.value)
    {
        return Ok(value);
    }
    #[cfg(test)]
    record_read_for_test(cache, end - first);
    let mut reader = crate::extraction::reopen(file)?;
    reader.seek(first)?;
    let mut left = end - first;
    let mut buffer = vec![0.; BLOCK_VALUES];
    let mut value = Aggregate::default();
    while left > 0 {
        ensure!(active(), "statistics cancelled");
        let frames = left.min((BLOCK_VALUES / index.channels) as u64) as usize;
        let values = frames * index.channels;
        ensure!(
            reader.read(&mut buffer[..values])? == values,
            "source ended during statistics"
        );
        value.fold(&buffer[..values], index.channels);
        left -= frames as u64;
    }
    let mut entries = cache
        .0
        .lock()
        .map_err(|_| anyhow::anyhow!("statistics cache unavailable"))?;
    if entries.len() == CACHE_ENTRIES {
        entries.pop_front();
    }
    entries.push_back(Boundary {
        index: Arc::downgrade(index),
        first,
        end,
        value,
    });
    Ok(value)
}

#[cfg(test)]
#[path = "sample_statistics_tests.rs"]
mod tests;

#[cfg(test)]
include!("sample_statistics_support_test.rs");
