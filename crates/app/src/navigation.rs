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

    pub fn bounded(self, total: u64, fft: usize, columns: usize) -> Self {
        let min = minimum_span(total, fft, columns);
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
    pub fn zoom(self, factor: f64, anchor: f64, total: u64, fft: usize, columns: usize) -> Self {
        if !factor.is_finite() || factor <= 0.0 || !anchor.is_finite() {
            return self;
        }
        let anchor = anchor.clamp(0.0, 1.0);
        let len = ((self.len as f64 * factor).round() as u64)
            .max(1)
            .clamp(minimum_span(total, fft, columns), total);
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

// Reserve two ULPs per display pixel (and at least ten pixels for 10% pans),
// so interpolation stays distinct across the entire view at extreme counts.
fn minimum_span(total: u64, fft: usize, columns: usize) -> u64 {
    let count = total as f64;
    let precision = (2.0 * (count.next_up() - count) * columns.max(10) as f64).ceil() as u64;
    (fft as u64).max(1).max(precision).min(total)
}

pub fn time_precision(seconds_per_pixel: f64) -> usize {
    (-seconds_per_pixel.log10()).ceil().clamp(3.0, 9.0) as usize
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
pub fn level_at(grid: &DbGrid, shown: (f64, f64), across: f64, from_top: f64) -> Option<f32> {
    let span = grid.t1 - grid.t0;
    if grid.width == 0
        || grid.height == 0
        || !span.is_finite()
        || span <= 0.0
        || !(0.0..1.0).contains(&across)
        || !(0.0..1.0).contains(&from_top)
    {
        return None;
    }
    // Keep interpolation relative to the held grid: adding a pixel offset to
    // a very large absolute time would discard its low bits before lookup.
    let column =
        ((shown.0 - grid.t0) / span + across * ((shown.1 - shown.0) / span)) * grid.width as f64;
    if !(0.0..grid.width as f64).contains(&column) {
        return None;
    }
    let index = |position: f64, cells: usize| {
        ((position + cells as f64 * f64::EPSILON * 8.0).floor() as usize).min(cells - 1)
    };
    let x = index(column, grid.width);
    let row = index(from_top * grid.height as f64, grid.height);
    grid.value(x, grid.height - 1 - row)
}

/// Normalized parts of the view for which the foreground has no data.
pub fn uncovered(shown: (f64, f64), held: Option<(f64, f64)>) -> Vec<(f64, f64)> {
    let Some(held) = held else {
        return vec![(0.0, 1.0)];
    };
    let span = shown.1 - shown.0;
    let left = ((held.0 - shown.0) / span).clamp(0.0, 1.0);
    let right = ((held.1 - shown.0) / span).clamp(0.0, 1.0);
    [(0.0, left), (right, 1.0)]
        .into_iter()
        .filter(|(a, b)| a < b)
        .collect()
}

#[cfg(test)]
mod tests {
    include!("navigation_tests.rs");
}
