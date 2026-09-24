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
    AnyView, App, AppContext, Context, CursorStyle, Entity, FocusHandle, InteractiveElement,
    IntoElement, ParentElement, Render, StatefulInteractiveElement, Styled, Window, div,
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
    window: &Window,
    cx: &mut App,
    build: impl FnOnce(&mut Context<Tooltip>) -> Tooltip,
) -> AnyView {
    let tooltip = cx.new(|cx| build(cx).m_0());
    cx.new(|cx| {
        cx.on_release_in(window, |surface: &mut Surface, window, cx| {
            surface.give_back(window, cx)
        })
        .detach();
        Surface {
            tooltip,
            focus: cx.focus_handle(),
            previous: None,
        }
    })
    .into()
}

/// An interactive hint, owning the pointer and the keyboard while the pointer is inside its box.
struct Surface {
    tooltip: Entity<Tooltip>,
    focus: FocusHandle,
    /// Where the keyboard goes back to when the pointer leaves or the hint closes under it.
    previous: Option<FocusHandle>,
}

impl Surface {
    fn hovered(&mut self, hovered: bool, window: &mut Window, cx: &mut App) {
        if !hovered {
            self.give_back(window, cx);
        } else if !self.focus.is_focused(window) {
            self.previous = window.focused(cx);
            window.focus(&self.focus, cx);
        }
    }

    fn give_back(&mut self, window: &mut Window, cx: &mut App) {
        if self.focus.is_focused(window)
            && let Some(previous) = self.previous.take()
        {
            window.focus(&previous, cx);
        }
    }
}

impl Render for Surface {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Only the box blocks the pointer, the margin around it stays transparent.
        div().p(gpui_kit::px(MARGIN)).child(
            div()
                .id("hint-surface")
                .occlude()
                .cursor(CursorStyle::Arrow)
                .track_focus(&self.focus)
                .on_hover(
                    cx.listener(|surface, hovered, window, cx| {
                        surface.hovered(*hovered, window, cx)
                    }),
                )
                .child(self.tooltip.clone()),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::prelude::FluentBuilder;
    use gpui_kit::{
        Modifiers, MouseButton, ScrollDelta, ScrollWheelEvent, TestAppContext, VisualTestContext,
        point, px, size,
    };

    gpui_kit::actions!(hint_tests, [Nudge]);

    /// A plot-like surface filling the window, with a hint trigger in its corner.
    struct Harness {
        plain: bool,
        /// The trigger goes away with its hint when this is cleared.
        trigger: bool,
        focus: FocusHandle,
        nudges: usize,
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
                        .track_focus(&self.focus)
                        .key_context("Plot")
                        .on_action(cx.listener(|harness, _: &Nudge, _, _| harness.nudges += 1))
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
                .when(self.trigger, |harness| {
                    harness.child(
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
                })
        }
    }

    fn hint(plain: bool, window: &mut Window, cx: &mut App) -> AnyView {
        let tooltip = Tooltip::new("A hint wide enough to enter");
        if plain {
            tooltip.build(window, cx)
        } else {
            interactive(window, cx, |_| tooltip)
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
        cx.update(|cx| {
            gpui_kit::init(cx);
            cx.bind_keys([gpui_kit::KeyBinding::new("left", Nudge, Some("Plot"))]);
        });
        let (harness, cx) = cx.add_window_view(|window, cx| {
            let focus = cx.focus_handle();
            window.focus(&focus, cx);
            Harness {
                plain,
                trigger: true,
                focus,
                nudges: 0,
                hovered: None,
                moves: 0,
                presses: 0,
                wheels: 0,
            }
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
    fn nudges(cx: &mut VisualTestContext, harness: &Entity<Harness>) -> usize {
        harness.read_with(cx, |harness, _| harness.nudges)
    }

    #[gpui_kit::test]
    fn an_interactive_hint_holds_the_keyboard_while_the_pointer_is_inside(cx: &mut TestAppContext) {
        let (harness, cx, _) = open(cx, false);
        cx.simulate_keystrokes("left");
        assert_eq!(
            nudges(cx, &harness),
            0,
            "the plot keys wait while in the hint"
        );
        cx.simulate_mouse_move(point(px(300.), px(250.)), None, Modifiers::default());
        cx.simulate_keystrokes("left");
        assert_eq!(nudges(cx, &harness), 1, "leaving gives the keyboard back");
    }

    #[gpui_kit::test]
    fn a_hint_closing_under_the_pointer_gives_the_keyboard_back(cx: &mut TestAppContext) {
        let (harness, cx, _) = open(cx, false);
        cx.simulate_keystrokes("left");
        assert_eq!(nudges(cx, &harness), 0);
        harness.update(cx, |harness, cx| {
            harness.trigger = false;
            cx.notify();
        });
        cx.run_until_parked();
        cx.simulate_keystrokes("left");
        assert_eq!(nudges(cx, &harness), 1);
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
