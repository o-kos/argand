//! Recording metadata and scale presentation in the shared corner panel.

use super::numeric_editor::{self, NumericEditor};
use super::settings_ui::{StatusPanel, editor as presentation};
use super::*;
use crate::navigation::time_ruler::{FrequencyMode, Mode, Origin as TimeOrigin};
use gpui_kit::base::actions::Confirm;
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::radio::Radio;
use gpui_kit::{AnyElement, AnyView};

pub(super) struct State {
    pub hint: Entity<hints::PinnedHint>,
    editor: Option<Entity<Editor>>,
    _subscription: Subscription,
}

impl State {
    pub fn new(window: &mut Window, cx: &mut Context<Shell>) -> Self {
        let owner = cx.entity().downgrade();
        let hint = cx.new(|cx| {
            hints::PinnedHint::new(move |window, cx| panel(owner.clone(), window, cx), cx)
        });
        let subscription = cx.subscribe_in(&hint, window, |shell, hint, event, window, cx| {
            match event {
                hints::Pinned::Opened => {
                    shell.reference.editor = hint
                        .read(cx)
                        .view()
                        .and_then(|view| view.downcast::<Editor>().ok());
                }
                hints::Pinned::Closed { revert } => {
                    finish_editor(shell.reference.editor.take(), *revert, window, cx);
                }
            }
            shell.information_panel_changed(hints::Panel::Reference, *event, cx);
        });
        Self {
            hint,
            editor: None,
            _subscription: subscription,
        }
    }
}

fn finish_editor(
    editor: Option<Entity<Editor>>,
    revert: bool,
    window: &mut Window,
    cx: &mut Context<Shell>,
) {
    let Some(editor) = editor.filter(|_| !revert) else {
        return;
    };
    // Release Shell's event borrow before committing with the original document guard.
    window.defer(cx, move |window, cx| {
        editor.update(cx, |editor, cx| {
            editor.commit(window, cx);
            editor.commit_start(window, cx);
        });
    });
}

struct Editor {
    owner: WeakEntity<Shell>,
    document: u64,
    opened: f64,
    opened_start: Option<argand_core::RecordingTime>,
    start_input: Entity<InputState>,
    start_refused: bool,
    number: Entity<NumericEditor>,
    input: Entity<InputState>,
    refused: bool,
    placed: bool,
    _subscriptions: Vec<Subscription>,
}

fn value(input: &InputState) -> Option<f64> {
    let compact: String = input
        .value()
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    crate::numbers::parse::<f64>(&compact)
        .ok()
        .filter(|v| v.is_finite() && (0. ..=f64::from(u32::MAX)).contains(v))
        .map(f64::round)
}

fn panel(owner: WeakEntity<Shell>, window: &mut Window, cx: &mut gpui_kit::App) -> AnyView {
    let (document, opened, opened_start) = owner
        .upgrade()
        .and_then(|owner| {
            let shell = owner.read(cx);
            Some((
                shell.file.as_ref()?.id,
                shell.editing()?.reference(),
                shell.editing()?.recording_start(),
            ))
        })
        .unwrap_or((0, 0., None));
    cx.new(|cx| {
        let number = cx.new(|cx| {
            NumericEditor::new(
                crate::numbers::number(opened.round()),
                numeric_editor::Config {
                    unit: "Hz",
                    integer: true,
                    step: 1.,
                    minimum: 0.,
                    maximum: f64::from(u32::MAX),
                    minimum_inclusive: true,
                    down: "reference-down",
                    up: "reference-up",
                },
                window,
                cx,
            )
        });
        let input = number.read(cx).input.clone();
        let changed = cx.subscribe_in(
            &number,
            window,
            |editor: &mut Editor, _, event: &numeric_editor::Changed, window, cx| {
                if event.committed {
                    editor.commit(window, cx);
                }
                cx.notify();
            },
        );
        let start_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("YYYY-MM-DDTHH:MM:SS.sssZ")
                .default_value(opened_start.map_or_else(String::new, |start| start.format(3)))
        });
        let start_changed = cx.subscribe_in(
            &start_input,
            window,
            |editor: &mut Editor, _, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::PressEnter { .. } | InputEvent::Blur) {
                    editor.commit_start(window, cx);
                }
                cx.notify();
            },
        );
        let mut subscriptions = vec![changed, start_changed];
        if let Some(owner) = owner.upgrade() {
            subscriptions.push(cx.observe(&owner, |_, _, cx| cx.notify()));
        }
        Editor {
            owner,
            document,
            opened,
            opened_start,
            start_input,
            start_refused: false,
            number,
            input,
            refused: false,
            placed: false,
            _subscriptions: subscriptions,
        }
    })
    .into()
}

