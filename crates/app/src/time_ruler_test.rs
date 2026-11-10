use super::*;

#[test]
fn modes_preserve_capture_origin_and_use_sample_units_once() {
    let view = View {
        start: 4_000_000,
        len: 2_000,
    };
    let mut ruler = Ruler {
        mode: Mode::Samples,
        view,
        total: 8_000_000,
    };
    let seconds = view.seconds(2e6);
    assert_eq!(ruler.bounds(seconds), (4_000_000., 4_002_000.));
    assert_eq!(ruler.readout(2.0005, 0.001, 1000., 0.5), "#4,001,000");
    assert_eq!(Mode::Samples.sample_step(100., 2e6), 100.);
    ruler.mode = Mode::Seconds;
    assert_eq!(ruler.bounds(seconds), seconds);
    assert_eq!(ruler.readout(2.0005, 0.001, 1000., 0.5), "2.000500 s");
    assert_eq!(Mode::Seconds.sample_step(0.001, 2e6), 2000.);
    ruler.mode = Mode::Clock;
    assert_eq!(ruler.readout(2.0005, 0.001, 1000., 0.5), "0:02.000500");
    assert_eq!(ruler.view, view);
}

#[test]
fn sample_badges_clamp_to_actual_capture_indices_without_large_origin_rounding() {
    let ruler = Ruler {
        mode: Mode::Samples,
        view: View {
            start: u64::MAX - 4096,
            len: 4096,
        },
        total: u64::MAX,
    };
    assert_eq!(
        ruler.readout(0., 1., 1000., 0.),
        format!("#{}", crate::numbers::number(u64::MAX - 4096))
    );
    assert_eq!(
        ruler.readout(0., 1., 1000., 0.5),
        format!("#{}", crate::numbers::number(u64::MAX - 2048))
    );
    assert_eq!(
        ruler.readout(0., 1., 1000., 1.),
        format!("#{}", crate::numbers::number(u64::MAX - 1))
    );
}

fn span(a: u64, b: u64) -> argand_core::SampleSpan {
    argand_core::SampleSpan::between(a, b).expect("a span")
}

fn texts(counters: [String; 3]) -> [String; 3] {
    counters
}

#[test]
fn a_selection_reads_sample_exact_in_every_mode() {
    let span = span(4_000_000, 4_001_000);
    assert_eq!(
        texts(Mode::Samples.counters(span, 2e6, 4_100_000, 1e-9)),
        ["#4,000,000", "#4,000,999", "1,000"]
    );
    assert_eq!(
        texts(Mode::Seconds.counters(span, 2e6, 4_100_000, 1e-9)),
        ["2.0000000 s", "2.0005000 s", "0.0005000 s"]
    );
    assert_eq!(
        texts(Mode::Clock.counters(span, 2e6, 4_100_000, 1e-9)),
        ["0:02.0000000", "0:02.0005000", "0:00.0005000"]
    );
}

#[test]
fn only_the_clock_keeps_the_capture_s_fields() {
    assert_eq!(
        Mode::Samples.counters(span(12, 345), 1e3, 1_000_000, 1e-9),
        ["#12", "#344", "333"]
    );
    assert_eq!(
        Mode::Seconds.counters(span(0, 10), 1e3, 100_000, 1e-9)[0],
        "0.000 s"
    );
    assert_eq!(
        Mode::Clock.counters(span(0, 3_600_000), 1e3, 7_200_000, 1e-9)[1],
        "1:00:00.000"
    );
}

#[test]
fn decimals_follow_the_zoom_but_never_pass_one_sample() {
    let span = span(4_000_000, 4_001_000);
    assert_eq!(
        texts(Mode::Seconds.counters(span, 2e6, 4_100_000, 0.004)),
        ["2.000 s", "2.001 s", "0.001 s"],
        "milliseconds where a pixel holds four of them"
    );
    assert_eq!(
        texts(Mode::Seconds.counters(span, 2e6, 4_100_000, 2e-6)),
        ["2.000000 s", "2.000500 s", "0.000500 s"]
    );
}

#[test]
fn a_selection_seconds_cannot_tell_apart_is_given_in_samples() {
    let fast = Mode::Seconds.counters(span(100, 101), 4e9, 200, 1e-12);
    assert_eq!(
        texts(fast)[0],
        "0.0000000250 s",
        "past nanoseconds where a pixel and a sample are finer"
    );
    assert_eq!(
        texts(Mode::Clock.counters(span(100, 101), 4e9, 200, 1e-12)),
        ["#100", "#100", "1"],
        "the clock stops at nanoseconds"
    );
    assert_eq!(
        texts(Mode::Clock.counters(span(u64::MAX - 1, u64::MAX), 24e6, u64::MAX, 1e-9)),
        texts(Mode::Samples.counters(span(u64::MAX - 1, u64::MAX), 24e6, u64::MAX, 1e-9))
    );
    assert_eq!(
        texts(Mode::Seconds.counters(span(0, 10), 0., 10, 1e-9)),
        ["#0", "#9", "10"],
        "no rate, no seconds"
    );
}
