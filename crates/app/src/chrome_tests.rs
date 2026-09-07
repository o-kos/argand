use super::*;

fn normal() -> Frame {
    Frame::new(size(px(1024.0), px(724.0)), Some(Tiling::default()), false)
}

fn edge_at(frame: &Frame, x: f32, y: f32) -> Option<ResizeEdge> {
    frame.regions.iter().find_map(|(edge, bounds)| {
        let point = point(px(x), px(y));
        bounds.contains(&point).then_some(*edge)
    })
}

#[test]
fn resize_regions_leave_all_content_to_its_own_cursor() {
    let frame = normal();
    for (x, y, edge) in [
        (6.0, 6.0, ResizeEdge::TopLeft),
        (500.0, 6.0, ResizeEdge::Top),
        (1018.0, 6.0, ResizeEdge::TopRight),
        (6.0, 350.0, ResizeEdge::Left),
        (1018.0, 350.0, ResizeEdge::Right),
        (6.0, 718.0, ResizeEdge::BottomLeft),
        (500.0, 718.0, ResizeEdge::Bottom),
        (1018.0, 718.0, ResizeEdge::BottomRight),
    ] {
        assert_eq!(edge_at(&frame, x, y), Some(edge));
    }
    for (x, y) in [(19.0, 19.0), (1000.0, 30.0), (500.0, 350.0), (1000.0, 700.0)] {
        assert_eq!(edge_at(&frame, x, y), None);
    }
}

#[test]
fn resize_grips_are_reachable_inside_the_visible_frame() {
    let frame = normal();
    assert_eq!(edge_at(&frame, 13.0, 350.0), Some(ResizeEdge::Left));
    assert_eq!(edge_at(&frame, 1011.0, 350.0), Some(ResizeEdge::Right));
    assert_eq!(edge_at(&frame, 500.0, 13.0), Some(ResizeEdge::Top));
    assert_eq!(edge_at(&frame, 500.0, 711.0), Some(ResizeEdge::Bottom));
    assert_eq!(edge_at(&frame, 13.0, 13.0), Some(ResizeEdge::TopLeft));
    assert_eq!(edge_at(&frame, 1011.0, 711.0), Some(ResizeEdge::BottomRight));
}

#[test]
fn expanded_windows_have_no_resize_regions_or_corners() {
    // Decorations can lag a state change: an untiled report must not leave
    // resize handlers over the right-hand part of a maximized title bar.
    let frame = Frame::new(size(px(1600.0), px(1000.0)), Some(Tiling::default()), true);
    assert!(frame.regions.is_empty());
    assert_eq!(frame.padding, Edges::default());
    assert_eq!(frame.corners, Corners::default());
    assert!(!frame.shadow);
}

#[test]
fn only_free_tiled_edges_can_resize_or_round() {
    let frame = Frame::new(
        size(px(1000.0), px(700.0)),
        Some(Tiling { top: true, left: true, ..Tiling::default() }),
        false,
    );
    assert_eq!(edge_at(&frame, 500.0, 6.0), None);
    assert_eq!(edge_at(&frame, 6.0, 350.0), None);
    assert_eq!(edge_at(&frame, 994.0, 350.0), Some(ResizeEdge::Right));
    assert_eq!(frame.corners.top_left, px(0.0));
    assert_eq!(frame.corners.top_right, px(0.0));
    assert_eq!(frame.corners.bottom_left, px(0.0));
    assert_eq!(frame.corners.bottom_right, RADIUS);
    assert!(!frame.shadow);
}

#[test]
fn native_decorations_do_not_get_a_second_frame() {
    let frame = Frame::new(size(px(1000.0), px(700.0)), None, false);
    assert!(frame.regions.is_empty());
    assert_eq!(frame.padding, Edges::default());
    assert_eq!(frame.corners, Corners::default());
    assert!(!frame.shadow);
}
