//! Sample-based time navigation and lookup in the picture currently displayed.

use argand_core::{DbGrid, SampleRange};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct View {
    pub start: u64,
    pub len: u64,
}

impl View {
    pub const fn full(total: u64) -> Self {
        Self {
            start: 0,
            len: total,
        }
    }

    pub fn bounded(self, total: u64, fft: usize) -> Self {
        let min = (fft as u64).max(1).min(total);
        let len = if self.len == 0 {
            total
        } else {
            self.len.clamp(min, total)
        };
        Self {
            start: self.start.min(total - len),
            len,
        }
    }

    pub fn range(self) -> SampleRange {
        SampleRange::new(self.start, self.len)
    }

    pub fn seconds(self, rate: f64) -> (f64, f64) {
        (
            self.start as f64 / rate,
            (self.start + self.len) as f64 / rate,
        )
    }

    /// Keep the sample under `anchor` at the same fractional screen position.
    pub fn zoom(self, factor: f64, anchor: f64, total: u64, fft: usize) -> Self {
        if !factor.is_finite() || factor <= 0.0 || !anchor.is_finite() {
            return self;
        }
        let anchor = anchor.clamp(0.0, 1.0);
        let len = ((self.len as f64 * factor).round() as u64)
            .max(1)
            .clamp((fft as u64).max(1).min(total), total);
        let shift = (self.len as f64 - len as f64) * anchor;
        Self {
            start: shifted(self.start, shift, total - len),
            len,
        }
    }

    pub fn pan(self, fraction: f64, total: u64) -> Self {
        Self {
            start: shifted(self.start, self.len as f64 * fraction, total - self.len),
            ..self
        }
    }
}

fn shifted(start: u64, delta: f64, max: u64) -> u64 {
    if !delta.is_finite() {
        return start;
    }
    if delta >= 0.0 {
        start.saturating_add(delta.round() as u64).min(max)
    } else {
        start.saturating_sub((-delta).round() as u64).min(max)
    }
}

/// Location of a held image within the requested time interval, in plot widths.
pub fn image_mapping(held: (f64, f64), shown: (f64, f64)) -> (f64, f64) {
    let span = shown.1 - shown.0;
    ((held.0 - shown.0) / span, (held.1 - held.0) / span)
}

/// Source columns needed for a deeply zoomed placeholder; no stretched buffer.
pub fn visible_columns(
    held: (f64, f64),
    shown: (f64, f64),
    width: usize,
) -> std::ops::Range<usize> {
    let index =
        |time: f64| ((time - held.0) / (held.1 - held.0) * width as f64).clamp(0.0, width as f64);
    index(shown.0).floor() as usize..index(shown.1).ceil() as usize
}

/// Clip in f64 before conversion to GPU coordinates, even at huge zoom ratios.
pub fn column_mapping(
    held: (f64, f64),
    shown: (f64, f64),
    width: usize,
    column: usize,
) -> (f64, f64) {
    let edge = |index: usize| {
        let time = held.0 + index as f64 / width as f64 * (held.1 - held.0);
        ((time - shown.0) / (shown.1 - shown.0)).clamp(0.0, 1.0)
    };
    (edge(column), edge(column + 1))
}

/// Use the shaded cell, with the highest frequency at the top of the image.
pub fn level_at(grid: &DbGrid, time: f64, from_top: f64) -> Option<f32> {
    if grid.width == 0
        || grid.height == 0
        || !time.is_finite()
        || !(grid.t0..grid.t1).contains(&time)
        || !(0.0..1.0).contains(&from_top)
    {
        return None;
    }
    let x = ((time - grid.t0) / (grid.t1 - grid.t0) * grid.width as f64) as usize;
    let row = (from_top * grid.height as f64) as usize;
    grid.value(x, grid.height - 1 - row)
}

#[cfg(test)]
mod tests {
    include!("navigation_tests.rs");
}
