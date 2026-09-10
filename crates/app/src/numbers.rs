//! GUI numeric presentation and input; persisted and CLI values stay canonical.

use std::{fmt::Display, str::FromStr, sync::OnceLock};

use icu_decimal::{DecimalFormatter, input::Decimal, options::GroupingStrategy};
use icu_locale_core::Locale;

#[path = "numeric_locale.rs"]
mod system;

static CURRENT: OnceLock<Numbers> = OnceLock::new();

pub fn initialize(format: &str) {
    CURRENT.get_or_init(|| configured(format, system::locale));
}

pub fn valid_format(format: &str) -> bool {
    matches!(format, "system" | "C" | "POSIX") || format.parse::<Locale>().is_ok()
}

fn configured(format: &str, system: impl FnOnce() -> String) -> Numbers {
    if format == "system" {
        Numbers::new(&system())
    } else {
        Numbers::new(format)
    }
}

pub fn current() -> &'static Numbers {
    // Tests use a deterministic locale without mutating the process environment.
    CURRENT.get_or_init(|| Numbers::new("en-US"))
}

pub fn number(value: impl Display) -> String {
    current().format(&value.to_string(), true)
}

pub fn input(value: impl Display) -> String {
    current().format(&value.to_string(), false)
}

pub fn parse<T: FromStr>(value: &str) -> Result<T, ()> {
    current().canonical(value)?.parse().map_err(|_| ())
}

pub fn text(value: &str) -> String {
    current().text(value)
}

pub fn clock(value: &str) -> String {
    current().clock(value)
}

pub struct Numbers {
    grouped: Option<DecimalFormatter>,
    plain: Option<DecimalFormatter>,
    digits: [char; 10],
    decimal: String,
    grouping: String,
    minus: String,
    plus: String,
}

impl Numbers {
    pub fn new(name: &str) -> Self {
        let c_locale = matches!(name, "C" | "POSIX" | "C.UTF-8" | "C.utf8");
        let locale = name
            .parse::<Locale>()
            .unwrap_or(icu_locale_core::locale!("en-US"));
        let plain =
            DecimalFormatter::try_new(locale.clone().into(), GroupingStrategy::Never.into()).ok();
        let grouped = if c_locale {
            plain.clone()
        } else {
            DecimalFormatter::try_new(locale.into(), Default::default()).ok()
        };
        let mut result = Self {
            grouped,
            plain,
            digits: ['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'],
            decimal: ".".into(),
            grouping: String::new(),
            minus: "-".into(),
            plus: "+".into(),
        };
        for digit in 0..10 {
            result.digits[digit] = result
                .format(&digit.to_string(), false)
                .chars()
                .next()
                .unwrap_or(result.digits[digit]);
        }
        let symbols = |value: &str, grouping| {
            result
                .format(value, grouping)
                .chars()
                .filter(|c| !result.digits.contains(c))
                .collect::<String>()
        };
        let decimal = symbols("1.1", false);
        let grouping = symbols("1000000", true).chars().take(1).collect();
        let minus = symbols("-1", false);
        let plus = symbols("+1", false);
        result.decimal = decimal;
        result.grouping = grouping;
        result.minus = minus;
        result.plus = plus;
        result
    }

    pub fn axis_label(&self, text: &str, kind: argand_core::axis::AxisKind) -> String {
        use argand_core::axis::AxisKind;
        match kind {
            AxisKind::PreciseTime | AxisKind::Time => {
                let clock = if text.contains(':') {
                    text.to_owned()
                } else {
                    text.replace('.', ":")
                };
                self.clock(&clock)
            }
            AxisKind::Seconds => self.format(text.trim_end_matches(" s"), true),
            AxisKind::Samples => self.format(text.trim_start_matches('#'), true),
            _ => self.format(text, true),
        }
    }

    pub fn digits(&self) -> [char; 10] {
        self.digits
    }

    pub fn format(&self, value: &str, grouped: bool) -> String {
        let normalized = value.replace('−', "-");
        let formatter = if grouped { &self.grouped } else { &self.plain };
        match (formatter, normalized.parse::<Decimal>()) {
            (Some(formatter), Ok(decimal)) => formatter.format_to_string(&decimal),
            _ => value.to_owned(),
        }
    }

    /// Clock fields keep their padding and punctuation; only the fraction is decimal.
    pub fn clock(&self, value: &str) -> String {
        value
            .chars()
            .map(|c| match c {
                '0'..='9' => self.digits[(c as u8 - b'0') as usize].to_string(),
                '.' => self.decimal.clone(),
                _ => c.to_string(),
            })
            .collect()
    }

    /// Use only for numeric descriptions, never identifiers, paths or diagnostics.
    pub fn text(&self, value: &str) -> String {
        let mut result = String::new();
        let mut start = None;
        let mut chars = value.char_indices().chain([(value.len(), '\0')]).peekable();
        while let Some((index, c)) = chars.next() {
            let next_digit = chars.peek().is_some_and(|(_, next)| next.is_ascii_digit());
            let numeric = c.is_ascii_digit()
                || (c == '.' && start.is_some() && next_digit)
                || (start.is_none() && matches!(c, '-' | '−' | '+') && next_digit);
            match (start, numeric) {
                (None, true) => start = Some(index),
                (Some(from), false) => {
                    result.push_str(&self.format(&value[from..index], true));
                    start = None;
                }
                _ => {}
            }
            if !numeric && c != '\0' {
                result.push(c);
            }
        }
        result
    }

