//! Standard tooltips and explicitly opened information popovers.

use gpui_kit::base::actions::{Cancel, Confirm};
use gpui_kit::component::Selectable;
use gpui_kit::component::popover::Popover;
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::{
    Anchor, AnyElement, AnyView, App, AppContext, Context, CursorStyle, Entity, EventEmitter,
    FocusHandle, InteractiveElement, IntoElement, KeyBinding, NoAction, ParentElement, Render,
    Styled, Window, deferred, div,
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

/// The view a `.tooltip` builder returns.
pub(super) fn passive(
    cx: &mut App,
    build: impl FnOnce(&mut Context<Tooltip>) -> Tooltip,
) -> AnyView {
    cx.new(build).into()
}

/// The information panel that owns interaction in one window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Panel {
    File,
    Analysis,
    About,
}

impl Panel {
    pub(super) const ALL: [Self; 3] = [Self::File, Self::Analysis, Self::About];
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
        }
    }

    pub(super) fn focus_handle(&self) -> FocusHandle {
        self.focus.clone()
    }

    pub(super) fn is_open(&self) -> bool {
        self.content.is_some()
    }

    /// The view the open hint shows.
    pub(super) fn view(&self) -> Option<AnyView> {
        self.content.clone()
    }

    pub(super) fn open(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.is_open() {
            return;
        }
        self.content = Some((self.build)(window, cx));
        cx.emit(Pinned::Opened);
        cx.notify();
    }

    pub(super) fn close(&mut self, revert: bool, cx: &mut Context<Self>) {
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
    let focus = state.focus_handle();
    let changed = hint.downgrade();
    let cancelled = hint.downgrade();
    let confirmed = hint.downgrade();
    Popover::new(id)
        .anchor(anchor)
        .appearance(false)
        // The backdrop closes the hint, so a part of it drawn outside its panel is not outside it.
        .overlay_closable(false)
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
