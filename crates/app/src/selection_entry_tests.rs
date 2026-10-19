use super::*;

const RATE: f64 = 1_000.;

fn scale(mode: Mode) -> Scale {
    Scale {
        mode,
        rate: RATE,
        total: 100_000,
        unit: "kHz",
        divisor: 1e3,
        decimals: 3,
        band: (-500., 500.),
    }
}

fn span(start: u64, end: u64) -> SampleSpan {
    SampleSpan::between(start, end).expect("a span")
}

fn band(low: f64, high: f64) -> FrequencyBand {
    FrequencyBand::between(low, high).expect("a band")
}

fn en() -> Numbers {
    Numbers::new("en-US")
}

#[test]
fn texts_read_back_as_the_same_samples_in_every_mode() {
    let selected = span(1_234, 56_789);
    for mode in [Mode::Clock, Mode::Seconds, Mode::Samples] {
        let scale = scale(mode);
        let texts = scale.time_texts(selected);
        for (field, text) in Field::TIME.into_iter().zip(&texts) {
            assert_eq!(
                scale.time_in(&en(), selected, field, text),
                Some(selected),
                "{mode:?} {field:?} {text}"
            );
        }
    }
}

#[test]
fn clock_takes_one_to_three_fields() {
    let scale = scale(Mode::Clock);
    let selected = span(0, 90_000);
    let start = |text| scale.time_in(&en(), selected, Field::TimeStart, text);
    assert_eq!(start("0:01:02.5"), Some(span(62_500, 90_000)));
    assert_eq!(start("1:02.5"), Some(span(62_500, 90_000)));
    assert_eq!(start("62.5"), Some(span(62_500, 90_000)));
    assert_eq!(start("1:0:01:02"), None);
    assert_eq!(start("1.5:02"), None);
    assert_eq!(start("1:-2"), None);
    assert_eq!(start(""), None);
    assert_eq!(start("abc"), None);
}

#[test]
fn seconds_take_an_optional_unit_and_samples_an_optional_sign() {
    let selected = span(0, 90_000);
    let seconds = scale(Mode::Seconds);
    assert_eq!(
        seconds.time_in(&en(), selected, Field::TimeStart, "2.5 s"),
        Some(span(2_500, 90_000))
    );
    assert_eq!(
        seconds.time_in(&en(), selected, Field::TimeEnd, "1,002.5"),
        Some(span(0, 100_000))
    );
    assert_eq!(
        seconds.time_in(&en(), selected, Field::TimeStart, "1,0.5"),
        None
    );
    let samples = scale(Mode::Samples);
    for text in ["#2,500", "2500", " # 2500 "] {
        assert_eq!(
            samples.time_in(&en(), selected, Field::TimeStart, text.trim_start()),
            Some(span(2_500, 90_000)),
            "{text}"
        );
    }
}

#[test]
fn an_index_names_the_last_sample_and_a_length_counts() {
    let selected = span(100, 200);
    for mode in [Mode::Clock, Mode::Seconds, Mode::Samples] {
        let scale = scale(mode);
        assert_eq!(
            scale.time_in(&en(), selected, Field::TimeEnd, "#299"),
            Some(span(100, 300))
        );
        assert_eq!(
            scale.time_in(&en(), selected, Field::TimeLength, "#50"),
            Some(span(100, 150))
        );
    }
}

#[test]
fn time_snaps_to_samples_and_is_held_to_the_capture() {
    let scale = scale(Mode::Seconds);
    let selected = span(1_000, 2_000);
    let typed = |field, text| scale.time_in(&en(), selected, field, text);
    assert_eq!(typed(Field::TimeStart, "0.0004"), Some(span(0, 2_000)));
    assert_eq!(typed(Field::TimeStart, "0.0006"), Some(span(1, 2_000)));
    assert_eq!(typed(Field::TimeStart, "-5"), Some(span(0, 2_000)));
    assert_eq!(typed(Field::TimeEnd, "1000"), Some(span(1_000, 100_000)));
    assert_eq!(typed(Field::TimeLength, "500"), Some(span(1_000, 100_000)));
}

