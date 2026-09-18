//! Status-bar file details and live analysis controls.

use super::*;
use argand_dsp::DynamicRange;
#[path = "settings_editor.rs"]
mod editor;

#[derive(Clone, Copy, Debug, PartialEq)]
enum RangeState {
    Warned(f32),
    Corrected,
    Full,
}

impl RangeState {
    fn from_range(recommendation: Option<f32>, range: DynamicRange) -> Self {
        match (recommendation, range) {
            (Some(db), _) => Self::Warned(db),
            (None, DynamicRange::Fixed(_)) => Self::Corrected,
            (None, DynamicRange::Default | DynamicRange::Auto) => Self::Full,
        }
    }

    const fn hint(self) -> &'static str {
        match self {
            Self::Warned(_) => "Spectrum peak sits low in this range",
            Self::Corrected => "Range narrowed from the full scale",
            Self::Full => "Full scale, nothing trimmed",
        }
    }

    const fn next_range(self) -> Option<DynamicRange> {
        match self {
            Self::Warned(db) => Some(DynamicRange::Fixed(db)),
            Self::Corrected => Some(DynamicRange::Default),
            Self::Full => None,
        }
    }
}

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

    fn cancel_settings_window(&mut self, id: gpui::WindowId, cx: &mut Context<Self>) {
        if self
            .settings_window
            .is_some_and(|handle| handle.window_id() == id)
        {
            self.finish_settings(false, cx);
        }
    }

    pub(super) fn finish_settings(&mut self, accept: bool, cx: &mut Context<Self>) {
        let Some(backup) = self.settings_backup.take() else {
            return;
        };
        self.settings_window = None;
        self.analysis_hovered = false;
        self.range_hovered = false;
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
        let state =
            RangeState::from_range(self.range_recommendation(), self.settings.dynamic_range);
        if let Some(dynamic_range) = state.next_range() {
            if dynamic_range == DynamicRange::Default {
                self.range_hovered = false;
            }
            self.set_settings(
                Settings {
                    dynamic_range,
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
        let status = self.file.as_ref().and_then(|file| {
            file.document
                .status()
                .presentation(self.ready_status_dismissed)
        });
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
            .when_some(self.cursor_readout(), |bar, (text, level)| {
                bar.child(
                    div()
                        .id("cursor-readout")
                        .min_w_0()
                        .overflow_hidden()
                        .text_ellipsis()
                        .whitespace_nowrap()
                        .child(text),
                )
                .when_some(level, |bar, level| {
                    bar.child(
                        div()
                            .id("cursor-level")
                            .border_l_1()
                            .border_color(cx.theme().border)
                            .px_2()
                            .flex_shrink_0()
                            .whitespace_nowrap()
                            .child(level),
                    )
                })
            })
            .when_some(status, |bar, (status, hint)| {
                bar.child(
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
            })
    }

    fn range_control(&self, displayed: Settings, cx: &mut Context<Self>) -> impl IntoElement {
        let range_text = self
            .displayed_range()
            .map(|range| format!("{} dB", crate::numbers::number(range.effective_db)))
            .unwrap_or_else(|| match displayed.dynamic_range {
                DynamicRange::Default => crate::numbers::text("110 dB"),
                DynamicRange::Fixed(db) => format!("{} dB", crate::numbers::number(db)),
                DynamicRange::Auto => "auto".into(),
            });
        let range_state = RangeState::from_range(
            self.displayed_range_recommendation(),
            displayed.dynamic_range,
        );
        let range_label = match range_state {
            RangeState::Warned(_) => format!("⚠ {range_text}"),
            RangeState::Corrected | RangeState::Full => range_text,
        };
        let actionable = range_state.next_range().is_some();
        let range_content = match range_state {
            RangeState::Warned(_) | RangeState::Corrected => {
                let (foreground, hover_foreground) = match range_state {
                    RangeState::Warned(_) => (advice_color(cx), advice_hover_color(cx)),
                    RangeState::Corrected => (cx.theme().muted_foreground, cx.theme().foreground),
                    RangeState::Full => (cx.theme().muted_foreground, cx.theme().muted_foreground),
                };
                Button::new("analysis-range")
                    .ghost()
                    .small()
                    .h_5()
                    .px_2()
                    .text_color(foreground)
                    .on_hover(cx.listener(move |shell, hovered, _, cx| {
                        shell.range_hovered = *hovered;
                        cx.notify();
                    }))
                    .when(self.range_hovered, |button| {
                        button
                            .bg(cx.theme().secondary_hover)
                            .text_color(hover_foreground)
                    })
                    .on_click(move |_, window, cx| {
                        window.dispatch_action(Box::new(UseRecommendedRange), cx);
                    })
                    .child(div().text_xs().whitespace_nowrap().child(range_label))
                    .into_any_element()
            }
            RangeState::Full => div()
                .id("analysis-range")
                .h_5()
                .px_2()
                .flex()
                .items_center()
                .text_xs()
                .whitespace_nowrap()
                .child(range_label)
                .into_any_element(),
        };
        let range_hint = range_state.hint();
        div()
            .id("analysis-range-item")
            .border_l_1()
            .border_color(cx.theme().border)
            .tooltip(move |window, cx| {
                let action =
                    actionable.then(|| Box::new(UseRecommendedRange) as Box<dyn gpui::Action>);
                shortcut_tooltip(range_hint.to_owned(), action, "Shell", px(320.)).build(window, cx)
            })
            .child(range_content)
    }

    fn analysis_control(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let displayed = self
            .file
            .as_ref()
            .and_then(|file| file.displayed_settings)
            .unwrap_or(self.settings);
        let hint_owner = cx.entity().downgrade();
        let foreground = if self.analysis_hovered {
            cx.theme().foreground
        } else {
            cx.theme().muted_foreground
        };
        let mut summary = Button::new("analysis-settings")
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
            .on_click(
                cx.listener(|shell, _, window, cx| shell.edit_analysis(&EditAnalysis, window, cx)),
            )
            .child(div().text_xs().text_color(foreground).child(format!(
                "{} · {}",
                crate::numbers::number(displayed.fft_size),
                displayed.window
            )));
        if self.settings_backup.is_none() {
            summary
                .interactivity()
                .hoverable_tooltip(move |_, cx| live_analysis_tooltip(hint_owner.clone(), cx));
        }
        div()
            .flex()
            .items_center()
            .gap_2()
            .child(
                div()
                    .id("analysis-summary")
                    .border_l_1()
                    .border_color(cx.theme().border)
                    .child(summary),
            )
            .child(self.range_control(displayed, cx))
    }

    pub(super) fn edit_analysis(
        &mut self,
        _: &EditAnalysis,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.dismiss_application_menu(window, cx);
        if self.settings_window.is_some_and(|handle| {
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
        self.range_hovered = false;
        cx.notify();
        let owner = cx.entity().downgrade();
        let settings = self.settings;
        let range = self
            .displayed_range()
            .map_or(110.0, |range| range.effective_db);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                None,
                size(px(460.), px(560.)),
                cx,
            ))),
            window_min_size: Some(size(px(440.), px(540.))),
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
                Err(error) => {
                    let _ = owner.update(cx, |shell, cx| shell.finish_settings(false, cx));
                    tracing::error!(%error, "cannot open analysis settings");
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

fn live_analysis_tooltip(owner: WeakEntity<Shell>, cx: &mut gpui::App) -> gpui::AnyView {
    cx.new(|cx| {
        if let Some(owner) = owner.upgrade() {
            cx.observe(&owner, |_, _, cx| cx.notify()).detach();
        }
        analysis_tooltip(owner)
    })
    .into()
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
            .map(|r| format!("{} dB", crate::numbers::number(r.effective_db)))
            .unwrap_or_else(|| crate::numbers::text(&settings.dynamic_range.to_string()));
        let fft = crate::numbers::number(settings.fft_size);
        let overlap = format!("{}%", crate::numbers::number(settings.overlap));
        let edit_owner = owner.clone();
        let apply_owner = owner.clone();
        div()
            .w(px(300.).min(window.viewport_size().width - px(48.)))
            .py_2()
            .flex()
            .flex_col()
            .gap_2()
            .child(div().font_weight(FontWeight::SEMIBOLD).child("Spectrogram"))
            .child(detail_row("FFT size", fft, cx))
            .child(detail_row("Window", settings.window.to_string(), cx))
            .child(detail_row("Overlap", overlap, cx))
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
                        .label(format!(
                            "Use recommended: {} dB",
                            crate::numbers::number(db)
                        ))
                        .when_some(
                            Kbd::binding_for_action(&UseRecommendedRange, None, window),
                            |button, kbd| button.child(shortcuts::keycap(kbd, cx)),
                        )
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
                                |button, kbd| button.child(shortcuts::keycap(kbd, cx)),
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

fn advice_hover_color(cx: &gpui::App) -> gpui::Hsla {
    let color = advice_color(cx);
    gpui::hsla(color.h, color.s, (color.l + 0.24).min(1.0), color.a)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn range_state_applies_advice_then_restores_full_scale() {
        let advised = RangeState::from_range(Some(42.0), DynamicRange::Default)
            .next_range()
            .unwrap();
        assert_eq!(advised, DynamicRange::Fixed(42.0));

        let restored = RangeState::from_range(None, advised).next_range().unwrap();
        assert_eq!(restored, DynamicRange::Default);
        assert_eq!(RangeState::from_range(None, restored).next_range(), None);
    }

    #[test]
    fn automatic_range_has_no_toggle_without_advice() {
        assert_eq!(
            RangeState::from_range(None, DynamicRange::Auto).next_range(),
            None
        );
        assert_eq!(
            RangeState::from_range(Some(38.0), DynamicRange::Auto).next_range(),
            Some(DynamicRange::Fixed(38.0))
        );
    }
}
