//! Status-bar file details and live analysis controls.

use super::*;
use argand_dsp::DynamicRange;
use gpui::{Corner, Focusable};
use gpui_component::button::ButtonCustomVariant;
use gpui_component::popover::Popover;
use std::time::Duration;
#[path = "settings_editor.rs"]
mod editor;
pub(super) use editor::Editor;

pub(super) fn init(cx: &mut gpui::App) {
    cx.bind_keys([KeyBinding::new(
        if cfg!(target_os = "macos") {
            "cmd-,"
        } else {
            "ctrl-,"
        },
        EditAnalysis,
        None,
    )]);
    cx.bind_keys([KeyBinding::new(
        if cfg!(target_os = "macos") {
            "cmd-r"
        } else {
            "ctrl-r"
        },
        UseRecommendedRange,
        None,
    )]);
    editor::init(cx);
}

impl Shell {
    fn set_settings(&mut self, settings: Settings, cx: &mut Context<Self>) {
        self.update_settings(settings, true, cx);
    }

    fn preview_settings(&mut self, settings: Settings, cx: &mut Context<Self>) {
        self.update_settings(settings, false, cx);
    }

    fn restore_settings(&mut self, settings: Settings, cx: &mut Context<Self>) {
        self.apply_settings(settings, false, cx);
    }

    fn focus_shell(&self, window: &mut Window) {
        window.focus(&self.focus);
    }

    fn update_settings(&mut self, settings: Settings, persist: bool, cx: &mut Context<Self>) {
        let samples = self
            .file
            .as_ref()
            .and_then(|file| file.document.meta())
            .map(|meta| meta.len_samples);
        if let Err(error) = settings.validate(samples) {
            self.settings_error = Some(error);
            cx.notify();
            return;
        }
        self.apply_settings(settings, persist, cx);
    }

    fn apply_settings(&mut self, settings: Settings, persist: bool, cx: &mut Context<Self>) {
        self.settings_error = None;
        tracing::debug!(?settings, "analysis settings requested");
        if let Some(file) = &mut self.file
            && file
                .displayed_settings
                .is_some_and(|displayed| displayed.equivalent(settings))
        {
            file.displayed_settings = Some(settings);
        }
        self.settings = settings;
        self.bound_view();
        if persist {
            self.session.analysis_settings = Some(settings);
            self.save();
        }
        self.ask_for_a_picture();
        cx.notify();
    }

    fn accept_settings(&mut self, cx: &mut Context<Self>) {
        self.session.analysis_settings = Some(self.settings);
        self.save();
        self.close_settings(cx);
    }

    fn pin_settings(&mut self, editor: Entity<Editor>, cx: &mut Context<Self>) {
        let current = self
            .settings_popup
            .as_ref()
            .is_some_and(|current| current.entity_id() == editor.entity_id());
        if !current {
            self.settings_popup = Some(editor);
        }
        self.settings_pinned = true;
        self.settings_hover_generation = self.settings_hover_generation.wrapping_add(1);
        cx.notify();
    }

    fn pin_hovered_settings(
        &mut self,
        editor: Entity<Editor>,
        cx: &mut Context<Self>,
    ) -> Option<FocusHandle> {
        if !self.settings_trigger_hovered {
            return None;
        }
        self.pin_settings(editor.clone(), cx);
        Some(editor.read(cx).focus_handle(cx))
    }

    pub(super) fn close_settings(&mut self, cx: &mut Context<Self>) {
        self.settings_popup = None;
        self.settings_pinned = false;
        self.settings_trigger_hovered = false;
        self.settings_surface_hovered = false;
        self.settings_hover_generation = self.settings_hover_generation.wrapping_add(1);
        self.settings_error = None;
        cx.notify();
    }

    pub(super) fn cancel_settings_preview(&mut self, cx: &mut Context<Self>) {
        let original = self
            .settings_popup
            .as_ref()
            .map(|editor| editor.read(cx).original_settings());
        if let Some(original) = original.filter(|original| *original != self.settings) {
            self.restore_settings(original, cx);
        }
        self.close_settings(cx);
    }

    pub(super) const fn settings_are_pinned(&self) -> bool {
        self.settings_pinned
    }

    fn hover_settings_trigger(
        &mut self,
        hovered: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.settings_trigger_hovered = hovered;
        self.settings_hover_generation = self.settings_hover_generation.wrapping_add(1);
        if hovered {
            self.start_settings_popup(false, window, cx);
        } else {
            self.schedule_hover_close(window, cx);
        }
    }

    fn hover_settings_surface(
        &mut self,
        hovered: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.settings_surface_hovered = hovered;
        self.settings_hover_generation = self.settings_hover_generation.wrapping_add(1);
        if !hovered {
            self.schedule_hover_close(window, cx);
        }
    }

