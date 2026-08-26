use super::*;
use crate::sample::{Domain, SampleFormat};

fn meta(domain: Domain, rate: f64, center: f64, len: u64) -> SignalMeta {
    SignalMeta {
        sample_rate: rate,
        center_freq: center,
        sample_type: SampleType::new(domain, SampleFormat::I16),
        len_samples: len,
        container: "wav",
        divisor: 32768.0,
        source: PathBuf::from("test"),
    }
}

#[test]
fn duration_uses_samples_not_scalar_values() {
    // 43 200 000 I/Q samples at 24 kHz is half an hour, not an hour.
    let m = meta(Domain::Iq, 24_000.0, 0.0, 43_200_000);
    assert_eq!(m.duration_seconds(), 1800.0);
    assert_eq!(m.channels(), 2);
}

#[test]
fn zero_rate_does_not_divide_by_zero() {
    assert_eq!(meta(Domain::Real, 0.0, 0.0, 100).duration_seconds(), 0.0);
}

#[test]
fn iq_span_is_two_sided_around_centre() {
    let (lo, hi) = meta(Domain::Iq, 24_000.0, 12_579_000.0, 10).frequency_span();
    assert_eq!((lo, hi), (12_567_000.0, 12_591_000.0));
}

#[test]
fn real_span_is_one_sided() {
    let (lo, hi) = meta(Domain::Real, 24_000.0, 0.0, 10).frequency_span();
    assert_eq!((lo, hi), (0.0, 12_000.0));
}

#[test]
fn range_clamps_to_available_samples() {
    assert_eq!(
        SampleRange::new(90, 50).clamped_to(100),
        SampleRange::new(90, 10)
    );
    assert_eq!(
        SampleRange::new(200, 50).clamped_to(100),
        SampleRange::new(100, 0)
    );
    assert!(SampleRange::new(200, 50).clamped_to(100).is_empty());
}
