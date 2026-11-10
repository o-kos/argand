use super::*;
use crate::testutil::DejaVuSans;

/// The size the plot labels its axes at.
const SIZE: f32 = 13.0;

fn across(text: &DejaVuSans) -> LabelMetrics<'_> {
    LabelMetrics::new(text, SIZE, LabelRun::Across)
}

fn down(text: &DejaVuSans) -> LabelMetrics<'_> {
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
fn labels_that_would_leave_the_canvas_are_dropped_with_their_grid_lines() {
    let text = DejaVuSans;
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
fn the_widest_label_bounds_every_label_the_axis_prints() {
    let text = DejaVuSans;
    let reserved = |kind, min, max| {
        widest_labels(kind, min, max)
            .iter()
            .map(|label| text.width(label, SIZE))
            .fold(0.0f32, f32::max)
    };
    let cases: [(AxisKind, f64, f64); 8] = [
        (AxisKind::Frequency, 12_567_000.0, 12_591_000.0),
        (AxisKind::Frequency, -12_000.0, 12_000.0),
        // Sitting on a unit threshold, where the unit chosen for the axis and
        // the one its smallest values would pick on their own differ.
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
fn no_tick_lands_outside_the_range_it_was_given() {
    let text = DejaVuSans;
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
fn a_span_no_ladder_was_written_for_still_terminates() {
    let text = DejaVuSans;
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
    let text = DejaVuSans;
    // `--dynamic-range 10000` is not refused by the CLI, so the reserve has to
    // survive it: five digits and a sign, not the f32 floor's three.
    let reserved = widest_labels(AxisKind::Decibels, -10_000.0, 0.0);
    assert!(reserved.contains(&"-10000".to_string()), "{reserved:?}");
    let bound = reserved
        .iter()
        .map(|label| text.width(label, SIZE))
        .fold(0.0f32, f32::max);
    for tick in ticks(AxisKind::Decibels, axis(900, -10_000.0, 0.0), &down(&text)) {
        assert!(
            text.width(&tick.label, SIZE) <= bound,
            "{:?} is wider than the reserved {reserved:?}",
            tick.label
        );
    }
}

#[test]
fn a_range_ending_one_ulp_short_of_a_multiple_stays_bounded() {
    let text = DejaVuSans;
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
    let text = DejaVuSans;
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

#[test]
fn start_anchored_labels_use_their_full_width_for_edges_and_spacing() {
    let text = DejaVuSans;
    let labels = across(&text).after_tick(6.0);
    for (min, max) in [(0.0, 60.0), (590.0, 650.0), (3500.0, 7500.0)] {
        for length in [80, 200, 600, 1200] {
            let axis = Axis { lead: 0, trail: 0, ..axis(length, min, max) };
            let marks = ticks(AxisKind::Time, axis, &labels);
            assert!(!marks.is_empty());
            for tick in &marks {
                let end = tick.offset as f64 + 6.0 + labels.extent(&tick.label);
                assert!(end <= (length - 1) as f64, "{length}px: {tick:?}");
            }
            for pair in marks.windows(2) {
                let clear = (pair[1].offset - pair[0].offset) as f64 - labels.extent(&pair[0].label);
                assert!(clear >= labels.gap());
            }
            if min == 0.0 {
                assert_eq!(marks[0].offset, 0);
            }
        }
    }
}


#[test]
fn precise_time_ticks_remain_readable_between_seconds_and_at_rf_spans() {
    let text = DejaVuSans;
    for (lo, hi) in [(39.744, 40.0), (3599.9998, 3600.0002), (0.0, 0.000002)] {
        let bounds = axis(1200, lo, hi);
        let labels = across(&text);
        let marks = ticks(AxisKind::PreciseTime, bounds, &labels);
        assert!(marks.len() >= 3, "{marks:?}");
        assert!(marks.windows(2).all(|pair| pair[0].label != pair[1].label));
        assert!(marks.iter().all(|tick| tick.label.contains(':')));
        assert!(marks.iter().all(|tick| tick.value >= lo - 1e-9 && tick.value <= hi + 1e-9));
    }
    assert_eq!(format_precise_clock(59.9999, Clock::MinutesSeconds, 0.001), "1:00.000");
    assert_eq!(format_precise_clock(39.75, Clock::MinutesSeconds, 0.01), "0:39.75");
}

#[test]
fn held_time_scheme_preserves_labels_and_spacing_during_pan() {
    let metrics = across(&DejaVuSans);
    for (start, span) in [(55., 10.), (599., 30.), (3595., 10.), (0.001, 0.01)] {
        let first_axis = axis(1200, start, start + span);
        let first = tick_layout(AxisKind::PreciseTime, first_axis, &metrics, None);
        let scheme = first.scheme.unwrap();
        let next_axis = axis(1200, start + scheme.step, start + span + scheme.step);
        let next = tick_layout(AxisKind::PreciseTime, next_axis, &metrics, Some(scheme));
        assert_eq!(next.scheme, Some(scheme));
        let displacement = scheme.step / span * 1199.;
        let mut shared = 0;
        for a in &first.ticks {
            if let Some(b) = next.ticks.iter().find(|b| b.value == a.value) {
                assert_eq!(a.label, b.label);
                assert!(((a.offset - b.offset) as f64 - displacement).abs() <= 1.);
                shared += 1;
            }
        }
        assert!(shared >= 2);
    }
}

#[test]
fn decimal_time_and_sample_rulers_keep_labels_readable_at_capture_offsets() {
    let text = DejaVuSans;
    for kind in [AxisKind::Seconds, AxisKind::Samples] {
        for length in [240, 800, 2000] {
            for (start, span) in [(0., 10.), (12_345., 0.1), (2e6, 1000.), (1e12, 1e6)] {
                let marks = ticks(kind, axis(length, start, start + span), &across(&text));
                assert_no_overlap(&marks, &across(&text), "time ruler");
                assert_ruler_values(kind, &marks, span);
            }
        }
    }
}

#[test]
fn sample_ticks_are_exact_integer_multiples_even_above_f64_unit_precision() {
    let text = DejaVuSans;
    for (min, max) in [(0., 1.), (1e18, 1e18 + 1e8), (u64::MAX as f64 - 1e8, u64::MAX as f64)] {
        let layout = tick_layout(AxisKind::Samples, axis(1800, min, max), &across(&text), None);
        let step = layout.scheme.unwrap().step as u128;
        assert!(step >= 1);
        assert!(!layout.ticks.is_empty());
        for mark in layout.ticks {
            let index: u128 = mark.label.trim_start_matches('#').parse().unwrap();
            assert_eq!(index % step, 0);
            assert_eq!(index as f64, mark.value);
        }
    }
    let held = TickScheme { step: 0.25, span: 1. };
    let result = tick_layout(AxisKind::Samples, axis(800, 0., 1.), &across(&text), Some(held));
    assert!(result.scheme.unwrap().step >= 1.);
}

fn assert_ruler_values(kind: AxisKind, marks: &[Tick], span: f64) {
    for mark in marks {
        let printed: f64 = match kind {
            AxisKind::Samples => mark.label.trim_start_matches('#').parse::<u64>().unwrap() as f64,
            _ => mark.label.trim_end_matches(" s").parse().unwrap(),
        };
        assert!((printed - mark.value).abs() < span * 1e-6);
    }
}

#[test]
fn keeping_edge_marks_preserves_spacing_and_all_readable_labels_on_resize() {
    let text = DejaVuSans;
    for kind in [AxisKind::PreciseTime, AxisKind::Seconds, AxisKind::Samples] {
        for length in [360, 480, 720, 1000, 1500] {
            let extent = Axis { length, min: 0., max: 19. * 3600., lead: 0, trail: 0 };
            let plain = tick_layout(kind, extent, &across(&text).after_tick(9.), None);
            let marks = tick_layout(kind, extent, &across(&text).after_tick(9.).keep_edge_marks(), None);
            assert_eq!(marks.scheme, plain.scheme);
            let visible: Vec<_> = marks.ticks.iter().filter(|tick| !tick.label.is_empty()).cloned().collect();
            assert_eq!(visible, plain.ticks);
            let step = marks.scheme.unwrap().step;
            assert_eq!(marks.ticks.last().unwrap().value, (extent.max / step).floor() * step);
            assert!(marks.ticks.iter().all(|tick| (0..length).contains(&tick.offset)));
        }
    }
}