    pub fn canonical(&self, value: &str) -> Result<String, ()> {
        let value = value.trim();
        let mut ascii = value.replace(&self.minus, "-").replace(&self.plus, "+");
        for (index, digit) in self.digits.iter().enumerate() {
            ascii = ascii.replace(*digit, &index.to_string());
        }
        let (mantissa, exponent) = ascii.split_once(['e', 'E']).unwrap_or((&ascii, ""));
        let mut mantissa = mantissa.to_owned();
        if self.grouping.chars().all(char::is_whitespace) && !self.grouping.is_empty() {
            mantissa = mantissa
                .chars()
                .map(|c| {
                    if c.is_whitespace() {
                        self.grouping.clone()
                    } else {
                        c.to_string()
                    }
                })
                .collect();
        }
        let grouped = !self.grouping.is_empty() && mantissa.contains(&self.grouping);
        let canonical = if grouped {
            mantissa.replace(&self.grouping, "")
        } else {
            mantissa.clone()
        }
        .replace(&self.decimal, ".");
        if grouped {
            let expected = self.format(&canonical, true);
            let unshaped = self
                .ascii_digits(&expected)
                .replace(&self.minus, "-")
                .replace(&self.plus, "+");
            if unshaped != mantissa {
                return Err(());
            }
        }
        if canonical.contains(|c: char| !c.is_ascii_digit() && !matches!(c, '.' | '-' | '+')) {
            return Err(());
        }
        if !exponent.is_empty() {
            let _: i32 = exponent.parse().map_err(|_| ())?;
            Ok(format!("{canonical}e{exponent}"))
        } else if ascii.contains(['e', 'E']) {
            Err(())
        } else {
            Ok(canonical)
        }
    }

    fn ascii_digits(&self, value: &str) -> String {
        value
            .chars()
            .map(|c| {
                self.digits
                    .iter()
                    .position(|d| *d == c)
                    .map_or(c, |n| char::from(b'0' + n as u8))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configuration_override_controls_labels_hints_and_input_without_reading_system() {
        let numbers = configured("ru-RU", || panic!("override must not read system locale"));
        assert_eq!(
            numbers.axis_label("#424703", argand_core::axis::AxisKind::Samples),
            "424\u{a0}703"
        );
        assert_eq!(
            numbers.axis_label("29.562 s", argand_core::axis::AxisKind::Seconds),
            "29,562"
        );
        assert_eq!(
            numbers.text("424703 samples, 58.987 s"),
            "424\u{a0}703 samples, 58,987 s"
        );
        assert_eq!(numbers.canonical("45,5"), Ok("45.5".into()));
    }

    #[test]
    fn system_setting_preserves_the_detected_numeric_locale() {
        for (locale, expected) in [("C", "1234.5"), ("ru-RU", "1\u{a0}234,5")] {
            let numbers = configured("system", || locale.into());
            assert_eq!(numbers.format("1234.5", true), expected);
        }
    }

    #[test]
    fn decimal_grouping_precision_and_large_integers_follow_locale() {
        for (locale, expected) in [
            ("en-US", "1,234,567.50"),
            ("de-DE", "1.234.567,50"),
            ("ru-RU", "1\u{a0}234\u{a0}567,50"),
            ("hi-IN", "12,34,567.50"),
        ] {
            let numbers = Numbers::new(locale);
            assert_eq!(numbers.format("1234567.50", true), expected);
            assert_eq!(numbers.canonical(expected), Ok("1234567.50".into()));
            let large = u64::MAX.to_string();
            assert_eq!(numbers.canonical(&numbers.format(&large, true)), Ok(large));
        }
    }

    #[test]
    fn input_rejects_malformed_groups_and_accepts_local_decimals_and_exponents() {
        let numbers = Numbers::new("de-DE");
        assert!(numbers.canonical("1.5").is_err());
        assert_eq!(numbers.canonical("1,5"), Ok("1.5".into()));
        assert_eq!(numbers.canonical("1,5e+2"), Ok("1.5e+2".into()));
        assert!(numbers.canonical("1,5e").is_err());
        let numbers = Numbers::new("ru-RU");
        assert_eq!(numbers.canonical("1 234,5"), Ok("1234.5".into()));
        assert_eq!(numbers.clock("1:02:03.040"), "1:02:03,040");
        assert_eq!(
            numbers.text("16-bit, 128.5 samples."),
            "16-bit, 128,5 samples."
        );
        assert_eq!(Numbers::new("C").format("1234.5", true), "1234.5");
    }

    #[test]
    fn native_digits_roundtrip() {
        let numbers = Numbers::new("ar-EG");
        let display = numbers.format("-12345.50", true);
        assert_eq!(numbers.canonical(&display), Ok("-12345.50".into()));
        assert_eq!(numbers.clock("0:02.050"), "٠:٠٢٫٠٥٠");
    }
}
