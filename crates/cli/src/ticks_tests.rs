use super::*;

/// The size the plot labels its axes at.
const SIZE: f32 = 13.0;

fn across(text: &TextRenderer) -> LabelMetrics<'_> {
    LabelMetrics::new(text, SIZE, LabelRun::Across)
}

fn down(text: &TextRenderer) -> LabelMetrics<'_> {
    LabelMetrics::new(text, SIZE, LabelRun::Down)
}

/// An axis with room to spare at both ends: most of these tests are about the
/// step that was chosen, not about the edge of the canvas.
fn axis(length: i64, min: f64, max: f64) -> Axis {
    Axis {
        length,
        min,
        max,
        lead: 200,
        trail: 200,
    }
}

fn assert_no_overlap(marks: &[Tick], labels: &LabelMetrics<'_>, what: &str) {
    assert!(!marks.is_empty(), "{what}: no ticks at all");
    for pair in marks.windows(2) {
        let (a, b) = (&pair[0], &pair[1]);
        let clear = (b.offset - a.offset) as f64
            - (labels.extent(&a.label) + labels.extent(&b.label)) / 2.0;
        assert!(
            clear >= labels.gap(),
            "{what}: {:?} and {:?} leave {clear:.1}px, under the {:.1}px minimum",
            a.label,
            b.label,
            labels.gap()
        );
    }
}

#[test]
fn a_longer_axis_carries_more_labels() {
    let text = TextRenderer::new();
    let counts: Vec<usize> = [200, 400, 900, 2000]
        .into_iter()
        .map(|length| {
            let marks = ticks(
                AxisKind::Frequency,
                axis(length, -12_000.0, 12_000.0),
                &across(&text),
            );
            assert_no_overlap(&marks, &across(&text), &format!("{length}px"));
            marks.len()
        })
        .collect();

    assert!(
        counts.windows(2).all(|w| w[1] > w[0]),
        "density did not grow with the axis: {counts:?}"
    );
    // The fixed six-tick target this replaced gave the same count at every one
    // of those lengths.
    assert!(counts[3] > 12, "a 2000px axis got only {} labels", counts[3]);
}

#[test]
fn decimal_steps_stay_on_one_two_or_five() {
    let text = TextRenderer::new();
    for length in [150, 300, 640, 1280, 2048] {
        let marks = ticks(
            AxisKind::Frequency,
            axis(length, -12_000.0, 12_000.0),
            &across(&text),
        );
        let Some(step) = marks.windows(2).map(|w| w[1].value - w[0].value).next() else {
            continue;
        };
        let mantissa = step / 10f64.powf(step.log10().floor());
        assert!(
            [1.0, 2.0, 5.0].iter().any(|m| (mantissa - m).abs() < 1e-9),
            "{length}px chose a step of {step}, mantissa {mantissa}"
        );
    }
}

#[test]
fn zero_is_exact_when_the_range_holds_it() {
    let text = TextRenderer::new();
    for (min, max) in [(-12_000.0, 12_000.0), (-3.0, 7.0), (-0.5, 0.25)] {
        let marks = ticks(AxisKind::Frequency, axis(600, min, max), &across(&text));
        let zero = marks
            .iter()
            .find(|t| t.value.abs() < 1e-12)
            .unwrap_or_else(|| panic!("no zero tick in {min}..{max}: {marks:?}"));
        assert_eq!(zero.value, 0.0, "zero drifted to {}", zero.value);
        assert_eq!(zero.label, "0 Hz");
    }
}

#[test]
fn labels_that_would_leave_the_canvas_are_dropped_with_their_grid_lines() {
    let text = TextRenderer::new();
    let labels = across(&text);
    // Nothing to overhang into: a label centred on either end has half of
    // itself outside the canvas, so both ends have to go.
    let tight = Axis {
        lead: 0,
        trail: 0,
        ..axis(600, 0.0, 12_000.0)
    };
    let marks = ticks(AxisKind::Frequency, tight, &labels);
    let (first, last) = (&marks[0], &marks[marks.len() - 1]);
    assert!(
        first.offset as f64 - labels.extent(&first.label) / 2.0 >= 0.0,
        "{:?} at {} runs off the near end",
        first.label,
        first.offset
    );
    assert!(
        last.offset as f64 + labels.extent(&last.label) / 2.0 <= 599.0,
        "{:?} at {} runs off the far end",
        last.label,
        last.offset
    );

    // The same axis, given room beside it, keeps the ends it had to drop.
    let roomy = ticks(AxisKind::Frequency, axis(600, 0.0, 12_000.0), &labels);
    assert!(roomy.len() > marks.len(), "{roomy:?} vs {marks:?}");
}

