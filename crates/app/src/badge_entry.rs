//! Typing a selection's value over its badge, opened by a double click there.

use super::plot_view::{PlotIntent, PlotSnapshot, PlotView};
use super::*;
use crate::selection_entry::Field;
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::{AnyElement, Focusable as _};

actions!(
    badge_entry,
    [
        CancelBadgeEntry,
        StepUp,
        StepDown,
        StepUpTenfold,
        StepDownTenfold
    ]
);

/// The input's surroundings, which take the Escape the input passes on.
const CONTEXT: &str = "BadgeEntry";

/// Room past the badge's text for the caret after its last character.
const CARET_ROOM: f32 = 4.;

/// The width the input adds to its badge and its height, which the badges keep clear of the selection.
///
/// The stock extra-small input is 1.25 rem high, and its frame adds a pixel on each side.
pub(super) fn entry_size(rem: f32) -> (f32, f32) {
    (CARET_ROOM, 1.25 * rem + 2.)
}

pub(super) fn init(cx: &mut gpui_kit::App) {
    cx.bind_keys([KeyBinding::new("escape", CancelBadgeEntry, Some(CONTEXT))]);
    step_keys(CONTEXT, cx);
}

/// Up and Down step a typed value by its last digit, ten of them with Shift, in `context`.
pub(super) fn step_keys(context: &str, cx: &mut gpui_kit::App) {
    cx.bind_keys([
        KeyBinding::new("up", StepUp, Some(context)),
        KeyBinding::new("down", StepDown, Some(context)),
        KeyBinding::new("shift-up", StepUpTenfold, Some(context)),
        KeyBinding::new("shift-down", StepDownTenfold, Some(context)),
    ]);
}

/// The listeners turning those keys into a number of steps.
pub(super) fn on_steps<T: 'static>(
    element: gpui_kit::Stateful<gpui_kit::Div>,
    step: impl Fn(&mut T, i64, &mut Window, &mut Context<T>) + Clone + 'static,
    cx: &mut Context<T>,
) -> gpui_kit::Stateful<gpui_kit::Div> {
    let (a, b, c, d) = (step.clone(), step.clone(), step.clone(), step);
    element
        .on_action(cx.listener(move |this, _: &StepUp, window, cx| a(this, 1, window, cx)))
        .on_action(cx.listener(move |this, _: &StepDown, window, cx| b(this, -1, window, cx)))
        .on_action(cx.listener(move |this, _: &StepUpTenfold, window, cx| c(this, 10, window, cx)))
        .on_action(
            cx.listener(move |this, _: &StepDownTenfold, window, cx| d(this, -10, window, cx)),
        )
}

/// A standard input standing over one badge.
pub(super) struct BadgeEntry {
    pub field: Field,
    input: Entity<InputState>,
    /// The text it opened with, which changes nothing when entered unchanged.
    opened: String,
    /// The units its text was written in, which it is read back in whatever the rulers show since.
    scale: crate::selection_entry::Scale,
    /// The selection it opened on, which it is closed without once changed elsewhere.
    selection: argand_core::Selection,
    /// The text last refused, marked while it stands unchanged.
    refused: Option<String>,
    _events: Subscription,
}

#[cfg(test)]
impl BadgeEntry {
    pub fn text(&self, cx: &gpui_kit::App) -> String {
        self.input.read(cx).value().to_string()
    }

    pub fn invalid(&self, cx: &gpui_kit::App) -> bool {
        self.marked(cx)
    }
}

impl BadgeEntry {
    fn marked(&self, cx: &gpui_kit::App) -> bool {
        self.refused
            .as_deref()
            .is_some_and(|refused| *self.input.read(cx).value() == *refused)
    }
}

impl PlotView {
    /// Keep where the badges were painted, which presses on them are told by.
    pub(super) fn place_badges(
        &mut self,
        badges: Vec<(Field, Bounds<Pixels>)>,
        cx: &mut Context<Self>,
    ) {
        if self.badges != badges {
            self.badges = badges;
            cx.notify();
        }
    }

    /// The badge under `position`, if any.
    pub(super) fn badge_at(&self, position: gpui_kit::Point<Pixels>) -> Option<Field> {
        self.snapshot.as_ref()?.badges.as_ref()?;
        self.badges
            .iter()
            .find(|(_, bounds)| bounds.contains(&position))
            .map(|(field, _)| *field)
    }

    /// Turn the badge of `field` into an input holding its value.
    pub(super) fn open_entry(&mut self, field: Field, window: &mut Window, cx: &mut Context<Self>) {
        let Some(snapshot) = &self.snapshot else {
            return;
        };
        let (Some(scale), Some(text)) = (snapshot.entry, entry_text(snapshot, field)) else {
            return;
        };
        let selection = snapshot.selected;
        let input = cx.new(|cx| InputState::new(window, cx).default_value(text.clone()));
        let events = cx.subscribe_in(&input, window, |plot, _, event, window, cx| match event {
            InputEvent::PressEnter { .. } => plot.enter_badge(window, cx),
            InputEvent::Blur => {
                // Focus taken by another control stays there, and otherwise the plot gets it back.
                let handle = plot
                    .typing
                    .as_ref()
                    .map(|entry| entry.input.focus_handle(cx));
                let elsewhere = window
                    .focused(cx)
                    .is_some_and(|focused| Some(&focused) != handle.as_ref());
                plot.close_entry((!elsewhere).then_some(window), cx);
            }
            _ => {}
        });
        input.update(cx, |input, cx| {
            input.focus(window, cx);
            input.select_all(window, cx);
        });
        self.typing = Some(BadgeEntry {
            field,
            input,
            opened: text,
            scale,
            selection,
            refused: None,
            _events: events,
        });
        cx.notify();
    }

