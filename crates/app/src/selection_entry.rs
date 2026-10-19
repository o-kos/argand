//! A selection's values as text a person types, read and written without a toolkit.
//!
//! Time is written in the time ruler's mode and frequency in the frequency
//! ruler's unit, both with the numeric locale, and read back the same way.

use crate::numbers::Numbers;
use crate::time_ruler::Mode;
use argand_core::{FrequencyBand, SampleSpan};

/// One value of a selection a person can type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Field {
    TimeStart,
    TimeEnd,
    TimeLength,
    BandLow,
    BandHigh,
    BandWidth,
}

impl Field {
    pub const TIME: [Self; 3] = [Self::TimeStart, Self::TimeEnd, Self::TimeLength];
    pub const BAND: [Self; 3] = [Self::BandLow, Self::BandHigh, Self::BandWidth];

    pub fn is_time(self) -> bool {
        Self::TIME.contains(&self)
    }

    /// Where the field stands among its axis's start or low edge, end or high edge and extent.
    pub fn index(self) -> usize {
        match self {
            Self::TimeStart | Self::BandLow => 0,
            Self::TimeEnd | Self::BandHigh => 1,
            Self::TimeLength | Self::BandWidth => 2,
        }
    }
}

/// What the typed values are measured against.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Scale {
    pub mode: Mode,
    pub rate: f64,
    /// The capture's length in samples.
    pub total: u64,
    /// The time one pixel of the plot shows, which the readouts' decimals tell apart.
    pub seconds_per_pixel: f64,
    /// The frequency ruler's unit and the hertz one of it stands for.
    pub unit: &'static str,
    pub divisor: f64,
    /// The decimals a frequency is written with in that unit.
    pub decimals: usize,
    /// The capture's band in hertz.
    pub band: (f64, f64),
}

impl Scale {
    /// A span's start, end and length as the selection's badges show them.
    pub fn time_texts(&self, span: SampleSpan) -> [String; 3] {
        self.mode
            .counters(span, self.rate, self.total, self.seconds_per_pixel)
    }

    /// A band's low edge, high edge and width in the ruler's unit.
    pub fn band_texts(&self, band: FrequencyBand) -> [String; 3] {
        [band.low(), band.high(), band.high() - band.low()].map(|hertz| {
            crate::numbers::text(&format!("{:.*}", self.decimals, hertz / self.divisor))
        })
    }

    /// `span` with the time `field` typed as `text`, none where the text gives no span.
    pub fn time(&self, span: SampleSpan, field: Field, text: &str) -> Option<SampleSpan> {
        self.time_in(crate::numbers::current(), span, field, text)
    }

    /// `band` with the frequency `field` typed as `text`, none where the text gives no band.
    pub fn band(&self, band: FrequencyBand, field: Field, text: &str) -> Option<FrequencyBand> {
        self.band_in(crate::numbers::current(), band, field, text)
    }

    fn time_in(
        &self,
        numbers: &Numbers,
        span: SampleSpan,
        field: Field,
        text: &str,
    ) -> Option<SampleSpan> {
        let samples = self.samples(numbers, field, text)?;
        let at = samples.clamp(0, i128::from(self.total)) as u64;
        match field {
            Field::TimeStart => SampleSpan::between(at, span.end()),
            Field::TimeEnd => SampleSpan::between(span.start(), at),
            Field::TimeLength if samples > 0 => {
                let end = span.start().saturating_add(at).min(self.total);
                SampleSpan::between(span.start(), end)
            }
            _ => None,
        }
    }

    /// The boundary or the count of samples `text` names for `field`, rounded to a whole sample.
    ///
    /// A leading `#` names a sample in every mode, as the readouts write one,
    /// and the end then names the last sample selected. Where the mode writes
    /// indexes, as it does where seconds cannot tell samples apart, a plain
    /// number is one too. A whole index is read as an integer, never through `f64`.
    fn samples(&self, numbers: &Numbers, field: Field, text: &str) -> Option<i128> {
        let text = text.trim();
        let (index, text) = match text.strip_prefix('#') {
            Some(rest) => (true, rest),
            None => (self.mode.writes_samples(self.rate, self.total), text),
        };
        if !index {
            return whole(self.seconds(numbers, text)? * self.rate);
        }
        let canonical = numbers.canonical(text.trim()).ok()?;
        let index = match canonical.parse::<i128>() {
            Ok(index) => index,
            Err(_) => whole(number(numbers, text)?)?,
        };
        Some(match field {
            Field::TimeEnd => index.saturating_add(1),
            _ => index,
        })
    }

    /// Seconds written in this mode: a clock of up to three fields, or a number with an optional `s`.
    fn seconds(&self, numbers: &Numbers, text: &str) -> Option<f64> {
        if self.mode == Mode::Seconds {
            return number(numbers, text.trim_end_matches('s'));
        }
        let fields: Vec<&str> = text.split(':').collect();
        let (seconds, whole) = fields.split_last()?;
        if whole.len() > 2 {
            return None;
        }
        let mut total = 0.;
        for field in whole {
            let value = number(numbers, field)?;
            if value < 0. || value.fract() != 0. {
                return None;
            }
            total = total * 60. + value;
        }
        let seconds = number(numbers, seconds)?;
        if !whole.is_empty() && seconds < 0. {
            return None;
        }
        Some(total * 60. + seconds)
    }

    fn band_in(
        &self,
        numbers: &Numbers,
        band: FrequencyBand,
        field: Field,
        text: &str,
    ) -> Option<FrequencyBand> {
        let text = text.trim();
        let text = match text.len().checked_sub(self.unit.len()) {
            Some(at) if text.is_char_boundary(at) && text[at..].eq_ignore_ascii_case(self.unit) => {
                &text[..at]
            }
            _ => text,
        };
        let hertz = number(numbers, text)? * self.divisor;
        if !hertz.is_finite() {
            return None;
        }
        let (floor, ceiling) = self.band;
        let at = hertz.clamp(floor, ceiling);
        match field {
            Field::BandLow => FrequencyBand::between(at, band.high()),
            Field::BandHigh => FrequencyBand::between(band.low(), at),
            Field::BandWidth if hertz > 0. => {
                FrequencyBand::between(band.low(), (band.low() + hertz).min(ceiling))
            }
            _ => None,
        }
    }
}

/// `samples` rounded to a whole count, saturating far beyond any capture.
fn whole(samples: f64) -> Option<i128> {
    samples.is_finite().then(|| samples.round() as i128)
}

/// A number written with the locale's digits, grouping and decimal mark.
fn number(numbers: &Numbers, text: &str) -> Option<f64> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    numbers
        .canonical(text)
        .ok()?
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite())
}

#[cfg(test)]
mod tests {
    include!("selection_entry_tests.rs");
}
