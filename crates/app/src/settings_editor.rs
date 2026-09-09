//! A separate toolkit root gives native inputs their focus and editing context.

use super::*;
use gpui::{Entity, Focusable};
use gpui_component::input::{InputEvent, InputState, NumberInput, NumberInputEvent, StepAction};
use gpui_component::select::{Select, SelectEvent, SelectState};

actions!(settings_editor, [CloseSettings]);

pub(super) fn init(cx: &mut gpui::App) {
    cx.bind_keys([KeyBinding::new(
        "escape",
        CloseSettings,
        Some("AnalysisEditor"),
    )]);
}

#[derive(Clone, Copy)]
enum Choice {
    Fft,
    Window,
    Aggregation,
    Mode,
    Palette,
}
#[derive(Clone, Copy)]
enum Number {
    Overlap,
    Range,
}

type Combo = Entity<SelectState<Vec<String>>>;

pub(super) struct Editor {
    owner: WeakEntity<Shell>,
    settings: Settings,
    fft: Combo,
    window: Combo,
    aggregation: Combo,
    mode: Combo,
    palette: Combo,
    overlap: Entity<InputState>,
    range: Entity<InputState>,
    error: Option<String>,
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
        let fft = combo(Choice::Fft, settings, window, cx, &mut subscriptions);
        let window_choice = combo(Choice::Window, settings, window, cx, &mut subscriptions);
        let aggregation = combo(
            Choice::Aggregation,
            settings,
            window,
            cx,
            &mut subscriptions,
        );
        let mode = combo(Choice::Mode, settings, window, cx, &mut subscriptions);
        let palette = combo(Choice::Palette, settings, window, cx, &mut subscriptions);
        let overlap = number(
            Number::Overlap,
            settings.overlap.to_string(),
            window,
            cx,
            &mut subscriptions,
        );
        let db = match settings.dynamic_range {
            DynamicRange::Fixed(db) => db,
            _ => effective_range,
        };
        let range = number(
            Number::Range,
            db.to_string(),
            window,
            cx,
            &mut subscriptions,
        );
        if let Some(shell) = owner.upgrade() {
            subscriptions.push(cx.observe_in(&shell, window, |editor, shell, window, cx| {
                let settings = shell.read(cx).settings;
                if editor.settings != settings {
                    editor.sync(settings, window, cx);
                } else {
                    editor.update_effective_range(shell.read(cx).displayed_range(), window, cx);
                }
                cx.notify();
            }));
        }
        if let Some(shell) = owner.upgrade() {
            subscriptions.push(
                cx.observe_release_in(&shell, window, |_, _, window, _| window.remove_window()),
            );
        }
        let closing_owner = owner.clone();
        let closing_id = window.window_handle().window_id();
        cx.on_release(move |_, cx| {
            cx.defer(move |cx| {
                let _ = closing_owner.update(cx, |shell, cx| {
                    shell.cancel_settings_window(closing_id, cx);
                });
            });
        })
        .detach();
        fft.focus_handle(cx).focus(window);
        Self {
            owner,
            settings,
            fft,
            window: window_choice,
            aggregation,
            mode,
            palette,
            overlap,
            range,
            error: None,
            _subscriptions: subscriptions,
        }
    }

    fn close(&mut self, accept: bool, window: &mut Window, cx: &mut Context<Self>) {
        if accept && !self.commit_numbers(window, cx) {
            return;
        }
        let _ = self
            .owner
            .update(cx, |shell, cx| shell.finish_settings(accept, cx));
        window.remove_window();
    }

    fn commit_numbers(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let overlap = self.overlap.read(cx).value();
        let range = self.range.read(cx).value();
        let fixed = matches!(self.settings.dynamic_range, DynamicRange::Fixed(_));
        match self
            .settings
            .edited_numbers(&overlap, fixed.then_some(range.as_ref()))
        {
            Ok(settings) => self.apply(settings, window, cx),
            Err(error) => {
                self.error = Some(error);
                cx.notify();
            }
        }
        self.error.is_none()
    }

    fn recommend(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let db = self
            .owner
            .upgrade()
            .and_then(|shell| shell.read(cx).range_recommendation());
        if let Some(db) = db {
            self.apply(
                Settings {
                    dynamic_range: DynamicRange::Fixed(db),
                    ..self.settings
                },
                window,
                cx,
            );
            self.range.focus_handle(cx).focus(window);
        }
    }

    fn reset(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(owner) = self.owner.upgrade() else {
            return;
        };
        self.apply(Settings::from_config(&owner.read(cx).config), window, cx);
    }

    fn update_effective_range(
        &mut self,
        range: Option<argand_dsp::DynamicRangeResult>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if matches!(self.settings.dynamic_range, DynamicRange::Fixed(_)) {
            return;
        }
        let Some(range) = range else {
            return;
        };
        let value = range.effective_db.to_string();
        if self.range.read(cx).value().as_ref() != value {
            self.range
                .update(cx, |s, cx| s.set_value(value, window, cx));
        }
    }

    fn sync(&mut self, settings: Settings, window: &mut Window, cx: &mut Context<Self>) {
        self.settings = settings;
        self.error = None;
        self.sync_choices(window, cx);
        self.overlap.update(cx, |s, cx| {
            s.set_value(settings.overlap.to_string(), window, cx)
        });
        let db = match settings.dynamic_range {
            DynamicRange::Fixed(db) => db,
            _ => self
                .owner
                .upgrade()
                .and_then(|s| s.read(cx).displayed_range())
                .map_or(110.0, |r| r.effective_db),
        };
        self.range
            .update(cx, |s, cx| s.set_value(db.to_string(), window, cx));
    }

    fn sync_choices(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let settings = self.settings;
        for (state, value) in [
            (&self.fft, settings.fft_size.to_string()),
            (&self.window, settings.window.to_string()),
            (&self.aggregation, settings.aggregation.label().into()),
            (&self.mode, mode_name(settings.dynamic_range).into()),
            (&self.palette, settings.colormap.to_string()),
        ] {
            state.update(cx, |state, cx| state.set_selected_value(&value, window, cx));
        }
    }

    fn apply(&mut self, settings: Settings, window: &mut Window, cx: &mut Context<Self>) {
        if settings == self.settings {
            self.error = None;
            cx.notify();
            return;
        }
        let result = self.owner.update(cx, |shell, cx| {
            shell.set_settings(settings, cx);
            shell.settings_error.clone()
        });
        self.error = result.unwrap_or_else(|_| Some("The signal window is closed".into()));
        if self.error.is_none() {
            self.sync(settings, window, cx);
        }
        cx.notify();
    }

    fn choose(&mut self, choice: Choice, value: &str, window: &mut Window, cx: &mut Context<Self>) {
        let mut settings = self.settings;
        match choice {
            Choice::Fft => {
                if let Ok(value) = value.parse() {
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
                }
            }
            Choice::Mode => {
                settings.dynamic_range = match value {
                    "Absolute full scale" => DynamicRange::Default,
                    "Automatic" => DynamicRange::Auto,
                    _ => DynamicRange::Fixed(self.range.read(cx).value().parse().unwrap_or(110.0)),
                }
            }
            Choice::Palette => {
                if let Ok(value) = value.parse() {
                    settings.colormap = value;
                }
            }
        }
        self.apply(settings, window, cx);
        if self.error.is_some() {
            self.sync_choices(window, cx);
        }
    }

    fn edit_number(
        &mut self,
        field: Number,
        step: Option<StepAction>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if matches!(field, Number::Range)
            && !matches!(self.settings.dynamic_range, DynamicRange::Fixed(_))
        {
            return;
        }
        let input = match field {
            Number::Overlap => &self.overlap,
            Number::Range => &self.range,
        };
        let parsed = input.read(cx).value().parse::<f32>();
        let Ok(mut value) = parsed else {
            self.error = Some("Enter a number".into());
            cx.notify();
            return;
        };
        if let Some(step) = step {
            value += if step == StepAction::Increment {
                1.0
            } else {
                -1.0
            };
        }
        let mut settings = self.settings;
        match field {
            Number::Overlap
                if value.is_finite() && (0.0..=95.0).contains(&value) && value.fract() == 0.0 =>
            {
                settings.overlap = value as u8
            }
            Number::Overlap => {
                self.error = Some("Overlap must be a whole number from 0 to 95%".into());
                cx.notify();
                return;
            }
            Number::Range => settings.dynamic_range = DynamicRange::Fixed(value),
        }
        self.apply(settings, window, cx);
    }

    fn transform_rows(&self, cx: &gpui::App) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(section("Transform", cx))
            .child(form_row("FFT size", Select::new(&self.fft), cx))
            .child(form_row("Window", Select::new(&self.window), cx))
            .child(form_row(
                "Overlap",
                NumberInput::new(&self.overlap).suffix("%"),
                cx,
            ))
            .child(form_row("Aggregation", Select::new(&self.aggregation), cx))
    }

    fn footer_status(
        &self,
        pending: bool,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        if let Some(error) = &self.error {
            return div()
                .flex_1()
                .text_xs()
                .text_color(cx.theme().danger)
                .child(error.clone())
                .into_any_element();
        }
        if let Some(db) = self
            .owner
            .upgrade()
            .and_then(|shell| shell.read(cx).range_recommendation())
        {
            return Button::new("use-recommendation")
                .outline()
                .small()
                .label(format!("Use recommended: {db} dB"))
                .when_some(
                    Kbd::binding_for_action(&UseRecommendedRange, None, window),
                    |button, kbd| button.child(kbd),
                )
                .on_click(cx.listener(|editor, _, window, cx| editor.recommend(window, cx)))
                .into_any_element();
        }
        div()
            .text_xs()
            .text_color(cx.theme().muted_foreground)
            .child(if pending {
                "Updating the picture…"
            } else {
                "Changes are previewed until OK"
            })
            .into_any_element()
    }

    fn display_rows(&self, cx: &gpui::App) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(section("Display", cx))
            .child(form_row("Colour scheme", Select::new(&self.palette), cx))
            .child(form_row("Range mode", Select::new(&self.mode), cx))
            .child(form_row(
                "Range",
                if matches!(self.settings.dynamic_range, DynamicRange::Fixed(_)) {
                    NumberInput::new(&self.range)
                        .suffix("dB")
                        .into_any_element()
                } else {
                    div()
                        .h_8()
                        .px_3()
                        .flex()
                        .items_center()
                        .justify_between()
                        .rounded(cx.theme().radius)
                        .bg(cx.theme().muted)
                        .text_color(cx.theme().muted_foreground)
                        .child(self.range.read(cx).value())
                        .child("dB")
                        .into_any_element()
                },
                cx,
            ))
    }
}

