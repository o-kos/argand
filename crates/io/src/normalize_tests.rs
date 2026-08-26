use super::*;
use std::str::FromStr;

fn floats(values: &[f32]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_le_bytes()).collect()
}

#[test]
fn only_unnormalised_float_normalises_by_default() {
    assert_eq!(
        Normalize::default_for(SampleFormat::F16x8),
        Normalize::Auto
    );
    for format in [
        SampleFormat::U8,
        SampleFormat::I16,
        SampleFormat::I32,
        SampleFormat::F32,
    ] {
        assert_eq!(Normalize::default_for(format), Normalize::None, "{format:?}");
    }
}

#[test]
fn parses_the_documented_spellings() {
    assert_eq!(Normalize::from_str("none").unwrap(), Normalize::None);
    assert_eq!(Normalize::from_str(" AUTO ").unwrap(), Normalize::Auto);
    assert_eq!(Normalize::from_str("2.5").unwrap(), Normalize::Factor(2.5));
    for bad in ["", "-1", "0", "yes", "nan", "inf"] {
        assert!(Normalize::from_str(bad).is_err(), "should reject {bad:?}");
    }
}

#[test]
fn none_uses_the_format_full_scale() {
    let data = floats(&[9.0]);
    assert_eq!(
        resolve_divisor(Normalize::None, SampleFormat::I16, &data),
        32768.0
    );
    assert_eq!(
        resolve_divisor(Normalize::None, SampleFormat::F16x8, &data),
        1.0
    );
}

#[test]
fn auto_divides_by_the_measured_peak_plus_headroom() {
    let data = floats(&[0.5, -4.0, 1.25]);
    let divisor = resolve_divisor(Normalize::Auto, SampleFormat::F16x8, &data);
    assert!((divisor - 4.0 * AUTO_HEADROOM).abs() < 1e-6, "{divisor}");

    // A sample at the measured peak lands just below full scale.
    assert!(4.0 / divisor < 1.0);
    assert!(4.0 / divisor > 0.9);
}

#[test]
fn auto_on_silence_falls_back_instead_of_dividing_by_zero() {
    let data = floats(&[0.0, 0.0, 0.0]);
    let divisor = resolve_divisor(Normalize::Auto, SampleFormat::F16x8, &data);
    assert_eq!(divisor, 1.0);
    assert!(divisor.is_finite() && divisor > 0.0);
}

#[test]
fn auto_peak_normalises_integer_formats_too() {
    // An explicit --normalize auto on a quiet i16 capture should lift it, not
    // silently do nothing.
    let quiet: Vec<u8> = [1000i16, -2000, 500]
        .iter()
        .flat_map(|v| v.to_le_bytes())
        .collect();
    let divisor = resolve_divisor(Normalize::Auto, SampleFormat::I16, &quiet);
    assert!((divisor - 2000.0 * AUTO_HEADROOM).abs() < 1e-3, "{divisor}");
    assert!(2000.0 / divisor > 0.9);
}

#[test]
fn auto_on_silent_integers_falls_back_to_full_scale() {
    let divisor = resolve_divisor(Normalize::Auto, SampleFormat::I16, &[0u8; 64]);
    assert_eq!(divisor, 32768.0);
}

#[test]
fn explicit_factor_is_used_verbatim() {
    let data = floats(&[100.0]);
    assert_eq!(
        resolve_divisor(Normalize::Factor(8.0), SampleFormat::F16x8, &data),
        8.0
    );
}

#[test]
fn full_scan_finds_a_peak_anywhere_in_the_buffer() {
    let mut values = vec![0.1f32; 300_000];
    values[299_999] = 7.5;
    let divisor = resolve_divisor(Normalize::Auto, SampleFormat::F16x8, &floats(&values));
    assert!((divisor - 7.5 * AUTO_HEADROOM).abs() < 1e-5, "{divisor}");
}

#[test]
fn gain_converts_decibels_to_a_multiplier() {
    assert_eq!(gain_factor(0.0), 1.0);
    assert!((gain_factor(6.0) - 1.9953).abs() < 1e-3);
    assert!((gain_factor(-20.0) - 0.1).abs() < 1e-6);
}
