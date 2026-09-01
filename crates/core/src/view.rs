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
    /// The value in column `x` at frequency bin `bin`, counting up from the
    /// lowest.
    ///
    /// Column-major is the layout the transform fills, one whole column per
    /// frame; naming the two dimensions here is what keeps that arithmetic out
    /// of everything that reads the grid.
    pub fn value(&self, x: usize, bin: usize) -> f32 {
        self.values[x * self.height + bin]
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