#[test]
fn time_labels_read_as_a_clock() {
    let text = TextRenderer::new();
    // Under an hour: minutes and seconds, the seconds zero-padded.
    let marks = ticks(AxisKind::Time, axis(900, 0.0, 200.0), &across(&text));
    assert_eq!(marks[0].label, "0.00");
    for tick in &marks {
        let (minutes, seconds) = tick.label.split_once('.').expect(&tick.label);
        assert_eq!(seconds.len(), 2, "{:?} did not pad its seconds", tick.label);
        assert!(minutes.parse::<u64>().is_ok(), "{:?}", tick.label);
    }

    // An hour and over: hours, then padded minutes and seconds.
    let marks = ticks(AxisKind::Time, axis(1600, 0.0, 4_000.0), &across(&text));
    assert_eq!(marks[0].label, "0:00:00");
    let hour = marks
        .iter()
        .find(|t| (t.value - 3600.0).abs() < 1e-9)
        .expect("an hour tick");
    assert_eq!(hour.label, "1:00:00");
}

#[test]
fn an_hour_two_minutes_and_nine_seconds_prints_as_the_issue_asks() {
    assert_eq!(format_clock(3729.0, Clock::HoursMinutesSeconds), "1:02:09");
    assert_eq!(format_clock(187.0, Clock::MinutesSeconds), "3.07");
    assert_eq!(format_clock(0.0, Clock::MinutesSeconds), "0.00");
    assert_eq!(format_clock(-90.0, Clock::MinutesSeconds), "-1.30");
}

#[test]
fn the_clock_format_follows_the_span_and_not_the_offset() {
    assert_eq!(clock_of(0.0, 3600.0), Clock::HoursMinutesSeconds);
    assert_eq!(clock_of(0.0, 3599.0), Clock::MinutesSeconds);

    // A one-minute window keeps the one-minute format wherever it is taken
    // from, so panning across the hour mark does not rewrite every label.
    assert_eq!(clock_of(3600.0, 3660.0), Clock::MinutesSeconds);
    assert_eq!(clock_of(86_400.0, 86_460.0), Clock::MinutesSeconds);
    // And the minutes field carries the full count when it passes sixty.
    let text = TextRenderer::new();
    let marks = ticks(AxisKind::Time, axis(900, 3600.0, 3660.0), &across(&text));
    assert_eq!(marks[0].label, "60.00");
    assert_eq!(marks[marks.len() - 1].label, "61.00");

    // An hour-long span keeps hours wherever it starts.
    assert_eq!(clock_of(7200.0, 10_800.0), Clock::HoursMinutesSeconds);
}

#[test]
fn time_ticks_never_step_finer_than_a_second() {
    let text = TextRenderer::new();
    // Room for far more marks; the clock does not offer any.
    let marks = ticks(AxisKind::Time, axis(2000, 0.0, 4.0), &across(&text));
    assert_eq!(
        marks.iter().map(|t| t.value).collect::<Vec<_>>(),
        vec![0.0, 1.0, 2.0, 3.0, 4.0]
    );
    for tick in &marks {
        assert_eq!(tick.label.len(), 4, "{:?} is not m.ss", tick.label);
    }
}

#[test]
fn a_span_shorter_than_a_second_shows_only_the_whole_seconds_in_it() {
    let text = TextRenderer::new();
    let marks = ticks(AxisKind::Time, axis(1200, 0.0, 0.2), &across(&text));
    assert_eq!(marks.len(), 1, "{marks:?}");
    assert_eq!(marks[0].value, 0.0);
    assert_eq!(marks[0].label, "0.00");

    // A window between two whole seconds has none to show.
    assert!(
        ticks(AxisKind::Time, axis(1200, 0.3, 0.8), &across(&text)).is_empty(),
        "invented a coordinate that is not a whole second"
    );
}

