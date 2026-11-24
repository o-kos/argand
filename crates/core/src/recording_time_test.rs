use super::*;

#[test]
fn calendar_offsets_and_midnight_are_unambiguous() {
    for text in [
        "1970-01-01T00:00:00.000Z",
        "2000-02-29T23:59:59.999Z",
        "1900-03-01T00:00:00.000Z",
        "2026-11-23T12:34:56.123Z",
    ] {
        let time = RecordingTime::parse(text).unwrap();
        assert_eq!(time.format(3), text);
        assert_eq!(
            RecordingTime::from_system_time(time.system_time()),
            Some(time)
        );
    }
    assert_eq!(
        RecordingTime::parse("2026-11-23T15:34:56.123+03:00")
            .unwrap()
            .format(3),
        "2026-11-23T12:34:56.123Z"
    );
    let time = RecordingTime::parse("2026-11-23T23:59:59.999Z").unwrap();
    assert_eq!(
        time.shifted_samples(1, 1000.).unwrap().format(3),
        "2026-11-24T00:00:00.000Z"
    );
}

#[test]
fn malformed_or_incomplete_dates_never_fabricate_an_instant() {
    for text in [
        "",
        "2026-11-23",
        "2026-11-23T12:34:56",
        "2026-02-29T00:00:00Z",
        "1900-02-29T00:00:00Z",
        "2026-01-01T24:00:00Z",
        "2026-01-01T00:00:60Z",
        "2026-01-01T00:00:00-00:00",
        "2026-01-01T00:00:00.1234567890Z",
        "é026-01-01T00:00:00Z",
        "2026-+1-01T00:00:00Z",
        "2026-01-01T00:00:00++1:00",
    ] {
        assert_eq!(RecordingTime::parse(text), None, "{text}");
    }
    assert_eq!(RecordingTime::from_system_time([0; 8]), None);
}

#[test]
fn storage_rounds_once_but_offsets_retain_mhz_sample_precision() {
    let time = RecordingTime::parse("2026-11-23T23:59:59.999Z").unwrap();
    assert_eq!(
        time.shifted_samples(499, 1_000_000.).unwrap().format(3),
        "2026-11-23T23:59:59.999Z"
    );
    assert_eq!(
        time.shifted_samples(500, 1_000_000.).unwrap().format(3),
        "2026-11-24T00:00:00.000Z"
    );
    let mut advanced = time;
    for _ in 0..1000 {
        advanced = advanced.shifted_samples(1, 1_000_000.).unwrap();
    }
    assert_eq!(advanced, time.shifted_samples(1000, 1_000_000.).unwrap());
    assert_eq!(
        time.shifted_samples(1, 1_000_000.).unwrap().format(6),
        "2026-11-23T23:59:59.999001Z"
    );
}

#[test]
fn calendar_range_and_rounded_serialization_stay_valid() {
    let last = RecordingTime::parse("9999-12-31T23:59:59.999499999Z").unwrap();
    assert!(RecordingTime::parse("9999-12-31T23:59:59.9995Z").is_none());
    assert_eq!(last.format(3), "9999-12-31T23:59:59.999Z");
    assert!(RecordingTime::from_system_time(last.system_time()).is_some());
    assert!(RecordingTime::parse("0001-01-01T00:00:00+00:01").is_none());
    assert!(RecordingTime::parse("9999-12-31T23:59:59-00:01").is_none());
    assert!(RecordingTime::parse("0001-01-01T00:00:00Z").is_some());
}

#[test]
fn repeated_nonintegral_sample_periods_do_not_accumulate_rounding() {
    let start = RecordingTime::parse("2026-11-23T00:00:00.000Z").unwrap();
    let mut edited = start;
    for _ in 0..44100 {
        edited = edited.shifted_samples(1, 44100.).unwrap();
    }
    assert_eq!(edited, start.shifted_samples(44100, 44100.).unwrap());
    assert_eq!(edited.format(9), "2026-11-23T00:00:01.000000000Z");
}

#[test]
fn filesystem_timestamp_conversion_preserves_subseconds_and_pre_epoch() {
    use std::time::{Duration, UNIX_EPOCH};
    assert_eq!(
        RecordingTime::from_timestamp(UNIX_EPOCH).unwrap().format(9),
        "1970-01-01T00:00:00.000000000Z"
    );
    assert_eq!(
        RecordingTime::from_timestamp(UNIX_EPOCH - Duration::from_nanos(1))
            .unwrap()
            .format(9),
        "1969-12-31T23:59:59.999999999Z"
    );
    assert_eq!(
        RecordingTime::from_timestamp(UNIX_EPOCH + Duration::from_nanos(123456789))
            .unwrap()
            .format(9),
        "1970-01-01T00:00:00.123456789Z"
    );
}
