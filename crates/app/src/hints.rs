//! Hints, and which of them own the pointer inside their visible box.
//!
//! A passive hint (`.tooltip`) lives only while its trigger is hovered, so the
//! pointer is never inside it anywhere but over the trigger, and the trigger
//! keeps the pointer and its clicks. An interactive hint (`.hoverable_tooltip`)
//! stays open when the pointer moves into it, and then owns its box. Whatever
//! lies under it -- the plot above all -- stops seeing the pointer there. It is
//! not hovered and gets no clicks, wheel gestures or cursor. The plot learns it
//! is covered from its own hitbox, so no surface needs to know which hints are
//! open.

use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::{
    AnyView, App, AppContext, Context, CursorStyle, Entity, InteractiveElement, IntoElement,
    ParentElement, Render, Styled, Window, div,
};

/// The gap between the pointer and a hint's box, which the standard tooltip
/// keeps as its own margin.
const MARGIN: f32 = 12.;

/// The view a `.tooltip` builder returns.
pub(super) fn passive(
    cx: &mut App,
    build: impl FnOnce(&mut Context<Tooltip>) -> Tooltip,
) -> AnyView {
    cx.new(build).into()
}

/// The view a `.hoverable_tooltip` builder returns.
pub(super) fn interactive(
    cx: &mut App,
    build: impl FnOnce(&mut Context<Tooltip>) -> Tooltip,
) -> AnyView {
    let tooltip = cx.new(|cx| build(cx).m_0());
    cx.new(|_| Surface { tooltip }).into()
}

struct Surface {
    tooltip: Entity<Tooltip>,
}

