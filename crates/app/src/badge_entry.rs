//! Typing a selection's value over its badge, opened by a double click there.

use super::plot_view::{PlotIntent, PlotSnapshot, PlotView};
use super::*;
use crate::selection_entry::Field;
use gpui_kit::AnyElement;
use gpui_kit::component::input::{Input, InputEvent, InputState};

actions!(badge_entry, [CancelBadgeEntry]);

/// The input's surroundings, which take the Escape the input passes on.
const CONTEXT: &str = "BadgeEntry";

/// The least width of the input, for a badge too short to type a value into.
const MIN_WIDTH: f32 = 120.;

pub(super) fn init(cx: &mut gpui_kit::App) {
    cx.bind_keys([KeyBinding::new("escape", CancelBadgeEntry, Some(CONTEXT))]);
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
            InputEvent::Blur => plot.close_entry(None, cx),
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
        let width = (badge.size.width + px(24.))
            .max(px(MIN_WIDTH))
            .min(spectrum.size.width);
        let left = (badge.center().x - width / 2.).clamp(spectrum.left(), spectrum.right() - width);
        let input = Input::new(&entry.input)
            .small()
            .w(width)
            .when(entry.marked(cx), |input| {
                input.border_color(cx.theme().danger)
            });
        Some(
            div()
                .id("badge-entry")
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
                .child(input)
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
