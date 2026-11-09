//! Shared measured advice for the two status property panels.
use super::*;
use gpui_kit::component::IconName;
use gpui_kit::{AnyElement, EntityId, TestSupportExt};
use std::cell::Cell;
use std::rc::Rc;

pub(super) fn color(cx: &gpui_kit::App) -> gpui_kit::Hsla {
    if cx.theme().is_dark() {
        gpui_kit::rgb(0xfacc15).into()
    } else {
        gpui_kit::rgb(0x946200).into()
    }
}

/// The widest the balloon's body grows, its pointer's length and the tip's distance from the sign.
const BALLOON: f32 = 320.0;
const POINTER: f32 = 9.0;
const BALLOON_GAP: f32 = 3.0;
/// The narrowest body the balloon takes on the right before it moves to the left.
const BALLOON_MIN: f32 = 220.0;
/// How far the balloon keeps from the window's edge, clear of the frame's resize grips.
const BALLOON_MARGIN: f32 = 24.0;

/// Records where an element was drawn into one field of `drawn`, and redraws the editor on a change.
fn measure(
    drawn: Rc<Cell<Drawn>>,
    editor: gpui_kit::EntityId,
    field: fn(&mut Drawn) -> &mut Option<gpui_kit::Bounds<Pixels>>,
) -> impl IntoElement {
    gpui_kit::canvas(
        move |bounds, _, cx| {
            let mut now = drawn.get();
            if *field(&mut now) != Some(bounds) {
                *field(&mut now) = Some(bounds);
                drawn.set(now);
                cx.notify(editor);
            }
        },
        |_, _, _, _| {},
    )
    .absolute()
    .size_full()
}

/// Where the last frame drew the panel and its advice anchor.
#[derive(Clone, Copy, Default, PartialEq)]
struct Drawn {
    sheet: Option<gpui_kit::Bounds<Pixels>>,
    readout: Option<gpui_kit::Bounds<Pixels>>,
}

/// Which side of the hint the balloon stands on, and how wide its body may grow.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Side {
    Right(Pixels),
    /// With the distance from the advice anchor's right edge to the balloon's right edge.
    Left(Pixels, Pixels),
}

/// Where the balloon fits beside a hint drawn at `sheet` with its advice anchor at `readout`.
///
/// It stands right of the hint, wrapping into the room there is, and moves left of the hint
/// once that room is narrower than a readable line. Without room on either side it is not
/// shown, and the value's hover hint says the same.
fn balloon_side(
    sheet: gpui_kit::Bounds<Pixels>,
    readout: gpui_kit::Bounds<Pixels>,
    viewport: Pixels,
) -> Option<Side> {
    let reach = px(BALLOON_GAP + POINTER + BALLOON_MARGIN);
    let right = (viewport - readout.right() - reach).min(px(BALLOON));
    let left = (sheet.left() - reach).min(px(BALLOON));
    let beside = readout.right() - sheet.left() + px(BALLOON_GAP);
    let readable = px(BALLOON_MIN);
    if right >= readable {
        Some(Side::Right(right))
    } else if left >= readable {
        Some(Side::Left(left, beside))
    } else {
        None
    }
}

#[derive(Clone, Default)]
pub(super) struct Advice {
    drawn: Rc<Cell<Drawn>>,
    closed: Rc<Cell<bool>>,
}

pub(super) type Click = Rc<dyn Fn(&gpui_kit::ClickEvent, &mut Window, &mut gpui_kit::App)>;
pub(super) type Hover = Rc<dyn Fn(&mut Window, &mut gpui_kit::App) -> gpui_kit::AnyView>;

pub(super) struct Action {
    pub click: Click,
    pub keycap: Option<AnyElement>,
}

pub(super) struct Content {
    pub anchor: &'static str,
    pub balloon: &'static str,
    pub close: &'static str,
    pub pointer: &'static str,
    pub warning: String,
    pub edge: gpui_kit::Hsla,
    pub action: Option<Action>,
    pub hover: Hover,
}

impl Advice {
    pub fn reset(&self) {
        self.closed.set(false);
        let mut drawn = self.drawn.get();
        drawn.readout = None;
        self.drawn.set(drawn);
    }

    pub fn sheet(&self, owner: EntityId) -> impl IntoElement {
        measure(self.drawn.clone(), owner, |drawn| &mut drawn.sheet)
    }

