use super::*;

#[test]
fn a_span_is_cut_to_the_capture_it_lies_in() {
    let span = SampleSpan::between(900, 1200).expect("a span");
    assert_eq!(span.within(1000), SampleSpan::between(900, 1000));
    assert_eq!(span.within(900), None);
    assert_eq!(span.within(5000), Some(span));
}

#[test]
fn a_band_is_ordered_and_rejects_what_is_not_a_frequency() {
    let band = FrequencyBand::between(12_580_000.0, 12_578_000.0).expect("a band");
    assert_eq!((band.low(), band.high()), (12_578_000.0, 12_580_000.0));
    assert_eq!(FrequencyBand::between(1.0, 1.0), None);
    assert_eq!(FrequencyBand::between(f64::NAN, 1.0), None);
}