impl Editor {
    fn is_current(&self, shell: &Shell) -> bool {
        shell.is_reference_editable()
            && shell
                .file
                .as_ref()
                .is_some_and(|file| file.id == self.document)
            && shell
                .editing()
                .is_some_and(|editing| editing.reference() == self.opened)
    }

    fn commit_start(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let text = self.start_input.read(cx).value();
        // The displayed millisecond value is only a draft. An unchanged draft
        // must preserve the precise origin/sample offset retained by editing.
        if text.as_ref()
            == self
                .opened_start
                .map_or_else(String::new, |start| start.format(3))
        {
            self.start_refused = false;
            return true;
        }
        let start = if text.trim().is_empty() {
            None
        } else {
            let Some(start) = argand_core::RecordingTime::parse(text.as_ref()) else {
                self.start_refused = true;
                cx.notify();
                return false;
            };
            Some(start.rounded(3))
        };
        let accepted = self
            .owner
            .update(cx, |shell, cx| {
                if !self.is_current(shell)
                    || shell
                        .editing()
                        .and_then(|editing| editing.recording_start())
                        != self.opened_start
                {
                    return false;
                }
                shell.set_recording_start(start, window, cx);
                true
            })
            .unwrap_or(false);
        self.start_refused = !accepted;
        if accepted {
            self.opened_start = start;
            let shown = start.map_or_else(String::new, |start| start.format(3));
            if self.start_input.read(cx).value().as_ref() != shown {
                self.start_input
                    .update(cx, |input, cx| input.set_value(shown, window, cx));
            }
        }
        cx.notify();
        accepted
    }

    fn frequency_controls(&self, physical: bool, cx: &Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .font_weight(gpui_kit::FontWeight::MEDIUM)
                    .child("Frequency"),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(self.radio(
                        ("frequency-relative", "Relative", !physical, false),
                        Choice::Frequency(FrequencyMode::Baseband),
                        cx,
                    ))
                    .child(self.radio(
                        ("frequency-absolute", "Absolute RF", physical, false),
                        Choice::Frequency(FrequencyMode::Physical),
                        cx,
                    )),
            )
    }

    fn time_controls(&self, state: (bool, Mode, bool), cx: &Context<Self>) -> impl IntoElement {
        let (absolute, format, has_start) = state;
        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .mt_2()
                    .font_weight(gpui_kit::FontWeight::MEDIUM)
                    .child("Time"),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(self.radio(
                        ("time-relative", "Relative", !absolute, false),
                        Choice::Time(TimeOrigin::Relative),
                        cx,
                    ))
                    .child(self.radio(
                        ("time-absolute", "Absolute UTC", absolute, !has_start),
                        Choice::Time(TimeOrigin::Absolute),
                        cx,
                    )),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(self.radio(
                        ("time-clock", "hms", format == Mode::Clock, absolute),
                        Choice::Format(Mode::Clock),
                        cx,
                    ))
                    .child(self.radio(
                        ("time-seconds", "Seconds", format == Mode::Seconds, absolute),
                        Choice::Format(Mode::Seconds),
                        cx,
                    ))
                    .child(self.radio(
                        ("time-samples", "Samples", format == Mode::Samples, absolute),
                        Choice::Format(Mode::Samples),
                        cx,
                    )),
            )
    }

    fn radio(
        &self,
        config: (&'static str, &'static str, bool, bool),
        choice: Choice,
        _cx: &Context<Self>,
    ) -> Radio {
        let (id, label, checked, disabled) = config;
        let owner = self.owner.clone();
        Radio::new(id)
            .label(label)
            .checked(checked)
            .disabled(disabled)
            .small()
            .on_change(move |_, _, cx| {
                let _ = owner.update(cx, |shell, cx| match choice {
                    Choice::Time(origin) => shell.set_time_origin(origin, cx),
                    Choice::Frequency(mode) => shell.set_frequency_mode(mode, cx),
                    Choice::Format(mode) => shell.set_time_ruler(mode, cx),
                });
            })
    }

    fn commit(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some(value) = value(self.input.read(cx)) else {
            self.refused = true;
            cx.notify();
            return false;
        };
        let accepted = self
            .owner
            .update(cx, |shell, cx| {
                if !self.is_current(shell) {
                    return false;
                }
                shell.set_reference(value, window, cx);
                true
            })
            .unwrap_or(false);
        self.refused = !accepted;
        if accepted {
            self.opened = value;
            let shown = crate::numbers::number(value);
            if self.input.read(cx).value().as_ref() != shown {
                self.input
                    .update(cx, |input, cx| input.set_value(shown, window, cx));
            }
        }
        cx.notify();
        accepted
    }
}

