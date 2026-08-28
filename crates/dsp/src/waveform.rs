//! Time-domain min/max envelope over a streamed signal.
//!
//! Columns divide the analysed range linearly, which is exactly how the
//! spectrogram's time ticks are interpolated across its own panel, so an
//! envelope built at the spectrogram's width lines up with it column for
//! column.
//!
//! Min and max are kept per column rather than a decimated value: a burst
//! shorter than one column is usually the thing being looked for, and
//! averaging is what loses it.

use argand_core::WaveformEnvelope;

/// Accumulates a [`WaveformEnvelope`] from blocks of samples as they arrive.
pub struct EnvelopeBuilder {
    columns: usize,
    channels: usize,
    total_samples: u64,
    min: Vec<f32>,
    max: Vec<f32>,
    seen: Vec<bool>,
}

impl EnvelopeBuilder {
    /// `total_samples` is the length of the analysed range, not of the file.
    pub fn new(columns: usize, channels: usize, total_samples: u64) -> Self {
        Self {
            columns,
            channels,
            total_samples,
            min: vec![f32::INFINITY; columns * channels],
            max: vec![f32::NEG_INFINITY; columns * channels],
            seen: vec![false; columns],
        }
    }

    fn is_degenerate(&self) -> bool {
        self.columns == 0 || self.channels == 0 || self.total_samples == 0
    }

    /// Column a sample index falls in.
    fn column_of(&self, sample: u64) -> usize {
        ((sample * self.columns as u64) / self.total_samples).min(self.columns as u64 - 1) as usize
    }

    /// First sample index belonging to `column`; `columns` yields the end.
    fn column_start(&self, column: usize) -> u64 {
        let n = column as u64 * self.total_samples;
        n.div_ceil(self.columns as u64)
    }

    /// Stretch one column's min/max envelope to cover one frame.
    ///
    /// The accumulators start at infinity and only ever take a value that
    /// compared smaller or larger, so they never hold NaN and a NaN sample
    /// never widens them.
    fn widen(&mut self, slot: usize, frame: &[f32]) {
        for (channel, value) in frame.iter().enumerate() {
            let i = slot + channel;
            if *value < self.min[i] {
                self.min[i] = *value;
            }
            if *value > self.max[i] {
                self.max[i] = *value;
            }
        }
    }

    /// Fold channel-interleaved `samples` beginning at range index `first`.
    ///
    /// Blocks must arrive in order and must not overlap; the caller owns that,
    /// because it is the same read loop that drives the transform.
    pub fn fold(&mut self, samples: &[f32], first: u64) {
        if self.is_degenerate() {
            return;
        }
        let count = (samples.len() / self.channels) as u64;
        if count == 0 || first >= self.total_samples {
            return;
        }
        let end = (first + count).min(self.total_samples);

        for column in self.column_of(first)..=self.column_of(end - 1) {
            let lo = self.column_start(column).max(first);
            let hi = self.column_start(column + 1).min(end);
            if lo >= hi {
                continue;
            }
            let from = (lo - first) as usize * self.channels;
            let to = (hi - first) as usize * self.channels;
            let slot = column * self.channels;
            self.seen[column] = true;

            for frame in samples[from..to].chunks_exact(self.channels) {
                self.widen(slot, frame);
            }
        }
    }

    pub fn finish(mut self, t0: f64, t1: f64) -> WaveformEnvelope {
        self.fill_gaps();
        WaveformEnvelope {
            columns: self.columns,
            channels: self.channels,
            min: self.min,
            max: self.max,
            t0,
            t1,
        }
    }

    /// A column no sample landed in borrows its nearest filled neighbour, so a
    /// range shorter than the panel is wide reads as a trace rather than a comb.
    fn fill_gaps(&mut self) {
        let Some(first) = (0..self.columns).find(|&c| self.seen[c]) else {
            self.min.fill(0.0);
            self.max.fill(0.0);
            return;
        };

        let mut source = first;
        for column in 0..self.columns {
            if self.seen[column] {
                source = column;
                continue;
            }
            let from = source * self.channels..(source + 1) * self.channels;
            let to = column * self.channels;
            self.min.copy_within(from.clone(), to);
            self.max.copy_within(from, to);
        }
    }
}

#[cfg(test)]
mod tests {
    include!("waveform_tests.rs");
}
