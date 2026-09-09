//! Status-bar file details and live analysis controls.

use super::*;
use argand_core::COLORMAP_NAMES;
use argand_dsp::{DynamicRange, WINDOW_NAMES, suggested_range_db};
use gpui_component::Selectable;
use gpui_component::popover::Popover;

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
        let range = self.displayed_range()?;
        suggested_range_db(
            range.requested == DynamicRange::Auto,
            range.effective_db,
            range.recommended_db,
        )
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
        let view = cx.entity().downgrade();
        let owner = view.clone();
        Popover::new("analysis-popover")
            .anchor(gpui::Corner::BottomLeft)
            .open(self.settings_open)
            .track_focus(&self.settings_focus)
            .on_open_change(move |open, window, cx| {
                let _ = owner.update(cx, |shell, cx| {
                    shell.settings_open = *open;
                    if !open {
                        window.focus(&shell.focus);
                    }
                    cx.notify();
                });
            })
            .trigger(
                Button::new("analysis-settings")
                    .ghost()
                    .small()
                    .h_5()
                    .px_2()
                    .border_l_1()
                    .border_color(cx.theme().border)
                    .child(
                        div()
                            .flex()
                            .gap_1()
                            .text_xs()
                            .child(format!("{} · {} ·", displayed.fft_size, displayed.window))
                            .child(
                                div()
                                    .when(self.range_recommendation().is_some(), |range| {
                                        range.text_color(advice_color(cx))
                                    })
                                    .child(range),
                            ),
                    ),
            )
            .content(move |_, window, cx| {
                view.update(cx, |shell, cx| shell.settings_panel(window, cx))
                    .unwrap_or_else(|_| div().into_any_element())
            })
    }

    fn setting_menu(
        &self,
        label: &'static str,
        value: String,
        choices: Vec<(String, Settings)>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let expanded = self.expanded_setting == Some(label);
        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_3()
                    .child(div().text_color(cx.theme().muted_foreground).child(label))
                    .child(
                        Button::new(label)
                            .ghost()
                            .small()
                            .label(value)
                            .selected(expanded)
                            .on_click(cx.listener(move |shell, _, window, cx| {
                                shell.expanded_setting = if expanded { None } else { Some(label) };
                                window.focus(&shell.settings_focus);
                                cx.notify();
                            })),
                    ),
            )
            .when(expanded, |row| row.child(self.setting_choices(choices, cx)))
    }

    fn setting_choices(
        &self,
        choices: Vec<(String, Settings)>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div()
            .flex()
            .flex_wrap()
            .gap_1()
            .py_1()
            .children(
                choices
                    .into_iter()
                    .enumerate()
                    .map(|(index, (label, settings))| {
                        Button::new(("setting-choice", index))
                            .ghost()
                            .small()
                            .label(label)
                            .selected(settings == self.settings)
                            .on_click(cx.listener(move |shell, _, window, cx| {
                                shell.expanded_setting = None;
                                shell.set_settings(settings, cx);
                                window.focus(&shell.settings_focus);
                            }))
                    }),
            )
    }

    fn transform_controls(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let settings = self.settings;
        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                self.setting_menu(
                    "FFT size",
                    settings.fft_size.to_string(),
                    (1..=crate::settings::MAX_FFT_SIZE.ilog2())
                        .map(|power| {
                            let fft_size = 1 << power;
                            (
                                fft_size.to_string(),
                                Settings {
                                    fft_size,
                                    ..settings
                                },
                            )
                        })
                        .collect(),
                    cx,
                ),
            )
            .child(
                self.setting_menu(
                    "Window",
                    settings.window.to_string(),
                    WINDOW_NAMES
                        .iter()
                        .filter_map(|name| {
                            name.parse()
                                .ok()
                                .map(|window| (name.to_string(), Settings { window, ..settings }))
                        })
                        .collect(),
                    cx,
                ),
            )
            .child(
                self.setting_menu(
                    "Overlap",
                    format!("{}%", settings.overlap),
                    [0, 25, 50, 75, 87, 90, 95]
                        .into_iter()
                        .map(|overlap| {
                            (
                                format!("{overlap}%"),
                                Settings {
                                    overlap,
                                    ..settings
                                },
                            )
                        })
                        .collect(),
                    cx,
                ),
            )
            .child(
                self.setting_menu(
                    "Aggregation",
                    settings.aggregation.label().into(),
                    Aggregation::ALL
                        .iter()
                        .map(|&aggregation| {
                            (
                                aggregation.label().into(),
                                Settings {
                                    aggregation,
                                    ..settings
                                },
                            )
                        })
                        .collect(),
                    cx,
                ),
            )
    }

    fn range_controls(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let settings = self.settings;
        let range = match settings.dynamic_range {
            DynamicRange::Fixed(db) => db,
            _ => self
                .displayed_range()
                .map_or(110.0, |range| range.effective_db),
        };
        let mode = match settings.dynamic_range {
            DynamicRange::Default => "Absolute full scale",
            DynamicRange::Fixed(_) => "Below measured peak",
            DynamicRange::Auto => "Automatic",
        };
        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                self.setting_menu(
                    "Range mode",
                    mode.into(),
                    [
                        ("Absolute full scale", DynamicRange::Default),
                        ("Below measured peak", DynamicRange::Fixed(range)),
                        ("Automatic", DynamicRange::Auto),
                    ]
                    .into_iter()
                    .map(|(name, dynamic_range)| {
                        (
                            name.into(),
                            Settings {
                                dynamic_range,
                                ..settings
                            },
                        )
                    })
                    .collect(),
                    cx,
                ),
            )
            .child(self.range_value_controls(range, cx))
            .child(
                self.setting_menu(
                    "Colour scheme",
                    settings.colormap.to_string(),
                    COLORMAP_NAMES
                        .iter()
                        .filter_map(|name| {
                            name.parse().ok().map(|colormap| {
                                (
                                    name.to_string(),
                                    Settings {
                                        colormap,
                                        ..settings
                                    },
                                )
                            })
                        })
                        .collect(),
                    cx,
                ),
            )
    }

    fn range_value_controls(&self, range: f32, cx: &mut Context<Self>) -> impl IntoElement {
        let settings = self.settings;
        div()
            .flex()
            .items_center()
            .justify_between()
            .gap_2()
            .child(
                self.setting_menu(
                    "Range",
                    format!("{range} dB"),
                    (1..=16)
                        .map(|n| {
                            let db = n as f32 * 10.0;
                            (
                                format!("{db} dB"),
                                Settings {
                                    dynamic_range: DynamicRange::Fixed(db),
                                    ..settings
                                },
                            )
                        })
                        .collect(),
                    cx,
                ),
            )
            .child(
                Button::new("range-less")
                    .ghost()
                    .small()
                    .label("−")
                    .on_click(cx.listener(move |shell, _, _, cx| {
                        shell.set_settings(
                            Settings {
                                dynamic_range: DynamicRange::Fixed((range - 1.0).max(1.0)),
                                ..shell.settings
                            },
                            cx,
                        )
                    })),
            )
            .child(
                Button::new("range-more")
                    .ghost()
                    .small()
                    .label("+")
                    .on_click(cx.listener(move |shell, _, _, cx| {
                        shell.set_settings(
                            Settings {
                                dynamic_range: DynamicRange::Fixed(range + 1.0),
                                ..shell.settings
                            },
                            cx,
                        )
                    })),
            )
    }

    fn settings_panel(&self, window: &Window, cx: &mut Context<Self>) -> gpui::AnyElement {
        let pending = self.file.as_ref().is_some_and(|file| {
            !matches!(file.document.status(), Status::Failed(_))
                && file.displayed_settings != Some(self.settings)
        });
        div()
            .id("analysis-panel")
            .track_focus(&self.settings_focus)
            .w(px(320.).min(window.viewport_size().width - px(64.)))
            .max_h(window.viewport_size().height - px(100.))
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap_2()
            .text_sm()
            .child(
                div()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Analysis settings"),
            )
            .child(self.transform_controls(cx))
            .child(div().border_t_1().border_color(cx.theme().border))
            .child(self.range_controls(cx))
            .when_some(self.range_recommendation(), |panel, db| {
                panel.child(
                    Button::new("apply-recommended-range")
                        .ghost()
                        .small()
                        .text_color(advice_color(cx))
                        .label(format!("Apply recommended range: {db} dB"))
                        .on_click(cx.listener(move |shell, _, _, cx| {
                            shell.set_settings(
                                Settings {
                                    dynamic_range: DynamicRange::Fixed(db),
                                    ..shell.settings
                                },
                                cx,
                            )
                        })),
                )
            })
            .when(pending, |panel| {
                panel.child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child("Updating the picture…"),
                )
            })
            .when_some(self.settings_error.clone(), |panel, error| {
                panel.child(div().text_xs().text_color(cx.theme().danger).child(error))
            })
            .when_some(
                self.file
                    .as_ref()
                    .and_then(|file| match file.document.status() {
                        Status::Failed(error) => Some(error.clone()),
                        _ => None,
                    }),
                |panel, error| {
                    panel.child(div().text_xs().text_color(cx.theme().danger).child(error))
                },
            )
            .into_any_element()
    }
}

fn advice_color(cx: &gpui::App) -> gpui::Hsla {
    if cx.theme().is_dark() {
        gpui::rgb(0xfacc15).into()
    } else {
        gpui::rgb(0x946200).into()
    }
}