    fn schedule_hover_close(&mut self, window: &Window, cx: &mut Context<Self>) {
        if self.settings_pinned || self.settings_trigger_hovered || self.settings_surface_hovered {
            return;
        }
        let generation = self.settings_hover_generation;
        let executor = cx.background_executor().clone();
        cx.spawn_in(window, async move |shell, cx| {
            executor.timer(Duration::from_millis(180)).await;
            let _ = shell.update_in(cx, |shell, _, cx| {
                if shell.hover_close_ready(generation, cx) {
                    shell.close_settings(cx);
                }
            });
        })
        .detach();
    }

    fn hover_close_ready(&self, generation: u64, cx: &gpui::App) -> bool {
        let repeating = self
            .settings_popup
            .as_ref()
            .is_some_and(|editor| editor.read(cx).is_repeating());
        !self.settings_pinned
            && !self.settings_trigger_hovered
            && !self.settings_surface_hovered
            && self.settings_hover_generation == generation
            && !repeating
    }

    pub(super) fn use_recommended_range(&mut self, cx: &mut Context<Self>) {
        if let Some(db) = self.range_recommendation() {
            let settings = Settings {
                dynamic_range: DynamicRange::Fixed(db),
                ..self.settings
            };
            if self.settings_pinned {
                self.preview_settings(settings, cx);
            } else {
                self.set_settings(settings, cx);
                if let Some(editor) = &self.settings_popup {
                    editor.update(cx, |editor, _| editor.rebase(settings));
                }
            }
        }
    }

    fn displayed_range(&self) -> Option<argand_dsp::DynamicRangeResult> {
        self.file
            .as_ref()?
            .document
            .analysis()
            .map(|analysis| analysis.dynamic_range)
    }

    fn range_recommendation(&self) -> Option<f32> {
        let file = self.file.as_ref()?;
        if matches!(file.document.status(), Status::Failed(_))
            || file.displayed_settings != Some(self.settings)
        {
            return None;
        }
        file.document.range_recommendation()
    }

    fn displayed_range_recommendation(&self) -> Option<f32> {
        let file = self.file.as_ref()?;
        if matches!(file.document.status(), Status::Failed(_)) {
            return None;
        }
        file.document.range_recommendation()
    }