impl Render for Editor {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let pending = self.owner.upgrade().is_some_and(|shell| {
            let shell = shell.read(cx);
            shell.file.as_ref().is_some_and(|f| {
                f.displayed_settings != Some(shell.settings)
                    && !matches!(f.document.status(), Status::Failed(_))
            })
        });
        div()
            .size_full()
            .flex()
            .flex_col()
            .key_context("AnalysisEditor")
            .on_action(cx.listener(|editor, _: &UseRecommendedRange, window, cx| {
                editor.recommend(window, cx)
            }))
            .on_action(
                cx.listener(|editor, _: &CloseSettings, window, cx| {
                    editor.close(false, window, cx)
                }),
            )
            .child(TitleBar::new().child(div().text_sm().child("Analysis settings")))
            .child(
                div()
                    .id("settings-form")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .p_5()
                    .flex()
                    .flex_col()
                    .gap_5()
                    .text_sm()
                    .child(self.transform_rows(cx))
                    .child(self.display_rows(cx)),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .px_5()
                    .py_3()
                    .border_t_1()
                    .border_color(cx.theme().border)
                    .child(self.footer_status(pending, window, cx))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                Button::new("reset-settings")
                                    .outline()
                                    .label("Reset to defaults")
                                    .on_click(cx.listener(|editor, _, window, cx| {
                                        editor.reset(window, cx)
                                    })),
                            )
                            .child(div().flex_1())
                            .child(
                                Button::new("cancel-settings")
                                    .outline()
                                    .label("Cancel")
                                    .on_click(cx.listener(|editor, _, window, cx| {
                                        editor.close(false, window, cx)
                                    })),
                            )
                            .child(
                                Button::new("accept-settings")
                                    .primary()
                                    .label("OK")
                                    .on_click(cx.listener(|editor, _, window, cx| {
                                        editor.close(true, window, cx)
                                    })),
                            ),
                    ),
            )
    }
}

