//! Render outputs: what the core hands to a UI, with no toolkit types attached.
//!
//! `argand-cli` turns these into a PNG; a future GPUI front end uploads the
//! same RGBA buffer as a texture. Nothing here may grow a GUI dependency.

use serde::Serialize;

/// Decibel values and the shape they are laid out in, before any colour is
/// chosen for them.
///
/// This is what a transform actually produced. Shading it is a separate step,
/// so a front end that changes the colour scheme or the dynamic range recolours
/// a grid it already holds instead of running the transform again over numbers
/// that did not change.
///
/// The extents travel with the values because they describe the same picture,
/// and a caller holding the grid has no other way back to the signal it came
/// from.
#[derive(Debug, Clone)]
pub struct DbGrid {
    pub width: usize,
    pub height: usize,
    /// Column-major, `width * height` values: `values[x * height + bin]`, with
    /// bin 0 the lowest frequency.
    pub values: Vec<f32>,
    /// Time extent in seconds, from the start of the file.
    pub t0: f64,
    pub t1: f64,
    /// Frequency extent in Hz, already offset by the centre frequency.
    pub f0: f64,
    pub f1: f64,
}

impl DbGrid {
    /// The shape the values actually cover, or `None` when they do not cover
    /// the one the grid declares.
    ///
    /// The fields are a caller's to set, so anything sizing a buffer from
    /// `width` and `height` has to ask here rather than multiply them: a
    /// height near `usize::MAX` makes that product an overflow, not a picture.
    pub fn shape(&self) -> Option<(usize, usize)> {
        let cells = self.width.checked_mul(self.height)?;
        (cells == self.values.len()).then_some((self.width, self.height))
    }

    /// The value in column `x` at frequency bin `bin`, counting up from the
    /// lowest, or `None` outside the grid.
    ///
    /// Column-major is the layout the transform fills, one whole column per
    /// frame; naming the two dimensions here is what keeps that arithmetic out
    /// of everything that reads the grid. Both are checked, because one index
    /// past the end of a column is a valid offset into the next one, and a
    /// caller that asked for a bin it does not have would get an answer that
    /// looks like data.
    pub fn value(&self, x: usize, bin: usize) -> Option<f32> {
        self.column(x)?.get(bin).copied()
    }

    /// Every bin of column `x`, lowest frequency first.
    ///
    /// This is how the grid is stored, so a reader working down a column gets
    /// the slice rather than a multiplication per value. It is also the one
    /// place the offset is worked out, which is why the arithmetic is checked
    /// here: the fields are a caller's to set, and a height near `usize::MAX`
    /// would otherwise overflow past the bounds test rather than fail it.
    pub fn column(&self, x: usize) -> Option<&[f32]> {
        if x >= self.width {
            return None;
        }
        let start = x.checked_mul(self.height)?;
        self.values.get(start..start.checked_add(self.height)?)
    }
}

/// A rendered spectrogram plus the axis extents it was drawn for.
#[derive(Debug, Clone)]
pub struct SpectrogramImage {
    pub width: usize,
    pub height: usize,
    /// Row-major RGBA8, `width * height * 4` bytes. Row 0 is the top of the
    /// image, which is the *highest* frequency.
    pub rgba: Vec<u8>,
    /// Time extent in seconds, from the start of the file.
    pub t0: f64,
    pub t1: f64,
    /// Frequency extent in Hz, already offset by the centre frequency.
    pub f0: f64,
    pub f1: f64,
    /// dB window the colours were mapped over.
    pub db_min: f32,
    pub db_max: f32,
}

impl SpectrogramImage {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            rgba: vec![0; width * height * 4],
            t0: 0.0,
            t1: 0.0,
            f0: 0.0,
            f1: 0.0,
            db_min: 0.0,
            db_max: 0.0,
        }
    }

    pub fn put(&mut self, x: usize, y: usize, rgb: [u8; 3]) {
        let i = (y * self.width + x) * 4;
        self.rgba[i] = rgb[0];
        self.rgba[i + 1] = rgb[1];
        self.rgba[i + 2] = rgb[2];
        self.rgba[i + 3] = 255;
    }

    pub fn get(&self, x: usize, y: usize) -> [u8; 4] {
        let i = (y * self.width + x) * 4;
        [
            self.rgba[i],
            self.rgba[i + 1],
            self.rgba[i + 2],
            self.rgba[i + 3],
        ]
    }
}

/// A time-domain min/max envelope, one column per output pixel.
///
/// Min and max are kept rather than a single decimated value because a burst
/// shorter than a column is the thing a capture is usually being checked for,
/// and averaging is exactly what loses it.
///
/// Values stay linear and on the [-1, 1] scale the transform worked with.
/// Mapping them to a decibel window is a presentation choice and belongs to
/// whatever draws the envelope.
#[derive(Debug, Clone)]
pub struct WaveformEnvelope {
    pub columns: usize,
    /// 1 for a real signal, 2 for interleaved I/Q.
    pub channels: usize,
    /// `columns * channels` values, channel-interleaved: every channel of
    /// column 0, then every channel of column 1.
    pub min: Vec<f32>,
    pub max: Vec<f32>,
    /// Time extent in seconds, from the start of the file.
    pub t0: f64,
    pub t1: f64,
}

