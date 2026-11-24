//! Human frequency presentation. Values and stored metadata remain physical hertz.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FrequencyUnit {
    Hz,
    KHz,
    MHz,
    GHz,
}

impl FrequencyUnit {
    pub fn symbol(self) -> &'static str {
        match self {
            Self::Hz => "Hz",
            Self::KHz => "kHz",
            Self::MHz => "MHz",
            Self::GHz => "GHz",
        }
    }

    pub fn divisor(self) -> f64 {
        10_f64.powi(self.exponent() as i32)
    }

    pub fn exponent(self) -> usize {
        match self {
            Self::Hz => 0,
            Self::KHz => 3,
            Self::MHz => 6,
            Self::GHz => 9,
        }
    }

    pub fn magnitude(low: f64, high: f64) -> Self {
        let peak = low.abs().max(high.abs());
        if peak >= 1e9 {
            Self::GHz
        } else if peak >= 1e6 {
            Self::MHz
        } else if peak >= 1e3 {
            Self::KHz
        } else {
            Self::Hz
        }
    }

    /// The common coordinate unit, independent of pointer or tooltip visibility.
    pub fn coordinates(low: f64, high: f64, hertz_per_pixel: f64) -> Self {
        let mut unit = Self::magnitude(low, high);
        while unit != Self::Hz && FrequencyFormat::resolution(unit, hertz_per_pixel).decimals() > 3
        {
            unit = unit.smaller();
        }
        unit
    }

    pub fn split_suffix(text: &str) -> (&str, Option<Self>) {
        for unit in [Self::GHz, Self::MHz, Self::KHz, Self::Hz] {
            if let Some(at) = text.len().checked_sub(unit.symbol().len())
                && text.is_char_boundary(at)
                && text[at..].eq_ignore_ascii_case(unit.symbol())
            {
                return (text[..at].trim_end(), Some(unit));
            }
        }
        (text, None)
    }

    fn smaller(self) -> Self {
        match self {
            Self::GHz => Self::MHz,
            Self::MHz => Self::KHz,
            Self::KHz | Self::Hz => Self::Hz,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct FrequencyFormat {
    unit: FrequencyUnit,
    decimals: usize,
}

impl FrequencyFormat {
    pub fn new(unit: FrequencyUnit, decimals: usize) -> Self {
        Self { unit, decimals }
    }

    pub fn unit(self) -> FrequencyUnit {
        self.unit
    }

    pub fn decimals(self) -> usize {
        self.decimals
    }

    pub fn resolution(unit: FrequencyUnit, hertz: f64) -> Self {
        let decimals = (-(hertz / unit.divisor()).log10()).ceil().clamp(0., 9.) as usize;
        Self { unit, decimals }
    }

    /// Tick precision preserves the established unit resolution and non-ladder steps.
    pub fn step(unit: FrequencyUnit, hertz: f64) -> Self {
        let scaled_step = hertz / unit.divisor();
        let cap = unit
            .exponent()
            .max(3)
            .max(Self::resolution(unit, hertz).decimals);
        let full_step = format!("{scaled_step:.cap$}");
        let decimals = (0..cap)
            .find(|&decimals| {
                let scaled = scaled_step * 10_f64.powi(decimals as i32);
                if scaled != scaled.round() {
                    return false;
                }
                // Multiplication can hide a fraction at large magnitudes.
                let mut candidate = format!("{scaled_step:.decimals$}");
                if decimals == 0 {
                    candidate.push('.');
                }
                candidate.push_str(&"0".repeat(cap - decimals));
                candidate == full_step
            })
            .unwrap_or(cap);
        Self { unit, decimals }
    }

    /// Existing measurement precision: one Hz, or a millihertz below one kHz.
    pub fn value(unit: FrequencyUnit, hertz: f64) -> Self {
        let decimals = unit.exponent() + if hertz.abs() < 1e3 { 3 } else { 0 };
        Self::trimmed(unit, hertz, decimals)
    }

    /// Editable fields keep the decimal precision of their canonical Hz value.
    pub fn exact(unit: FrequencyUnit, hertz: f64) -> Self {
        let raw = hertz.to_string();
        let decimals = raw
            .split_once('.')
            .map_or(0, |(_, fraction)| fraction.len());
        Self::trimmed(unit, hertz, decimals + unit.exponent())
    }

    fn trimmed(unit: FrequencyUnit, hertz: f64, decimals: usize) -> Self {
        let text = format!("{:.*}", decimals, hertz / unit.divisor());
        let decimals = text
            .split_once('.')
            .map_or(0, |(_, fraction)| fraction.trim_end_matches('0').len());
        Self { unit, decimals }
    }

    /// Full fractional text for conservative measurement, including integer endpoints.
    pub fn padded(self, hertz: f64) -> String {
        format!("{:.*}", self.decimals, hertz / self.unit.divisor())
    }

    pub fn format(self, hertz: f64) -> String {
        let text = self.padded(hertz);
        let integer = text.split_once('.').and_then(|(whole, fraction)| {
            fraction.chars().all(|digit| digit == '0').then_some(whole)
        });
        match integer.unwrap_or(&text) {
            "-0" => "0".to_owned(),
            value => value.to_owned(),
        }
    }

    pub fn with_unit(self, hertz: f64) -> String {
        format!("{} {}", self.format(hertz), self.unit.symbol())
    }
}

/// Standalone human text when no document/report presentation context exists.
pub fn standalone(hertz: f64) -> String {
    let mut unit = FrequencyUnit::magnitude(hertz, hertz);
    while unit != FrequencyUnit::Hz && FrequencyFormat::value(unit, hertz).decimals() > 3 {
        unit = unit.smaller();
    }
    FrequencyFormat::value(unit, hertz).with_unit(hertz)
}

#[cfg(test)]
#[path = "frequency_tests.rs"]
mod tests;
