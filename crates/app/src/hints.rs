//! Hints: passive ones that follow the pointer, and pinned ones that stay until dismissed.
//!
//! A passive hint (`.tooltip`) lives only while its trigger is hovered, so the
//! pointer is never inside it anywhere but over the trigger, and the trigger
//! keeps the pointer and its clicks.
//!
//! A pinned hint is a standard `Popover` that opens when its trigger has been
//! hovered for a moment and then stays open until a click outside, Enter or
//! Escape closes it. While it is open it owns the input: it holds keyboard
//! focus, and a transparent backdrop covers everything else, so the plot gets
//! no pointer, clicks or wheel, and the click that closes the hint goes no
//! further. Escape also asks its owner to revert what changed while it was open.

use std::time::Duration;

use gpui_kit::base::actions::Cancel;
use gpui_kit::component::button::Button;
use gpui_kit::component::popover::Popover;
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::{
    Anchor, AnyElement, AnyView, App, AppContext, Context, CursorStyle, Entity, EventEmitter,
    FocusHandle, InteractiveElement, IntoElement, KeyBinding, MouseButton, NoAction, ParentElement,
    Render, Styled, Task, Window, deferred, div,
};

/// The key context of a pinned hint's content, inside the popover's own.
const CONTEXT: &str = "PinnedHint";

pub(super) fn init(cx: &mut App) {
    // The popover confirms on Space as on Enter, which is not one of the ways a pinned hint closes.
    cx.bind_keys([KeyBinding::new("space", NoAction, Some(CONTEXT))]);
}

/// How long the trigger must be hovered before a pinned hint opens, as for a tooltip.
const OPEN_DELAY: Duration = Duration::from_millis(500);

/// The view a `.tooltip` builder returns.
pub(super) fn passive(
    cx: &mut App,
    build: impl FnOnce(&mut Context<Tooltip>) -> Tooltip,
) -> AnyView {
    cx.new(build).into()
}

/// What a pinned hint tells its owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Pinned {
    Opened,
    /// Closed, and whether the owner should undo what changed since it opened.
    Closed {
        revert: bool,
    },
}

type Build = Box<dyn Fn(&mut Window, &mut App) -> Tooltip>;

/// The open state of one pinned hint and the content it shows while open.
pub(super) struct PinnedHint {
    build: Build,
    content: Option<Entity<Tooltip>>,
    focus: FocusHandle,
    pending: Option<Task<()>>,
}

impl EventEmitter<Pinned> for PinnedHint {}

impl PinnedHint {
    pub(super) fn new(
        build: impl Fn(&mut Window, &mut App) -> Tooltip + 'static,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            build: Box::new(build),
            content: None,
            focus: cx.focus_handle(),
            pending: None,
        }
    }

    pub(super) fn is_open(&self) -> bool {
        self.content.is_some()
    }

    /// Open after the trigger has been hovered for a moment, unless the pointer leaves first.
    pub(super) fn hover(&mut self, hovered: bool, window: &mut Window, cx: &mut Context<Self>) {
        if !hovered || self.is_open() {
            self.pending = None;
            return;
        }
        self.pending = Some(cx.spawn_in(window, async move |hint, cx| {
            cx.background_executor().timer(OPEN_DELAY).await;
            let _ = hint.update_in(cx, |hint, window, cx| hint.open(window, cx));
        }));
    }

    pub(super) fn open(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.pending = None;
        if self.is_open() {
            return;
        }
        let tooltip = (self.build)(window, cx).m_0();
        self.content = Some(cx.new(|_| tooltip));
        cx.emit(Pinned::Opened);
        cx.notify();
    }

    pub(super) fn close(&mut self, revert: bool, cx: &mut Context<Self>) {
        self.pending = None;
        if self.content.take().is_some() {
            cx.emit(Pinned::Closed { revert });
            cx.notify();
        }
    }
}

