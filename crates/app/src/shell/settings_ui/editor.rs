//! The analysis settings the pinned FFT hint edits, in the hint's own look.
//!
//! Every value is a standard select or number input drawn without its frame, so
//! the surface reads as the hint it is. A choice previews as soon as it is made,
//! a number on Enter, blur or a step, and the hint's own lifecycle decides
//! whether the previewed settings are kept or restored.

use super::super::numeric_editor::{self, NumericEditor};
use super::super::property_advice::{self, Advice};
use super::*;
use gpui_kit::base::actions::Confirm;
use gpui_kit::component::button::ButtonCustomVariant;
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::component::select::{Select, SelectEvent, SelectItem, SelectState};
use gpui_kit::{Entity, Focusable, rems};
use std::rc::Rc;

/// The key context of the editor inside the hint.
const CONTEXT: &str = "AnalysisEditor";

pub(super) fn init(cx: &mut gpui_kit::App) {
    numeric_editor::init(cx);
}

/// A value of the hint that holds the keyboard.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Stop {
    Choice(Choice),
    Number(Number),
}

/// The width of the surface, fixed so that changing values never move it.
const WIDTH_REM: f32 = 26.0;

/// Font-relative room for the longest shared property label, Reference frequency.
const LABEL_REM: f32 = 9.0;

/// The width of a list, which is wider than any value it offers.
const LIST: f32 = 220.0;
/// The width of the text in a number field, enough for any value it takes.
const NUMBER: f32 = 44.0;

#[cfg(test)]
include!("editor_support_test.rs");

#[derive(Clone, Copy, Debug, PartialEq)]
enum Choice {
    Fft,
    Window,
    Aggregation,
    Mode,
    Palette,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Number {
    Overlap,
    Range,
}

/// A list item whose chosen name stands at the right of its list, beside the chevron.
#[derive(Clone)]
pub(in crate::shell) struct Name(pub(in crate::shell) String);

impl SelectItem for Name {
    type Value = String;

    fn title(&self) -> gpui_kit::SharedString {
        self.0.clone().into()
    }

    fn display_title(&self) -> Option<gpui_kit::AnyElement> {
        let name = div().flex().justify_end().child(self.0.clone());
        Some(name.into_any_element())
    }