impl Render for Surface {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        // Only the box blocks the pointer, the margin around it stays transparent.
        div().p(gpui_kit::px(MARGIN)).child(
            div()
                .occlude()
                .cursor(CursorStyle::Arrow)
                .child(self.tooltip.clone()),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::{
        Modifiers, MouseButton, ScrollDelta, ScrollWheelEvent, StatefulInteractiveElement,
        TestAppContext, VisualTestContext, point, px, size,
    };

    /// A plot-like surface filling the window, with a hint trigger in its corner.
    #[derive(Default)]
    struct Harness {
        plain: bool,
        hovered: Option<bool>,
        moves: usize,
        presses: usize,
        wheels: usize,
    }

    impl Render for Harness {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .size_full()
                .relative()
                .child(
                    div()
                        .id("plot")
                        .absolute()
                        .inset_0()
                        .cursor(CursorStyle::Crosshair)
                        .on_hover(
                            cx.listener(|harness, hovered, _, _| harness.hovered = Some(*hovered)),
                        )
                        .on_mouse_move(cx.listener(|harness, _, _, _| harness.moves += 1))
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|harness, _, _, _| harness.presses += 1),
                        )
                        .on_scroll_wheel(cx.listener(|harness, _, _, _| harness.wheels += 1)),
                )
                .child(
                    div()
                        .id("trigger")
                        .absolute()
                        .left(px(10.))
                        .top(px(10.))
                        .size(px(20.))
                        .hoverable_tooltip({
                            let plain = self.plain;
                            move |window, cx| hint(plain, window, cx)
                        }),
                )
        }
    }

    fn hint(plain: bool, window: &mut Window, cx: &mut App) -> AnyView {
        let tooltip = Tooltip::new("A hint wide enough to enter");
        if plain {
            tooltip.build(window, cx)
        } else {
            interactive(cx, |_| tooltip)
        }
    }

    fn state(
        cx: &mut VisualTestContext,
        harness: &Entity<Harness>,
    ) -> (Option<bool>, usize, usize, usize) {
        harness.read_with(cx, |h, _| (h.hovered, h.moves, h.presses, h.wheels))
    }

    /// Opens the hint and returns a point inside its visible box.
    fn open(
        cx: &mut TestAppContext,
        plain: bool,
    ) -> (
        Entity<Harness>,
        &mut VisualTestContext,
        gpui_kit::Point<gpui_kit::Pixels>,
    ) {
        cx.update(gpui_kit::init);
        let (harness, cx) = cx.add_window_view(|_, _| Harness {
            plain,
            ..Default::default()
        });
        cx.simulate_resize(size(px(400.), px(300.)));
        cx.simulate_mouse_move(point(px(15.), px(15.)), None, Modifiers::default());
        cx.executor()
            .advance_clock(std::time::Duration::from_secs(1));
        cx.run_until_parked();
        // The tooltip opens one pixel past the pointer, its box one margin further.
        let inside = point(px(15. + 1. + MARGIN + 10.), px(15. + 1. + MARGIN + 6.));
        cx.simulate_mouse_move(inside, None, Modifiers::default());
        (harness, cx, inside)
    }

    #[gpui_kit::test]
    fn a_plain_tooltip_leaves_the_plot_under_the_pointer(cx: &mut TestAppContext) {
        let (harness, cx, inside) = open(cx, true);
        assert_ne!(state(cx, &harness).0, Some(false));
        cx.simulate_mouse_down(inside, MouseButton::Left, Modifiers::default());
        assert_eq!(state(cx, &harness).2, 1, "the click falls through");
    }

    #[gpui_kit::test]
    fn a_hint_box_takes_the_pointer_clicks_and_wheel(cx: &mut TestAppContext) {
        let (harness, cx, inside) = open(cx, false);
        let none = Modifiers::default();
        let (hovered, moves, ..) = state(cx, &harness);
        assert_eq!(
            hovered,
            Some(false),
            "the box hides the plot from the pointer"
        );
        cx.simulate_mouse_move(inside + point(px(3.), px(0.)), None, none);
        assert_eq!(
            state(cx, &harness).1,
            moves,
            "moves over the box skip the plot"
        );
        cx.simulate_mouse_down(inside, MouseButton::Left, none);
        cx.simulate_mouse_up(inside, MouseButton::Left, none);
        assert_eq!(state(cx, &harness).2, 0, "the box takes the click");
        cx.simulate_event(ScrollWheelEvent {
            position: inside,
            delta: ScrollDelta::Pixels(point(px(0.), px(10.))),
            ..Default::default()
        });
        assert_eq!(state(cx, &harness).3, 0, "the box takes the wheel");
        let margin = point(px(15. + 1. + MARGIN / 2.), inside.y);
        cx.simulate_mouse_move(margin, None, none);
        let (hovered, after, ..) = state(cx, &harness);
        assert_eq!(hovered, Some(true), "the margin leaves the plot hovered");
        assert!(after > moves, "the plot follows the pointer in the margin");
    }
    /// A large button whose own hint opens partly over it.
    #[derive(Default)]
    struct Trigger {
        presses: usize,
    }

    impl Render for Trigger {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(
                div()
                    .id("trigger")
                    .absolute()
                    .left(px(10.))
                    .top(px(10.))
                    .size(px(60.))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|trigger, _, _, _| trigger.presses += 1),
                    )
                    .tooltip(|_, cx| passive(cx, |_| Tooltip::new("A hint over its own button"))),
            )
        }
    }

    #[gpui_kit::test]
    fn a_passive_hint_leaves_its_trigger_the_click(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        let (trigger, cx) = cx.add_window_view(|_, _| Trigger::default());
        cx.simulate_resize(size(px(400.), px(300.)));
        let none = Modifiers::default();
        cx.simulate_mouse_move(point(px(15.), px(15.)), None, none);
        cx.executor()
            .advance_clock(std::time::Duration::from_secs(1));
        cx.run_until_parked();
        // Still on the button, and inside the box of its hint.
        let overlap = point(px(15. + 1. + MARGIN + 8.), px(15. + 1. + MARGIN + 8.));
        cx.simulate_mouse_move(overlap, None, none);
        cx.simulate_mouse_down(overlap, MouseButton::Left, none);
        assert_eq!(trigger.read_with(cx, |trigger, _| trigger.presses), 1);
    }
}