    pub fn render(
        &self,
        value: impl IntoElement,
        content: Content,
        suppressed: bool,
        owner: EntityId,
        window: &Window,
        cx: &gpui_kit::App,
    ) -> AnyElement {
        let drawn = self.drawn.get();
        let measured = drawn.sheet.zip(drawn.readout);
        let side = measured.and_then(|(sheet, readout)| {
            balloon_side(sheet, readout, window.viewport_size().width)
        });
        let hover = self.closed.get() || (measured.is_some() && side.is_none());
        let activate = content.action.as_ref().map(|action| action.click.clone());
        let anchor = content.anchor;
        let edge = content.edge;
        let tip = content.hover.clone();
        let balloon = side
            .filter(|_| !self.closed.get() && !suppressed)
            .map(|side| self.balloon(side, content, owner, cx));
        div()
            .id(anchor)
            .test_support()
            .relative()
            .text_color(edge)
            .when_some(activate, |value, action| {
                value
                    .border_b_1()
                    .border_dashed()
                    .border_color(edge.opacity(0.6))
                    .cursor_pointer()
                    .on_click(move |event, window, cx| action(event, window, cx))
            })
            .when(hover && !suppressed, |value| {
                value.tooltip(move |window, cx| tip(window, cx))
            })
            .child(value)
            .child(measure(self.drawn.clone(), owner, |drawn| {
                &mut drawn.readout
            }))
            .children(balloon)
            .into_any_element()
    }

    fn balloon(
        &self,
        side: Side,
        content: Content,
        owner: EntityId,
        cx: &gpui_kit::App,
    ) -> AnyElement {
        let (room, left) = match side {
            Side::Right(room) => (room, true),
            Side::Left(room, _) => (room, false),
        };
        let activate = content.action.as_ref().map(|action| action.click.clone());
        let pointer_action = activate.clone();
        let shortcut = content.action.and_then(|action| action.keycap);
        let closed = self.closed.clone();
        let close = Button::new(content.close)
            .ghost()
            .xsmall()
            .icon(IconName::Close)
            .tab_stop(false)
            .on_click(move |_, _, cx| {
                cx.stop_propagation();
                closed.set(true);
                cx.notify(owner);
            });
        let edge = content.edge;
        let body = div()
            .id(content.balloon)
            .occlude()
            .when_some(activate, |body, action| {
                body.cursor_pointer()
                    .on_click(move |event, window, cx| action(event, window, cx))
            })
            .when(left, |body| body.ml(px(POINTER - 1.)))
            .when(!left, |body| body.mr(px(POINTER - 1.)))
            .max_w(room)
            .flex()
            .items_center()
            .gap_2()
            .pl_2()
            .pr_1()
            .py_0p5()
            .bg(cx.theme().tokens.popover)
            .border_1()
            .border_color(edge)
            .rounded(cx.theme().radius)
            .shadow_md()
            .text_color(cx.theme().popover_foreground)
            .child(div().flex_1().min_w_0().child(content.warning))
            .when_some(shortcut, |body, shortcut| body.child(shortcut))
            .child(close)
            .test_support();
        let pointer = div()
            .id(content.pointer)
            .occlude()
            .when_some(pointer_action, |pointer, action| {
                pointer
                    .cursor_pointer()
                    .on_click(move |event, window, cx| action(event, window, cx))
            })
            .absolute()
            .top_0()
            .bottom_0()
            .when(left, |pointer| pointer.left_0())
            .when(!left, |pointer| pointer.right_0())
            .flex()
            .items_center()
            .child(balloon_pointer(cx.theme().tokens.popover, edge, left));
        let balloon = div()
            .absolute()
            .top_0()
            .bottom_0()
            .w(room + px(POINTER))
            .flex()
            .items_center()
            .map(|balloon| match side {
                Side::Right(_) => balloon.left_full().ml(px(BALLOON_GAP)),
                Side::Left(_, beside) => balloon.right(beside).justify_end(),
            })
            .child(body)
            .child(pointer);
        gpui_kit::deferred(balloon)
            .with_priority(gpui_kit::base::POPUP_PRIORITY + 1)
            .into_any_element()
    }
}

/// The balloon's pointer, its tip towards the Range row on the left or the right.
fn balloon_pointer(
    fill: impl Into<gpui_kit::Hsla>,
    edge: gpui_kit::Hsla,
    leftward: bool,
) -> impl IntoElement {
    let fill = fill.into();
    gpui_kit::canvas(
        |_, _, _| (),
        move |bounds, _, window, _| {
            let (tip, base) = if leftward {
                (bounds.left(), bounds.right())
            } else {
                (bounds.right(), bounds.left())
            };
            let tip = gpui_kit::point(tip, bounds.center().y);
            let upper = gpui_kit::point(base, bounds.top());
            let lower = gpui_kit::point(base, bounds.bottom());
            let mut body = gpui_kit::PathBuilder::fill();
            body.add_polygon(&[upper, tip, lower], true);
            if let Ok(path) = body.build() {
                window.paint_path(path, fill);
            }
            let mut outline = gpui_kit::PathBuilder::stroke(px(1.0));
            outline.add_polygon(&[upper, tip, lower], false);
            if let Ok(path) = outline.build() {
                window.paint_path(path, edge);
            }
        },
    )
    .w(px(POINTER))
    .h(px(16.0))
}

#[cfg(test)]
mod balloon_tests {
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
}