#[test]
fn a_length_moves_the_end_and_edges_swap_or_refuse() {
    let scale = scale(Mode::Seconds);
    let selected = span(1_000, 2_000);
    let typed = |field, text| scale.time_in(&en(), selected, field, text);
    assert_eq!(typed(Field::TimeLength, "0.5"), Some(span(1_000, 1_500)));
    assert_eq!(typed(Field::TimeLength, "0"), None);
    assert_eq!(typed(Field::TimeLength, "-1"), None);
    // An edge past the other swaps them, as dragging it there does.
    assert_eq!(typed(Field::TimeStart, "3"), Some(span(2_000, 3_000)));
    assert_eq!(typed(Field::TimeEnd, "0.5"), Some(span(500, 1_000)));
    assert_eq!(typed(Field::TimeEnd, "1"), None);
}

#[test]
fn a_locale_with_a_comma_reads_its_own_numbers() {
    let de = Numbers::new("de-DE");
    let selected = span(0, 90_000);
    assert_eq!(
        scale(Mode::Seconds).time_in(&de, selected, Field::TimeStart, "2,5 s"),
        Some(span(2_500, 90_000))
    );
    assert_eq!(
        scale(Mode::Clock).time_in(&de, selected, Field::TimeStart, "1:02,5"),
        Some(span(62_500, 90_000))
    );
    assert_eq!(
        scale(Mode::Samples).time_in(&de, selected, Field::TimeStart, "#2.500"),
        Some(span(2_500, 90_000))
    );
    assert_eq!(
        scale(Mode::Clock).band_in(&de, band(0., 100.), Field::BandHigh, "0,25"),
        Some(band(0., 250.))
    );
}

#[test]
fn indexes_stand_in_where_seconds_cannot_tell_samples_apart() {
    let scale = Scale {
        rate: 1e15,
        total: 1 << 54,
        ..scale(Mode::Clock)
    };
    let selected = span(0, 1_000);
    assert!(Mode::Clock.writes_samples(scale.rate, scale.total));
    assert_eq!(
        scale.time_in(&en(), selected, Field::TimeLength, "500"),
        Some(span(0, 500))
    );
    let texts = scale.time_texts(selected);
    assert_eq!(
        scale.time_in(&en(), selected, Field::TimeEnd, &texts[1]),
        Some(selected)
    );
}

#[test]
fn band_fields_move_one_edge_and_hold_to_the_capture() {
    let scale = scale(Mode::Clock);
    let selected = band(-100., 100.);
    let typed = |field, text| scale.band_in(&en(), selected, field, text);
    assert_eq!(typed(Field::BandLow, "-0.2"), Some(band(-200., 100.)));
    assert_eq!(typed(Field::BandLow, "-9"), Some(band(-500., 100.)));
    assert_eq!(typed(Field::BandHigh, "0.3 kHz"), Some(band(-100., 300.)));
    assert_eq!(typed(Field::BandHigh, "0.3khz"), Some(band(-100., 300.)));
    assert_eq!(typed(Field::BandWidth, "0.05"), Some(band(-100., -50.)));
    assert_eq!(typed(Field::BandWidth, "9"), Some(band(-100., 500.)));
    assert_eq!(typed(Field::BandWidth, "0"), None);
    assert_eq!(typed(Field::BandHigh, "-0.1"), None);
    assert_eq!(typed(Field::BandHigh, "-0.2"), Some(band(-200., -100.)));
    assert_eq!(typed(Field::BandLow, "MHz"), None);
}

#[test]
fn band_texts_read_back_as_the_same_band() {
    let scale = scale(Mode::Clock);
    let selected = band(-123.456, 234.6);
    let texts = scale.band_texts(selected);
    assert_eq!(texts, ["-0.123", "0.235", "0.358"]);
    for (field, text) in Field::BAND.into_iter().zip(&texts) {
        let read = scale.band_in(&en(), selected, field, text).expect("a band");
        assert!((read.low() - selected.low()).abs() < 1. && (read.high() - selected.high()).abs() < 1.);
    }
}