fn mode_name(range: DynamicRange) -> &'static str {
    match range {
        DynamicRange::Default => "Absolute full scale",
        DynamicRange::Fixed(_) => "Below measured peak",
        DynamicRange::Auto => "Automatic",
    }
}

fn section(title: &'static str, cx: &gpui::App) -> impl IntoElement {
    div()
        .text_xs()
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(cx.theme().muted_foreground)
        .child(title)
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

fn combo(
    choice: Choice,
    settings: Settings,
    window: &mut Window,
    cx: &mut Context<Editor>,
    subscriptions: &mut Vec<Subscription>,
) -> Combo {
    let (selected, items): (String, Vec<String>) = match choice {
        Choice::Fft => (
            settings.fft_size.to_string(),
            (1..=crate::settings::MAX_FFT_SIZE.ilog2())
                .map(|n| (1usize << n).to_string())
                .collect(),
        ),
        Choice::Window => (
            settings.window.to_string(),
            argand_dsp::WINDOW_NAMES
                .iter()
                .map(|s| s.to_string())
                .collect(),
        ),
        Choice::Aggregation => (
            settings.aggregation.label().into(),
            Aggregation::ALL.iter().map(|s| s.label().into()).collect(),
        ),
        Choice::Mode => (
            mode_name(settings.dynamic_range).into(),
            ["Absolute full scale", "Below measured peak", "Automatic"]
                .map(String::from)
                .to_vec(),
        ),
        Choice::Palette => (
            settings.colormap.to_string(),
            argand_core::COLORMAP_NAMES
                .iter()
                .map(|s| s.to_string())
                .collect(),
        ),
    };
    let index = items
        .iter()
        .position(|value| value == &selected)
        .map(gpui_component::IndexPath::new);
    let state = cx.new(|cx| SelectState::new(items, index, window, cx));
    subscriptions.push(
        cx.subscribe_in(&state, window, move |editor, _, event, window, cx| {
            if let SelectEvent::Confirm(Some(value)) = event {
                editor.choose(choice, value, window, cx);
            }
        }),
    );
    state
}

fn number(
    field: Number,
    value: String,
    window: &mut Window,
    cx: &mut Context<Editor>,
    subscriptions: &mut Vec<Subscription>,
) -> Entity<InputState> {
    let state = cx.new(|cx| InputState::new(window, cx).default_value(value));
    subscriptions.push(
        cx.subscribe_in(&state, window, move |editor, _, event, window, cx| {
            if matches!(event, InputEvent::PressEnter { .. } | InputEvent::Blur) {
                editor.edit_number(field, None, window, cx);
            }
        }),
    );
    subscriptions.push(cx.subscribe_in(
        &state,
        window,
        move |editor, _, event: &NumberInputEvent, window, cx| {
            let NumberInputEvent::Step(step) = event;
            editor.edit_number(field, Some(*step), window, cx);
        },
    ));
    state
}
