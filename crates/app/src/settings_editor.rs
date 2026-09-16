//! Inline controls rendered inside the analysis-settings hint.

use super::*;
use gpui::{Focusable, KeyDownEvent, ScrollHandle};
use std::time::Duration;

const CONTEXT: &str = "AnalysisSettings";

actions!(
    analysis_settings,
    [
        NextControl,
        PreviousControl,
        MoveUp,
        MoveDown,
        FirstOption,
        LastOption,
        ActivateControl,
        AcceptSettings,
        CloseSettings,
        EraseCharacter,
        ClearNumber,
        PasteNumber
    ]
);

pub(super) fn init(cx: &mut gpui::App) {
    cx.bind_keys([
        KeyBinding::new("tab", NextControl, Some(CONTEXT)),
        KeyBinding::new("shift-tab", PreviousControl, Some(CONTEXT)),
        KeyBinding::new("up", MoveUp, Some(CONTEXT)),
        KeyBinding::new("down", MoveDown, Some(CONTEXT)),
        KeyBinding::new("home", FirstOption, Some(CONTEXT)),
        KeyBinding::new("end", LastOption, Some(CONTEXT)),
        KeyBinding::new("enter", AcceptSettings, Some(CONTEXT)),
        KeyBinding::new("space", ActivateControl, Some(CONTEXT)),
        KeyBinding::new("escape", CloseSettings, Some(CONTEXT)),
        KeyBinding::new("backspace", EraseCharacter, Some(CONTEXT)),
        KeyBinding::new("delete", ClearNumber, Some(CONTEXT)),
        KeyBinding::new(
            if cfg!(target_os = "macos") {
                "cmd-v"
            } else {
                "ctrl-v"
            },
            PasteNumber,
            Some(CONTEXT),
        ),
    ]);
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Choice {
    Fft,
    Window,
    Aggregation,
    Palette,
    Mode,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Number {
    Overlap,
    Range,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Control {
    Fft,
    Window,
    Overlap,
    Aggregation,
    Palette,
    Mode,
    Range,
    Reset,
}

impl Control {
    const ALL: [Self; 8] = [
        Self::Fft,
        Self::Window,
        Self::Overlap,
        Self::Aggregation,
        Self::Palette,
        Self::Mode,
        Self::Range,
        Self::Reset,
    ];

    const fn choice(self) -> Option<Choice> {
        match self {
            Self::Fft => Some(Choice::Fft),
            Self::Window => Some(Choice::Window),
            Self::Aggregation => Some(Choice::Aggregation),
            Self::Palette => Some(Choice::Palette),
            Self::Mode => Some(Choice::Mode),
            _ => None,
        }
    }

    const fn number(self) -> Option<Number> {
        match self {
            Self::Overlap => Some(Number::Overlap),
            Self::Range => Some(Number::Range),
            _ => None,
        }
    }
}

pub(in crate::shell) struct Editor {
    owner: WeakEntity<Shell>,
    original_settings: Settings,
    settings: Settings,
    overlap: String,
    range: String,
    active: Control,
    expanded: Option<Choice>,
    highlighted: usize,
    replace_number: bool,
    choice_scroll: ScrollHandle,
    repeat_generation: u64,
    repeating: bool,
    closed: bool,
    error: Option<String>,
    focus: FocusHandle,
    _subscriptions: Vec<Subscription>,
}

impl Editor {
    pub(super) fn new(
        owner: WeakEntity<Shell>,
        settings: Settings,
        effective_range: f32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let mut subscriptions = Vec::new();
        let focus = cx.focus_handle();
        if let Some(shell) = owner.upgrade() {
            subscriptions.push(cx.observe(&shell, |editor, shell, cx| {
                let shell = shell.read(cx);
                if editor.settings != shell.settings {
                    editor.sync(shell.settings, shell.displayed_range());
                } else {
                    editor.update_effective_range(shell.displayed_range());
                }
                cx.notify();
            }));
        }
        subscriptions.push(cx.observe_window_activation(window, |editor, window, cx| {
            if !window.is_window_active() {
                editor.dismiss(cx);
            }
        }));
        subscriptions.push(cx.on_blur(&focus, window, |editor, _, cx| editor.dismiss(cx)));
        Self {
            owner,
            original_settings: settings,
            settings,
            overlap: crate::numbers::input(settings.overlap),
            range: crate::numbers::input(match settings.dynamic_range {
                DynamicRange::Fixed(db) => db,
                _ => effective_range,
            }),
            active: Control::Fft,
            expanded: None,
            highlighted: selected_index(Choice::Fft, settings),
            replace_number: false,
            choice_scroll: ScrollHandle::new(),
            repeat_generation: 0,
            repeating: false,
            closed: false,
            error: None,
            focus,
            _subscriptions: subscriptions,
        }
    }

    fn sync(&mut self, settings: Settings, displayed: Option<argand_dsp::DynamicRangeResult>) {
        self.settings = settings;
        self.overlap = crate::numbers::input(settings.overlap);
        self.range = crate::numbers::input(match settings.dynamic_range {
            DynamicRange::Fixed(db) => db,
            _ => displayed.map_or(110.0, |range| range.effective_db),
        });
        if let Some(choice) = self.expanded {
            self.highlighted = selected_index(choice, settings);
            self.choice_scroll.scroll_to_item(self.highlighted);
        }
        self.error = None;
    }

    fn update_effective_range(&mut self, range: Option<argand_dsp::DynamicRangeResult>) {
        if matches!(self.settings.dynamic_range, DynamicRange::Fixed(_)) {
            return;
        }
        if let Some(range) = range {
            self.range = crate::numbers::input(range.effective_db);
        }
    }

    fn apply(&mut self, settings: Settings, cx: &mut Context<Self>) -> bool {
        if settings == self.settings {
            self.error = None;
            cx.notify();
            return true;
        }
        let result = self.owner.update(cx, |shell, cx| {
            shell.preview_settings(settings, cx);
            shell.settings_error.clone()
        });
        self.error = result.unwrap_or_else(|_| Some("The signal window is closed".into()));
        if self.error.is_none() {
            let displayed = self
                .owner
                .upgrade()
                .and_then(|shell| shell.read(cx).displayed_range());
            self.sync(settings, displayed);
        }
        cx.notify();
        self.error.is_none()
    }

    fn commit_numbers(&mut self, cx: &mut Context<Self>) -> bool {
        let fixed = matches!(self.settings.dynamic_range, DynamicRange::Fixed(_));
        match self
            .settings
            .edited_numbers(&self.overlap, fixed.then_some(self.range.as_str()))
        {
            Ok(settings) => self.apply(settings, cx),
            Err(error) => {
                self.error = Some(error);
                cx.notify();
                false
            }
        }
    }

    fn finish_number(&mut self, cx: &mut Context<Self>) {
        if self.active.number().is_some() && !self.commit_numbers(cx) {
            self.overlap = crate::numbers::input(self.settings.overlap);
            if let DynamicRange::Fixed(db) = self.settings.dynamic_range {
                self.range = crate::numbers::input(db);
            }
            self.error = None;
        }
    }

    pub(in crate::shell) const fn original_settings(&self) -> Settings {
        self.original_settings
    }

    pub(super) fn rebase(&mut self, settings: Settings) {
        self.original_settings = settings;
    }

    fn finish_accept(&mut self, preserve_invalid: bool, cx: &mut Context<Self>) -> bool {
        if self.closed {
            return false;
        }
        self.stop_repeat();
        if self.active.number().is_some() && !self.commit_numbers(cx) {
            if preserve_invalid {
                return false;
            }
            self.overlap = crate::numbers::input(self.settings.overlap);
            if let DynamicRange::Fixed(db) = self.settings.dynamic_range {
                self.range = crate::numbers::input(db);
            }
            self.error = None;
        }
        self.closed = true;
        let _ = self.owner.update(cx, |shell, cx| shell.accept_settings(cx));
        true
    }

    pub(super) fn accept(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.finish_accept(true, cx) {
            let _ = self.owner.update(cx, |shell, _| shell.focus_shell(window));
        }
    }

    pub(super) fn dismiss(&mut self, cx: &mut Context<Self>) {
        if self.closed {
            return;
        }
        let pinned = self
            .owner
            .upgrade()
            .is_some_and(|shell| shell.read(cx).settings_are_pinned());
        if pinned {
            self.finish_accept(false, cx);
            return;
        }
        self.stop_repeat();
        self.closed = true;
        let _ = self.owner.update(cx, |shell, cx| shell.close_settings(cx));
    }

    fn cancel(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.closed {
            return;
        }
        self.stop_repeat();
        self.closed = true;
        let original = self.original_settings;
        let _ = self.owner.update(cx, |shell, cx| {
            shell.restore_settings(original, cx);
            shell.close_settings(cx);
            shell.focus_shell(window);
        });
    }

    fn pin(&self, cx: &mut Context<Self>) {
        let editor = cx.entity();
        let _ = self
            .owner
            .update(cx, |shell, cx| shell.pin_settings(editor, cx));
    }

    fn hover(&self, hovered: bool, window: &mut Window, cx: &mut Context<Self>) {
        let _ = self.owner.update(cx, |shell, cx| {
            shell.hover_settings_surface(hovered, window, cx);
        });
    }

    fn reset(&mut self, cx: &mut Context<Self>) {
        self.pin(cx);
        let Some(owner) = self.owner.upgrade() else {
            return;
        };
        self.apply(Settings::from_config(&owner.read(cx).config), cx);
    }

    fn activate(&mut self, control: Control, window: &mut Window, cx: &mut Context<Self>) {
        self.pin(cx);
        if self.active != control {
            self.finish_number(cx);
        }
        window.focus(&self.focus);
        let collapse = self.active == control && self.expanded == control.choice();
        self.active = control;
        self.replace_number = control.number().is_some();
        self.expanded = None;
        if let Some(choice) = control.choice().filter(|_| !collapse) {
            self.highlighted = selected_index(choice, self.settings);
            self.choice_scroll.scroll_to_item(self.highlighted);
            self.expanded = Some(choice);
        }
        cx.notify();
    }

    fn select_choice(&mut self, choice: Choice, index: usize, cx: &mut Context<Self>) {
        let Some(value) = choice_items(choice).get(index).cloned() else {
            return;
        };
        let mut settings = self.settings;
        match choice {
            Choice::Fft => {
                if let Ok(value) = crate::numbers::parse(&value) {
                    settings.fft_size = value;
                }
            }
            Choice::Window => {
                if let Ok(value) = value.parse() {
                    settings.window = value;
                }
            }
            Choice::Aggregation => {
                settings.aggregation = if value == "Mean power" {
                    Aggregation::MeanPower
                } else {
                    Aggregation::Max
                };
            }
            Choice::Palette => {
                if let Ok(value) = value.parse() {
                    settings.colormap = value;
                }
            }
            Choice::Mode => {
                settings.dynamic_range = match value.as_str() {
                    "Absolute full scale" => DynamicRange::Default,
                    "Automatic" => DynamicRange::Auto,
                    _ => DynamicRange::Fixed(crate::numbers::parse(&self.range).unwrap_or(110.0)),
                };
            }
        }
        self.expanded = None;
        self.apply(settings, cx);
    }

    fn move_control(&mut self, forward: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(current) = Control::ALL
            .iter()
            .position(|control| *control == self.active)
        else {
            return;
        };
        self.finish_number(cx);
        let mut next = current;
        loop {
            next = if forward {
                (next + 1) % Control::ALL.len()
            } else {
                (next + Control::ALL.len() - 1) % Control::ALL.len()
            };
            let control = Control::ALL[next];
            if control != Control::Range
                || matches!(self.settings.dynamic_range, DynamicRange::Fixed(_))
            {
                self.active = control;
                break;
            }
        }
        self.expanded = None;
        self.replace_number = self.active.number().is_some();
        window.focus(&self.focus);
        cx.notify();
    }

    fn move_option(&mut self, forward: bool, cx: &mut Context<Self>) {
        let Some(choice) = self.active.choice() else {
            if let Some(field) = self.active.number() {
                self.step_number(field, if forward { 1.0 } else { -1.0 }, cx);
            }
            return;
        };
        let count = choice_items(choice).len();
        if self.expanded.is_none() {
            self.expanded = Some(choice);
            self.highlighted = selected_index(choice, self.settings);
        } else if forward {
            self.highlighted = (self.highlighted + 1) % count;
        } else {
            self.highlighted = (self.highlighted + count - 1) % count;
        }
        self.choice_scroll.scroll_to_item(self.highlighted);
        cx.notify();
    }

    fn edge_option(&mut self, last: bool, cx: &mut Context<Self>) {
        let Some(choice) = self.expanded else {
            return;
        };
        self.highlighted = if last {
            choice_items(choice).len() - 1
        } else {
            0
        };
        self.choice_scroll.scroll_to_item(self.highlighted);
        cx.notify();
    }

    fn activate_current(&mut self, cx: &mut Context<Self>) {
        if let Some(choice) = self.active.choice() {
            if self.expanded == Some(choice) {
                self.select_choice(choice, self.highlighted, cx);
            } else {
                self.expanded = Some(choice);
                self.highlighted = selected_index(choice, self.settings);
                self.choice_scroll.scroll_to_item(self.highlighted);
                cx.notify();
            }
            return;
        }
        match self.active {
            Control::Overlap | Control::Range => {
                self.commit_numbers(cx);
                self.replace_number = true;
            }
            Control::Reset => self.reset(cx),
            _ => {}
        }
    }

    fn step_number(&mut self, field: Number, delta: f32, cx: &mut Context<Self>) {
        if field == Number::Range && !matches!(self.settings.dynamic_range, DynamicRange::Fixed(_))
        {
            return;
        }
        let text = match field {
            Number::Overlap => &mut self.overlap,
            Number::Range => &mut self.range,
        };
        let Ok(value) = crate::numbers::parse::<f32>(text) else {
            self.error = Some("Enter a number".into());
            cx.notify();
            return;
        };
        let value = match field {
            Number::Overlap => (value + delta).clamp(0.0, 95.0),
            Number::Range => (value + delta).max(1.0),
        };
        *text = crate::numbers::input(value);
        self.replace_number = true;
        self.commit_numbers(cx);
    }

    fn begin_repeat(
        &mut self,
        field: Number,
        control: Control,
        delta: f32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.pin(cx);
        window.prevent_default();
        window.focus(&self.focus);
        self.active = control;
        self.expanded = None;
        self.stop_repeat();
        self.repeating = true;
        let generation = self.repeat_generation;
        self.step_number(field, delta, cx);
        let executor = cx.background_executor().clone();
        cx.spawn(async move |editor, cx| {
            executor.timer(Duration::from_millis(400)).await;
            loop {
                let repeating = editor
                    .update(cx, |editor, cx| {
                        editor.repeat_step(generation, field, delta, cx)
                    })
                    .unwrap_or(false);
                if !repeating {
                    break;
                }
                executor.timer(Duration::from_millis(75)).await;
            }
        })
        .detach();
    }

    fn stop_repeat(&mut self) {
        self.repeating = false;
        self.repeat_generation = self.repeat_generation.wrapping_add(1);
    }

    pub(super) const fn is_repeating(&self) -> bool {
        self.repeating
    }

    fn finish_repeat(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.stop_repeat();
        let _ = self.owner.update(cx, |shell, cx| {
            shell.schedule_hover_close(window, cx);
        });
    }

    fn repeat_step(
        &mut self,
        generation: u64,
        field: Number,
        delta: f32,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self.repeating || self.repeat_generation != generation {
            return false;
        }
        self.step_number(field, delta, cx);
        true
    }

    fn edit_text(&mut self, text: &str, cx: &mut Context<Self>) {
        let Some(field) = self.active.number() else {
            return;
        };
        if field == Number::Range && !matches!(self.settings.dynamic_range, DynamicRange::Fixed(_))
        {
            return;
        }
        let value = match field {
            Number::Overlap => &mut self.overlap,
            Number::Range => &mut self.range,
        };
        if self.replace_number {
            value.clear();
            self.replace_number = false;
        }
        value.push_str(text);
        self.error = None;
        cx.notify();
    }

    fn erase(&mut self, clear: bool, cx: &mut Context<Self>) {
        let Some(field) = self.active.number() else {
            return;
        };
        let value = match field {
            Number::Overlap => &mut self.overlap,
            Number::Range => &mut self.range,
        };
        if clear || self.replace_number {
            value.clear();
        } else {
            value.pop();
        }
        self.replace_number = false;
        self.error = None;
        cx.notify();
    }

    fn paste(&mut self, cx: &mut Context<Self>) {
        let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) else {
            return;
        };
        self.replace_number = true;
        self.edit_text(text.trim(), cx);
    }

    fn key_down(&mut self, event: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        let stroke = &event.keystroke;
        if stroke.modifiers.control || stroke.modifiers.platform || stroke.modifiers.alt {
            return;
        }
        if let Some(text) = stroke.key_char.as_ref()
            && self.active.number().is_some()
        {
            self.edit_text(text, cx);
            cx.stop_propagation();
        }
    }

    fn choice_row(
        &self,
        label: &'static str,
        choice: Choice,
        control: Control,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let expanded = self.expanded == Some(choice);
        let active = self.active == control && self.focus.is_focused(window);
        let value = choice_value(choice, self.settings);
        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(form_row(
                label,
                div()
                    .id(("settings-choice", choice as usize))
                    .h_6()
                    .w_full()
                    .px_1()
                    .flex()
                    .items_center()
                    .justify_end()
                    .rounded(cx.theme().radius)
                    .cursor_pointer()
                    .hover(|row| row.bg(cx.theme().secondary_hover))
                    .when(expanded || active, |row| row.bg(cx.theme().secondary_hover))
                    .child(
                        div()
                            .border_b_1()
                            .border_dashed()
                            .border_color(cx.theme().muted_foreground)
                            .child(value),
                    )
                    .on_click(cx.listener(move |editor, _, window, cx| {
                        editor.activate(control, window, cx)
                    })),
                cx,
            ))
            .when(expanded, |column| {
                column.child(
                    div()
                        .id(("settings-options", choice as usize))
                        .ml(px(112.))
                        .max_h(px(176.))
                        .overflow_y_scroll()
                        .track_scroll(&self.choice_scroll)
                        .rounded(cx.theme().radius)
                        .border_1()
                        .border_color(cx.theme().border)
                        .p_1()
                        .children(choice_items(choice).into_iter().enumerate().map(
                            |(index, value)| {
                                Button::new(("settings-option", choice as usize * 100 + index))
                                    .ghost()
                                    .small()
                                    .tab_stop(false)
                                    .w_full()
                                    .justify_start()
                                    .when(index == self.highlighted, |button| {
                                        button.bg(cx.theme().secondary_hover)
                                    })
                                    .label(value)
                                    .on_click(cx.listener(move |editor, _, window, cx| {
                                        window.focus(&editor.focus);
                                        editor.select_choice(choice, index, cx);
                                    }))
                            },
                        )),
                )
            })
            .into_any_element()
    }

    fn number_row(
        &self,
        label: &'static str,
        field: Number,
        control: Control,
        suffix: &'static str,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let enabled =
            field != Number::Range || matches!(self.settings.dynamic_range, DynamicRange::Fixed(_));
        let value = match field {
            Number::Overlap => &self.overlap,
            Number::Range => &self.range,
        };
        let active = self.active == control && enabled;
        let display = if active {
            format!("{value}▏")
        } else {
            value.clone()
        };
        let value = div()
            .border_b_1()
            .border_dashed()
            .border_color(cx.theme().muted_foreground)
            .child(format!("{display} {suffix}"));
        let field = if enabled {
            let down = if active {
                self.step_button("settings-step-down", "−", field, control, -1.0, cx)
            } else {
                div().size_5().into_any_element()
            };
            let up = if active {
                self.step_button("settings-step-up", "+", field, control, 1.0, cx)
            } else {
                div().size_5().into_any_element()
            };
            div()
                .h_6()
                .w_full()
                .flex()
                .items_center()
                .justify_end()
                .gap_1()
                .child(down)
                .child(
                    div()
                        .id(("settings-number", field as usize))
                        .h_6()
                        .w(px(88.))
                        .px_1()
                        .flex()
                        .items_center()
                        .justify_end()
                        .rounded(cx.theme().radius)
                        .when(active, |value| value.cursor_text())
                        .when(!active, |value| {
                            value
                                .cursor_pointer()
                                .hover(|value| value.bg(cx.theme().secondary_hover))
                        })
                        .child(value)
                        .on_click(cx.listener(move |editor, _, window, cx| {
                            editor.activate(control, window, cx)
                        })),
                )
                .child(up)
                .into_any_element()
        } else {
            div()
                .h_6()
                .w_full()
                .flex()
                .items_center()
                .justify_end()
                .gap_1()
                .child(div().size_5())
                .child(
                    div()
                        .h_6()
                        .w(px(88.))
                        .px_1()
                        .flex()
                        .items_center()
                        .justify_end()
                        .text_color(cx.theme().muted_foreground)
                        .child(format!("{display} {suffix}")),
                )
                .child(div().size_5())
                .into_any_element()
        };
        form_row(label, field, cx).into_any_element()
    }

    fn step_button(
        &self,
        id: &'static str,
        label: &'static str,
        field: Number,
        control: Control,
        delta: f32,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        div()
            .id((id, field as usize))
            .size_5()
            .flex()
            .items_center()
            .justify_center()
            .rounded(cx.theme().radius)
            .cursor_pointer()
            .hover(|button| button.bg(cx.theme().secondary_hover))
            .active(|button| button.bg(cx.theme().secondary_active))
            .child(label)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |editor, _, window, cx| {
                    editor.begin_repeat(field, control, delta, window, cx);
                    cx.stop_propagation();
                }),
            )
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|editor, _, window, cx| editor.finish_repeat(window, cx)),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|editor, _, window, cx| editor.finish_repeat(window, cx)),
            )
            .into_any_element()
    }

    fn footer(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        if let Some(error) = &self.error {
            return div()
                .h_6()
                .px_1()
                .flex()
                .items_center()
                .text_xs()
                .text_color(cx.theme().danger)
                .child(error.clone())
                .into_any_element();
        }
        div()
            .h_6()
            .flex()
            .items_center()
            .justify_end()
            .child(self.reset_button(cx))
            .into_any_element()
    }

    fn reset_button(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("reset-settings")
            .h_6()
            .px_1()
            .flex()
            .items_center()
            .rounded(cx.theme().radius)
            .text_xs()
            .text_color(cx.theme().muted_foreground)
            .cursor_pointer()
            .hover(|row| row.bg(cx.theme().secondary_hover))
            .when(self.active == Control::Reset, |row| {
                row.bg(cx.theme().secondary_hover)
            })
            .child("Reset to defaults")
            .on_click(cx.listener(|editor, _, window, cx| {
                window.focus(&editor.focus);
                editor.active = Control::Reset;
                editor.reset(cx);
            }))
    }
}

