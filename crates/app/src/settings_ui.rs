//! Status-bar file details and live analysis controls.

use super::*;
use argand_dsp::DynamicRange;
use gpui_component::button::ButtonCustomVariant;
use std::{cell::Cell, rc::Rc};
#[path = "settings_editor.rs"]
mod editor;

pub(super) type Anchor = Rc<Cell<Bounds<Pixels>>>;

const POPUP_WIDTH: Pixels = px(440.);
const POPUP_HEIGHT: Pixels = px(540.);
const POPUP_MARGIN: Pixels = px(8.);

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
        self.apply_settings(settings, cx);
    }

    fn apply_settings(&mut self, settings: Settings, cx: &mut Context<Self>) {
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
        if self.settings_backup.is_none() {
            self.session.analysis_settings = Some(settings);
            self.save();
        }
        self.ask_for_a_picture();
        cx.notify();
    }

    fn cancel_settings_popup(&mut self, id: gpui::WindowId, cx: &mut Context<Self>) {
        if self
            .settings_popup
            .is_some_and(|handle| handle.window_id() == id)
        {
            self.settings_popup = None;
            self.finish_settings(false, cx);
        }
    }

    fn attach_settings_popup(&mut self, handle: gpui::WindowHandle<gpui_component::Root>) -> bool {
        let keep = self.settings_backup.is_some();
        self.settings_popup = keep.then_some(handle);
        keep
    }

    pub(super) fn finish_settings(&mut self, accept: bool, cx: &mut Context<Self>) {
        let Some(backup) = self.settings_backup.take() else {
            return;
        };
        self.analysis_hovered = false;
        let view = self.settings_view_backup.take();
        let frequency = self.settings_frequency_backup.take();
        if !accept {
            self.view = view;
            if let Some(frequency) = frequency {
                self.frequency = frequency;
                self.frequency_scheme = None;
            }
        }
        self.apply_settings(if accept { self.settings } else { backup }, cx);
    }

    pub(super) fn use_recommended_range(&mut self, cx: &mut Context<Self>) {
        if let Some(db) = self.range_recommendation() {
            self.set_settings(
                Settings {
                    dynamic_range: DynamicRange::Fixed(db),
                    ..self.settings
                },
                cx,
            );
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
        let foreground = if self.analysis_hovered {
            cx.theme().foreground
        } else {
            cx.theme().muted_foreground
        };
        let anchor = self.settings_anchor.clone();
        let trigger = Button::new("analysis-settings")
            .custom(
                ButtonCustomVariant::new(cx)
                    .hover(cx.theme().secondary_hover)
                    .active(cx.theme().secondary_hover),
            )
            .small()
            .h_5()
            .px_2()
            .when(self.analysis_hovered, |button| {
                button.bg(cx.theme().secondary_hover)
            })
            .on_hover(cx.listener(|shell, hovered, _, cx| {
                shell.analysis_hovered = *hovered;
                cx.notify();
            }))
            .child(
                div()
                    .flex()
                    .gap_1()
                    .text_xs()
                    .text_color(foreground)
                    .child(format!(
                        "{} · {} ·",
                        crate::numbers::number(displayed.fft_size),
                        displayed.window
                    ))
                    .child(
                        div()
                            .id("analysis-range")
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
                            }),
                    ),
            )
            .on_click(
                cx.listener(|shell, _, window, cx| shell.edit_analysis(&EditAnalysis, window, cx)),
            )
            .child(
                canvas(move |bounds, _, _| anchor.set(bounds), |_, _, _, _| {})
                    .absolute()
                    .size_full(),
            );
        div()
            .id("analysis-summary")
            .border_l_1()
            .border_color(cx.theme().border)
            .child(trigger)
    }

    pub(super) fn edit_analysis(
        &mut self,
        _: &EditAnalysis,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.dismiss_application_menu(window, cx);
        if self.settings_popup.is_some_and(|handle| {
            handle
                .update(cx, |_, window, _| window.activate_window())
                .is_ok()
        }) {
            return;
        }
        if self.settings_backup.is_some() {
            return;
        }
        self.settings_backup = Some(self.settings);
        self.settings_view_backup = self.view;
        self.settings_frequency_backup = Some(self.frequency);
        self.analysis_hovered = false;
        cx.notify();
        let owner = cx.entity().downgrade();
        let settings = self.settings;
        let range = self
            .displayed_range()
            .map_or(110.0, |range| range.effective_db);
        let parent = window.bounds();
        let viewport = window.viewport_size();
        let trigger = self.settings_anchor.get();
        let popup_size = size(
            POPUP_WIDTH.min(viewport.width - POPUP_MARGIN * 2.),
            POPUP_HEIGHT.min(viewport.height - POPUP_MARGIN * 2.),
        );
        let available_x = (viewport.width - popup_size.width - POPUP_MARGIN).max(POPUP_MARGIN);
        let x = trigger.origin.x.clamp(POPUP_MARGIN, available_x);
        let y = (trigger.origin.y - popup_size.height - POPUP_MARGIN).max(POPUP_MARGIN);
        let bounds = Bounds::new(parent.origin + point(x, y), popup_size);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: None,
            kind: if cfg!(target_os = "windows") {
                WindowKind::PopUp
            } else {
                WindowKind::Floating
            },
            is_movable: false,
            is_resizable: false,
            is_minimizable: false,
            display_id: window.display(cx).map(|display| display.id()),
            window_min_size: Some(popup_size),
            window_decorations: Some(WindowDecorations::Client),
            app_id: Some(APP_ID.into()),
            ..Default::default()
        };
        cx.defer(move |cx| {
            if owner
                .upgrade()
                .is_none_or(|shell| shell.read(cx).settings_backup.is_none())
            {
                return;
            }
            let form_owner = owner.clone();
            let opened = cx.open_window(options, move |window, cx| {
                let form =
                    cx.new(|cx| editor::Editor::new(form_owner, settings, range, window, cx));
                cx.new(|cx| gpui_component::Root::new(form, window, cx))
            });
            match opened {
                Ok(handle) => {
                    let keep = owner
                        .update(cx, |shell, _| shell.attach_settings_popup(handle))
                        .unwrap_or(false);
                    if !keep {
                        let _ = handle.update(cx, |_, window, _| window.remove_window());
                    }
                }
                Err(error) => {
                    let _ = owner.update(cx, |shell, cx| shell.finish_settings(false, cx));
                    tracing::error!(%error, "cannot open analysis settings popup");
                }
            }
        });
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
