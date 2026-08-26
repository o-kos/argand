//! Render outputs: what the core hands to a UI, with no toolkit types attached.
//!
//! `argand-cli` turns these into a PNG; a future GPUI front end uploads the
//! same RGBA buffer as a texture. Nothing here may grow a GUI dependency.

use serde::Serialize;

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