impl Focusable for Editor {
    fn focus_handle(&self, _: &gpui::App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for Editor {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let width = px(340.).min(window.viewport_size().width - px(32.));
        let height = window.viewport_size().height - px(48.);
        div()
            .id("analysis-settings-editor")
            .w(width)
            .max_h(height)
            .m_3()
            .py_0p5()
            .px_2()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap_1()
            .font_family(cx.theme().font_family.clone())
            .text_sm()
            .text_color(cx.theme().popover_foreground)
            .bg(cx.theme().popover)
            .border_1()
            .border_color(cx.theme().border)
            .rounded(px(6.))
            .shadow_md()
            .track_focus(&self.focus)
            .on_any_mouse_down(cx.listener(|editor, _, _, cx| editor.pin(cx)))
            .on_hover(cx.listener(|editor, hovered, window, cx| {
                editor.hover(*hovered, window, cx);
            }))
            .key_context(CONTEXT)
            .on_key_down(cx.listener(Self::key_down))
            .on_action(cx.listener(|editor, _: &NextControl, window, cx| {
                editor.move_control(true, window, cx)
            }))
            .on_action(cx.listener(|editor, _: &PreviousControl, window, cx| {
                editor.move_control(false, window, cx)
            }))
            .on_action(cx.listener(|editor, _: &MoveUp, _, cx| editor.move_option(false, cx)))
            .on_action(cx.listener(|editor, _: &MoveDown, _, cx| editor.move_option(true, cx)))
            .on_action(cx.listener(|editor, _: &FirstOption, _, cx| editor.edge_option(false, cx)))
            .on_action(cx.listener(|editor, _: &LastOption, _, cx| editor.edge_option(true, cx)))
            .on_action(
                cx.listener(|editor, _: &ActivateControl, _, cx| editor.activate_current(cx)),
            )
            .on_action(
                cx.listener(|editor, _: &AcceptSettings, window, cx| editor.accept(window, cx)),
            )
            .on_action(
                cx.listener(|editor, _: &CloseSettings, window, cx| editor.cancel(window, cx)),
            )
            .on_action(cx.listener(|editor, _: &EraseCharacter, _, cx| editor.erase(false, cx)))
            .on_action(cx.listener(|editor, _: &ClearNumber, _, cx| editor.erase(true, cx)))
            .on_action(cx.listener(|editor, _: &PasteNumber, _, cx| editor.paste(cx)))
            .child(
                div()
                    .h_7()
                    .px_1()
                    .flex()
                    .items_center()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Analysis settings"),
            )
            .child(self.choice_row("FFT size", Choice::Fft, Control::Fft, window, cx))
            .child(self.choice_row("Window", Choice::Window, Control::Window, window, cx))
            .child(self.number_row("Overlap", Number::Overlap, Control::Overlap, "%", cx))
            .child(self.choice_row(
                "Aggregation",
                Choice::Aggregation,
                Control::Aggregation,
                window,
                cx,
            ))
            .child(div().my_1().border_t_1().border_color(cx.theme().border))
            .child(self.choice_row(
                "Colour scheme",
                Choice::Palette,
                Control::Palette,
                window,
                cx,
            ))
            .child(self.choice_row("Range mode", Choice::Mode, Control::Mode, window, cx))
            .child(self.number_row("Range", Number::Range, Control::Range, "dB", cx))
            .child(div().mt_1().border_t_1().border_color(cx.theme().border))
            .child(self.footer(cx))
    }
}

fn choice_items(choice: Choice) -> Vec<String> {
    match choice {
        Choice::Fft => (1..=crate::settings::MAX_FFT_SIZE.ilog2())
            .map(|power| crate::numbers::number(1usize << power))
            .collect(),
        Choice::Window => argand_dsp::WINDOW_NAMES
            .iter()
            .map(|value| value.to_string())
            .collect(),
        Choice::Aggregation => Aggregation::ALL
            .iter()
            .map(|value| value.label().into())
            .collect(),
        Choice::Palette => argand_core::COLORMAP_NAMES
            .iter()
            .map(|value| value.to_string())
            .collect(),
        Choice::Mode => ["Absolute full scale", "Below measured peak", "Automatic"]
            .map(String::from)
            .to_vec(),
    }
}

fn choice_value(choice: Choice, settings: Settings) -> String {
    match choice {
        Choice::Fft => crate::numbers::number(settings.fft_size),
        Choice::Window => settings.window.to_string(),
        Choice::Aggregation => settings.aggregation.label().into(),
        Choice::Palette => settings.colormap.to_string(),
        Choice::Mode => mode_name(settings.dynamic_range).into(),
    }
}

fn selected_index(choice: Choice, settings: Settings) -> usize {
    let value = choice_value(choice, settings);
    choice_items(choice)
        .iter()
        .position(|item| item == &value)
        .unwrap_or_default()
}

fn mode_name(range: DynamicRange) -> &'static str {
    match range {
        DynamicRange::Default => "Absolute full scale",
        DynamicRange::Fixed(_) => "Below measured peak",
        DynamicRange::Auto => "Automatic",
    }
}

fn form_row(label: &'static str, control: impl IntoElement, cx: &gpui::App) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap_4()
        .child(
            div()
                .w(px(104.))
                .flex_shrink_0()
                .text_color(cx.theme().muted_foreground)
                .child(label),
        )
        .child(div().flex_1().min_w_0().child(control))
}