#[test]
fn a_step_that_would_print_the_same_number_twice_is_refused() {
    let text = TextRenderer::new();
    // A ten-decibel span across 900 pixels has room for half-decibel marks,
    // but the labels are whole decibels, so half of them would repeat.
    let marks = ticks(AxisKind::Decibels, axis(900, -70.0, -60.0), &down(&text));
    assert!(marks.len() >= 6, "{marks:?}");
    for pair in marks.windows(2) {
        assert_ne!(pair[0].label, pair[1].label, "{marks:?}");
        assert!(
            pair[1].value - pair[0].value >= 1.0,
            "a sub-decibel step got through: {marks:?}"
        );
    }
}

#[test]
fn stacked_labels_are_spaced_by_their_ink_not_by_their_width() {
    let text = TextRenderer::new();
    // `12.579887 MHz` is wide and short, so stacking the same axis fits many
    // more marks than laying its labels out end to end.
    let (min, max) = (12_567_000.0, 12_591_000.0);
    let stacked = ticks(AxisKind::Frequency, axis(600, min, max), &down(&text));
    let along = ticks(AxisKind::Frequency, axis(600, min, max), &across(&text));
    assert!(
        stacked.len() > along.len() * 2,
        "stacked {} vs along {}",
        stacked.len(),
        along.len()
    );
    assert_no_overlap(&stacked, &down(&text), "stacked");
    assert_no_overlap(&along, &across(&text), "along");
}

#[test]
fn real_and_complex_frequency_ranges_both_land_on_round_hertz() {
    let text = TextRenderer::new();
    for (min, max) in [
        (0.0, 12_000.0),              // real baseband
        (-12_000.0, 12_000.0),        // complex baseband
        (12_579_000.0, 12_591_000.0), // real, tuned to HF
        (12_567_000.0, 12_591_000.0), // complex, tuned to HF
    ] {
        let marks = ticks(AxisKind::Frequency, axis(700, min, max), &down(&text));
        assert_no_overlap(&marks, &down(&text), &format!("{min}..{max}"));
        for tick in &marks {
            assert!(
                tick.value.fract().abs() < 1e-6,
                "{} is not a whole hertz",
                tick.value
            );
        }
    }
}

#[test]
fn the_widest_label_bounds_every_label_the_axis_prints() {
    let text = TextRenderer::new();
    let reserved = |kind, min, max| {
        widest_labels(kind, min, max)
            .iter()
            .map(|label| text.width(label, SIZE))
            .fold(0.0f32, f32::max)
    };
    let cases: [(AxisKind, f64, f64); 8] = [
        (AxisKind::Frequency, 12_567_000.0, 12_591_000.0),
        (AxisKind::Frequency, -12_000.0, 12_000.0),
        // Straddling a unit threshold: the reserve comes from `1.000 kHz` but
        // the axis can still print the wider `999.995 Hz`.
        (AxisKind::Frequency, 0.0, 1_000.0),
        (AxisKind::Frequency, 900.0, 1_100.0),
        (AxisKind::Frequency, 999_000.0, 1_001_000.0),
        (AxisKind::Time, 0.0, 4_000.0),
        (AxisKind::Time, 0.0, 200.0),
        (AxisKind::Decibels, -120.0, 0.0),
    ];
    for (kind, min, max) in cases {
        let bound = reserved(kind, min, max);
        for length in [300, 900, 1600] {
            for tick in ticks(kind, axis(length, min, max), &across(&text)) {
                assert!(
                    text.width(&tick.label, SIZE) <= bound,
                    "{kind:?} {min}..{max}: {:?} is wider than the {bound:.1}px reserved",
                    tick.label
                );
            }
        }
    }
}

#[test]
fn the_decibel_bound_covers_anything_f32_can_reach() {
    // The transform clamps silence at -300 dB rather than letting it reach
    // `-inf`, and `argand-dsp` publishes that floor.
    assert_eq!(f64::from(argand_dsp::DB_FLOOR), crate::render::DB_FLOOR);
    assert!(widest_labels(AxisKind::Decibels, crate::render::DB_FLOOR, 0.0)
        .contains(&"-300".to_string()));
    assert!(
        widest_labels(AxisKind::DecibelsWithUnit, crate::render::DB_FLOOR, 0.0)
            .contains(&"-300 dB".to_string())
    );
}