    pub(super) fn status_bar(
        &self,
        corners: Corners<Pixels>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let file = self
            .file
            .as_ref()
            .and_then(|file| file.document.file_summary());
        let status = self
            .file
            .as_ref()
            .map_or_else(|| "ready".into(), |file| file.document.status().message());
        let hint = self
            .file
            .as_ref()
            .and_then(|file| file.document.status().hint());
        div()
            .flex()
            .items_center()
            .gap_2()
            .px_2()
            .h_6()
            .flex_shrink_0()
            .rounded_bl(corners.bottom_left)
            .rounded_br(corners.bottom_right)
            .border_t_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().secondary)
            .text_xs()
            .text_color(cx.theme().muted_foreground)
            .when_some(file, |bar, field| {
                bar.child(
                    div()
                        .id("file-summary")
                        .px_2()
                        .min_w_0()
                        .flex_shrink()
                        .tooltip(move |window, cx| {
                            metadata_tooltip(field.hint.clone()).build(window, cx)
                        })
                        .child(
                            div()
                                .overflow_hidden()
                                .text_ellipsis()
                                .whitespace_nowrap()
                                .child(field.value),
                        ),
                )
            })
            .child(self.analysis_control(cx))
            .child(div().flex_1().min_w_0())
            .when_some(self.cursor_readout(), |bar, text| {
                bar.child(
                    div()
                        .id("cursor-readout")
                        .min_w_0()
                        .overflow_hidden()
                        .text_ellipsis()
                        .whitespace_nowrap()
                        .child(text),
                )
            })
            .child(
                div()
                    .id("analysis-status")
                    .min_w_0()
                    .max_w(px(140.))
                    .when_some(hint, |status, hint| {
                        status.tooltip(move |window, cx| {
                            metadata_tooltip(hint.clone()).build(window, cx)
                        })
                    })
                    .child(
                        div()
                            .overflow_hidden()
                            .text_ellipsis()
                            .whitespace_nowrap()
                            .child(status),
                    ),
            )
    }

    fn analysis_control(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let displayed = self
            .file
            .as_ref()
            .and_then(|file| file.displayed_settings)
            .unwrap_or(self.settings);
        let range = self
            .displayed_range()
            .map(|range| format!("{} dB", crate::numbers::number(range.effective_db)))
            .unwrap_or_else(|| match self.settings.dynamic_range {
                DynamicRange::Default => crate::numbers::text("110 dB"),
                DynamicRange::Fixed(db) => format!("{} dB", crate::numbers::number(db)),
                DynamicRange::Auto => "auto".into(),
            });
        let warning = self.displayed_range_recommendation().is_some();
        let transform = div()
            .id("analysis-transform")
            .on_hover(cx.listener(|shell, hovered, window, cx| {
                shell.hover_settings_trigger(*hovered, window, cx);
            }))
            .child(format!(
                "{} · {} ·",
                crate::numbers::number(displayed.fft_size),
                displayed.window
            ));
        let range = div()
            .id("analysis-range")
            .when(!warning, |range| {
                range.on_hover(cx.listener(|shell, hovered, window, cx| {
                    shell.hover_settings_trigger(*hovered, window, cx);
                }))
            })
            .when(warning, |range| {
                range
                    .text_color(advice_color(cx))
                    .cursor_pointer()
                    .on_mouse_down(MouseButton::Left, |_, window, cx| {
                        window.prevent_default();
                        cx.stop_propagation();
                    })
                    .on_click(|_, window, cx| {
                        cx.stop_propagation();
                        window.dispatch_action(Box::new(UseRecommendedRange), cx);
                    })
            })
            .child(if warning {
                format!("⚠ {range}")
            } else {
                range
            });
        let content = div()
            .id("analysis-settings-hint-trigger")
            .h_5()
            .px_2()
            .flex()
            .items_center()
            .gap_1()
            .text_xs()
            .text_color(cx.theme().muted_foreground)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|shell, _, window, cx| {
                    window.prevent_default();
                    cx.stop_propagation();
                    if let Some(focus) = shell.start_settings_popup(true, window, cx) {
                        window.focus(&focus);
                    }
                }),
            )
            .on_click(|_, _, cx| cx.stop_propagation())
            .child(transform)
            .child(range);
        let trigger = Button::new("analysis-settings")
            .custom(
                ButtonCustomVariant::new(cx)
                    .hover(cx.theme().secondary)
                    .active(cx.theme().secondary),
            )
            .small()
            .h_5()
            .px_0()
            .child(content);
        div()
            .id("analysis-summary")
            .border_l_1()
            .border_color(cx.theme().border)
            .child(self.settings_popover(trigger, cx))
    }

    fn settings_popover<T>(&self, trigger: T, cx: &mut Context<Self>) -> impl IntoElement
    where
        T: gpui_component::Selectable + IntoElement + 'static,
    {
        let owner = cx.entity().downgrade();
        let editor = self.settings_popup.clone();
        let dismiss_editor = editor.clone();
        let focus = editor
            .as_ref()
            .map(|editor| editor.read(cx).focus_handle(cx));
        Popover::new("analysis-settings-popover")
            .anchor(Corner::BottomLeft)
            .appearance(false)
            .open(editor.is_some())
            .trigger(trigger)
            .on_open_change(move |open, window, cx| {
                if *open {
                    let focus = owner
                        .update(cx, |shell, cx| shell.start_settings_popup(true, window, cx))
                        .ok()
                        .flatten();
                    if let Some(focus) = focus {
                        window.focus(&focus);
                    }
                } else if let Some(editor) = &dismiss_editor {
                    let focus = owner
                        .update(cx, |shell, cx| {
                            shell.pin_hovered_settings(editor.clone(), cx)
                        })
                        .ok()
                        .flatten();
                    if let Some(focus) = focus {
                        window.focus(&focus);
                    } else {
                        editor.update(cx, |editor, cx| editor.dismiss(cx));
                    }
                } else {
                    let _ = owner.update(cx, |shell, cx| shell.close_settings(cx));
                }
            })
            .when_some(focus, |popup, focus| popup.track_focus(&focus))
            .content(move |_, _, _| {
                editor.clone().map_or_else(
                    || div().into_any_element(),
                    |editor| editor.into_any_element(),
                )
            })
    }

    fn start_settings_popup(
        &mut self,
        pinned: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<FocusHandle> {
        if let Some(editor) = &self.settings_popup {
            if pinned {
                self.settings_pinned = true;
                self.settings_hover_generation = self.settings_hover_generation.wrapping_add(1);
            }
            return Some(editor.read(cx).focus_handle(cx));
        }
        let owner = cx.entity().downgrade();
        let settings = self.settings;
        let range = self
            .displayed_range()
            .map_or(110.0, |range| range.effective_db);
        let editor = cx.new(|cx| Editor::new(owner, settings, range, window, cx));
        let focus = editor.read(cx).focus_handle(cx);
        self.settings_popup = Some(editor);
        self.settings_pinned = pinned;
        cx.notify();
        Some(focus)
    }

    pub(super) fn edit_analysis(
        &mut self,
        _: &EditAnalysis,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.dismiss_application_menu(window, cx);
        if let Some(focus) = self.start_settings_popup(true, window, cx) {
            window.focus(&focus);
        }
    }
}

pub(super) fn detail_row(
    label: impl Into<gpui::SharedString>,
    value: impl Into<gpui::SharedString>,
    cx: &gpui::App,
) -> impl IntoElement {
    div()
        .flex()
        .items_start()
        .gap_4()
        .w_full()
        .child(
            div()
                .w(px(100.))
                .flex_shrink_0()
                .text_color(cx.theme().muted_foreground)
                .child(label.into()),
        )
        .child(div().flex_1().min_w_0().text_right().child(value.into()))
}

fn advice_color(cx: &gpui::App) -> gpui::Hsla {
    if cx.theme().is_dark() {
        gpui::rgb(0xfacc15).into()
    } else {
        gpui::rgb(0x946200).into()
    }
}