    fn value(&self) -> &String {
        &self.0
    }
}

type Combo = Entity<SelectState<Vec<Name>>>;

pub(super) struct Editor {
    owner: WeakEntity<Shell>,
    settings: Settings,
    fft: Combo,
    window: Combo,
    aggregation: Combo,
    mode: Combo,
    palette: Combo,
    /// The width each item of each list takes when shown as the value.
    widths: [Vec<Pixels>; 5],
    overlap: Entity<InputState>,
    range: Entity<InputState>,
    error: Option<String>,
    overlap_editor: Entity<NumericEditor>,
    range_editor: Entity<NumericEditor>,
    /// Each list's handle while it is closed, which an open list no longer answers with.
    closed: [FocusHandle; 5],
    advice: Advice,
    /// Whether the first frame has put the keyboard on the first value.
    placed: bool,
    /// The hint's own scope, which Tab and Shift+Tab never leave.
    scope: FocusHandle,
    /// The keyboard's place on Reset to defaults, marked as the values are.
    defaults_focus: FocusHandle,
    _subscriptions: Vec<Subscription>,
}

impl Editor {
    pub(super) fn new(
        owner: WeakEntity<Shell>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let (settings, effective_range) = owner.upgrade().map_or(
            (Settings::from_config(&Config::default()), 110.0),
            |shell| {
                let shell = shell.read(cx);
                let range = shell
                    .displayed_range()
                    .map_or(110.0, |range| range.effective_db);
                (shell.settings, range)
            },
        );
        let mut subscriptions = Vec::new();
        let mut widths: [Vec<Pixels>; 5] = Default::default();
        let mut select = |choice: Choice, cx: &mut Context<Self>| {
            let (state, shown) = combo(choice, settings, window, cx, &mut subscriptions);
            widths[choice as usize] = shown;
            state
        };
        let fft = select(Choice::Fft, cx);
        let window_choice = select(Choice::Window, cx);
        let aggregation = select(Choice::Aggregation, cx);
        let mode = select(Choice::Mode, cx);
        let palette = select(Choice::Palette, cx);
        for select in [&fft, &window_choice, &aggregation, &mode, &palette] {
            subscriptions.push(cx.observe(select, |_, _, cx| cx.notify()));
        }
        let closed = [&fft, &window_choice, &aggregation, &mode, &palette]
            .map(|state| state.read(cx).focus_handle(cx));
        let (overlap_editor, overlap) = number(
            Number::Overlap,
            crate::numbers::number(settings.overlap),
            window,
            cx,
            &mut subscriptions,
        );
        let db = match settings.dynamic_range {
            DynamicRange::Fixed(db) => db,
            _ => effective_range,
        };
        let (range_editor, range) = number(
            Number::Range,
            crate::numbers::number(db),
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
        Self {
            owner,
            settings,
            fft,
            window: window_choice,
            aggregation,
            mode,
            palette,
            widths,
            overlap,
            range,
            error: None,
            overlap_editor,
            range_editor,
            closed,
            advice: Advice::default(),
            placed: false,
            scope: cx.focus_handle(),
            defaults_focus: cx.focus_handle().tab_stop(true),
            _subscriptions: subscriptions,
        }
    }

    fn stop_repeat(&mut self, cx: &mut Context<Self>) {
        self.overlap_editor
            .update(cx, |editor, _| editor.stop_repeat());
        self.range_editor
            .update(cx, |editor, _| editor.stop_repeat());
    }

    fn toggle_range(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.stop_repeat(cx);
        let dynamic_range = self
            .owner
            .upgrade()
            .and_then(|shell| shell.read(cx).next_range_action());
        if let Some(dynamic_range) = dynamic_range {
            self.apply(
                Settings {
                    dynamic_range,
                    ..self.settings
                },
                window,
                cx,
            );
        }
    }

    fn reset(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.stop_repeat(cx);
        let Some(owner) = self.owner.upgrade() else {
            return;
        };
        self.apply(Settings::from_config(&owner.read(cx).config), window, cx);
    }

    fn update_effective_range(
        &mut self,
        range: Option<DisplayedRange>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if matches!(self.settings.dynamic_range, DynamicRange::Fixed(_)) {
            return;
        }
        let Some(range) = range else {
            return;
        };
        let value = crate::numbers::number(range.effective_db);
        if self.range.read(cx).value().as_ref() != value {
            self.range
                .update(cx, |s, cx| s.set_value(value, window, cx));
        }
    }

    fn sync(&mut self, settings: Settings, window: &mut Window, cx: &mut Context<Self>) {
        if !matches!(settings.dynamic_range, DynamicRange::Fixed(_)) {
            self.range_editor
                .update(cx, |editor, _| editor.stop_repeat());
        }
        self.settings = settings;
        self.error = None;
        self.sync_choices(window, cx);
        self.overlap.update(cx, |s, cx| {
            s.set_value(crate::numbers::number(settings.overlap), window, cx)
        });
        let db = match settings.dynamic_range {
            DynamicRange::Fixed(db) => db,
            _ => self
                .owner
                .upgrade()
                .and_then(|s| s.read(cx).displayed_range())
                .map_or(110.0, |r| r.effective_db),
        };
        self.range.update(cx, |s, cx| {
            s.set_value(crate::numbers::number(db), window, cx)
        });
    }

    fn sync_choices(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let settings = self.settings;
        for (state, value) in [
            (&self.fft, crate::numbers::number(settings.fft_size)),
            (&self.window, display_name(&settings.window.to_string())),
            (&self.aggregation, settings.aggregation.label().into()),
            (&self.mode, mode_name(settings.dynamic_range).into()),
            (&self.palette, display_name(&settings.colormap.to_string())),
        ] {
            state.update(cx, |state, cx| state.set_selected_value(&value, window, cx));
        }
    }

    fn apply(&mut self, settings: Settings, window: &mut Window, cx: &mut Context<Self>) {
        if settings == self.settings {
            // The fields may still hold text the settings never took, such as a refused number.
            self.sync(settings, window, cx);
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
        // A list can remove the stepper a repeat was started on.
        self.stop_repeat(cx);
        let mut settings = self.settings;
        match choice {
            Choice::Fft => {
                if let Ok(value) = crate::numbers::parse(value) {
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
                    _ => DynamicRange::Fixed(
                        crate::numbers::parse(self.range.read(cx).value().as_ref())
                            .unwrap_or(110.0),
                    ),
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

    /// Preview the number fields, reporting whether they hold usable values.
    fn edit_number(&mut self, field: Number, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let fixed = matches!(self.settings.dynamic_range, DynamicRange::Fixed(_));
        if matches!(field, Number::Range) && !fixed {
            return true;
        }
        let overlap = self.overlap.read(cx).value().to_string();
        let range = fixed.then(|| self.range.read(cx).value().to_string());
        match self.settings.edited_numbers(&overlap, range.as_deref()) {
            Ok(settings) => {
                self.apply(settings, window, cx);
                self.error.is_none()
            }
            Err(error) => {
                self.error = Some(error);
                cx.notify();
                false
            }
        }
    }

    /// The title with the pending state beside it, or an error in its place, always one fixed line.
    fn heading(&self, pending: bool, cx: &mut Context<Self>) -> impl IntoElement {
        let line = div().h_6().mb_1().flex().items_center();
        if let Some(error) = &self.error {
            return line
                .text_xs()
                .text_color(cx.theme().danger)
                .child(error.clone());
        }
        let title = div()
            .font_weight(FontWeight::SEMIBOLD)
            .child("Analysis settings");
        let updating = div()
            .text_xs()
            .text_color(cx.theme().muted_foreground)
            .child("Updating the picture…");
        line.justify_between()
            .child(title)
            .when(pending, |line| line.child(updating))
    }

    /// Resetting to the configuration, below the values it replaces.
    fn defaults(&self, window: &Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Enter on the focused button resets, where elsewhere it would close the hint.
        let enter = cx.listener(|editor, _: &Confirm, window, cx| editor.reset(window, cx));
        let focused = self.defaults_focus.is_focused(window);
        div()
            .mt_1()
            .pt_1()
            .track_focus(&self.defaults_focus)
            .on_action(enter)
            .child(
                outlined_button(
                    "analysis-defaults",
                    DEFAULTS,
                    "Reset to defaults",
                    focused,
                    true,
                    cx,
                )
                .tab_stop(false)
                .on_click(cx.listener(|editor, _, window, cx| editor.reset(window, cx))),
            )
    }

    fn combo(&self, choice: Choice) -> &Combo {
        match choice {
            Choice::Fft => &self.fft,
            Choice::Window => &self.window,
            Choice::Aggregation => &self.aggregation,
            Choice::Mode => &self.mode,
            Choice::Palette => &self.palette,
        }
    }

    fn number_state(&self, field: Number) -> &Entity<InputState> {
        match field {
            Number::Overlap => &self.overlap,
            Number::Range => &self.range,
        }
    }

    /// The hint's values, Range only while it is a number.
    fn stops(&self) -> Vec<Stop> {
        let fixed = matches!(self.settings.dynamic_range, DynamicRange::Fixed(_));
        let range = fixed.then_some(Stop::Number(Number::Range));
        [
            Some(Stop::Choice(Choice::Fft)),
            Some(Stop::Choice(Choice::Window)),
            Some(Stop::Number(Number::Overlap)),
            Some(Stop::Choice(Choice::Aggregation)),
            Some(Stop::Choice(Choice::Mode)),
            range,
            Some(Stop::Choice(Choice::Palette)),
        ]
        .into_iter()
        .flatten()
        .collect()
    }

    /// Whether `stop` holds the keyboard, a list counting while it is open.
    fn holds(&self, stop: Stop, window: &Window, cx: &gpui_kit::App) -> bool {
        match stop {
            Stop::Choice(choice) => self
                .combo(choice)
                .read(cx)
                .focus_handle(cx)
                .is_focused(window),
            Stop::Number(field) => self.number_state(field).focus_handle(cx).is_focused(window),
        }
    }

    fn focused(&self, window: &Window, cx: &gpui_kit::App) -> Option<Stop> {
        self.stops()
            .into_iter()
            .find(|&stop| self.holds(stop, window, cx))
    }

    /// Moves the keyboard to the window's next or previous stop that lies in the hint.
    fn traverse(&mut self, forward: bool, window: &mut Window, cx: &mut Context<Self>) {
        numeric_editor::traverse(
            &self.scope,
            &[self.overlap_editor.clone(), self.range_editor.clone()],
            self.stops().len() + 2,
            forward,
            window,
            cx,
        );
        cx.notify();
    }

    /// Tab, Shift+Tab, Up and Down within the hint.
    fn arrows(
        editor: gpui_kit::Stateful<gpui_kit::Div>,
        cx: &mut Context<Self>,
    ) -> gpui_kit::Stateful<gpui_kit::Div> {
        editor
            .on_action(
                cx.listener(|editor, _: &FocusNext, window, cx| editor.traverse(true, window, cx)),
            )
            .on_action(cx.listener(|editor, _: &FocusPrevious, window, cx| {
                editor.traverse(false, window, cx)
            }))
    }

    pub(super) fn open_select_trigger(&self, cx: &gpui_kit::App) -> Option<FocusHandle> {
        [
            &self.fft,
            &self.window,
            &self.aggregation,
            &self.mode,
            &self.palette,
        ]
        .iter()
        .zip(&self.closed)
        .find_map(|(state, closed)| {
            (state.read(cx).focus_handle(cx) != *closed).then(|| closed.clone())
        })
    }

    /// Whether any list of the hint is open, which the balloon would cover.
    fn list_open(&self, cx: &gpui_kit::App) -> bool {
        [
            &self.fft,
            &self.window,
            &self.aggregation,
            &self.mode,
            &self.palette,
        ]
        .iter()
        .zip(&self.closed)
        .any(|(state, closed)| select_open(state, closed, cx))
    }

    /// The effective range, with its warning in a balloon pointing at the sign that ends it.
    fn range_readout(&self, window: &Window, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let text = format!("{} dB", self.range.read(cx).value());
        let warning = self
            .owner
            .upgrade()
            .and_then(|shell| shell.read(cx).range_warning());
        let Some((warning, actionable)) = warning else {
            self.advice.deactivate();
            return div().child(text).into_any_element();
        };
        let edge = advice_color(cx);
        let shortcut = actionable
            .then(|| Kbd::binding_for_action(&UseRecommendedRange, Some("Shell"), window))
            .flatten();
        let action = actionable.then(|| property_advice::Action {
            click: Rc::new(cx.listener(|editor, _, window, cx| editor.toggle_range(window, cx))),
            keycap: shortcut.map(|shortcut| shortcuts::keycap(shortcut, cx).into_any_element()),
        });
        self.advice.render(
            div().child(format!("{text} ⚠")),
            property_advice::Content {
                anchor: "analysis-range-advice",
                balloon: "analysis-range-balloon",
                close: "analysis-range-balloon-close",
                pointer: "analysis-range-balloon-pointer",
                warning: warning.clone(),
                edge,
                action,
                hover: Rc::new(move |_, cx| {
                    let action = actionable
                        .then(|| Box::new(UseRecommendedRange) as Box<dyn gpui_kit::Action>);
                    shortcut_tooltip(warning.clone(), action, "Shell", px(320.), cx)
                }),
            },
            self.list_open(cx),
            cx.entity_id(),
            window,
            cx,
        )
    }

    /// A list as wide as its widest item, underlined only under the value it shows.
    fn choice(
        &self,
        id: &'static str,
        choice: Choice,
        window: &Window,
        cx: &gpui_kit::App,
    ) -> impl IntoElement {
        choice_control(
            id,
            self.combo(choice),
            &self.widths[choice as usize],
            None,
            window,
            cx,
        )
    }
}

impl Render for Editor {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let pending = self.owner.upgrade().is_some_and(|shell| {
            let shell = shell.read(cx);
            shell.file.as_ref().is_some_and(|f| {
                f.state.displayed_settings() != Some(&shell.settings)
                    && !matches!(f.state.document.status(), Status::Failed(_))
            })
        });
        let fixed = matches!(self.settings.dynamic_range, DynamicRange::Fixed(_));
        let overlap = self.overlap_editor.clone().into_any_element();
        let range = if fixed {
            self.advice.deactivate();
            self.range_editor.clone().into_any_element()
        } else {
            self.range_readout(window, cx)
        };
        let measure = self.advice.sheet(cx.entity_id());
        // The keyboard comes to the first value once the popover has taken it for itself.
        if !self.placed {
            self.placed = true;
            let first = self.closed[Choice::Fft as usize].clone();
            window.defer(cx, move |window, cx| first.focus(window, cx));
        }
        div()
            .id("analysis-editor")
            .relative()
            .child(measure)
            .key_context(CONTEXT)
            .track_focus(&self.scope)
            .map(|editor| Self::arrows(editor, cx))
            .map(gpui_kit::TestSupportExt::test_support)
            .on_action(cx.listener(|editor, _: &UseRecommendedRange, window, cx| {
                editor.toggle_range(window, cx)
            }))
            // Enter belongs to a focused list or number, and elsewhere closes the hint on usable numbers.
            .on_action(cx.listener(|editor, _: &Confirm, window, cx| {
                let usable = editor.edit_number(Number::Overlap, window, cx);
                if usable && editor.focused(window, cx).is_none() {
                    cx.propagate();
                }
            }))
            .w(property_width(window))
            .font_family(cx.theme().font_family.clone())
            .bg(crate::theme::sheet(cx))
            .text_color(cx.theme().popover_foreground)
            .border_1()
            .border_color(cx.theme().border)
            .shadow_md()
            .rounded(cx.theme().radius)
            .px_3()
            .py_2()
            .text_sm()
            .flex()
            .flex_col()
            .gap_1()
            .child(self.heading(pending, cx))
            .child(row(
                "FFT size",
                self.choice("analysis-fft", Choice::Fft, window, cx),
                cx,
            ))
            .child(row(
                "Window",
                self.choice("analysis-window", Choice::Window, window, cx),
                cx,
            ))
            .child(row("Overlap", overlap, cx))
            .child(row(
                "Aggregation",
                self.choice("analysis-aggregation", Choice::Aggregation, window, cx),
                cx,
            ))
            .child(row(
                "Range mode",
                self.choice("analysis-mode", Choice::Mode, window, cx),
                cx,
            ))
            .child(row("Range", range, cx))
            .child(row(
                "Colour scheme",
                self.choice("analysis-palette", Choice::Palette, window, cx),
                cx,
            ))
            .child(self.defaults(window, cx))
    }
}

fn mode_name(range: DynamicRange) -> &'static str {
    match range {
        DynamicRange::Default => "Absolute full scale",
        DynamicRange::Fixed(_) => "Below measured peak",
        DynamicRange::Auto => "Automatic",
    }
}

/// Frameless numeric input and unit, with the same editable mark as choices.
pub(in crate::shell) fn number_control(
    state: &Entity<InputState>,
    width: Pixels,
    unit: &'static str,
    focused: bool,
    disabled: bool,
    cx: &gpui_kit::App,
) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap_1()
        .child(
            div()
                .border_b_1()
                .map(|line| mark(line, focused, cx))
                .child(
                    Input::new(state)
                        .disabled(disabled)
                        .appearance(false)
                        .small()
                        .w(width)
                        .text_align(gpui_kit::TextAlign::Right),
                ),
        )
        .child(div().text_color(cx.theme().muted_foreground).child(unit))
}

pub(in crate::shell) fn property_width(window: &Window) -> Pixels {
    (window.rem_size() * WIDTH_REM)
        .min((window.viewport_size().width - window.rem_size() * 3.).max(px(0.)))
}

pub(in crate::shell) fn value_column_width(window: &Window) -> Pixels {
    (property_width(window) - window.rem_size() * (1.5 + LABEL_REM + 1.)).max(px(0.))
}

pub(in crate::shell) fn number_width(value: &str, window: &Window, cx: &gpui_kit::App) -> Pixels {
    let widths = shown_widths(&[value.to_owned()], window, cx);
    // Reserve the small Input's horizontal text padding, without a Select chevron.
    (widths[0] - choice_chrome(window) + px(16.)).max(px(NUMBER))
}

/// Existing frameless Select presentation shared by both property editors.
pub(in crate::shell) fn choice_control<D>(
    id: &'static str,
    state: &Entity<SelectState<D>>,
    widths: &[Pixels],
    placeholder: Option<&str>,
    window: &Window,
    cx: &gpui_kit::App,
) -> impl IntoElement
where
    D: gpui_kit::component::searchable_list::SearchableListDelegate + 'static,
    D::Item: gpui_kit::component::searchable_list::SearchableListItem<Value = String>,
{
    let focused = state.read(cx).focus_handle(cx).is_focused(window);
    let widest = widths.iter().copied().fold(px(0.), Pixels::max);
    let shown = state
        .read(cx)
        .selected_value()
        .map(String::as_str)
        .or(placeholder)
        .and_then(|value| {
            shown_widths(&[value.to_owned()], window, cx)
                .first()
                .copied()
        })
        .unwrap_or(widest);
    let underline = div()
        .absolute()
        .bottom_0()
        .right_0()
        .w(shown)
        .border_b_1()
        .map(|line| mark(line, focused, cx));
    let list = Select::new(state)
        .id(id)
        .appearance(false)
        .small()
        .when_some(placeholder, |list, value| {
            list.placeholder(value.to_owned())
        })
        .w_full()
        .menu_width(widest.max(px(LIST)))
        .menu_max_h((window.viewport_size().height - px(48.)).max(px(0.)));
    div().relative().w(shown).child(list).child(underline)
}

pub(in crate::shell) fn select_open<D>(
    state: &Entity<SelectState<D>>,
    closed: &FocusHandle,
    cx: &gpui_kit::App,
) -> bool
where
    D: gpui_kit::component::searchable_list::SearchableListDelegate + 'static,
    D::Item: gpui_kit::component::searchable_list::SearchableListItem<Value = String>,
{
    state.read(cx).focus_handle(cx) != *closed
}

pub(in crate::shell) fn outlined_button(
    id: &'static str,
    group: &'static str,
    label: &'static str,
    focused: bool,
    enabled: bool,
    cx: &gpui_kit::App,
) -> Button {
    let frame = if focused {
        super::super::app_menu_ui::toolbar_accent(cx)
    } else {
        cx.theme().muted_foreground.opacity(0.45)
    };
    Button::new(id)
        .custom(
            ButtonCustomVariant::new(cx)
                .color(cx.theme().popover)
                .hover(cx.theme().accent)
                .active(cx.theme().border),
        )
        .disabled(!enabled)
        .group(group)
        .when(focused, |button| button.border_2())
        .when(!focused, |button| button.border_1())
        .border_color(frame)
        .small()
        .child(
            div()
                .id(group)
                .text_color(cx.theme().muted_foreground)
                .when(enabled, |ink| {
                    ink.group_hover(group, |style| style.text_color(cx.theme().foreground))
                        .group_active(group, |style| {
                            style.text_color(cx.theme().muted_foreground.opacity(0.6))
                        })
                })
                .child(label),
        )
}

/// A label and its value, in the metadata hint's layout.
pub(in crate::shell) fn row(
    label: &'static str,
    value: impl IntoElement,
    cx: &gpui_kit::App,
) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap_4()
        .w_full()
        .min_h_7()
        .child(
            div()
                .w(rems(LABEL_REM))
                .flex_shrink_0()
                .whitespace_nowrap()
                .text_color(cx.theme().muted_foreground)
                .child(label),
        )
        .child(div().flex_1().min_w_0().flex().justify_end().child(value))
}

const DEFAULTS: &str = "analysis-defaults";

/// The ink of a button's sign or text, brighter under the pointer and dimmer while held.
pub(in crate::shell) fn pointer_ink(
    group: &'static str,
    cx: &gpui_kit::App,
) -> gpui_kit::Stateful<gpui_kit::Div> {
    let rest = cx.theme().muted_foreground;
    let lit = cx.theme().foreground;
    div()
        .id(group)
        .text_color(rest)
        .group_hover(group, move |style| style.text_color(lit))
        .group_active(group, move |style| style.text_color(rest.opacity(0.6)))
}

/// The underline of a value, dashed at rest and solid in the accent while it holds the keyboard.
pub(in crate::shell) fn mark(
    line: gpui_kit::Div,
    focused: bool,
    cx: &gpui_kit::App,
) -> gpui_kit::Div {
    if focused {
        line.border_b_2()
            .border_color(super::super::app_menu_ui::toolbar_accent(cx))
    } else {
        line.border_dashed().border_color(underline_color(cx))
    }
}

fn underline_color(cx: &gpui_kit::App) -> gpui_kit::Hsla {
    cx.theme().muted_foreground.opacity(0.6)
}

fn combo(
    choice: Choice,
    settings: Settings,
    window: &mut Window,
    cx: &mut Context<Editor>,
    subscriptions: &mut Vec<Subscription>,
) -> (Combo, Vec<Pixels>) {
    let (selected, items): (String, Vec<String>) = match choice {
        Choice::Fft => (
            crate::numbers::number(settings.fft_size),
            (1..=crate::settings::MAX_FFT_SIZE.ilog2())
                .map(|n| crate::numbers::number(1usize << n))
                .collect(),
        ),
        Choice::Window => (
            display_name(&settings.window.to_string()),
            argand_dsp::WINDOW_NAMES
                .iter()
                .map(|s| display_name(s))
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
            display_name(&settings.colormap.to_string()),
            argand_core::COLORMAP_NAMES
                .iter()
                .map(|s| display_name(s))
                .collect(),
        ),
    };
    let index = items
        .iter()
        .position(|value| value == &selected)
        .map(gpui_kit::component::IndexPath::new);
    let shown = shown_widths(&items, window, cx);
    let items: Vec<Name> = items.into_iter().map(Name).collect();
    let state = cx.new(|cx| SelectState::new(items, index, window, cx));
    subscriptions.push(
        cx.subscribe_in(&state, window, move |editor, _, event, window, cx| {
            if let SelectEvent::Confirm(Some(value)) = event {
                editor.choose(choice, value, window, cx);
            }
        }),
    );
    (state, shown)
}

// Stock small Select: two 1px borders, horizontal padding, gap_1 and a size_3p5 caret.
fn choice_chrome(window: &Window) -> Pixels {
    gpui_kit::component::Size::Small.input_px() * 2. + px(2.) + window.rem_size() * 1.125
}

/// The width a frameless select takes to show each of its items with the chevron.
pub(in crate::shell) fn shown_widths(
    items: &[String],
    window: &Window,
    cx: &gpui_kit::App,
) -> Vec<Pixels> {
    let style = gpui_kit::TextStyle {
        font_family: cx.theme().font_family.clone(),
        ..Default::default()
    };
    let size = window.rem_size() * 0.875;
    let text = window.text_system();
    items
        .iter()
        .map(|item| {
            let run = style.to_run(item.len());
            let line = text.shape_line(item.clone().into(), size, &[run], None);
            line.width.ceil() + choice_chrome(window)
        })
        .collect()
}

fn number(
    field: Number,
    value: String,
    window: &mut Window,
    cx: &mut Context<Editor>,
    subscriptions: &mut Vec<Subscription>,
) -> (Entity<NumericEditor>, Entity<InputState>) {
    let (unit, minimum, maximum, minimum_inclusive, down, up) = match field {
        Number::Overlap => ("%", 0.0, 95.0, true, "overlap-down", "overlap-up"),
        Number::Range => ("dB", 0.0, f64::MAX, false, "range-down", "range-up"),
    };
    let numeric = cx.new(|cx| {
        NumericEditor::new(
            value,
            numeric_editor::Config {
                integer: false,
                unit,
                step: 1.0,
                minimum,
                maximum,
                minimum_inclusive,
                down,
                up,
            },
            window,
            cx,
        )
    });
    let input = numeric.read(cx).input.clone();
    subscriptions.push(cx.subscribe_in(
        &numeric,
        window,
        move |editor, _, event: &numeric_editor::Changed, window, cx| {
            if event.committed {
                editor.edit_number(field, window, cx);
            }
            cx.notify();
        },
    ));
    (numeric, input)
}

#[cfg(test)]
#[path = "editor_hint_test.rs"]
mod hint_tests;

/// A window or palette name as the interface shows it, which parses back unchanged.
pub(super) fn display_name(name: &str) -> String {
    if name == "rect" {
        return "Rectangular".into();
    }
    name.split('-')
        .map(|part| {
            let mut chars = part.chars();
            chars.next().map_or_else(String::new, |first| {
                first.to_uppercase().chain(chars).collect()
            })
        })
        .collect::<Vec<_>>()
        .join("-")
}