#[derive(Clone, Copy)]
enum Choice {
    Time(TimeOrigin),
    Frequency(FrequencyMode),
    Format(Mode),
}

impl Render for Editor {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let editable = self
            .owner
            .upgrade()
            .is_some_and(|owner| self.is_current(owner.read(cx)));
        let valid = value(self.input.read(cx));
        let (physical, absolute, format, has_start) = self
            .owner
            .upgrade()
            .map(|owner| {
                let shell = owner.read(cx);
                (
                    shell.session.frequency_mode == FrequencyMode::Physical,
                    shell.absolute_start().is_some(),
                    shell.session.time_ruler,
                    shell
                        .editing()
                        .and_then(|editing| editing.recording_start())
                        .is_some(),
                )
            })
            .unwrap_or((true, false, Mode::Clock, false));
        self.number
            .update(cx, |number, cx| number.set_disabled(!editable, cx));
        if !self.placed {
            self.placed = true;
            let input = self.input.clone();
            window.defer(cx, move |window, cx| {
                input.update(cx, |input, cx| {
                    input.focus(window, cx);
                    input.select_all(window, cx);
                })
            });
        }
        div()
            .id("reference-editor")
            .map(gpui_kit::TestSupportExt::test_support)
            .key_context("ReferenceEditor")
            .on_action(cx.listener(|editor, _: &Confirm, window, cx| {
                editor.commit(window, cx);
                editor.commit_start(window, cx);
                cx.stop_propagation();
            }))
            .w(presentation::property_width(window))
            .font_family(cx.theme().font_family.clone())
            .bg(crate::theme::sheet(cx))
            .text_color(cx.theme().popover_foreground)
            .border_1()
            .border_color(cx.theme().border)
            .shadow_md()
            .rounded(cx.theme().radius)
            .px_3()
            .py_2()
            .text_sm()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .font_weight(gpui_kit::FontWeight::MEDIUM)
                    .child("Recording metadata and scales"),
            )
            .child(self.frequency_controls(physical, cx))
            .child(presentation::row("Reference", self.number.clone(), cx))
            .child(self.time_controls((absolute, format, has_start), cx))
            .child(presentation::row(
                "Start UTC",
                Input::new(&self.start_input).small().disabled(!editable),
                cx,
            ))
            .when(!has_start, |panel| {
                panel.child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child("Recording start is not set"),
                )
            })
            .when(self.start_refused, |panel| {
                panel.child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().danger)
                        .child("Enter a valid UTC date and time, or leave empty"),
                )
            })
            .when(valid.is_none() || self.refused, |panel| {
                panel.child(div().text_color(cx.theme().danger).child(if !editable {
                    "Reference editing is unavailable"
                } else {
                    "Enter a frequency from 0 to 4 294 967 295 Hz"
                }))
            })
    }
}

#[cfg(test)]
#[path = "reference_ui_test.rs"]
mod tests;

impl Shell {
    pub(super) fn edit_reference(
        &mut self,
        _: &EditReference,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open_reference(window, cx);
    }

    pub(super) fn open_reference(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.activate_information_panel(hints::Panel::Reference, window, cx);
    }

    pub(super) fn reference_control(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        self.editing()?;
        let frequency = if self.session.frequency_mode == FrequencyMode::Physical {
            "A"
        } else {
            "R"
        };
        let time = if self.absolute_start().is_some() {
            "A"
        } else {
            "R"
        };
        let control =
            self.status_panel_control(StatusPanel::Reference, format!("{frequency}/{time}"), cx);
        Some(
            gpui_kit::deferred(
                div()
                    .absolute()
                    .when(self.session.orientation.vertical(), |button| {
                        button.left_0().bottom_0()
                    })
                    .when(!self.session.orientation.vertical(), |button| {
                        button.right_0().top_0()
                    })
                    .child(control),
            )
            .with_priority(2)
            .into_any_element(),
        )
    }
}