impl Render for PinnedHint {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

/// The trigger with its pinned hint anchored above it.
pub(super) fn pinned(
    id: &'static str,
    hint: &Entity<PinnedHint>,
    trigger: Button,
    cx: &App,
) -> impl IntoElement {
    let state = hint.read(cx);
    let content = state.content.clone();
    let focus = state.focus.clone();
    let changed = hint.downgrade();
    let cancelled = hint.downgrade();
    Popover::new(id)
        .anchor(Anchor::BottomLeft)
        .appearance(false)
        // The left click keeps its own meaning on the trigger.
        .mouse_button(MouseButton::Right)
        .open(content.is_some())
        .track_focus(&focus)
        .on_open_change(move |open, window, cx| {
            let _ = changed.update(cx, |hint, cx| {
                if *open {
                    hint.open(window, cx);
                } else {
                    hint.close(false, cx);
                }
            });
        })
        .trigger(trigger)
        .content(move |_, _, _| {
            let cancelled = cancelled.clone();
            div()
                .track_focus(&focus)
                .key_context(CONTEXT)
                .cursor(CursorStyle::Arrow)
                .on_action(move |_: &Cancel, _, cx| {
                    let _ = cancelled.update(cx, |hint, cx| hint.close(true, cx));
                    // The popover's own Escape handling still closes it.
                    cx.propagate();
                })
                .children(content.clone())
        })
}

/// While the hint is open, covers the window beneath it so nothing else sees the pointer.
pub(super) fn backdrop(hint: &Entity<PinnedHint>, cx: &App) -> Option<AnyElement> {
    hint.read(cx).is_open().then(|| {
        deferred(
            div()
                .id("pinned-hint-backdrop")
                .absolute()
                .inset_0()
                .occlude()
                .cursor(CursorStyle::Arrow),
        )
        .with_priority(1)
        .into_any_element()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::test::TestWindowExt;
    use gpui_kit::{
        Bounds, Modifiers, ScrollDelta, ScrollWheelEvent, StatefulInteractiveElement, Subscription,
        TestAppContext, TestSupportExt, VisualTestContext, point, px, size,
    };

    gpui_kit::actions!(hint_tests, [Nudge]);

    /// A plot filling the window, with a pinned hint's trigger at its bottom left.
    struct Harness {
        hint: Entity<PinnedHint>,
        focus: FocusHandle,
        events: Vec<Pinned>,
        nudges: usize,
        presses: usize,
        wheels: usize,
        hovered: Option<bool>,
        _events: Subscription,
    }

    impl Render for Harness {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let hint = self.hint.clone();
            let trigger =
                Button::new("trigger")
                    .label("FFT")
                    .on_hover(move |hovered, window, cx| {
                        hint.update(cx, |hint, cx| hint.hover(*hovered, window, cx))
                    });
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
                        .on_hover(
                            cx.listener(|harness, hovered, _, _| harness.hovered = Some(*hovered)),
                        )
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|harness, _, _, _| harness.presses += 1),
                        )
                        .on_scroll_wheel(cx.listener(|harness, _, _, _| harness.wheels += 1)),
                )
                .child(
                    div()
                        .absolute()
                        .left(px(20.))
                        .top(px(260.))
                        .child(pinned("hint", &self.hint, trigger, cx)),
                )
                .children(backdrop(&self.hint, cx))
        }
    }

    const TRIGGER: (f32, f32) = (30., 268.);
    const OUTSIDE: (f32, f32) = (300., 60.);

    fn at((x, y): (f32, f32)) -> gpui_kit::Point<gpui_kit::Pixels> {
        point(px(x), px(y))
    }

    fn frame(cx: &mut VisualTestContext) {
        cx.run_until_parked();
        cx.update(|window, cx| window.render_frame(cx));
        cx.run_until_parked();
    }

    fn open_window(cx: &mut TestAppContext) -> (Entity<Harness>, &mut VisualTestContext) {
        cx.update(|cx| {
            gpui_kit::init(cx);
            init(cx);
            cx.bind_keys([KeyBinding::new("left", Nudge, Some("Plot"))]);
        });
        let (harness, cx) = cx.add_window_view(|window, cx| {
            let focus = cx.focus_handle();
            window.focus(&focus, cx);
            let hint = cx.new(|cx| PinnedHint::new(|_, _| Tooltip::new("Analysis"), cx));
            let events = cx.subscribe(&hint, |harness: &mut Harness, _, event, _| {
                harness.events.push(*event)
            });
            Harness {
                hint,
                focus,
                events: Vec::new(),
                nudges: 0,
                presses: 0,
                wheels: 0,
                hovered: None,
                _events: events,
            }
        });
        cx.simulate_resize(size(px(400.), px(300.)));
        frame(cx);
        (harness, cx)
    }

    /// Hovers the trigger long enough to open the hint, then moves away from it.
    fn open_and_leave(cx: &mut TestAppContext) -> (Entity<Harness>, &mut VisualTestContext) {
        let (harness, cx) = open_window(cx);
        cx.simulate_mouse_move(at(TRIGGER), None, Modifiers::default());
        cx.executor().advance_clock(Duration::from_secs(1));
        frame(cx);
        cx.simulate_mouse_move(at(OUTSIDE), None, Modifiers::default());
        cx.executor().advance_clock(Duration::from_secs(1));
        frame(cx);
        (harness, cx)
    }

    fn read<T>(
        cx: &mut VisualTestContext,
        harness: &Entity<Harness>,
        f: impl FnOnce(&Harness, &App) -> T,
    ) -> T {
        harness.read_with(cx, |harness, cx| f(harness, cx))
    }

    fn is_open(cx: &mut VisualTestContext, harness: &Entity<Harness>) -> bool {
        read(cx, harness, |harness, cx| harness.hint.read(cx).is_open())
    }

    fn events(cx: &mut VisualTestContext, harness: &Entity<Harness>) -> Vec<Pinned> {
        read(cx, harness, |harness, _| harness.events.clone())
    }

    fn nudge(cx: &mut VisualTestContext, harness: &Entity<Harness>) -> usize {
        cx.simulate_keystrokes("left");
        read(cx, harness, |harness, _| harness.nudges)
    }

    #[gpui_kit::test]
    fn a_short_hover_opens_nothing(cx: &mut TestAppContext) {
        let (harness, cx) = open_window(cx);
        cx.simulate_mouse_move(at(TRIGGER), None, Modifiers::default());
        cx.simulate_mouse_move(at(OUTSIDE), None, Modifiers::default());
        cx.executor().advance_clock(Duration::from_secs(1));
        frame(cx);
        assert!(!is_open(cx, &harness));
    }

    #[gpui_kit::test]
    fn leaving_the_trigger_keeps_the_hint_open(cx: &mut TestAppContext) {
        let (harness, cx) = open_and_leave(cx);
        assert!(is_open(cx, &harness));
        assert_eq!(events(cx, &harness), [Pinned::Opened]);
    }

    #[gpui_kit::test]
    fn an_open_hint_keeps_the_plot_from_input(cx: &mut TestAppContext) {
        let (harness, cx) = open_and_leave(cx);
        assert_eq!(read(cx, &harness, |h, _| h.hovered), Some(false));
        cx.simulate_event(ScrollWheelEvent {
            position: at(OUTSIDE),
            delta: ScrollDelta::Pixels(point(px(0.), px(40.))),
            ..Default::default()
        });
        assert_eq!(nudge(cx, &harness), 0, "keys do not reach the plot");
        assert_eq!(read(cx, &harness, |h, _| h.wheels), 0, "nor does the wheel");
    }

    #[gpui_kit::test]
    fn a_click_outside_closes_the_hint_and_goes_no_further(cx: &mut TestAppContext) {
        let (harness, cx) = open_and_leave(cx);
        cx.simulate_mouse_down(at(OUTSIDE), MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(at(OUTSIDE), MouseButton::Left, Modifiers::default());
        frame(cx);
        assert!(!is_open(cx, &harness));
        assert_eq!(read(cx, &harness, |h, _| h.presses), 0);
        assert_eq!(
            events(cx, &harness),
            [Pinned::Opened, Pinned::Closed { revert: false }]
        );
        assert_eq!(nudge(cx, &harness), 1, "the keyboard is back");
    }

    #[gpui_kit::test]
    fn enter_closes_the_hint_and_keeps_the_values(cx: &mut TestAppContext) {
        let (harness, cx) = open_and_leave(cx);
        cx.simulate_keystrokes("enter");
        frame(cx);
        assert!(!is_open(cx, &harness));
        assert_eq!(
            events(cx, &harness),
            [Pinned::Opened, Pinned::Closed { revert: false }]
        );
        assert_eq!(nudge(cx, &harness), 1);
    }

    #[gpui_kit::test]
    fn space_leaves_the_hint_open(cx: &mut TestAppContext) {
        let (harness, cx) = open_and_leave(cx);
        cx.simulate_keystrokes("space");
        frame(cx);
        assert!(is_open(cx, &harness));
        assert_eq!(events(cx, &harness), [Pinned::Opened]);
    }

    #[gpui_kit::test]
    fn escape_closes_the_hint_and_reverts(cx: &mut TestAppContext) {
        let (harness, cx) = open_and_leave(cx);
        cx.simulate_keystrokes("escape");
        frame(cx);
        assert!(!is_open(cx, &harness));
        assert_eq!(
            events(cx, &harness),
            [Pinned::Opened, Pinned::Closed { revert: true }]
        );
        assert_eq!(nudge(cx, &harness), 1);
    }

    /// A zoom-half-sized button whose own hint opens partly over it.
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
                    .size(px(22.))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|trigger, _, _, _| trigger.presses += 1),
                    )
                    .tooltip(|_, cx| passive(cx, |_| observed_hint())),
            )
        }
    }

    fn observed_hint() -> Tooltip {
        Tooltip::element(|_, _| div().id("hint-text").test_support().child("A hint"))
    }

    #[gpui_kit::test]
    fn a_passive_hint_leaves_its_trigger_the_click(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        let (trigger, cx) = cx.add_window_view(|_, _| Trigger::default());
        cx.simulate_resize(size(px(400.), px(300.)));
        let none = Modifiers::default();
        cx.simulate_mouse_move(point(px(11.), px(11.)), None, none);
        cx.executor().advance_clock(Duration::from_secs(1));
        frame(cx);
        // Still on the button, and inside the box its hint opens 13 pixels away.
        let overlap = point(px(28.), px(28.));
        cx.simulate_mouse_move(overlap, None, none);
        frame(cx);
        let text = cx
            .update(|window, _| window.try_find("hint-text"))
            .expect("the hint is shown");
        // The standard tooltip pads its text by 8 and 2 pixels inside a 1-pixel border.
        let frame = point(px(9.), px(3.));
        let hint = Bounds::from_corners(
            text.bounds().origin - frame,
            text.bounds().bottom_right() + frame,
        );
        assert!(text.visible() && hint.contains(&overlap), "{hint:?}");
        cx.simulate_mouse_down(overlap, MouseButton::Left, none);
        assert_eq!(trigger.read_with(cx, |trigger, _| trigger.presses), 1);
    }
}
