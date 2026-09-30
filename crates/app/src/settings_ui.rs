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

/// The text colours a status control paints. The toolkit gives a custom
/// variant one foreground for every pointer state, so the two colours a
/// control passes through are its resting one and that single state colour.
#[derive(Clone, Copy)]
struct ControlForegrounds {
    normal: gpui_kit::Hsla,
    active: gpui_kit::Hsla,
}

impl ControlForegrounds {
    fn between(normal: gpui_kit::Hsla, hovered: gpui_kit::Hsla) -> Self {
        Self {
            normal,
            active: normal.mix(hovered, 0.5),
        }
    }

    fn button_style(self, cx: &gpui_kit::App) -> ButtonCustomVariant {
        ButtonCustomVariant::new(cx)
            .foreground(self.active)
            .hover(cx.theme().secondary_hover)
            .active(cx.theme().secondary_active)
    }

    /// The text of a control, which follows the pointer through its group.
    fn text(self, group: &'static str) -> gpui_kit::Div {
        div()
            .text_xs()
            .text_color(self.normal)
            .group_hover(group, |style| style.text_color(self.active))
    }
}

/// The hover group each status control publishes for its own text.
const SUMMARY_HOVER: &str = "analysis-summary-hover";
const RANGE_HOVER: &str = "analysis-range-hover";

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
        // Settings previewed in the open hint are saved when it closes keeping them.
        if self.hint_opening.is_none() {
            self.persist_settings();
        }
        self.ask_for_a_picture();
        cx.notify();
    }

    fn persist_settings(&mut self) {
        self.session.analysis_settings = Some(self.settings);
        self.save();
    }

    /// Close the analysis hint, keeping what was changed while it was open.
    pub(super) fn close_analysis_hint(&mut self, cx: &mut Context<Self>) {
        self.analysis_hint
            .update(cx, |hint, cx| hint.close(false, cx));
    }

    pub(super) fn analysis_hint_changed(
        &mut self,
        _: gpui_kit::Entity<hints::PinnedHint>,
        event: &hints::Pinned,
        cx: &mut Context<Self>,
    ) {
        match *event {
            hints::Pinned::Opened => {
                self.interrupt_plot(cx);
                self.hint_opening = Some(HintOpening {
                    settings: self.settings,
                    view: self.view,
                    frequency: self.frequency,
                });
            }
            hints::Pinned::Closed { revert } => {
                let Some(opening) = self.hint_opening.take() else {
                    return;
                };
                if revert {
                    self.restore_opening(opening, cx);
                } else {
                    self.persist_settings();
                }
            }
        }
        cx.notify();
    }

    /// Put back the settings and views the hint opened with.
    fn restore_opening(&mut self, opening: HintOpening, cx: &mut Context<Self>) {
        self.view = opening.view;
        if self.frequency != opening.frequency {
            self.frequency = opening.frequency;
            self.frequency_scheme = None;
        }
        self.apply_settings(opening.settings, cx);
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
    ) -> (gpui_kit::AnyElement, Option<gpui_kit::AnyView>) {
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
        let range_content = if actionable {
            Button::new("analysis-range")
                .tab_stop(false)
                .group(RANGE_HOVER)
                .custom(foregrounds.button_style(cx))
                .small()
                .h_5()
                .px_2()
                .on_click(move |_, window, cx| {
                    window.dispatch_action(Box::new(UseRecommendedRange), cx);
                })
                .child(
                    foregrounds
                        .text(RANGE_HOVER)
                        .whitespace_nowrap()
                        .child(range_label),
                )
                .into_any_element()
        } else {
            div()
                .id("analysis-range")
                .h_5()
                .px_2()
                .flex()
                .items_center()
                .text_xs()
                .text_color(foregrounds.normal)
                .whitespace_nowrap()
                .child(range_label)
                .into_any_element()
        };
        let range_hint = range_hint(range_state);
        // The item cannot be hovered under the settings hint's backdrop, so its hint is shown for it.
        let warned = matches!(range_state, RangeState::Warned(_));
        let balloon = (warned && self.analysis_hint.read(cx).is_open()).then(|| {
            let action =
                actionable.then(|| Box::new(UseRecommendedRange) as Box<dyn gpui_kit::Action>);
            shortcut_tooltip(range_hint.clone(), action, "Shell", px(320.), cx)
        });
        let item = div()
            .id("analysis-range-item")
            .border_l_1()
            .border_color(cx.theme().border)
            .tooltip(move |_, cx| {
                let action =
                    actionable.then(|| Box::new(UseRecommendedRange) as Box<dyn gpui_kit::Action>);
                shortcut_tooltip(range_hint.clone(), action, "Shell", px(320.), cx)
            })
            .child(range_content)
            .into_any_element();
        (item, balloon)
    }

    fn analysis_control(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let displayed = self.file.as_ref().and_then(|file| file.displayed_settings);
        let visible = displayed.unwrap_or(self.settings);
        let foregrounds =
            ControlForegrounds::between(cx.theme().muted_foreground, cx.theme().foreground);
        // The hint's backdrop keeps the pointer off the window, so the summary stays lit.
        let lit = self.analysis_hint.read(cx).is_open();
        let summary = Button::new("analysis-settings")
            .tab_stop(false)
            .group(SUMMARY_HOVER)
            .custom(foregrounds.button_style(cx))
            .small()
            .h_5()
            .px_2()
            .when(lit, |button| button.bg(cx.theme().secondary_hover))
            .on_hover(cx.listener(|shell, hovered, window, cx| {
                shell
                    .analysis_hint
                    .update(cx, |hint, cx| hint.hover(*hovered, window, cx));
            }))
            // A press, not a click, so the press that closes an open hint cannot reopen it.
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|shell, _, window, cx| shell.edit_analysis(&EditAnalysis, window, cx)),
            )
            .child(
                foregrounds
                    .text(SUMMARY_HOVER)
                    .when(lit, |text| text.text_color(foregrounds.active))
                    .child(format!(
                        "{} · {}",
                        crate::numbers::number(visible.fft_size),
                        editor::display_name(&visible.window.to_string())
                    )),
            );
        let (range, balloon) = self.range_control(displayed, cx);
        // Beside the settings hint, which starts at the summary's left edge and would cover it.
        let balloon = balloon.map(|hint| {
            let beside = div()
                .absolute()
                .bottom_full()
                .left(px(editor::WIDTH + 8.0))
                .child(hint);
            gpui_kit::deferred(beside).with_priority(2)
        });
        div()
            .flex()
            .items_center()
            .gap_2()
            .child(
                div()
                    .id("analysis-summary")
                    .relative()
                    .border_l_1()
                    .border_color(cx.theme().border)
                    .child(hints::pinned(
                        "analysis-hint",
                        &self.analysis_hint,
                        summary,
                        cx,
                    ))
                    .children(balloon),
            )
            .child(range)
    }

    /// Open the analysis hint with the keyboard in it, or close it keeping its values.
    pub(super) fn edit_analysis(
        &mut self,
        _: &EditAnalysis,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.dismiss_application_menu(window, cx);
        // Without a document there is no summary to anchor the hint to.
        if self.file.is_none() {
            return;
        }
        let hint = self.analysis_hint.clone();
        if hint.read(cx).is_open() {
            hint.update(cx, |hint, cx| hint.close(false, cx));
            return;
        }
        // The editor reads the shell as it is built, so it waits until this update ends.
        window.defer(cx, move |window, cx| {
            hint.update(cx, |hint, cx| hint.open(window, cx));
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

/// The analysis settings surface the pinned hint shows, built afresh each time it opens.
pub(super) fn analysis_editor(
    owner: WeakEntity<Shell>,
    window: &mut Window,
    cx: &mut gpui_kit::App,
) -> gpui_kit::AnyView {
    cx.new(|cx| editor::Editor::new(owner, window, cx)).into()
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
    fn control_foregrounds_keep_a_resting_colour_and_a_distinct_state_one() {
        let normal = gpui_kit::hsla(0.15, 0.8, 0.4, 1.0);
        let hovered = gpui_kit::hsla(0.15, 0.8, 0.8, 1.0);
        let foregrounds = ControlForegrounds::between(normal, hovered);

        assert_eq!(foregrounds.normal, normal);
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
