//! Status-bar file details and live analysis controls.

use super::*;
use argand_dsp::DynamicRange;
#[path = "settings_editor.rs"]
mod editor;

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
        self.session.analysis_settings = Some(settings);
        self.save();
        self.ask_for_a_picture();
        cx.notify();
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
            .map(|range| format!("{} dB", range.effective_db))
            .unwrap_or_else(|| match self.settings.dynamic_range {
                DynamicRange::Default => "110 dB".into(),
                DynamicRange::Fixed(db) => format!("{db} dB"),
                DynamicRange::Auto => "auto".into(),
            });
        let hint_owner = cx.entity().downgrade();
        let warning = self.range_recommendation().is_some();
        let foreground = if self.analysis_hovered {
            cx.theme().foreground
        } else {
            cx.theme().muted_foreground
        };
        div()
            .id("analysis-summary")
            .border_l_1()
            .border_color(cx.theme().border)
            .hoverable_tooltip(move |window, cx| {
                analysis_tooltip(hint_owner.clone()).build(window, cx)
            })
            .child(
                Button::new("analysis-settings")
                    .ghost()
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
                    .on_click(cx.listener(|shell, _, window, cx| {
                        shell.edit_analysis(&EditAnalysis, window, cx)
                    }))
                    .child(
                        div()
                            .flex()
                            .gap_1()
                            .text_xs()
                            .text_color(foreground)
                            .child(format!("{} · {} ·", displayed.fft_size, displayed.window))
                            .child(
                                div()
                                    .when(warning, |s| s.text_color(advice_color(cx)))
                                    .child(range),
                            ),
                    ),
            )
    }

    pub(super) fn edit_analysis(
        &mut self,
        _: &EditAnalysis,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.settings_window.is_some_and(|handle| {
            handle
                .update(cx, |_, window, _| window.activate_window())
                .is_ok()
        }) {
            return;
        }
        let owner = cx.entity().downgrade();
        let settings = self.settings;
        let range = self
            .displayed_range()
            .map_or(110.0, |range| range.effective_db);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                None,
                size(px(440.), px(520.)),
                cx,
            ))),
            window_min_size: Some(size(px(400.), px(480.))),
            window_decorations: Some(WindowDecorations::Client),
            titlebar: Some(TitleBar::title_bar_options()),
            app_id: Some(APP_ID.into()),
            ..Default::default()
        };
        cx.defer(move |cx| {
            let form_owner = owner.clone();
            let opened = cx.open_window(options, move |window, cx| {
                window.set_window_title("Analysis settings · argand");
                let form =
                    cx.new(|cx| editor::Editor::new(form_owner, settings, range, window, cx));
                cx.new(|cx| gpui_component::Root::new(form, window, cx))
            });
            match opened {
                Ok(handle) => {
                    let _ = owner.update(cx, |shell, _| shell.settings_window = Some(handle));
                }
                Err(error) => tracing::error!(%error, "cannot open analysis settings"),
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

fn analysis_tooltip(owner: WeakEntity<Shell>) -> Tooltip {
    Tooltip::element(move |window, cx| {
        let Some(shell) = owner.upgrade() else {
            return div();
        };
        let shell = shell.read(cx);
        let settings = shell
            .file
            .as_ref()
            .and_then(|f| f.displayed_settings)
            .unwrap_or(shell.settings);
        let mode = match settings.dynamic_range {
            DynamicRange::Default => "Absolute full scale",
            DynamicRange::Auto => "Automatic",
            DynamicRange::Fixed(_) => "Below measured peak",
        };
        let recommendation = shell.range_recommendation();
        let range = shell
            .displayed_range()
            .map(|r| format!("{} dB", r.effective_db))
            .unwrap_or_else(|| settings.dynamic_range.to_string());
        let edit_owner = owner.clone();
        let apply_owner = owner.clone();
        div()
            .w(px(300.).min(window.viewport_size().width - px(48.)))
            .py_2()
            .flex()
            .flex_col()
            .gap_2()
            .child(div().font_weight(FontWeight::SEMIBOLD).child("Spectrogram"))
            .child(detail_row("FFT size", settings.fft_size.to_string(), cx))
            .child(detail_row("Window", settings.window.to_string(), cx))
            .child(detail_row("Overlap", format!("{}%", settings.overlap), cx))
            .child(detail_row("Aggregation", settings.aggregation.label(), cx))
            .child(detail_row("Range mode", mode, cx))
            .child(detail_row("Range", range, cx))
            .child(detail_row(
                "Colour scheme",
                settings.colormap.to_string(),
                cx,
            ))
            .when_some(recommendation, |hint, db| {
                hint.child(
                    div()
                        .text_xs()
                        .text_color(advice_color(cx))
                        .child("Low signal level leaves the upper half of the colour scale unused"),
                )
                .child(
                    Button::new("hint-recommendation")
                        .ghost()
                        .small()
                        .label(format!("Use recommended range: {db} dB"))
                        .on_click(move |_, _, cx| {
                            let _ = apply_owner.update(cx, |shell, cx| {
                                shell.set_settings(
                                    Settings {
                                        dynamic_range: DynamicRange::Fixed(db),
                                        ..shell.settings
                                    },
                                    cx,
                                )
                            });
                        }),
                )
            })
            .child(
                div()
                    .border_t_1()
                    .border_color(cx.theme().border)
                    .pt_2()
                    .child(
                        Button::new("edit-analysis-hint")
                            .ghost()
                            .small()
                            .label("Edit settings…")
                            .when_some(
                                Kbd::binding_for_action(&EditAnalysis, Some("Shell"), window),
                                |button, kbd| button.child(kbd),
                            )
                            .on_click(move |_, window, cx| {
                                let _ = edit_owner.update(cx, |shell, cx| {
                                    shell.edit_analysis(&EditAnalysis, window, cx)
                                });
                            }),
                    ),
            )
    })
}

fn advice_color(cx: &gpui::App) -> gpui::Hsla {
    if cx.theme().is_dark() {
        gpui::rgb(0xfacc15).into()
    } else {
        gpui::rgb(0x946200).into()
    }
}
