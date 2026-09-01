use super::*;

#[test]
fn sample_counts_scale_and_trim() {
    assert_eq!(format_samples(0), "0 spl");
    assert_eq!(format_samples(999), "999 spl");
    assert_eq!(format_samples(1_000), "1 kspl");
    assert_eq!(format_samples(43_200_000), "43.2 Mspl");
    assert_eq!(format_samples(1_500_000_000), "1.5 Gspl");
    assert_eq!(format_samples(2_000_000_000_000), "2 Tspl");
}

#[test]
fn sample_counts_promote_instead_of_printing_1000k() {
    assert_eq!(format_samples(999_990), "1 Mspl");
}

#[test]
fn durations_cover_every_branch() {
    assert_eq!(format_duration(0.0), "0ms");
    assert_eq!(format_duration(0.25), "250ms");
    assert_eq!(format_duration(12.5), "12.5s");
    assert_eq!(format_duration(59.0), "59s");
    assert_eq!(format_duration(330.0), "5m30");
    assert_eq!(format_duration(1800.0), "30m");
    assert_eq!(format_duration(3600.0), "1h");
    assert_eq!(format_duration(4800.0), "1h20m");
    assert_eq!(format_duration(4805.5), "1h20m05.5");
}

#[test]
fn durations_handle_odd_input() {
    assert_eq!(format_duration(-0.25), "-250ms");
    assert_eq!(format_duration(f64::NAN), "?");
}

#[test]
fn frequencies_pick_a_readable_unit() {
    assert_eq!(format_hz(0.0), "0 Hz");
    assert_eq!(format_hz(24_000.0), "24 kHz");
    assert_eq!(format_hz(999.0), "999 Hz");
    assert_eq!(format_hz(12_579_000.0), "12.579 MHz");
    assert_eq!(format_hz(-2404.0), "-2.404 kHz");
    assert_eq!(format_hz(1.5e9), "1.5 GHz");
}

#[test]
fn frequencies_resolve_to_one_hertz_in_every_unit() {
    // Six decimals everywhere would print 10.886719 kHz here.
    assert_eq!(format_hz(10_886.719), "10.887 kHz");
    assert_eq!(format_hz(17.578125), "17.578 Hz");
    assert_eq!(format_hz(12_589_886.7), "12.589887 MHz");
}

#[test]
fn byte_counts_use_binary_units() {
    assert_eq!(format_bytes(512), "512 B");
    assert_eq!(format_bytes(253_952), "248 KiB");
    assert_eq!(format_bytes(172_800_056), "164.8 MiB");
}