#[test]
fn an_axis_with_nothing_to_show_produces_nothing() {
    let text = TextRenderer::new();
    let labels = across(&text);
    assert!(ticks(AxisKind::Frequency, axis(600, 5.0, 5.0), &labels).is_empty());
    assert!(ticks(AxisKind::Frequency, axis(600, 10.0, 1.0), &labels).is_empty());
    assert!(ticks(AxisKind::Frequency, axis(1, 0.0, 10.0), &labels).is_empty());
    assert!(ticks(AxisKind::Time, axis(600, f64::NAN, 1.0), &labels).is_empty());
    assert!(ticks(AxisKind::Time, axis(600, 0.0, f64::INFINITY), &labels).is_empty());
}


#[test]
fn no_tick_lands_outside_the_range_it_was_given() {
    let text = TextRenderer::new();
    let cases = [
        // A one-hertz window a terahertz up: the slack that finds a tick on the
        // boundary is far wider than the span if it is measured in hertz.
        (1_000_000_000_123.0, 1_000_000_000_124.0),
        (12_579_000.5, 12_579_001.5),
        (-0.000_000_1, 0.000_000_1),
        (0.3, 0.8),
    ];
    let strays = |kind, (min, max): (f64, f64), labels: &LabelMetrics<'_>| {
        ticks(kind, axis(600, min, max), labels)
            .into_iter()
            .filter(|tick| tick.value < min || tick.value > max)
            .collect::<Vec<_>>()
    };
    for (min, max) in cases {
        for kind in [AxisKind::Frequency, AxisKind::Decibels] {
            let outside = strays(kind, (min, max), &across(&text));
            assert!(outside.is_empty(), "{kind:?} left {min}..{max}: {outside:?}");
            let outside = strays(kind, (min, max), &down(&text));
            assert!(outside.is_empty(), "{kind:?} left {min}..{max}: {outside:?}");
        }
    }
}

#[test]
fn a_tick_sitting_exactly_on_an_end_of_the_range_is_kept() {
    let text = TextRenderer::new();
    // Both ends are whole multiples of the step the axis will pick, and the
    // low end only reaches one after a division that cannot represent it.
    let marks = ticks(AxisKind::Frequency, axis(900, 0.1 + 0.2, 0.9), &across(&text));
    assert_eq!(marks.first().map(|t| t.value), Some(0.1 + 0.2));
    assert_eq!(marks.last().map(|t| t.value), Some(0.9));

    let marks = ticks(AxisKind::Time, axis(900, 0.0, 1800.0), &across(&text));
    assert_eq!(marks.first().map(|t| t.value), Some(0.0));
    assert_eq!(marks.last().map(|t| t.value), Some(1800.0));
}

#[test]
fn a_span_no_ladder_was_written_for_still_terminates() {
    let text = TextRenderer::new();
    // A span smaller than the smallest normal. Reaching the end of this test
    // is most of the point: the decimal ladder used to compute a decade of
    // zero here, which neither produced a step nor grew when multiplied, so
    // the search spun for ever.
    let subnormal = f64::from_bits(1);
    for length in [2, 600] {
        let marks = ticks(
            AxisKind::Frequency,
            axis(length, 0.0, subnormal),
            &across(&text),
        );
        assert!(marks.len() <= 1, "{length}px: {marks:?}");
        for tick in &marks {
            assert_eq!(tick.value, 0.0, "{length}px: {marks:?}");
        }
    }

    // Three thousand years, which is past where the clock ladder stops naming
    // its steps. It carries on in days rather than giving up.
    let millennia = 1e12;
    let marks = ticks(AxisKind::Time, axis(1200, 0.0, millennia), &across(&text));
    assert!(!marks.is_empty(), "the clock ladder ran out");
    assert_no_overlap(&marks, &across(&text), "three thousand years");
    for pair in marks.windows(2) {
        let step = pair[1].value - pair[0].value;
        assert!(
            (step / 86_400.0).fract().abs() < 1e-6,
            "{step} is not a whole number of days"
        );
    }
}

