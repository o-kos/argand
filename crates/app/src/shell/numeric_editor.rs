//! One numeric editing contract for the two status property panels.

use super::*;
use gpui_kit::component::input::{InputEvent, InputState, StepAction};
use gpui_kit::component::{Disableable, Icon, IconName};
use gpui_kit::{EventEmitter, Focusable};
use std::time::Duration;

gpui_kit::actions!(numeric_editor, [Increase, Decrease]);

pub(super) const REPEAT_DELAY: Duration = Duration::from_millis(400);
pub(super) const REPEAT_EVERY: Duration = Duration::from_millis(80);

pub(super) fn init(cx: &mut gpui_kit::App) {
    cx.bind_keys([
        KeyBinding::new("up", Increase, Some("NumericEditor")),
        KeyBinding::new("down", Decrease, Some("NumericEditor")),
    ]);
}

/// Tab visits values once, while the non-tab-stop steppers remain pointer controls.
pub(super) fn traverse(
    scope: &FocusHandle,
    numbers: &[Entity<NumericEditor>],
    attempts: usize,
    forward: bool,
    window: &mut Window,
    cx: &mut gpui_kit::App,
) {
    for _ in 0..attempts {
        if forward {
            window.focus_next(cx);
        } else {
            window.focus_prev(cx);
        }
        let disabled = numbers.iter().any(|number| {
            let number = number.read(cx);
            number.disabled && number.input.read(cx).focus_handle(cx).is_focused(window)
        });
        if !disabled && scope.contains_focused(window, cx) {
            break;
        }
    }
}

#[derive(Clone, Copy)]
pub(super) struct Config {
    pub unit: &'static str,
    pub integer: bool,
    pub step: f64,
    pub minimum: f64,
    pub maximum: f64,
    pub minimum_inclusive: bool,
    pub down: &'static str,
    pub up: &'static str,
}

impl Config {
    fn accepts(self, value: f64) -> bool {
        value.is_finite()
            && value <= self.maximum
            && if self.minimum_inclusive {
                value >= self.minimum
            } else {
                value > self.minimum
            }
    }
}

/// Committed numbers preview in Analysis and remain a draft in File information.
pub(super) struct Changed {
    pub committed: bool,
}

pub(super) struct NumericEditor {
    pub input: Entity<InputState>,
    config: Config,
    repeat: Option<Task<()>>,
    disabled: bool,
    _subscription: Subscription,
}

impl EventEmitter<Changed> for NumericEditor {}

impl NumericEditor {
    pub(super) fn new(
        value: String,
        config: Config,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let input = cx.new(|cx| InputState::new(window, cx).default_value(value));
        let subscription = cx.subscribe_in(
            &input,
            window,
            |editor: &mut Self, _, event: &InputEvent, window, cx| {
                if editor.disabled {
                    return;
                }
                let committed = matches!(event, InputEvent::PressEnter { .. } | InputEvent::Blur);
                if committed {
                    editor.format(window, cx);
                }
                cx.emit(Changed { committed });
                cx.notify();
            },
        );
        Self {
            input,
            config,
            repeat: None,
            disabled: false,
            _subscription: subscription,
        }
    }

    pub(super) fn set_disabled(&mut self, disabled: bool, cx: &mut Context<Self>) {
        self.disabled = disabled;
        if disabled {
            self.stop_repeat();
        }
        cx.notify();
    }

    fn format(&self, window: &mut Window, cx: &mut Context<Self>) {
        let Ok(value) = crate::numbers::parse::<f64>(self.input.read(cx).value().as_ref()) else {
            return;
        };
        if !self.config.accepts(value) {
            return;
        }
        let value = if self.config.integer {
            value.round()
        } else {
            value
        };
        let shown = crate::numbers::number(value);
        if self.input.read(cx).value().as_ref() != shown {
            self.input
                .update(cx, |input, cx| input.set_value(shown, window, cx));
        }
    }

    fn step(&mut self, step: StepAction, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if self.disabled {
            return false;
        }
        let Ok(value) = crate::numbers::parse::<f64>(self.input.read(cx).value().as_ref()) else {
            return false;
        };
        let delta = if step == StepAction::Increment {
            self.config.step
        } else {
            -self.config.step
        };
        let value = if self.config.integer {
            value.round()
        } else {
            value
        } + delta;
        if !self.config.accepts(value) {
            return false;
        }
        self.input.update(cx, |input, cx| {
            input.set_value(crate::numbers::number(value), window, cx);
        });
        cx.emit(Changed { committed: true });
        cx.notify();
        true
    }

    fn start_repeat(&mut self, step: StepAction, window: &mut Window, cx: &mut Context<Self>) {
        if !self.step(step, window, cx) {
            return;
        }
        self.repeat = Some(cx.spawn_in(window, async move |editor, cx| {
            cx.background_executor().timer(REPEAT_DELAY).await;
            loop {
                if !matches!(
                    editor.update_in(cx, |editor, window, cx| editor.step(step, window, cx)),
                    Ok(true)
                ) {
                    break;
                }
                cx.background_executor().timer(REPEAT_EVERY).await;
            }
        }));
    }

    pub(super) fn stop_repeat(&mut self) {
        self.repeat = None;
    }

    fn button(
        &self,
        id: &'static str,
        icon: IconName,
        step: StepAction,
        cx: &mut Context<Self>,
    ) -> Button {
        let ink = cx.theme().foreground;
        let style = ButtonCustomVariant::new(cx)
            .color(cx.theme().transparent)
            .hover(ink.opacity(0.10))
            .active(ink.opacity(0.18));
        Button::new(id)
            .custom(style)
            .disabled(self.disabled)
            .group(id)
            .xsmall()
            .child(settings_ui::editor::pointer_ink(id, cx).child(Icon::new(icon).xsmall()))
            .tab_stop(false)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |editor, _, window, cx| editor.start_repeat(step, window, cx)),
            )
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|editor, _, _, _| editor.stop_repeat()),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|editor, _, _, _| editor.stop_repeat()),
            )
            .on_hover(cx.listener(|editor, hovered: &bool, _, _| {
                if !hovered {
                    editor.stop_repeat();
                }
            }))
    }
}

impl Render for NumericEditor {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let presentation = &self.input;
        let focused = presentation.read(cx).focus_handle(cx).is_focused(window);
        let available = (settings_ui::editor::value_column_width(window)
            - settings_ui::editor::number_width(self.config.unit, window, cx)
            - px(52.))
        .max(px(0.));
        let width =
            settings_ui::editor::number_width(presentation.read(cx).value().as_ref(), window, cx)
                .min(available);
        div()
            .flex()
            .items_center()
            .gap_1()
            .key_context("NumericEditor")
            .on_action(cx.listener(|editor, _: &Increase, window, cx| {
                editor.step(StepAction::Increment, window, cx);
            }))
            .on_action(cx.listener(|editor, _: &Decrease, window, cx| {
                editor.step(StepAction::Decrement, window, cx);
            }))
            .child(self.button(self.config.down, IconName::Minus, StepAction::Decrement, cx))
            .child(settings_ui::editor::number_control(
                &self.input,
                width,
                self.config.unit,
                focused,
                self.disabled,
                cx,
            ))
            .child(self.button(self.config.up, IconName::Plus, StepAction::Increment, cx))
    }
}
