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

use std::time::{Duration, Instant};

use gpui_kit::base::actions::{Cancel, Confirm};
use gpui_kit::component::Selectable;
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
    cx.bind_keys([
        KeyBinding::new("space", NoAction, Some(CONTEXT)),
        KeyBinding::new("enter", Confirm { secondary: false }, Some(CONTEXT)),
    ]);
}

/// How long the trigger must be hovered before a pinned hint opens, as for a tooltip.
const OPEN_DELAY: Duration = Duration::from_millis(500);

/// How soon after closing a hover counts as the trigger uncovered, not a new approach.
const REHOVER_GRACE: Duration = Duration::from_millis(400);

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

type Build = Box<dyn Fn(&mut Window, &mut App) -> AnyView>;

/// The open state of one pinned hint and the content it shows while open.
pub(super) struct PinnedHint {
    build: Build,
    content: Option<AnyView>,
    focus: FocusHandle,
    pending: Option<Task<()>>,
    /// When the hint last closed, so the trigger uncovered beneath a still pointer does not reopen it.
    closed_at: Option<Instant>,
}

impl EventEmitter<Pinned> for PinnedHint {}

impl PinnedHint {
    pub(super) fn new(
        build: impl Fn(&mut Window, &mut App) -> AnyView + 'static,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            build: Box::new(build),
            content: None,
            focus: cx.focus_handle(),
            pending: None,
            closed_at: None,
        }
    }

    pub(super) fn is_open(&self) -> bool {
        self.content.is_some()
    }

    /// The view the open hint shows.
    pub(super) fn view(&self) -> Option<AnyView> {
        self.content.clone()
    }

    /// Open after the trigger has been hovered for a moment, unless the pointer leaves first.
    pub(super) fn hover(&mut self, hovered: bool, window: &mut Window, cx: &mut Context<Self>) {
        if !hovered || self.is_open() || self.just_closed() {
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
        self.content = Some((self.build)(window, cx));
        cx.emit(Pinned::Opened);
        cx.notify();
    }

    /// Whether a hover now is the trigger reappearing under the pointer that just closed the hint.
    fn just_closed(&self) -> bool {
        self.closed_at
            .is_some_and(|closed| closed.elapsed() < REHOVER_GRACE)
    }

    pub(super) fn close(&mut self, revert: bool, cx: &mut Context<Self>) {
        self.pending = None;
        if self.content.take().is_some() {
            self.closed_at = Some(Instant::now());
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

/// The trigger with its pinned hint anchored to it at `anchor`.
pub(super) fn pinned(
    id: &'static str,
    hint: &Entity<PinnedHint>,
    trigger: impl Selectable + IntoElement + 'static,
    anchor: Anchor,
    cx: &App,
) -> AnyElement {
    let state = hint.read(cx);
    let content = state.content.clone();
    let focus = state.focus.clone();
    let changed = hint.downgrade();
    let cancelled = hint.downgrade();
    let confirmed = hint.downgrade();
    Popover::new(id)
        .anchor(anchor)
        .appearance(false)
        // The backdrop closes the hint, so a part of it drawn outside its panel is not outside it.
        .overlay_closable(false)
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
            let confirmed = confirmed.clone();
            div()
                .track_focus(&focus)
                .key_context(CONTEXT)
                .cursor(CursorStyle::Arrow)
                .on_action(move |_: &Confirm, _, cx| {
                    let _ = confirmed.update(cx, |hint, cx| hint.close(false, cx));
                })
                .on_action(move |_: &Cancel, _, cx| {
                    let _ = cancelled.update(cx, |hint, cx| hint.close(true, cx));
                    // The popover's own Escape handling still closes it.
                    cx.propagate();
                })
                .children(content.clone())
        })
        .into_any_element()
}

/// While the hint is open, covers `area` and closes the hint on a press there, keeping its values.
///
/// Outside `area` the title bar and the frame's resize edges stay live, and close the hint themselves.
pub(super) fn backdrop(
    hint: &Entity<PinnedHint>,
    area: gpui_kit::Bounds<gpui_kit::Pixels>,
    cx: &App,
) -> Option<AnyElement> {
    hint.read(cx).is_open().then(|| {
        let pressed = hint.downgrade();
        let cover = div()
            .id("pinned-hint-backdrop")
            .absolute()
            .left(area.origin.x)
            .top(area.origin.y)
            .w(area.size.width)
            .h(area.size.height)
            .occlude()
            .cursor(CursorStyle::Arrow)
            .on_any_mouse_down(move |_, _, cx| {
                let _ = pressed.update(cx, |hint, cx| hint.close(false, cx));
            });
        deferred(cover).with_priority(1).into_any_element()
    })
}

#[cfg(test)]
#[path = "hints_test.rs"]
mod tests;