impl WaveformEnvelope {
    pub fn new(columns: usize, channels: usize) -> Self {
        Self {
            columns,
            channels,
            min: vec![0.0; columns * channels],
            max: vec![0.0; columns * channels],
            t0: 0.0,
            t1: 0.0,
        }
    }

    /// Lowest and highest value `column` reached on `channel`.
    pub fn column(&self, column: usize, channel: usize) -> Option<(f32, f32)> {
        if channel >= self.channels {
            return None;
        }
        let i = column * self.channels + channel;
        Some((*self.min.get(i)?, *self.max.get(i)?))
    }

    /// Merged channel spans in pixel offsets from zero, with positive values up.
    /// Adjacent spans are joined so sparse traces remain continuous. Front ends
    /// choose the time direction and translate these offsets into their canvas.
    pub fn pixel_spans(
        &self,
        columns: usize,
        half: i64,
        full_scale: f32,
    ) -> impl Iterator<Item = Option<(i64, i64)>> + '_ {
        self.spans_at(
            (0..columns).map(move |step| Some(step * self.columns / columns.max(1))),
            half,
            full_scale,
        )
    }

    /// Paint a held envelope in a different time view without allocating a
    /// stretched buffer. Uncovered columns break continuity.
    pub fn pixel_spans_in(
        &self,
        columns: usize,
        half: i64,
        full_scale: f32,
        seconds: (f64, f64),
    ) -> impl Iterator<Item = Option<(i64, i64)>> + '_ {
        let indices = (0..columns).map(move |step| {
            let time = seconds.0 + step as f64 / columns.max(1) as f64 * (seconds.1 - seconds.0);
            if !(self.t0..self.t1).contains(&time) {
                return None;
            }
            Some(((time - self.t0) / (self.t1 - self.t0) * self.columns as f64) as usize)
        });
        self.spans_at(indices, half, full_scale)
    }

    fn spans_at(
        &self,
        indices: impl Iterator<Item = Option<usize>>,
        half: i64,
        full_scale: f32,
    ) -> impl Iterator<Item = Option<(i64, i64)>> {
        let full_scale = full_scale.max(1e-6);
        let offset = move |value: f32| {
            let level = (value.abs() / full_scale).clamp(0.0, 1.0);
            let distance = (level * half as f32).round() as i64;
            if value >= 0.0 { distance } else { -distance }
        };
        let mut previous: Option<(i64, i64)> = None;
        indices.map(move |column| {
            let Some(column) = column else {
                previous = None;
                return None;
            };
            let mut span: Option<(i64, i64)> = None;
            for channel in 0..self.channels {
                let (min, max) = self.column(column, channel)?;
                let (lo, hi) = (offset(min).min(offset(max)), offset(min).max(offset(max)));
                span = Some(match span {
                    Some((s_lo, s_hi)) => (s_lo.min(lo), s_hi.max(hi)),
                    None => (lo, hi),
                });
            }
            let (mut lo, mut hi) = span?;
            if let Some((prev_lo, prev_hi)) = previous {
                lo = lo.min(prev_hi);
                hi = hi.max(prev_lo);
            }
            previous = Some((lo, hi));
            previous
        })
    }

    /// Largest excursion from zero anywhere in the envelope.
    pub fn peak(&self) -> f32 {
        self.min
            .iter()
            .chain(self.max.iter())
            .filter(|v| v.is_finite())
            .fold(0.0f32, |m, v| m.max(v.abs()))
    }
}

/// An averaged power spectrum.
#[derive(Debug, Clone, Serialize)]
pub struct Psd {
    /// Bin centre frequencies in Hz, already offset by the centre frequency.
    pub freqs_hz: Vec<f64>,
    /// Magnitude per bin in dBFS.
    pub db: Vec<f32>,
    /// Segments averaged together.
    pub segments: u64,
}

/// A located maximum in a spectrum.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct SpectrumPeak {
    pub bin: usize,
    /// Offset from the centre frequency in Hz.
    pub offset_hz: f64,
    /// Absolute frequency in Hz.
    pub freq_hz: f64,
    /// Linear magnitude, full scale = 1.0.
    pub magnitude: f32,
    pub db: f32,
}

impl Psd {
    /// Strongest bin, or `None` for an empty spectrum.
    pub fn peak(&self, center_freq: f64) -> Option<SpectrumPeak> {
        let (bin, &db) = self
            .db
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.total_cmp(b.1))?;
        let freq_hz = *self.freqs_hz.get(bin)?;
        Some(SpectrumPeak {
            bin,
            offset_hz: freq_hz - center_freq,
            freq_hz,
            magnitude: 10f32.powf(db / 20.0),
            db,
        })
    }

    /// Noise floor estimate: the median bin, which is robust to a few strong
    /// carriers in a mostly empty band.
    pub fn floor_db(&self) -> Option<f32> {
        if self.db.is_empty() {
            return None;
        }
        let mut sorted = self.db.clone();
        sorted.sort_by(f32::total_cmp);
        Some(sorted[sorted.len() / 2])
    }
}

#[cfg(test)]
mod tests {
    include!("view_tests.rs");
}