#[test]
fn a_huge_decibel_window_still_gets_a_bound_that_holds_it() {
    let text = TextRenderer::new();
    // `--dynamic-range 10000` is not refused by the CLI, so the reserve has to
    // survive it: five digits and a sign, not the f32 floor's three.
    let reserved = widest_labels(AxisKind::DecibelsWithUnit, -10_000.0, 0.0);
    assert!(reserved.contains(&"-10000 dB".to_string()), "{reserved:?}");
    let bound = reserved
        .iter()
        .map(|label| text.width(label, SIZE))
        .fold(0.0f32, f32::max);
    for tick in ticks(
        AxisKind::DecibelsWithUnit,
        axis(900, -10_000.0, 0.0),
        &down(&text),
    ) {
        assert!(
            text.width(&tick.label, SIZE) <= bound,
            "{:?} is wider than the reserved {reserved:?}",
            tick.label
        );
    }
}

#[test]
fn a_range_ending_one_ulp_short_of_a_multiple_stays_bounded() {
    let text = TextRenderer::new();
    // The quotient here is large enough that one of its ulps is wider than the
    // distance from `max` up to the next whole multiple, so the slack cannot
    // tell a rounded division from a range that genuinely stops just short.
    // What it must not do is drag the tick further out than the cap allows.
    let boundary = 1e12f64;
    let wide = axis(600, boundary - 1.0, boundary.next_down());
    let marks = ticks(AxisKind::Frequency, wide, &across(&text));
    assert!(!marks.is_empty(), "nothing to check");
    let pixel = (wide.max - wide.min) / (wide.length - 1) as f64;
    for tick in &marks {
        assert!(
            (wide.min - pixel..=wide.max + pixel).contains(&tick.value),
            "{} is more than a pixel past {}",
            tick.value,
            wide.max
        );
    }

    // Spans of a few ulps, where the division's own error is far wider than the
    // slack cap, so only the check on the finished value keeps a tick from
    // landing a whole span outside. Measuring the distance rather than
    // widening the end matters here too: at these magnitudes `max + pixel`
    // rounds straight back to `max`.
    for exponent in [-10, 0, 20, 40] {
        let boundary = 2f64.powi(exponent);
        let ulp = boundary.next_up() - boundary;
        let tight = axis(600, boundary - 367.0 * ulp, boundary - ulp);
        let marks = ticks(AxisKind::Frequency, tight, &across(&text));
        assert!(
            !marks.iter().any(|t| t.value >= boundary),
            "2^{exponent}: a tick reached {boundary}, past the end of the axis: {marks:?}"
        );
        let pixel = (tight.max - tight.min) / (tight.length - 1) as f64;
        for tick in &marks {
            let outside = (tight.min - tick.value).max(tick.value - tight.max);
            assert!(outside <= pixel, "2^{exponent}: {outside} past a {pixel} pixel");
        }
    }
}

#[test]
fn a_quotient_too_large_to_index_is_refused_rather_than_saturated() {
    let text = TextRenderer::new();
    // A window fifteen orders of magnitude narrower than where it sits, which
    // is what puts `min / step` past 2^53. There an index no longer round-trips
    // through f64, and past i64 the cast saturates into a tick at some
    // arbitrary value.
    let (min, max) = (1e300, 1e300 + 1e285);
    assert!(max > min, "the range collapsed before the test began");
    // Refusing outright is the right answer here -- no round step can label
    // this -- so what is asserted is that nothing came back at some arbitrary
    // value. Before the guard, a saturated index put a tick at `i64::MAX`
    // times the step.
    let marks = ticks(AxisKind::Frequency, axis(600, min, max), &across(&text));
    for tick in &marks {
        assert!(
            (min..=max).contains(&tick.value),
            "{} is outside {min}..{max}: {marks:?}",
            tick.value
        );
    }

    // A quotient just inside the limit, where one ulp is already most of a
    // multiple. The marks that belong here still come back, and none other.
    let base = 2f64.powi(49);
    let marks = ticks(
        AxisKind::Frequency,
        axis(900, base, base + 2.0),
        &across(&text),
    );
    assert!(!marks.is_empty(), "a workable axis came back empty");
    for tick in &marks {
        assert!((base..=base + 2.0).contains(&tick.value), "{marks:?}");
    }
}
