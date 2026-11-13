//! Direct reference editing at the frequency ruler, using standard Button and Input.

use super::plot_view::{PlotIntent, PlotSnapshot, PlotView};
use crate::shell::*;
use gpui_kit::component::Disableable;
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::{AnyElement, rems};

actions!(reference_entry, [CancelReference]);
const CONTEXT: &str = "ReferenceEntry";
const WIDTH: f32 = 8.25;
const INPUT_WIDTH: f32 = 11.25;
pub(super) const HEIGHT: f32 = 1.5;

fn reference_value(text: &str) -> Option<f64> {
    let compact: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    crate::numbers::parse::<f64>(&compact)
        .ok()
        .filter(|v| v.is_finite() && (0. ..=f64::from(u32::MAX)).contains(v))
}

pub(in crate::shell) struct ReferenceEntry {
    input: Entity<InputState>,
    opened: f64,
    refused: bool,
    _events: Subscription,
}

pub(in crate::shell) fn init(cx: &mut gpui_kit::App) {
    cx.bind_keys([KeyBinding::new("escape", CancelReference, Some(CONTEXT))]);
    super::super::badge_entry::step_keys(CONTEXT, cx);
}

impl PlotView {
    fn open_reference(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(snapshot) = &self.snapshot else {
            return;
        };
        let Some(reference) = snapshot.reference.filter(|_| snapshot.reference_enabled) else {
            return;
        };
        let input = cx.new(|cx| {
            InputState::new(window, cx).default_value(crate::numbers::number(reference.round()))
        });
        let events = cx.subscribe_in(&input, window, |plot, _, event, window, cx| match event {
            InputEvent::PressEnter { .. } => plot.commit_reference(true, window, cx),
            InputEvent::Blur => plot.commit_reference(false, window, cx),
            _ => cx.notify(),
        });
        input.update(cx, |input, cx| {
            input.focus(window, cx);
            input.select_all(window, cx);
        });
        self.reference_entry = Some(ReferenceEntry {
            input,
            opened: reference,
            refused: false,
            _events: events,
        });
        cx.notify();
    }

    fn close_reference(&mut self, focus: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.reference_entry = None;
        if focus {
            window.focus(&self.focus, cx);
        }
        cx.notify();
    }

    fn commit_reference(&mut self, focus: bool, _window: &mut Window, cx: &mut Context<Self>) {
        let Some(entry) = self.reference_entry.as_mut() else {
            return;
        };
        let current = self
            .snapshot
            .as_ref()
            .filter(|s| s.reference_enabled)
            .and_then(|s| s.reference);
        if current != Some(entry.opened) {
            entry.refused = true;
            cx.notify();
            return;
        }
        let value = reference_value(entry.input.read(cx).value().as_ref()).map(f64::round);
        let Some(value) = value else {
            entry.refused = true;
            cx.notify();
            return;
        };
        cx.emit(PlotIntent::Reference {
            value,
            opened: entry.opened,
            focus,
        });
    }

    pub(in crate::shell) fn reference_committed(
        &mut self,
        accepted: bool,
        focus: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if accepted {
            self.close_reference(focus, window, cx);
        } else if let Some(entry) = &mut self.reference_entry {
            entry.refused = true;
            cx.notify();
        }
    }

    fn step_reference(&mut self, steps: i64, window: &mut Window, cx: &mut Context<Self>) {
        let Some(entry) = self.reference_entry.as_mut() else {
            return;
        };
        let value = reference_value(entry.input.read(cx).value().as_ref())
            .map(|value| value.round() + steps as f64)
            .filter(|v| v.is_finite() && (0. ..=f64::from(u32::MAX)).contains(v));
        match value {
            Some(value) => {
                entry.refused = false;
                entry.input.update(cx, |input, cx| {
                    input.set_value(crate::numbers::number(value), window, cx)
                });
            }
            None => entry.refused = true,
        }
        cx.notify();
    }

    pub(super) fn reference_width(&self) -> f32 {
        if self.reference_entry.is_some() {
            INPUT_WIDTH
        } else {
            WIDTH
        }
    }

    pub(super) fn reference_control(
        &self,
        snapshot: &PlotSnapshot,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let reference = snapshot.reference?;
        let element = if let Some(entry) = &self.reference_entry {
            let input = Input::new(&entry.input)
                .xsmall()
                .suffix("Hz")
                .w_full()
                .h_full()
                .id("reference-frequency-value")
                .disabled(!snapshot.reference_enabled);
            super::super::badge_entry::on_steps(
                div().id("reference-frequency-input"),
                Self::step_reference,
                cx,
            )
            .size_full()
            .key_context(CONTEXT)
            .on_action(cx.listener(|plot, _: &CancelReference, window, cx| {
                plot.close_reference(true, window, cx)
            }))
            .border_1()
            .border_color(if entry.refused {
                cx.theme().danger
            } else {
                cx.theme().border
            })
            .child(input)
            .into_any_element()
        } else {
            Button::new("reference-frequency")
                .xsmall()
                .text_size(rems(0.6875))
                .w_full()
                .h_full()
                .label(crate::numbers::text(&argand_core::format_hz(reference)))
                .disabled(!snapshot.reference_enabled)
                .tooltip("Edit reference frequency")
                .on_click(cx.listener(|plot, _, window, cx| plot.open_reference(window, cx)))
                .into_any_element()
        };
        Some(
            div()
                .id("reference-frequency-control")
                .absolute()
                .when(snapshot.extents.orientation.vertical(), |d| {
                    d.left_0().bottom_0()
                })
                .when(!snapshot.extents.orientation.vertical(), |d| {
                    d.right_0().top_0()
                })
                .w(rems(self.reference_width()))
                .h(rems(HEIGHT))
                .bg(cx.theme().background)
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .occlude()
                .child(element)
                .into_any_element(),
        )
    }
}

#[cfg(test)]
#[path = "reference_ui_test.rs"]
mod tests;
