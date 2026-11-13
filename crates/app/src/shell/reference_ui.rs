//! Reference metadata editor anchored between the file and analysis summaries.

use super::numeric_editor::{self, NumericEditor};
use super::settings_ui::{StatusPanel, editor as presentation};
use super::*;
use gpui_kit::base::actions::Confirm;
use gpui_kit::component::input::InputState;
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
            shell.status_panel_changed(StatusPanel::Reference, *event, cx);
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
        });
    });
}

struct Editor {
    owner: WeakEntity<Shell>,
    document: u64,
    opened: f64,
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
    let (document, opened) = owner
        .upgrade()
        .and_then(|owner| {
            let shell = owner.read(cx);
            Some((shell.file.as_ref()?.id, shell.editing()?.reference()))
        })
        .unwrap_or((0, 0.));
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
        let mut subscriptions = vec![changed];
        if let Some(owner) = owner.upgrade() {
            subscriptions.push(cx.observe(&owner, |_, _, cx| cx.notify()));
        }
        Editor {
            owner,
            document,
            opened,
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

    fn save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.commit(window, cx) {
            return;
        }
        let _ = self.owner.update(cx, |shell, cx| {
            if shell.command_availability().is_enabled::<Save>() {
                shell.save_over(window, cx);
            }
        });
    }
}

impl Render for Editor {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (editable, dirty) = self.owner.upgrade().map_or((false, false), |owner| {
            let shell = owner.read(cx);
            (
                self.is_current(shell),
                shell.command_availability().is_enabled::<Save>(),
            )
        });
        let valid = value(self.input.read(cx));
        let save = editable && valid.is_some_and(|value| value != self.opened || dirty);
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
            .key_context("ReferenceEditor")
            .on_action(cx.listener(|editor, _: &Confirm, window, cx| {
                editor.commit(window, cx);
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
                    .child("Reference frequency"),
            )
            .child(presentation::row("Frequency", self.number.clone(), cx))
            .when(valid.is_none() || self.refused, |panel| {
                panel.child(div().text_color(cx.theme().danger).child(if !editable {
                    "Reference editing is unavailable"
                } else {
                    "Enter a frequency from 0 to 4 294 967 295 Hz"
                }))
            })
            .child(
                div().key_context("ReferenceSave").child(
                    presentation::outlined_button(
                        "save-reference",
                        "save-reference-hover",
                        "Save to file",
                        false,
                        save,
                        cx,
                    )
                    .on_click(cx.listener(Self::save_click)),
                ),
            )
    }
}

impl Editor {
    fn save_click(
        &mut self,
        _: &gpui_kit::ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.save(window, cx);
    }
}

#[cfg(test)]
#[path = "reference_ui_test.rs"]
mod tests;

impl Shell {
    pub(super) fn reference_control(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let reference = self.editing()?.reference();
        Some(
            settings_ui::separated_status_item("reference-summary", cx)
                .child(self.status_panel_control(
                    StatusPanel::Reference,
                    crate::numbers::text(&argand_core::format_hz(reference)),
                    cx,
                ))
                .into_any_element(),
        )
    }
}

pub(super) fn init(cx: &mut gpui_kit::App) {
    // Let the standard Button receive Enter instead of the pinned hint's Confirm.
    cx.bind_keys([KeyBinding::new(
        "enter",
        gpui_kit::NoAction,
        Some("ReferenceSave"),
    )]);
}
