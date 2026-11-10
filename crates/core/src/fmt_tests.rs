use super::*;

#[test]
fn sample_counts_promote_instead_of_printing_1000k() {
    assert_eq!(format_samples(999_990), "1 Mspl");
}

#[test]
fn frequencies_resolve_to_one_hertz_in_every_unit() {
    // Six decimals everywhere would print 10.886719 kHz here.
    assert_eq!(format_hz(10_886.719), "10.887 kHz");
    assert_eq!(format_hz(17.578125), "17.578 Hz");
    assert_eq!(format_hz(12_589_886.7), "12.589887 MHz");
}
