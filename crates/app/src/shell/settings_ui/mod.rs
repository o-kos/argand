//! Status-bar file details and live analysis controls.

use super::property_advice::color as advice_color;
use super::*;
use crate::settings::{DisplayedRange, RangeState};
use argand_dsp::DynamicRange;
use gpui_kit::AnyElement;
use gpui_kit::component::Disableable;
pub(super) mod editor;

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

/// Shared typography, pointer ink and geometry for status summary buttons.
pub(super) fn status_control(
    id: &'static str,
    group: &'static str,
    text: String,
    lit: bool,
    cx: &gpui_kit::App,
) -> Button {
    let foregrounds =
        ControlForegrounds::between(cx.theme().muted_foreground, cx.theme().foreground);
    Button::new(id)
        .tab_stop(false)
        .group(group)
        .custom(foregrounds.button_style(cx))
        .small()
        .h_5()
        .px_2()
        .when(lit, |button| button.bg(cx.theme().secondary_hover))
        .child(
            foregrounds
                .text(group)
                .when(lit, |text| text.text_color(foregrounds.active))
                .child(text),
        )
}

/// The two property editors anchored in the status bar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum StatusPanel {
    File,
    Analysis,
}

impl From<StatusPanel> for hints::Panel {
    fn from(panel: StatusPanel) -> Self {
        match panel {
            StatusPanel::File => Self::File,
            StatusPanel::Analysis => Self::Analysis,
        }
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
    editor::init(cx);
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
    pub(super) fn information_hint(&self, panel: hints::Panel) -> Entity<hints::PinnedHint> {
        match panel {
            hints::Panel::File => self.conversion.hint.clone(),
            hints::Panel::Analysis => self.analysis_hint.clone(),
            hints::Panel::About => self.about.hint.clone(),
        }
    }

    fn information_panel_available(&self, panel: hints::Panel) -> bool {
        if panel == hints::Panel::About {
            return true;
        }
        if self.conversion_active() {
            return false;
        }
        match panel {
            hints::Panel::File => self.conversion_request().is_some(),
            hints::Panel::Analysis => self.file.is_some(),
            hints::Panel::About => true,
        }
    }

    pub(super) fn status_panel_available(&self, panel: StatusPanel) -> bool {
        self.information_panel_available(panel.into())
    }

    pub(super) fn information_panel_changed(
        &mut self,
        panel: hints::Panel,
        event: hints::Pinned,
        cx: &mut Context<Self>,
    ) {
        let open = self.information_hint(panel).read(cx).is_open();
        if open && !self.information_panel_available(panel) {
            self.information_hint(panel)
                .update(cx, |hint, cx| hint.close(false, cx));
            return;
        }
        match event {
            hints::Pinned::Opened if open => {
                self.suppress_external_tooltips();
                self.information_panel_request += 1;
                // Choose the new owner before closing the old editor and delivering Closed.
                self.information_panel = Some(panel);
                self.close_other_information_panels(panel, cx);
                self.close_selection_hint(cx);
                self.interrupt_plot(cx);
            }
            hints::Pinned::Closed { .. } if !open => {
                if self.information_panel == Some(panel) {
                    self.information_panel = None;
                }
            }
            _ => return,
        }
        if panel == hints::Panel::Analysis {
            self.analysis_panel_changed(event, cx);
        }
        cx.notify();
    }

    fn close_other_information_panels(&mut self, panel: hints::Panel, cx: &mut Context<Self>) {
        for other in hints::Panel::ALL {
            if other != panel {
                self.information_hint(other)
                    .update(cx, |hint, cx| hint.close(false, cx));
            }
        }
    }

    pub(super) fn activate_information_panel(
        &mut self,
        panel: hints::Panel,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.information_panel_available(panel) {
            return;
        }
        self.dismiss_application_menu(window, cx);
        self.information_panel_request += 1;
        let request = self.information_panel_request;
        let hint = self.information_hint(panel);
        if hint.read(cx).is_open() {
            hint.update(cx, |hint, cx| hint.close(false, cx));
            return;
        }
        self.close_other_information_panels(panel, cx);
        self.close_selection_hint(cx);
        let owner = cx.entity().downgrade();
        window.defer(cx, move |window, cx| {
            let current = owner.upgrade().is_some_and(|owner| {
                let shell = owner.read(cx);
                shell.information_panel_request == request
                    && shell.information_panel_available(panel)
            });
            if current {
                hint.update(cx, |hint, cx| hint.open(window, cx));
            }
        });
    }

    pub(super) fn status_panel_control(
        &self,
        panel: StatusPanel,
        text: String,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let (id, group, popover, focus) = match panel {
            StatusPanel::File => (
                "file-information",
                "file-summary-hover",
                "file-information-hint",
                "file-summary-keeps-focus",
            ),
            StatusPanel::Analysis => (
                "analysis-settings",
                SUMMARY_HOVER,
                "analysis-hint",
                "fft-summary-keeps-focus",
            ),
        };
        let hint = self.information_hint(panel.into());
        let lit = self.information_panel == Some(panel.into()) && hint.read(cx).is_open();
        let mut button =
            status_control(id, group, text, lit, cx).disabled(!self.status_panel_available(panel));
        if self.external_tooltips_allowed(cx) {
            button
                .interactivity()
                .tooltip(move |_, cx| status_panel_tooltip(panel, cx));
        }
        super::app_menu_ui::keeps_focus(
            focus,
            hints::pinned(popover, &hint, button, gpui_kit::Anchor::BottomLeft, cx),
        )
        .into_any_element()
    }

    pub(super) fn information_panel_backdrop(
        &self,
        area: gpui_kit::Bounds<Pixels>,
        cx: &gpui_kit::App,
    ) -> Option<AnyElement> {
        self.information_panel
            .and_then(|panel| hints::backdrop(&self.information_hint(panel), area, cx))
    }

    fn set_settings(&mut self, settings: Settings, cx: &mut Context<Self>) {
        let samples = self
            .file
            .as_ref()
            .and_then(|file| file.state.document.meta())
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
        if let Some(file) = &mut self.file {
            file.state.retake_equivalent(&settings);
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

    /// Save the settings, unless the file would keep exactly what it already has in effect.
    fn persist_settings(&mut self) {
        let in_effect = Settings::restored(self.session.analysis_settings, &self.config);
        // A range change alone is never stored and must not claim the settings as this instance's
        if in_effect.persisted() == self.settings.persisted() {
            return;
        }
        self.session.analysis_settings = Some(self.settings);
        self.save();
    }

    /// Close the pinned hints, the analysis settings and the selection panel, keeping their values.
    pub(super) fn cancel_status_select(&self, window: &mut Window, cx: &mut Context<Self>) {
        let trigger = self.file_select_trigger(cx).or_else(|| {
            let editor = self
                .analysis_hint
                .read(cx)
                .view()?
                .downcast::<editor::Editor>()
                .ok()?;
            editor.read(cx).open_select_trigger(cx)
        });
        if let Some(trigger) = trigger {
            // Close synchronously so the old frame's outside-click listener will propagate.
            trigger.dispatch_action(&gpui_kit::base::actions::Cancel, window, cx);
            cx.propagate();
        }
    }

    pub(super) fn close_hints(&mut self, cx: &mut Context<Self>) {
        self.information_panel_request += 1;
        for panel in hints::Panel::ALL {
            self.information_hint(panel)
                .update(cx, |hint, cx| hint.close(false, cx));
        }
        self.close_selection_hint(cx);
    }

    /// Close the analysis hint, keeping what was changed while it was open.
    pub(super) fn close_analysis_hint(&mut self, cx: &mut Context<Self>) {
        self.analysis_hint
            .update(cx, |hint, cx| hint.close(false, cx));
    }

    fn analysis_panel_changed(&mut self, event: hints::Pinned, cx: &mut Context<Self>) {
        match event {
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
            &file.state.document,
            file.state.displayed_settings().cloned(),
            self.settings,
        ))
    }

    /// The warning of the range shown, and whether Ctrl+R can act on it.
    fn range_warning(&self) -> Option<(String, bool)> {
        let presentation = self.range_presentation()?;
        let state = presentation.displayed?.state;
        matches!(state, RangeState::Warned(_))
            .then(|| (range_hint(state), presentation.next_action.is_some()))
    }

    pub(super) fn status_bar(
        &self,
        corners: Corners<Pixels>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let has_file = self.file.is_some();
        let file = self.file.as_ref().and_then(|file| {
            file.state.document.file_summary_for_container(
                file.unsaved
                    .as_ref()
                    .map(|unsaved| unsaved.container.extension()),
            )
        });
        let status = self.file.as_ref().and_then(|file| {
            file.state
                .document
                .status()
                .presentation(self.ready_status_dismissed)
        });
        div()
            .on_any_mouse_down(cx.listener(|shell, _, _, cx| shell.close_hints(cx)))
            .flex()
            .items_center()
            .gap_2()
            .px_2()
            .when(has_file, |bar| bar.pl_0p5())
            .h_6()
            .flex_shrink_0()
            .rounded_bl(corners.bottom_left)
            .rounded_br(corners.bottom_right)
            .border_t_1()
            .border_color(cx.theme().status_bar_border)
            .bg(cx.theme().status_bar)
            .text_xs()
            .text_color(cx.theme().muted_foreground)
            .when(has_file, |bar| {
                bar.child(
                    div()
                        .flex()
                        .items_center()
                        .gap_0p5()
                        .when_some(file, |items, field| {
                            items.child(self.file_control(field.value, cx))
                        })
                        .children(self.analysis_controls(cx)),
                )
            })
            .when(!has_file, |bar| {
                bar.child(div().px_2().whitespace_nowrap().child("No signal loaded"))
            })
            .children(self.adjustment_item(cx))
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
            .when_some(
                status.filter(|_| !self.capture_changed()),
                |bar, (status, hint)| {
                    bar.child(
                        div()
                            .id("analysis-status")
                            .min_w_0()
                            .max_w(px(140.))
                            .when_some(
                                hint.filter(|_| self.external_tooltips_allowed(cx)),
                                |status, hint| {
                                    status.tooltip(move |_, cx| metadata_tooltip(hint.clone(), cx))
                                },
                            )
                            .child(
                                div()
                                    .overflow_hidden()
                                    .text_ellipsis()
                                    .whitespace_nowrap()
                                    .child(status),
                            ),
                    )
                },
            )
            .children(self.changed_item(cx))
            // Last, so it keeps its place whatever the readouts beside it do.
            .children(self.suppress_item(cx))
            .children(self.level_item(cx))
            .children(self.save_item(cx))
            .children(self.conversion_item(cx))
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
        let range_content = if actionable {
            super::app_menu_ui::keeps_focus(
                "analysis-range-keeps-focus",
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
                    ),
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
        separated_status_item("analysis-range-item", cx)
            .when(self.external_tooltips_allowed(cx), |element| {
                element.tooltip(move |_, cx| {
                    let action = actionable
                        .then(|| Box::new(UseRecommendedRange) as Box<dyn gpui_kit::Action>);
                    shortcut_tooltip(range_hint.clone(), action, "Shell", px(320.), cx)
                })
            })
            .child(range_content)
    }

    fn analysis_controls(&self, cx: &mut Context<Self>) -> [AnyElement; 2] {
        let displayed = self
            .file
            .as_ref()
            .and_then(|file| file.state.displayed_settings().cloned());
        let visible = displayed.unwrap_or(self.settings);
        let summary = self.status_panel_control(
            StatusPanel::Analysis,
            format!(
                "{} · {}",
                crate::numbers::number(visible.fft_size),
                editor::display_name(&visible.window.to_string())
            ),
            cx,
        );
        [
            separated_status_item("analysis-summary", cx)
                .child(summary)
                .into_any_element(),
            self.range_control(displayed, cx).into_any_element(),
        ]
    }

    /// Open the analysis hint with the keyboard in it, or close it keeping its values.
    pub(super) fn edit_analysis(
        &mut self,
        _: &EditAnalysis,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.activate_information_panel(hints::Panel::Analysis, window, cx);
    }
}

fn status_panel_tooltip(panel: StatusPanel, cx: &mut gpui_kit::App) -> gpui_kit::AnyView {
    let (title, description, action): (_, _, Box<dyn Action>) = match panel {
        StatusPanel::File => (
            "File information",
            "View file properties and conversion options",
            Box::new(ConvertCapture),
        ),
        StatusPanel::Analysis => (
            "Analysis settings",
            "Adjust FFT, window function and display range",
            Box::new(EditAnalysis),
        ),
    };
    hints::passive(cx, |_| {
        Tooltip::element(move |window, cx| {
            div()
                .id("shortcut-tooltip")
                .map(gpui_kit::TestSupportExt::test_support)
                .max_w(px(420.).min(window.viewport_size().width - px(48.)))
                .flex()
                .items_center()
                .gap_3()
                .child(
                    div()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(title)
                        .child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child(description),
                        ),
                )
                .when_some(
                    Kbd::binding_for_action(action.as_ref(), Some("Shell"), window),
                    |hint, shortcut| hint.child(shortcuts::keycap(shortcut, cx)),
                )
        })
    })
}

fn separated_status_item(
    id: &'static str,
    cx: &gpui_kit::App,
) -> gpui_kit::Stateful<gpui_kit::Div> {
    div()
        .id(id)
        .flex()
        .items_center()
        .border_l_1()
        .border_color(cx.theme().border)
        .pl_0p5()
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

fn advice_hover_color(cx: &gpui_kit::App) -> gpui_kit::Hsla {
    let color = advice_color(cx);
    gpui_kit::hsla(color.h, color.s, (color.l + 0.24).min(1.0), color.a)
}

#[cfg(test)]
#[path = "settings_ui_test.rs"]
mod tests;

#[cfg(test)]
include!("settings_ui_support_test.rs");
