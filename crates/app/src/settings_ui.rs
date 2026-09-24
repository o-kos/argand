//! Status-bar file details and live analysis controls.

use super::*;
use crate::settings::{DisplayedRange, RangeState};
use argand_dsp::DynamicRange;
#[path = "settings_editor.rs"]
mod editor;

fn low_signal_level_hint(db: f32) -> String {
    format!(
        "Low signal level leaves the upper half of the colour scale unused, so use the recommended {} dB range",
        crate::numbers::number(db)
    )
}

fn range_hint(state: RangeState) -> String {
    match state {
        RangeState::Warned(db) => low_signal_level_hint(db),
        RangeState::Corrected => "Range narrowed from the full scale".into(),
        RangeState::Full => "Full scale, nothing trimmed".into(),
    }
}

fn next_range_action(
    displayed: Option<Settings>,
    requested: Settings,
    range: Option<DisplayedRange>,
    analysis_failed: bool,
) -> Option<DynamicRange> {
    if analysis_failed || displayed != Some(requested) {
        return None;
    }
    range?.state.next_range()
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct RangePresentation {
    displayed: Option<DisplayedRange>,
    next_action: Option<DynamicRange>,
}

#[derive(Clone, Copy)]
struct ControlForegrounds {
    normal: gpui_kit::Hsla,
    hovered: gpui_kit::Hsla,
    active: gpui_kit::Hsla,
}

impl ControlForegrounds {
    fn between(normal: gpui_kit::Hsla, hovered: gpui_kit::Hsla) -> Self {
        Self {
            normal,
            hovered,
            active: normal.mix(hovered, 0.5),
        }
    }

    fn current(self, hovered: bool) -> gpui_kit::Hsla {
        if hovered { self.hovered } else { self.normal }
    }

    fn button_style(self, cx: &gpui_kit::App) -> ButtonCustomVariant {
        ButtonCustomVariant::new(cx)
            .foreground(self.active)
            .hover(cx.theme().secondary_hover)
            .active(cx.theme().secondary_active)
    }
}

fn document_range_presentation(
    document: &Document,
    displayed_settings: Option<Settings>,
    requested: Settings,
) -> RangePresentation {
    let displayed = document.displayed_range();
    let next_action = next_range_action(
        displayed_settings,
        requested,
        displayed,
        matches!(document.status(), Status::Failed(_)),
    );
    RangePresentation {
        displayed,
        next_action,
    }
}

pub(super) fn init(cx: &mut gpui_kit::App) {
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
        self.bound_view(cx);
        if self.settings_backup.is_none() {
            self.session.analysis_settings = Some(settings);
            self.save();
        }
        self.ask_for_a_picture();
        cx.notify();
    }

    fn cancel_settings_window(&mut self, id: gpui_kit::WindowId, cx: &mut Context<Self>) {
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
        if let Some(dynamic_range) = self.next_range_action() {
            self.set_settings(
                Settings {
                    dynamic_range,
                    ..self.settings
                },
                cx,
            );
        }
    }

    fn displayed_range(&self) -> Option<DisplayedRange> {
        self.range_presentation()?.displayed
    }

    fn next_range_action(&self) -> Option<DynamicRange> {
        self.range_presentation()?.next_action
    }

    fn range_presentation(&self) -> Option<RangePresentation> {
        let file = self.file.as_ref()?;
        Some(document_range_presentation(
            &file.document,
            file.displayed_settings,
            self.settings,
        ))
    }

    fn range_recommendation(&self) -> Option<f32> {
        match self.next_range_action() {
            Some(DynamicRange::Fixed(db)) => Some(db),
            Some(DynamicRange::Default | DynamicRange::Auto) | None => None,
        }
    }

    pub(super) fn status_bar(
        &self,
        corners: Corners<Pixels>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let has_file = self.file.is_some();
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
                        .flex_shrink(1.)
                        .tooltip(move |_, cx| metadata_tooltip(field.hint.clone(), cx))
                        .child(
                            div()
                                .overflow_hidden()
                                .text_ellipsis()
                                .whitespace_nowrap()
                                .child(field.value),
                        ),
                )
            })
            .when(!has_file, |bar| {
                bar.child(div().px_2().whitespace_nowrap().child("No signal loaded"))
            })
            .when(has_file, |bar| bar.child(self.analysis_control(cx)))
            .child(div().flex_1().min_w_0())
            .when_some(self.cursor_readout(cx), |bar, (text, level)| {
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
                            status.tooltip(move |_, cx| metadata_tooltip(hint.clone(), cx))
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

    fn range_control(
        &self,
        displayed: Option<Settings>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let visible = displayed.unwrap_or(self.settings);
        let presentation = self.range_presentation();
        let displayed_range = presentation.and_then(|presentation| presentation.displayed);
        let range_text = displayed_range
            .map(|range| format!("{} dB", crate::numbers::number(range.effective_db)))
            .unwrap_or_else(|| match visible.dynamic_range {
                DynamicRange::Default => crate::numbers::text("110 dB"),
                DynamicRange::Fixed(db) => format!("{} dB", crate::numbers::number(db)),
                DynamicRange::Auto => "auto".into(),
            });
        let range_state = displayed_range.map_or_else(
            || RangeState::from_request(None, visible.dynamic_range),
            |range| range.state,
        );
        let range_label = match range_state {
            RangeState::Warned(_) => format!("⚠ {range_text}"),
            RangeState::Corrected | RangeState::Full => range_text,
        };
        let actionable =
            presentation.is_some_and(|presentation| presentation.next_action.is_some());
        let foregrounds = match range_state {
            RangeState::Warned(_) => {
                ControlForegrounds::between(advice_color(cx), advice_hover_color(cx))
            }
            RangeState::Corrected | RangeState::Full => {
                ControlForegrounds::between(cx.theme().muted_foreground, cx.theme().foreground)
            }
        };
        let foreground = foregrounds.current(self.range_hovered && actionable);
        let range_content = if actionable {
            Button::new("analysis-range")
                .tab_stop(false)
                .custom(foregrounds.button_style(cx))
                .small()
                .h_5()
                .px_2()
                .text_color(foreground)
                .on_hover(cx.listener(move |shell, hovered, _, cx| {
                    shell.range_hovered = *hovered;
                    cx.notify();
                }))
                .when(self.range_hovered && actionable, |button| {
                    button.bg(cx.theme().secondary_hover)
                })
                .on_click(move |_, window, cx| {
                    window.dispatch_action(Box::new(UseRecommendedRange), cx);
                })
                .child(div().text_xs().whitespace_nowrap().child(range_label))
                .into_any_element()
        } else {
            div()
                .id("analysis-range")
                .h_5()
                .px_2()
                .flex()
                .items_center()
                .text_xs()
                .text_color(foreground)
                .whitespace_nowrap()
                .on_hover(cx.listener(move |shell, hovered, _, cx| {
                    shell.range_hovered = *hovered;
                    cx.notify();
                }))
                .child(range_label)
                .into_any_element()
        };
        let range_hint = range_hint(range_state);
        div()
            .id("analysis-range-item")
            .border_l_1()
            .border_color(cx.theme().border)
            .tooltip(move |_, cx| {
                let action =
                    actionable.then(|| Box::new(UseRecommendedRange) as Box<dyn gpui_kit::Action>);
                shortcut_tooltip(range_hint.clone(), action, "Shell", px(320.), cx)
            })
            .child(range_content)
    }

    fn analysis_control(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let displayed = self.file.as_ref().and_then(|file| file.displayed_settings);
        let visible = displayed.unwrap_or(self.settings);
        let hint_owner = cx.entity().downgrade();
        let foregrounds =
            ControlForegrounds::between(cx.theme().muted_foreground, cx.theme().foreground);
        let foreground = foregrounds.current(self.analysis_hovered);
        let mut summary = Button::new("analysis-settings")
            .tab_stop(false)
            .custom(foregrounds.button_style(cx))
            .small()
            .h_5()
            .px_2()
            .text_color(foreground)
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
            .child(div().text_xs().child(format!(
                "{} · {}",
                crate::numbers::number(visible.fft_size),
                visible.window
            )));
        if self.settings_backup.is_none() {
            summary
                .interactivity()
                .hoverable_tooltip(move |window, cx| {
                    live_analysis_tooltip(hint_owner.clone(), window, cx)
                });
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
        self.interrupt_plot(cx);
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
            // The frame draws its shadow into the client inset; an opaque
            // surface would show that ring as a solid border instead.
            window_background: WindowBackgroundAppearance::Transparent,
            ..Default::default()
        };
        cx.defer(move |cx| {
            let form_owner = owner.clone();
            let opened = cx.open_window(options, move |window, cx| {
                window.set_window_title("Analysis settings · argand");
                let form =
                    cx.new(|cx| editor::Editor::new(form_owner, settings, range, window, cx));
                cx.new(|cx| gpui_kit::component::Root::new(form, window, cx))
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
    label: impl Into<gpui_kit::SharedString>,
    value: impl Into<gpui_kit::SharedString>,
    cx: &gpui_kit::App,
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

fn live_analysis_tooltip(
    owner: WeakEntity<Shell>,
    window: &mut Window,
    cx: &mut gpui_kit::App,
) -> gpui_kit::AnyView {
    hints::interactive(window, cx, |cx| {
        if let Some(owner) = owner.upgrade() {
            cx.observe(&owner, |_, _, cx| cx.notify()).detach();
        }
        analysis_tooltip(owner)
    })
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
                        .child(low_signal_level_hint(db)),
                )
                .child(
                    Button::new("hint-recommendation")
                        .tab_stop(false)
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
                        .on_click(move |_, window, cx| {
                            window.dispatch_action(Box::new(UseRecommendedRange), cx);
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
                            .tab_stop(false)
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

fn advice_color(cx: &gpui_kit::App) -> gpui_kit::Hsla {
    if cx.theme().is_dark() {
        gpui_kit::rgb(0xfacc15).into()
    } else {
        gpui_kit::rgb(0x946200).into()
    }
}

fn advice_hover_color(cx: &gpui_kit::App) -> gpui_kit::Hsla {
    let color = advice_color(cx);
    gpui_kit::hsla(color.h, color.s, (color.l + 0.24).min(1.0), color.a)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::{FileInfo, Update};
    use argand_core::{
        DbGrid, Domain, Psd, SampleFormat, SampleType, SignalMeta, SpectrogramImage,
    };
    use argand_dsp::{Analysis, DynamicRangeResult};
    use std::path::PathBuf;
    use std::time::Duration;

    fn warned_analysis() -> Box<Analysis> {
        Box::new(Analysis {
            spectrogram: SpectrogramImage::new(1, 1),
            db: DbGrid {
                width: 1,
                height: 1,
                values: vec![-60.0],
                t0: 0.0,
                t1: 1.0,
                f0: 0.0,
                f1: 1.0,
            },
            psd: Psd {
                freqs_hz: Vec::new(),
                db: Vec::new(),
                segments: 0,
            },
            waveform: None,
            time_peak: 0.01,
            frames: 1,
            enbw_hz: 1.0,
            dynamic_range: DynamicRangeResult {
                requested: DynamicRange::Default,
                effective_db: 110.0,
                recommended_db: 42.0,
            },
        })
    }

    #[test]
    fn control_foregrounds_keep_distinct_normal_hover_and_pressed_steps() {
        let normal = gpui_kit::hsla(0.15, 0.8, 0.4, 1.0);
        let hovered = gpui_kit::hsla(0.15, 0.8, 0.8, 1.0);
        let foregrounds = ControlForegrounds::between(normal, hovered);

        assert_eq!(foregrounds.current(false), normal);
        assert_eq!(foregrounds.current(true), hovered);
        assert_eq!(foregrounds.active, normal.mix(hovered, 0.5));
        assert_ne!(foregrounds.active, normal);
        assert_ne!(foregrounds.active, hovered);
    }

    #[test]
    fn range_state_applies_advice_then_restores_full_scale() {
        let advised = RangeState::from_request(Some(42.0), DynamicRange::Default)
            .next_range()
            .unwrap();
        assert_eq!(advised, DynamicRange::Fixed(42.0));

        let restored = RangeState::from_request(None, advised)
            .next_range()
            .unwrap();
        assert_eq!(restored, DynamicRange::Default);
        assert_eq!(RangeState::from_request(None, restored).next_range(), None);
    }

    #[test]
    fn automatic_range_has_no_toggle_without_advice() {
        assert_eq!(
            RangeState::from_request(None, DynamicRange::Auto).next_range(),
            None
        );
        assert_eq!(
            RangeState::from_request(Some(38.0), DynamicRange::Auto).next_range(),
            None
        );
    }

    #[test]
    fn pending_picture_suspends_an_otherwise_available_range_action() {
        let displayed = Settings::from_config(&Config::default());
        let requested = Settings {
            dynamic_range: DynamicRange::Fixed(42.0),
            ..displayed
        };
        let warned = Some(DisplayedRange {
            effective_db: 110.0,
            state: RangeState::Warned(42.0),
        });

        assert_eq!(
            next_range_action(Some(displayed), displayed, warned, false),
            Some(DynamicRange::Fixed(42.0))
        );
        assert_eq!(
            next_range_action(Some(displayed), requested, warned, false),
            None
        );
        assert_eq!(next_range_action(None, requested, warned, false), None);
    }

    #[test]
    fn failed_analysis_suspends_the_retained_range_action() {
        let settings = Settings::from_config(&Config::default());
        let mut document = Document::opening(Origin::new(PathBuf::from("test.iqw")));
        document.apply(Update::Opened(
            SignalMeta {
                sample_rate: 24_000.0,
                center_freq: 0.0,
                sample_type: SampleType::new(Domain::Iq, SampleFormat::I16),
                len_samples: 48_000,
                container: "raw",
                divisor: 32_768.0,
                source: PathBuf::from("test.iqw"),
            },
            FileInfo::default(),
        ));
        document.apply(Update::Ready {
            analysis: warned_analysis(),
            elapsed: Duration::ZERO,
        });
        let ready = document_range_presentation(&document, Some(settings), settings);
        assert_eq!(
            ready,
            RangePresentation {
                displayed: Some(DisplayedRange {
                    effective_db: 110.0,
                    state: RangeState::Warned(42.0),
                }),
                next_action: Some(DynamicRange::Fixed(42.0)),
            }
        );

        document.apply(Update::Failed(anyhow::anyhow!("replacement failed")));
        assert_eq!(
            document_range_presentation(&document, Some(settings), settings),
            RangePresentation {
                displayed: ready.displayed,
                next_action: None,
            }
        );
    }

    #[test]
    fn corrected_range_action_restores_full_scale_only_when_current() {
        let corrected = Settings {
            dynamic_range: DynamicRange::Fixed(42.0),
            ..Settings::from_config(&Config::default())
        };
        let range = Some(DisplayedRange {
            effective_db: 42.0,
            state: RangeState::Corrected,
        });

        assert_eq!(
            next_range_action(Some(corrected), corrected, range, false),
            Some(DynamicRange::Default)
        );
        assert_eq!(next_range_action(None, corrected, range, false), None);
    }
}