    /// Apply the typed value, or mark it where it gives no selection.
    fn enter_badge(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(entry) = &self.typing else {
            return;
        };
        let text = entry.input.read(cx).value().to_string();
        if text == entry.opened {
            return self.close_entry(Some(window), cx);
        }
        let typed = typed(&entry.scale, entry.selection, entry.field, &text);
        match typed {
            Some(selection) => {
                self.close_entry(Some(window), cx);
                cx.emit(PlotIntent::Select(Some(selection)));
            }
            None => {
                if let Some(entry) = self.typing.as_mut() {
                    entry.refused = Some(text);
                }
                cx.notify();
            }
        }
    }

    /// Step the value by its last digit and show it at once, reading a changed text first as Enter does.
    fn step_badge(&mut self, steps: i64, window: &mut Window, cx: &mut Context<Self>) {
        let Some(entry) = &mut self.typing else {
            return;
        };
        let text = entry.input.read(cx).value().to_string();
        let base = match text == entry.opened {
            true => Some(entry.selection),
            false => typed(&entry.scale, entry.selection, entry.field, &text),
        };
        let Some(mut selection) = base else {
            entry.refused = Some(text);
            cx.notify();
            return;
        };
        let (scale, field) = (entry.scale, entry.field);
        let shown = match field.is_time() {
            true => {
                let span = selection
                    .time
                    .and_then(|span| scale.time_step(span, field, &text, steps));
                selection.time = span.or(selection.time);
                selection.time.map(|span| scale.time_texts(span))
            }
            false => {
                let band = selection
                    .band
                    .and_then(|band| scale.band_step(band, field, &text, steps));
                selection.band = band.or(selection.band);
                selection.band.map(|band| scale.band_texts(band))
            }
        };
        let Some(shown) = shown.and_then(|texts| texts.into_iter().nth(field.index())) else {
            return;
        };
        entry.selection = selection;
        entry.opened = shown.clone();
        entry.refused = None;
        entry
            .input
            .update(cx, |input, cx| input.set_value(shown, window, cx));
        cx.emit(PlotIntent::Select(Some(selection)));
        cx.notify();
    }

    /// Close the input unchanged, handing the keyboard back to the plot where a window is given.
    pub(super) fn close_entry(&mut self, window: Option<&mut Window>, cx: &mut Context<Self>) {
        if self.typing.take().is_none() {
            return;
        }
        if let Some(window) = window {
            window.focus(&self.focus, cx);
        }
        cx.notify();
    }

    /// The input over its badge, held inside the spectrum.
    pub(super) fn badge_entry(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let entry = self.typing.as_ref()?;
        let changed = self
            .snapshot
            .as_ref()
            .is_none_or(|snapshot| snapshot.selected != entry.selection);
        let badge = self
            .badges
            .iter()
            .find(|(field, _)| *field == entry.field)
            .map(|(_, bounds)| *bounds);
        let (Some(badge), Some(geometry), Some(origin), false) = (
            badge,
            self.geometry,
            self.panel_bounds.map(|b| b.origin),
            changed,
        ) else {
            // Its badge went with the selection it stood for, so the plot takes the keyboard back.
            let focus = self.focus.clone();
            window.defer(cx, move |window, cx| window.focus(&focus, cx));
            self.typing = None;
            return None;
        };
        let spectrum = geometry.spectrum;
        let width = (badge.size.width + px(CARET_ROOM)).min(spectrum.size.width);
        let left = (badge.center().x - width / 2.).clamp(spectrum.left(), spectrum.right() - width);
        let frame = match entry.marked(cx) {
            true => cx.theme().danger,
            false => cx.theme().foreground,
        };
        // The badge's own type and padding, so the value stays where the badge showed it.
        let input = Input::new(&entry.input)
            .xsmall()
            .appearance(false)
            .text_size(px(axes::LABEL_SIZE))
            .px(px(axes::BADGE_PAD))
            .w_full();
        Some(
            on_steps(div().id("badge-entry"), Self::step_badge, cx)
                .key_context(CONTEXT)
                .on_action(cx.listener(|plot, _: &CancelBadgeEntry, window, cx| {
                    plot.close_entry(Some(window), cx)
                }))
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .occlude()
                .absolute()
                .left(left - origin.x)
                .top(badge.top() - origin.y)
                .h(badge.size.height)
                .flex()
                .items_center()
                // The input keeps its own height, centred on the badge, and the frame holds it.
                .child(
                    div()
                        .w(width)
                        .bg(cx.theme().background)
                        .border_1()
                        .border_color(frame)
                        .rounded(px(3.))
                        .child(input),
                )
                .into_any_element(),
        )
    }
}

/// The value `field` holds in the selection, precise enough to read back unchanged.
fn entry_text(snapshot: &PlotSnapshot, field: Field) -> Option<String> {
    let scale = snapshot.entry?;
    let selected = snapshot.selected;
    let texts = match field.is_time() {
        true => scale.time_texts(selected.time?),
        false => scale.band_texts(selected.band?),
    };
    texts.into_iter().nth(field.index())
}

/// `selection` with `field` typed as `text`, none where the text gives none.
fn typed(
    scale: &crate::selection_entry::Scale,
    mut selection: argand_core::Selection,
    field: Field,
    text: &str,
) -> Option<argand_core::Selection> {
    match field.is_time() {
        true => selection.time = Some(scale.time(selection.time?, field, text)?),
        false => selection.band = Some(scale.band(selection.band?, field, text)?),
    }
    Some(selection)
}
