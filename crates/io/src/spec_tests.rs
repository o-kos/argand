use super::*;
use argand_core::{Domain, SampleFormat};

#[test]
fn raw_spec_carries_type_and_rate() {
    let spec: RawSpec = "iq_i16@24k".parse().unwrap();
    assert_eq!(
        spec.sample_type,
        SampleType::new(Domain::Iq, SampleFormat::I16)
    );
    assert_eq!(spec.sample_rate, Some(24_000.0));
    assert_eq!(spec.to_string(), "iq_i16@24000");
}

#[test]
fn raw_spec_rate_is_optional() {
    let spec: RawSpec = "rl_f16x8".parse().unwrap();
    assert_eq!(spec.sample_rate, None);
    assert_eq!(spec.to_string(), "rl_f16x8");
}

#[test]
fn raw_spec_reports_which_half_was_wrong() {
    assert!(matches!(
        "iq_f64@24k".parse::<RawSpec>().unwrap_err(),
        ParseRawSpecError::SampleType(_)
    ));
    assert!(matches!(
        "iq_i16@fast".parse::<RawSpec>().unwrap_err(),
        ParseRawSpecError::Rate(_)
    ));
    assert!(matches!(
        "  ".parse::<RawSpec>().unwrap_err(),
        ParseRawSpecError::Empty
    ));
}

#[test]
fn bare_numbers_are_hertz() {
    assert_eq!(parse_hz("24000").unwrap(), 24_000.0);
    assert_eq!(parse_hz(" 0 ").unwrap(), 0.0);
    assert_eq!(parse_hz("-2404").unwrap(), -2404.0);
}

#[test]
fn suffixes_scale_and_ignore_case() {
    assert_eq!(parse_hz("24k").unwrap(), 24_000.0);
    assert_eq!(parse_hz("24K").unwrap(), 24_000.0);
    assert_eq!(parse_hz("2.4M").unwrap(), 2_400_000.0);
    assert_eq!(parse_hz("2.4m").unwrap(), 2_400_000.0);
    assert_eq!(parse_hz("1G").unwrap(), 1e9);
    assert_eq!(parse_hz("12.579MHz").unwrap(), 12_579_000.0);
    assert_eq!(parse_hz("24 kHz").unwrap(), 24_000.0);
}

#[test]
fn rejects_frequencies_it_cannot_read() {
    for bad in ["", "  ", "k", "abc", "1.2.3", "24kk", "inf", "nan"] {
        assert!(parse_hz(bad).is_err(), "should reject {bad:?}");
    }
}

#[test]
fn times_accept_seconds_colons_and_suffixes() {
    assert_eq!(parse_time("12.5").unwrap(), 12.5);
    assert_eq!(parse_time("90s").unwrap(), 90.0);
    assert_eq!(parse_time("1m30").unwrap(), 90.0);
    assert_eq!(parse_time("1m30s").unwrap(), 90.0);
    assert_eq!(parse_time("01:30").unwrap(), 90.0);
    assert_eq!(parse_time("1:00:00").unwrap(), 3600.0);
    assert_eq!(parse_time("1h02m03").unwrap(), 3723.0);
    assert_eq!(parse_time("250ms").unwrap(), 0.25);
    assert_eq!(parse_time("0").unwrap(), 0.0);
}

#[test]
fn rejects_times_it_cannot_read() {
    for bad in ["", "abc", "-5", "1x30", "1:aa"] {
        assert!(parse_time(bad).is_err(), "should reject {bad:?}");
    }
}
