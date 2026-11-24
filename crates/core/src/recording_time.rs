//! UTC recording origins. Integer nanoseconds retain sample offsets between edits;
//! file metadata is rounded once to milliseconds at serialization.

use serde::{Serialize, Serializer};

const SECOND: i128 = 1_000_000_000;
const DAY: i128 = 86_400 * SECOND;

#[derive(Debug, Clone, Copy)]
pub struct RecordingTime {
    nanos: i128,
    sample_offset: u64,
    rate_bits: u64,
}

impl RecordingTime {
    /// Convert a filesystem creation timestamp to UTC without host-zone inference.
    pub fn from_timestamp(time: std::time::SystemTime) -> Option<Self> {
        let nanos = match time.duration_since(std::time::UNIX_EPOCH) {
            Ok(duration) => i128::try_from(duration.as_nanos()).ok()?,
            Err(error) => -i128::try_from(error.duration().as_nanos()).ok()?,
        };
        let result = Self {
            nanos,
            sample_offset: 0,
            rate_bits: 0,
        };
        result.is_valid().then_some(result)
    }

    /// Parse a complete RFC3339 timestamp with an explicit UTC offset.
    pub fn parse(text: &str) -> Option<Self> {
        let text = text.trim();
        let b = text.as_bytes();
        if b.len() < 20
            || b[4] != b'-'
            || b[7] != b'-'
            || b[10] != b'T'
            || b[13] != b':'
            || b[16] != b':'
        {
            return None;
        }
        let number = |a, z| {
            let digits = text.get(a..z)?;
            digits
                .bytes()
                .all(|b| b.is_ascii_digit())
                .then(|| digits.parse::<u16>().ok())
                .flatten()
        };
        let fields = [
            number(0, 4)?,
            number(5, 7)?,
            0,
            number(8, 10)?,
            number(11, 13)?,
            number(14, 16)?,
            number(17, 19)?,
            0,
        ];
        let mut result = Self::from_system_time(fields)?;
        let mut at = 19;
        if b.get(at) == Some(&b'.') {
            at += 1;
            let first = at;
            while b.get(at).is_some_and(u8::is_ascii_digit) {
                at += 1;
            }
            let digits = at - first;
            if !(1..=9).contains(&digits) {
                return None;
            }
            result.nanos +=
                text.get(first..at)?.parse::<i128>().ok()? * 10_i128.pow(9 - digits as u32);
        }
        let zone = text.get(at..)?;
        let offset = if zone == "Z" {
            0
        } else {
            let z = zone.as_bytes();
            if z.len() != 6 || !matches!(z[0], b'+' | b'-') || z[3] != b':' {
                return None;
            }
            if !z[1..3].iter().chain(&z[4..6]).all(u8::is_ascii_digit) {
                return None;
            }
            let hours = zone.get(1..3)?.parse::<i128>().ok()?;
            let minutes = zone.get(4..6)?.parse::<i128>().ok()?;
            if hours > 23 || minutes > 59 || zone == "-00:00" {
                return None;
            }
            (hours * 3600 + minutes * 60) * if z[0] == b'+' { 1 } else { -1 }
        };
        result.nanos -= offset * SECOND;
        result.is_valid().then_some(result)
    }

    /// Interpret the SDR auxi SYSTEMTIME calendar fields as UTC.
    pub fn from_system_time(f: [u16; 8]) -> Option<Self> {
        let [year, month, _, day, hour, minute, second, millis] = f;
        if !(1..=9999).contains(&year)
            || !(1..=12).contains(&month)
            || day == 0
            || day > days_in_month(year, month)
            || hour > 23
            || minute > 59
            || second > 59
            || millis > 999
        {
            return None;
        }
        let days = days_from_civil(i64::from(year), i64::from(month), i64::from(day));
        Some(Self {
            sample_offset: 0,
            rate_bits: 0,
            nanos: i128::from(days) * DAY
                + (i128::from(hour) * 3600 + i128::from(minute) * 60 + i128::from(second)) * SECOND
                + i128::from(millis) * 1_000_000,
        })
    }

    /// Rounded auxi fields. A weekday never determines calendar validity.
    pub fn system_time(self) -> [u16; 8] {
        let rounded = self.rounded(3);
        let days = rounded.nanos.div_euclid(DAY);
        let (year, month, day) = civil_from_days(days as i64);
        let remainder = rounded.nanos.rem_euclid(DAY);
        let seconds = remainder / SECOND;
        [
            year as u16,
            month as u16,
            (days + 4).rem_euclid(7) as u16,
            day as u16,
            (seconds / 3600) as u16,
            ((seconds / 60) % 60) as u16,
            (seconds % 60) as u16,
            ((remainder % SECOND) / 1_000_000) as u16,
        ]
    }

