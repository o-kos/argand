use super::*;
use gpui_kit::{Bounds, point, size};

/// A hint 370 wide from `left`, with its Range value ending 12 inside its right edge.
fn at(left: f32) -> (Bounds<Pixels>, Bounds<Pixels>) {
    let sheet = Bounds::new(point(px(left), px(400.)), size(px(370.), px(300.)));
    let readout = Bounds::new(point(px(left + 300.), px(600.)), size(px(58.), px(20.)));
    (sheet, readout)
}

#[test]
fn the_balloon_stands_right_while_a_readable_line_fits_there() {
    let (sheet, readout) = at(170.);
    assert_eq!(
        balloon_side(sheet, readout, px(1400.)),
        Some(Side::Right(px(BALLOON))),
        "a wide window gives the whole balloon"
    );
    let room = px(820.) - readout.right() - px(BALLOON_GAP + POINTER + BALLOON_MARGIN);
    assert_eq!(
        balloon_side(sheet, readout, px(820.)),
        Some(Side::Right(room)),
        "a narrower one wraps it into the room up to the edge"
    );
}

#[test]
fn the_balloon_moves_left_of_the_hint_without_room_on_the_right() {
    let (sheet, readout) = at(600.);
    let Some(Side::Left(room, beside)) = balloon_side(sheet, readout, px(1000.)) else {
        panic!("the balloon should stand on the left");
    };
    assert_eq!(room, px(BALLOON), "the left has room for all of it");
    assert_eq!(
        readout.right() - beside,
        sheet.left() - px(BALLOON_GAP),
        "its right edge stands just clear of the hint"
    );
}

#[test]
fn without_room_on_either_side_there_is_no_balloon() {
    let (sheet, readout) = at(170.);
    assert_eq!(balloon_side(sheet, readout, px(700.)), None);
}
