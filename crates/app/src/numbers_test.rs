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
        for value in [
            8000.123456789012_f64,
            1.2345678901234567e10,
            -1.2345678901234567e-12,
            f64::MAX,
            f64::MIN_POSITIVE,
        ] {
            let formatted = numbers.format(&value.to_string(), true);
            let parsed: f64 = numbers.canonical(&formatted).unwrap().parse().unwrap();
            assert_eq!(parsed.to_bits(), value.to_bits(), "{locale}: {formatted}");
        }
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