    pub fn shifted_samples(self, samples: u64, rate: f64) -> Option<Self> {
        if !rate.is_finite() || rate <= 0.0 {
            return None;
        }
        let result = if self.rate_bits == rate.to_bits() {
            Self {
                sample_offset: self.sample_offset.checked_add(samples)?,
                ..self
            }
        } else {
            Self {
                nanos: self.resolved_nanos(),
                sample_offset: samples,
                rate_bits: rate.to_bits(),
            }
        };
        let seconds = result.sample_offset as f64 / rate;
        if !seconds.is_finite() || seconds > 365_250_000_000.0 {
            return None;
        }
        result.is_valid().then_some(result)
    }

    fn resolved_nanos(self) -> i128 {
        if self.sample_offset == 0 {
            return self.nanos;
        }
        self.nanos
            + (self.sample_offset as f64 / f64::from_bits(self.rate_bits) * SECOND as f64).round()
                as i128
    }

    pub fn shifted_seconds(self, seconds: f64) -> Option<Self> {
        if !seconds.is_finite() || seconds.abs() > 365_250_000_000.0 {
            return None;
        }
        let result = Self {
            sample_offset: 0,
            rate_bits: 0,
            nanos: self
                .resolved_nanos()
                .checked_add((seconds * SECOND as f64).round() as i128)?,
        };
        result.is_valid().then_some(result)
    }

    /// Difference is formed as integers before converting a short relative offset.
    pub fn seconds_since(self, origin: Self) -> f64 {
        (self.resolved_nanos() - origin.resolved_nanos()) as f64 / SECOND as f64
    }

    pub fn rounded(self, precision: usize) -> Self {
        let quantum = 10_i128.pow(9 - precision.min(9) as u32);
        let maximum = i128::from(days_from_civil(10000, 1, 1)) * DAY - quantum;
        let nanos =
            ((self.resolved_nanos() + quantum / 2).div_euclid(quantum) * quantum).min(maximum);
        Self {
            nanos,
            sample_offset: 0,
            rate_bits: 0,
        }
    }

    pub fn format(self, precision: usize) -> String {
        let precision = precision.min(9);
        let rounded = self.rounded(precision);
        let f = rounded.system_time_unrounded();
        let fraction = rounded.nanos.rem_euclid(SECOND) / 10_i128.pow(9 - precision as u32);
        let suffix = if precision == 0 {
            String::new()
        } else {
            format!(".{fraction:0precision$}")
        };
        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}{suffix}Z",
            f[0], f[1], f[3], f[4], f[5], f[6]
        )
    }

    fn system_time_unrounded(self) -> [u16; 8] {
        let days = self.nanos.div_euclid(DAY);
        let (year, month, day) = civil_from_days(days as i64);
        let seconds = self.nanos.rem_euclid(DAY) / SECOND;
        [
            year as u16,
            month as u16,
            0,
            day as u16,
            (seconds / 3600) as u16,
            ((seconds / 60) % 60) as u16,
            (seconds % 60) as u16,
            0,
        ]
    }

    fn is_valid(self) -> bool {
        let nanos = self.resolved_nanos();
        let minimum = i128::from(days_from_civil(1, 1, 1)) * DAY;
        let end = i128::from(days_from_civil(10000, 1, 1)) * DAY;
        nanos >= minimum && nanos < end - 500_000
    }
}

impl PartialEq for RecordingTime {
    fn eq(&self, other: &Self) -> bool {
        self.resolved_nanos() == other.resolved_nanos()
    }
}
impl Eq for RecordingTime {}

impl Serialize for RecordingTime {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.format(3))
    }
}

fn days_in_month(year: u16, month: u16) -> u16 {
    match month {
        4 | 6 | 9 | 11 => 30,
        2 => {
            28 + u16::from(
                year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400)),
            )
        }
        _ => 31,
    }
}

// Proleptic Gregorian era decomposition, with 1970-01-01 as day zero.
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = year - i64::from(month <= 2);
    let era = year.div_euclid(400);
    let y = year - era * 400;
    let m = month + if month > 2 { -3 } else { 9 };
    era * 146097 + y * 365 + y / 4 - y / 100 + (153 * m + 2) / 5 + day - 1 - 719468
}

fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let days = days + 719468;
    let era = days.div_euclid(146097);
    let d = days - era * 146097;
    let y = (d - d / 1460 + d / 36524 - d / 146096) / 365;
    let ordinal = d - (365 * y + y / 4 - y / 100);
    let m = (5 * ordinal + 2) / 153;
    let day = ordinal - (153 * m + 2) / 5 + 1;
    let month = m + if m < 10 { 3 } else { -9 };
    (y + era * 400 + i64::from(month <= 2), month, day)
}

#[cfg(test)]
#[path = "recording_time_test.rs"]
mod tests;
