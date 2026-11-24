use super::*;

#[test]
fn approved_values_use_at_most_three_fractional_digits_above_hertz() {
    assert_eq!(standalone(12_582_000.), "12.582 MHz");
    assert_eq!(standalone(12_582_500.), "12582.5 kHz");
    assert_eq!(standalone(12_582_550.), "12582.55 kHz");
    assert_eq!(standalone(13_000_000.), "13 MHz");
}

#[test]
fn common_coordinate_unit_follows_resolution_and_keeps_one_unit_for_other_values() {
    for (resolution, expected) in [
        (1000., FrequencyUnit::MHz),
        (100., FrequencyUnit::KHz),
        (10., FrequencyUnit::KHz),
        (0.1, FrequencyUnit::Hz),
    ] {
        let unit = FrequencyUnit::coordinates(12_570_000., 12_590_000., resolution);
        assert_eq!(unit, expected);
        assert_eq!(FrequencyFormat::value(unit, 24_000.).unit(), unit);
        assert_eq!(FrequencyFormat::value(unit, 17.578).unit(), unit);
    }
    let unit = FrequencyUnit::KHz;
    assert_eq!(
        FrequencyFormat::value(unit, 17.578).with_unit(17.578),
        "0.017578 kHz"
    );
    assert_eq!(
        FrequencyFormat::exact(unit, 12_582_500.25).format(12_582_500.25),
        "12582.50025"
    );
}

#[test]
fn shared_fractional_precision_keeps_integers_bare_and_preserves_suffix_meaning() {
    let format = FrequencyFormat::step(FrequencyUnit::MHz, 1000.);
    assert_eq!(format.format(12_590_000.), "12.590");
    assert_eq!(format.format(13_000_000.), "13");
    assert_eq!(format.format(-0.00001), "0");
    assert_eq!(
        FrequencyUnit::split_suffix("12,5 MHz"),
        ("12,5", Some(FrequencyUnit::MHz))
    );
    assert_eq!(FrequencyUnit::split_suffix("12,5"), ("12,5", None));
}
