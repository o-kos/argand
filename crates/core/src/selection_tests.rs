use super::*;

#[test]
fn a_span_is_the_same_whichever_way_it_was_dragged() {
    let forward = SampleSpan::between(100, 250).expect("a span");
    assert_eq!(SampleSpan::between(250, 100), Some(forward));
    assert_eq!((forward.start(), forward.end(), forward.count()), (100, 250, 150));
}

#[test]
fn two_equal_boundaries_hold_no_samples() {
    assert_eq!(SampleSpan::between(42, 42), None);
}

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

#[test]
fn a_time_selection_covers_every_frequency() {
    let selection = Selection::time(SampleSpan::between(0, 10).expect("a span"));
    assert!(selection.band.is_none());
    assert!(!selection.is_empty());
    assert!(Selection::default().is_empty());
}
