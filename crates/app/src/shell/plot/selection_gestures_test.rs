use super::*;

#[test]
fn the_nearer_edge_is_grabbed_within_its_zone() {
    let extent = Extent {
        low: 0.25,
        high: 0.75,
        pixels: 400.,
    };
    assert_eq!(extent.edge_at(0.25 + 3. / 400.), Some(Edge::Low));
    assert_eq!(extent.edge_at(0.75 - 3.9 / 400.), Some(Edge::High));
    assert_eq!(extent.edge_at(0.75 - 4.1 / 400.), None);
    assert_eq!(extent.edge_at(0.5), None);
    let narrow = Extent {
        low: 0.5,
        high: 0.505,
        pixels: 400.,
    };
    assert_eq!(narrow.edge_at(0.504), Some(Edge::High));
    assert!(extent.spans(0.25 - 3. / 400.) && !extent.spans(0.2));
}

#[test]
fn a_moved_span_stays_exact_and_inside_a_huge_capture() {
    let span = SampleSpan::between(u64::MAX - (1 << 20), u64::MAX).unwrap();
    assert_eq!(moved_span(span, 0., u64::MAX), span);
    assert_eq!(moved_span(span, 1e6, u64::MAX), span, "held at the end");
    let moved = moved_span(span, -3., u64::MAX);
    assert_eq!(moved.start(), span.start() - 3);
    let early = SampleSpan::between(5, 10).unwrap();
    assert_eq!(moved_span(early, -100., 1000).start(), 0);
}
