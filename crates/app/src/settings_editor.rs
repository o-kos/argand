//! The analysis settings the pinned FFT hint edits, in the hint's own look.
//!
//! Every value is a standard select or number input drawn without its frame, so
//! the surface reads as the hint it is. A choice previews as soon as it is made,
//! a number on Enter, blur or a step, and the hint's own lifecycle decides
//! whether the previewed settings are kept or restored.

use super::*;
use gpui_kit::base::actions::Confirm;
use gpui_kit::component::IconName;
use gpui_kit::component::input::{Input, InputEvent, InputState, StepAction};
use gpui_kit::component::select::{Select, SelectEvent, SelectItem, SelectState};
use gpui_kit::{Entity, Focusable};
use std::time::Duration;

/// The key context of the editor inside the hint.
const CONTEXT: &str = "AnalysisEditor";

/// The width of the surface, fixed so that changing values never move it.
const WIDTH: f32 = 370.0;

/// The width of every row's label column.
const LABEL: f32 = 110.0;

/// What a frameless select adds to its text, which is its padding and chevron.
const CHEVRON: f32 = 34.0;

/// The width of a list, which is wider than any value it offers.
const LIST: f32 = 220.0;
/// The widest the balloon's body grows, its pointer's length and the tip's distance from the sign.
const BALLOON: f32 = 320.0;
const POINTER: f32 = 9.0;
const BALLOON_GAP: f32 = 3.0;

/// The width of the text in a number field, enough for any value it takes.
const NUMBER: f32 = 44.0;

/// How long a stepper is held before it starts repeating.
const REPEAT_DELAY: Duration = Duration::from_millis(400);

/// How often a held stepper repeats.
const REPEAT_EVERY: Duration = Duration::from_millis(80);

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

/// A list item whose chosen name stands at the right of its list, beside the chevron.
#[derive(Clone)]
struct Name(String);

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
    /// The repeating step of a held stepper, dropped when it is released.
    repeat: Option<Task<()>>,
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
        let overlap = number(
            Number::Overlap,
            crate::numbers::input(settings.overlap),
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
            crate::numbers::input(db),
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
            repeat: None,
            _subscriptions: subscriptions,
        }
    }

    /// Step once at the press, then keep stepping while the stepper stays held.
    fn start_repeat(
        &mut self,
        field: Number,
        step: StepAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.edit_number(field, Some(step), window, cx) {
            return;
        }
        self.repeat = Some(cx.spawn_in(window, async move |editor, cx| {
            cx.background_executor().timer(REPEAT_DELAY).await;
            loop {
                let stepped = editor.update_in(cx, |editor, window, cx| {
                    editor.edit_number(field, Some(step), window, cx)
                });
                if !matches!(stepped, Ok(true)) {
                    break;
                }
                cx.background_executor().timer(REPEAT_EVERY).await;
            }
        }));
    }

    fn stop_repeat(&mut self) {
        self.repeat = None;
    }

    /// A number with its unit between two steppers that act on press and repeat while held.
    fn stepper(
        &self,
        field: Number,
        unit: &'static str,
        cx: &mut Context<Self>,
    ) -> gpui_kit::AnyElement {
        let state = match field {
            Number::Overlap => &self.overlap,
            Number::Range => &self.range,
        };
        let (down, up) = match field {
            Number::Overlap => ("overlap-down", "overlap-up"),
            Number::Range => ("range-down", "range-up"),
        };
        div()
            .flex()
            .items_center()
            .gap_1()
            .child(self.step_button(down, IconName::Minus, field, StepAction::Decrement, cx))
            .child(editable(
                Input::new(state)
                    .appearance(false)
                    .small()
                    .w(px(NUMBER))
                    .text_align(gpui_kit::TextAlign::Right),
                cx,
            ))
            .child(div().text_color(cx.theme().muted_foreground).child(unit))
            .child(self.step_button(up, IconName::Plus, field, StepAction::Increment, cx))
            .into_any_element()
    }

    fn step_button(
        &self,
        id: &'static str,
        icon: IconName,
        field: Number,
        step: StepAction,
        cx: &mut Context<Self>,
    ) -> Button {
        Button::new(id)
            .ghost()
            .xsmall()
            .icon(icon)
            .tab_stop(false)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |editor, _, window, cx| {
                    editor.start_repeat(field, step, window, cx);
                }),
            )
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|editor, _, _, _| editor.stop_repeat()),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|editor, _, _, _| editor.stop_repeat()),
            )
    }

    fn toggle_range(&mut self, window: &mut Window, cx: &mut Context<Self>) {
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
        let value = crate::numbers::input(range.effective_db);
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
            s.set_value(crate::numbers::input(settings.overlap), window, cx)
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
            s.set_value(crate::numbers::input(db), window, cx)
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
    fn edit_number(
        &mut self,
        field: Number,
        step: Option<StepAction>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let fixed = matches!(self.settings.dynamic_range, DynamicRange::Fixed(_));
        if matches!(field, Number::Range) && !fixed {
            return true;
        }
        let mut overlap = self.overlap.read(cx).value().to_string();
        let mut range = fixed.then(|| self.range.read(cx).value().to_string());
        if let Some(step) = step {
            let text = match field {
                Number::Overlap => &mut overlap,
                Number::Range => range.get_or_insert_with(String::new),
            };
            let Ok(value) = crate::numbers::parse::<f32>(text) else {
                self.error = Some("Enter a number".into());
                cx.notify();
                return false;
            };
            let delta = if step == StepAction::Increment {
                1.0
            } else {
                -1.0
            };
            *text = crate::numbers::input(value + delta);
        }
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
    fn defaults(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div().mt_1().pt_1().child(
            Button::new("analysis-defaults")
                .outline()
                .small()
                .label("Reset to defaults")
                .on_click(cx.listener(|editor, _, window, cx| editor.reset(window, cx))),
        )
    }

    /// The effective range, with its warning in a balloon pointing at the sign that ends it.
    fn range_readout(&self, window: &Window, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let text = format!("{} dB", self.range.read(cx).value());
        let warning = self
            .owner
            .upgrade()
            .and_then(|shell| shell.read(cx).range_warning());
        let Some((warning, actionable)) = warning else {
            return div().child(text).into_any_element();
        };
        let shortcut = actionable
            .then(|| Kbd::binding_for_action(&UseRecommendedRange, Some("Shell"), window))
            .flatten();
        // The warning colour edges the balloon, which a border of the hint's own would lose on the hint.
        let edge = advice_color(cx);
        let body = div()
            .id("analysis-range-balloon")
            .occlude()
            .when(actionable, |body| {
                body.cursor_pointer()
                    .on_click(cx.listener(|editor, _, window, cx| editor.toggle_range(window, cx)))
            })
            .ml(px(POINTER - 1.0))
            .max_w(px(BALLOON))
            .flex()
            .items_center()
            .gap_3()
            .px_2()
            .py_0p5()
            .bg(cx.theme().tokens.popover)
            .border_1()
            .border_color(edge)
            .rounded(cx.theme().radius)
            .shadow_md()
            .text_color(cx.theme().popover_foreground)
            .child(div().min_w_0().child(warning))
            .when_some(shortcut, |body, shortcut| {
                body.child(shortcuts::keycap(shortcut, cx))
            });
        let pointer = div()
            .absolute()
            .left_0()
            .top_0()
            .bottom_0()
            .flex()
            .items_center()
            .child(balloon_pointer(cx.theme().tokens.popover, edge));
        // The pointer comes last so it is painted over the border of the body it joins.
        let balloon = div()
            .absolute()
            .top_0()
            .bottom_0()
            .left_full()
            .ml(px(BALLOON_GAP))
            .w(px(BALLOON + POINTER))
            .flex()
            .items_center()
            .child(body)
            .child(pointer);
        div()
            .id("analysis-range-advice")
            .relative()
            .text_color(edge)
            .when(actionable, |value| {
                value
                    .border_b_1()
                    .border_dashed()
                    .border_color(edge.opacity(0.6))
                    .cursor_pointer()
                    .on_click(cx.listener(|editor, _, window, cx| editor.toggle_range(window, cx)))
            })
            .child(format!("{text} ⚠"))
            .child(gpui_kit::deferred(balloon).with_priority(gpui_kit::base::POPUP_PRIORITY + 1))
            .into_any_element()
    }

    /// A list as wide as its widest item, underlined only under the value it shows.
    fn choice(&self, id: &'static str, choice: Choice, cx: &gpui_kit::App) -> impl IntoElement {
        let state = match choice {
            Choice::Fft => &self.fft,
            Choice::Window => &self.window,
            Choice::Aggregation => &self.aggregation,
            Choice::Mode => &self.mode,
            Choice::Palette => &self.palette,
        };
        let widths = &self.widths[choice as usize];
        let widest = widths.iter().copied().fold(px(0.), Pixels::max);
        // The list keeps its place while the keyboard walks it, and the value shown follows the walk.
        let shown = state
            .read(cx)
            .selected_index(cx)
            .and_then(|index| widths.get(index.row).copied())
            .unwrap_or(widest);
        let underline = div()
            .absolute()
            .bottom_0()
            .right_0()
            .w(shown)
            .border_b_1()
            .border_dashed()
            .border_color(underline_color(cx));
        let list = Select::new(state)
            .id(id)
            .appearance(false)
            .small()
            .w_full()
            .menu_width(px(LIST));
        div().relative().w(widest).child(list).child(underline)
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
        let fixed = matches!(self.settings.dynamic_range, DynamicRange::Fixed(_));
        let overlap = self.stepper(Number::Overlap, "%", cx);
        let range = if fixed {
            self.stepper(Number::Range, "dB", cx)
        } else {
            self.range_readout(window, cx)
        };
        div()
            .id("analysis-editor")
            .key_context(CONTEXT)
            .on_action(cx.listener(|editor, _: &UseRecommendedRange, window, cx| {
                editor.toggle_range(window, cx)
            }))
            // Enter in a number applies it and keeps the hint, and elsewhere closes it on usable numbers.
            .on_action(cx.listener(|editor, _: &Confirm, window, cx| {
                let usable = editor.edit_number(Number::Overlap, None, window, cx);
                let in_number = editor.overlap.focus_handle(cx).is_focused(window)
                    || editor.range.focus_handle(cx).is_focused(window);
                if usable && !in_number {
                    cx.propagate();
                }
            }))
            .w(px(WIDTH).min(window.viewport_size().width - px(48.)))
            .font_family(cx.theme().font_family.clone())
            .bg(cx.theme().tokens.popover)
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
                self.choice("analysis-fft", Choice::Fft, cx),
                cx,
            ))
            .child(row(
                "Window",
                self.choice("analysis-window", Choice::Window, cx),
                cx,
            ))
            .child(row("Overlap", overlap, cx))
            .child(row(
                "Aggregation",
                self.choice("analysis-aggregation", Choice::Aggregation, cx),
                cx,
            ))
            .child(row(
                "Range mode",
                self.choice("analysis-mode", Choice::Mode, cx),
                cx,
            ))
            .child(row("Range", range, cx))
            .child(row(
                "Colour scheme",
                self.choice("analysis-palette", Choice::Palette, cx),
                cx,
            ))
            .child(self.defaults(cx))
    }
}

fn mode_name(range: DynamicRange) -> &'static str {
    match range {
        DynamicRange::Default => "Absolute full scale",
        DynamicRange::Fixed(_) => "Below measured peak",
        DynamicRange::Auto => "Automatic",
    }
}

/// A label and its value, in the metadata hint's layout.
fn row(label: &'static str, value: impl IntoElement, cx: &gpui_kit::App) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap_4()
        .w_full()
        .min_h_7()
        .child(
            div()
                .w(px(LABEL))
                .flex_shrink_0()
                .text_color(cx.theme().muted_foreground)
                .child(label),
        )
        .child(div().flex_1().min_w_0().flex().justify_end().child(value))
}

/// A value that opens a list or takes a number, marked by a dashed underline.
fn editable(control: impl IntoElement, cx: &gpui_kit::App) -> impl IntoElement {
    div()
        .border_b_1()
        .border_dashed()
        .border_color(underline_color(cx))
        .child(control)
}

/// The pointer on the balloon's left side, its tip towards the warning sign.
fn balloon_pointer(fill: impl Into<gpui_kit::Hsla>, edge: gpui_kit::Hsla) -> impl IntoElement {
    let fill = fill.into();
    gpui_kit::canvas(
        |_, _, _| (),
        move |bounds, _, window, _| {
            let tip = gpui_kit::point(bounds.left(), bounds.center().y);
            let upper = gpui_kit::point(bounds.right(), bounds.top());
            let lower = gpui_kit::point(bounds.right(), bounds.bottom());
            let mut body = gpui_kit::PathBuilder::fill();
            body.add_polygon(&[upper, tip, lower], true);
            if let Ok(path) = body.build() {
                window.paint_path(path, fill);
            }
            let mut outline = gpui_kit::PathBuilder::stroke(px(1.0));
            outline.add_polygon(&[upper, tip, lower], false);
            if let Ok(path) = outline.build() {
                window.paint_path(path, edge);
            }
        },
    )
    .w(px(POINTER))
    .h(px(16.0))
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

/// The width a frameless select takes to show each of its items with the chevron.
fn shown_widths(items: &[String], window: &Window, cx: &gpui_kit::App) -> Vec<Pixels> {
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
            line.width.ceil() + px(CHEVRON)
        })
        .collect()
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
        cx.subscribe_in(
            &state,
            window,
            move |editor, _, event, window, cx| match event {
                InputEvent::PressEnter { .. } | InputEvent::Blur => {
                    editor.edit_number(field, None, window, cx);
                }
                _ => {}
            },
        ),
    );
    state
}

#[cfg(test)]
mod standard_input_tests {
    use super::*;
    use gpui_kit::component::input::NumberInput;
    use gpui_kit::test::TestWindowExt;
    use gpui_kit::{App, TestAppContext, WindowHandle};

    type Handle = WindowHandle<Root>;

    gpui_kit::actions!(standard_input_tests, [OuterEscape]);

    /// An outer context whose escape stands for the surface around the controls.
    const OUTER: &str = "StandardInputs";

    /// A window's worth of standard controls, with nothing of Argand's own.
    struct Form {
        number: Entity<InputState>,
        select: Entity<SelectState<Vec<String>>>,
        /// How often the outer context saw an escape.
        outer_escape: usize,
    }

    impl Form {
        fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
            let number = cx.new(|cx| InputState::new(window, cx).default_value("50"));
            let select = cx.new(|cx| {
                SelectState::new(
                    ["1024", "2048", "4096"]
                        .into_iter()
                        .map(String::from)
                        .collect::<Vec<String>>(),
                    Some(gpui_kit::component::IndexPath::new(0)),
                    window,
                    cx,
                )
            });
            Self {
                number,
                select,
                outer_escape: 0,
            }
        }
    }

    impl Render for Form {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .flex_col()
                .gap_4()
                .p_4()
                .key_context(OUTER)
                .on_action(cx.listener(|form, _: &OuterEscape, _, _| {
                    form.outer_escape += 1;
                }))
                .child(NumberInput::new(&self.number))
                .child(Select::new(&self.select).id("select").w_full())
        }
    }

    /// Opens a window whose top-level entity is a standard `Root`, as the main window is.
    fn open(cx: &mut TestAppContext) -> Handle {
        cx.update(|cx| {
            gpui_kit::init(cx);
            cx.bind_keys([KeyBinding::new("escape", OuterEscape, Some(OUTER))]);
        });
        let handle = cx.add_window(|window, cx| {
            let form = cx.new(|cx| Form::new(window, cx));
            Root::new(form, window, cx)
        });
        frame(cx, handle);
        handle
    }

    fn frame(cx: &mut TestAppContext, handle: Handle) {
        cx.update_window(handle.into(), |_, window, cx| window.render_frame(cx))
            .expect("the window is open");
        cx.run_until_parked();
    }

    /// Reaches the form through the window's `Root`, which is the only way to it.
    fn with_form<R>(
        cx: &mut TestAppContext,
        handle: Handle,
        f: impl FnOnce(&Entity<Form>, &mut Window, &mut App) -> R,
    ) -> R {
        cx.update_window(handle.into(), |_, window, cx| {
            let form = Root::read(window, cx)
                .view()
                .clone()
                .downcast::<Form>()
                .expect("the window's Root owns the form");
            f(&form, window, cx)
        })
        .expect("the window is open")
    }

    /// Clicks the select trigger, which opens the standard list.
    fn open_list(cx: &mut TestAppContext, handle: Handle) {
        cx.update_window(handle.into(), |_, window, cx| {
            window.within("select").click("input", cx);
        })
        .expect("the window is open");
        cx.run_until_parked();
    }

    /// The number input's text as the window holds it now.
    fn number(cx: &mut TestAppContext, handle: Handle) -> String {
        with_form(cx, handle, |form, _, cx| {
            form.read(cx).number.read(cx).value().to_string()
        })
    }

    /// How many escapes reached the outer context instead of the select.
    fn outer_escape(cx: &mut TestAppContext, handle: Handle) -> usize {
        with_form(cx, handle, |form, _, cx| form.read(cx).outer_escape)
    }

    /// Whether the window is still open and the keyboard is on this handle.
    fn focused(cx: &mut TestAppContext, handle: Handle, target: &FocusHandle) -> bool {
        with_form(cx, handle, |_, window, _| target.is_focused(window))
    }

    /// The number input's own focus handle.
    fn typed_handle(cx: &mut TestAppContext, handle: Handle) -> FocusHandle {
        with_form(cx, handle, |form, _, cx| {
            form.read(cx).number.focus_handle(cx)
        })
    }

    /// A shut select answers with its own handle, so this is the trigger's.
    fn trigger_handle(cx: &mut TestAppContext, handle: Handle) -> FocusHandle {
        with_form(cx, handle, |form, _, cx| {
            form.read(cx).select.focus_handle(cx)
        })
    }

    /// Sends one key to whatever holds the keyboard.
    fn press(cx: &mut TestAppContext, handle: Handle, key: &str) {
        cx.update_window(handle.into(), |_, window, cx| window.press(key, cx))
            .expect("the window is open");
        cx.run_until_parked();
    }

    #[gpui_kit::test]
    fn standard_inputs_type_choose_and_dismiss_inside_a_root_window(cx: &mut TestAppContext) {
        let handle = open(cx);
        with_form(cx, handle, |form, window, cx| {
            form.update(cx, |form, cx| {
                form.number.read(cx).focus_handle(cx).focus(window, cx);
            });
        });
        let typed = typed_handle(cx, handle);
        let trigger = trigger_handle(cx, handle);
        cx.update_window(handle.into(), |_, window, cx| {
            window.press("secondary-a", cx);
            window.input("4096", cx);
        })
        .expect("the window is open");
        cx.run_until_parked();
        assert_eq!(
            number(cx, handle),
            "4096",
            "the standard input took the typed text"
        );

        open_list(cx, handle);
        assert!(
            !focused(cx, handle, &typed) && !focused(cx, handle, &trigger),
            "the open list took the keyboard from the input and the trigger"
        );
        press(cx, handle, "escape");
        assert!(
            focused(cx, handle, &trigger),
            "escape closed the list and left the keyboard on the select"
        );
        assert_eq!(
            number(cx, handle),
            "4096",
            "escape closed the list, not the window"
        );
        assert_eq!(
            outer_escape(cx, handle),
            0,
            "the list consumed its escape instead of letting it reach the outer context"
        );
        press(cx, handle, "escape");
        assert_eq!(
            outer_escape(cx, handle),
            1,
            "a second escape, with the list shut, reaches the outer context"
        );

        open_list(cx, handle);
        press(cx, handle, "down");
        press(cx, handle, "enter");
        assert_eq!(
            with_form(cx, handle, |form, _, cx| form
                .read(cx)
                .select
                .read(cx)
                .selected_value()
                .cloned()),
            Some(String::from("2048")),
            "the standard list confirmed its second entry"
        );
        assert!(
            focused(cx, handle, &trigger),
            "choosing returns the keyboard to the select"
        );
    }
}

#[cfg(test)]
mod hint_tests {
    use super::*;
    use gpui_kit::component::Root;
    use gpui_kit::test::TestWindowExt;
    use gpui_kit::{TestAppContext, WindowHandle};
    use std::path::PathBuf;

    struct Window_ {
        handle: WindowHandle<Root>,
        shell: Entity<Shell>,
    }

    fn open(cx: &mut TestAppContext) -> Window_ {
        cx.update(|cx| {
            gpui_kit::init(cx);
            settings_ui::init(cx);
            navigation_ui::init(cx);
            hints::init(cx);
        });
        let mut shell = None;
        let handle = cx.add_window(|window, cx| {
            let view =
                cx.new(|cx| Shell::new(Config::default(), None, Session::default(), window, cx));
            shell = Some(view.clone());
            Root::new(view, window, cx).bordered(false)
        });
        let shell = shell.expect("the window built its shell");
        let window = Window_ { handle, shell };
        // An inactive test window reports no focus path, so no field would ever blur.
        cx.update_window(handle.into(), |_, window, _| window.activate_window())
            .expect("the window is open");
        frame(cx, &window);
        // A document is what puts the FFT summary, and so the hint, in the status bar.
        let opened = window.shell.clone();
        cx.update_window(window.handle.into(), |_, win, cx| {
            opened.update(cx, |shell, cx| {
                shell.open(Origin::new(PathBuf::from("/captures/session.iqw")), win, cx);
            });
        })
        .expect("the window is open");
        cx.run_until_parked();
        frame(cx, &window);
        window
    }

    fn frame(cx: &mut TestAppContext, w: &Window_) {
        cx.update_window(w.handle.into(), |_, window, cx| window.render_frame(cx))
            .expect("the window is open");
        cx.run_until_parked();
    }

    /// Opens the hint the way Ctrl+, does.
    fn open_hint(cx: &mut TestAppContext, w: &Window_) {
        let shell = w.shell.clone();
        cx.update_window(w.handle.into(), |_, window, cx| {
            shell.update(cx, |shell, cx| {
                shell.edit_analysis(&EditAnalysis, window, cx)
            });
        })
        .expect("the window is open");
        cx.run_until_parked();
        frame(cx, w);
    }

    fn is_open(cx: &mut TestAppContext, w: &Window_) -> bool {
        w.shell
            .read_with(cx, |shell, cx| shell.analysis_hint.read(cx).is_open())
    }

    fn settings(cx: &mut TestAppContext, w: &Window_) -> Settings {
        w.shell.read_with(cx, |shell, _| shell.settings)
    }

    fn editor(cx: &mut TestAppContext, w: &Window_) -> Entity<Editor> {
        w.shell.read_with(cx, |shell, cx| {
            shell
                .analysis_hint
                .read(cx)
                .view()
                .and_then(|view| view.downcast::<Editor>().ok())
                .expect("the open hint shows the editor")
        })
    }

    fn press(cx: &mut TestAppContext, w: &Window_, key: &str) {
        cx.update_window(w.handle.into(), |_, window, cx| window.press(key, cx))
            .expect("the window is open");
        cx.run_until_parked();
        frame(cx, w);
    }

    /// Replaces the overlap field's text as a person typing into it would.
    fn type_overlap(cx: &mut TestAppContext, w: &Window_, text: &str) {
        let editor = editor(cx, w);
        cx.update_window(w.handle.into(), |_, window, cx| {
            let field = editor.read(cx).overlap.focus_handle(cx);
            field.focus(window, cx);
            window.press("secondary-a", cx);
            window.input(text, cx);
        })
        .expect("the window is open");
        cx.run_until_parked();
        // A later blur is measured against the frame that drew the field focused.
        frame(cx, w);
    }

    #[gpui_kit::test]
    fn a_choice_in_a_list_applies_and_keeps_the_hint(cx: &mut TestAppContext) {
        let w = open(cx);
        open_hint(cx, &w);
        let opening = settings(cx, &w);
        cx.update_window(w.handle.into(), |_, window, cx| {
            window.within("analysis-fft").click("input", cx);
        })
        .expect("the window is open");
        cx.run_until_parked();
        frame(cx, &w);
        press(cx, &w, "down");
        press(cx, &w, "enter");
        assert_ne!(
            settings(cx, &w).fft_size,
            opening.fft_size,
            "the choice applies"
        );
        assert!(is_open(cx, &w), "choosing in a list keeps the hint");
    }

    #[gpui_kit::test]
    fn a_list_stays_where_it_is_while_the_keyboard_walks_it(cx: &mut TestAppContext) {
        let w = open(cx);
        open_hint(cx, &w);
        let opening = settings(cx, &w);
        let bounds = |cx: &mut TestAppContext| {
            cx.update_window(w.handle.into(), |_, window, _| {
                window.find("analysis-window").bounds()
            })
            .expect("the window is open")
        };
        let resting = bounds(cx);
        cx.update_window(w.handle.into(), |_, window, cx| {
            window.within("analysis-window").click("input", cx);
        })
        .expect("the window is open");
        cx.run_until_parked();
        frame(cx, &w);
        for _ in 0..3 {
            press(cx, &w, "down");
            assert_eq!(bounds(cx), resting, "walking the list moves nothing");
        }
        press(cx, &w, "enter");
        assert_ne!(
            settings(cx, &w).window,
            opening.window,
            "the choice applies"
        );
        assert_eq!(
            bounds(cx),
            resting,
            "a name of another length moves nothing"
        );
    }

    #[gpui_kit::test]
    fn enter_on_a_number_applies_it_and_a_keeping_close_saves_it(cx: &mut TestAppContext) {
        let w = open(cx);
        open_hint(cx, &w);
        type_overlap(cx, &w, "25");
        press(cx, &w, "enter");
        assert_eq!(settings(cx, &w).overlap, 25);
        assert!(is_open(cx, &w), "Enter in a number keeps the hint");
        // Ctrl+, again closes the hint keeping what it applied.
        open_hint(cx, &w);
        assert!(!is_open(cx, &w));
        let saved = w
            .shell
            .read_with(cx, |shell, _| shell.session.analysis_settings);
        assert_eq!(
            saved.map(|s| s.overlap),
            Some(25),
            "and the kept value is saved"
        );
    }

    #[gpui_kit::test]
    fn an_unusable_number_keeps_the_hint_with_its_error(cx: &mut TestAppContext) {
        let w = open(cx);
        open_hint(cx, &w);
        let opening = settings(cx, &w);
        type_overlap(cx, &w, "96");
        press(cx, &w, "enter");
        assert!(is_open(cx, &w), "the hint stays for a correction");
        let error = editor(cx, &w).read_with(cx, |editor, _| editor.error.clone());
        assert!(error.is_some(), "the error is shown");
        assert_eq!(settings(cx, &w), opening, "nothing is previewed");
    }

    #[gpui_kit::test]
    fn escape_restores_the_settings_and_views_the_hint_opened_with(cx: &mut TestAppContext) {
        let w = open(cx);
        let full = crate::navigation::View::full(1000);
        w.shell.update(cx, |shell, _| shell.view = Some(full));
        open_hint(cx, &w);
        let opening = settings(cx, &w);
        let frequency = w.shell.read_with(cx, |shell, _| shell.frequency);
        type_overlap(cx, &w, "10");
        press(cx, &w, "tab");
        assert_eq!(settings(cx, &w).overlap, 10, "blur previews the number");
        let saved = w
            .shell
            .read_with(cx, |shell, _| shell.session.analysis_settings);
        assert_ne!(saved.map(|s| s.overlap), Some(10), "a preview is not saved");
        w.shell.update(cx, |shell, _| {
            shell.view = Some(crate::navigation::View {
                start: 10,
                len: 100,
            });
            shell.frequency = shell.frequency.zoom(2., 0.5, 2048);
        });
        press(cx, &w, "escape");
        assert!(!is_open(cx, &w));
        assert_eq!(settings(cx, &w), opening, "the settings come back");
        let (view, restored) = w
            .shell
            .read_with(cx, |shell, _| (shell.view, shell.frequency));
        assert_eq!(view, Some(full), "and so does the time view");
        assert_eq!(restored, frequency, "and the frequency view");
    }

    #[gpui_kit::test]
    fn defaults_put_back_a_refused_number_even_when_nothing_changed(cx: &mut TestAppContext) {
        let w = open(cx);
        open_hint(cx, &w);
        type_overlap(cx, &w, "96");
        press(cx, &w, "tab");
        cx.update_window(w.handle.into(), |_, window, cx| {
            window.click("analysis-defaults", cx);
        })
        .expect("the window is open");
        cx.run_until_parked();
        frame(cx, &w);
        let editor = editor(cx, &w);
        let (text, error) = editor.read_with(cx, |editor, cx| {
            (
                editor.overlap.read(cx).value().to_string(),
                editor.error.clone(),
            )
        });
        let overlap = settings(cx, &w).overlap;
        assert_eq!(
            text,
            crate::numbers::input(overlap),
            "the field shows the value in force"
        );
        assert!(error.is_none());
    }

    #[gpui_kit::test]
    fn defaults_apply_the_configuration_and_keep_the_hint(cx: &mut TestAppContext) {
        let w = open(cx);
        open_hint(cx, &w);
        type_overlap(cx, &w, "10");
        press(cx, &w, "tab");
        assert_eq!(settings(cx, &w).overlap, 10);
        cx.update_window(w.handle.into(), |_, window, cx| {
            window.click("analysis-defaults", cx);
        })
        .expect("the window is open");
        cx.run_until_parked();
        frame(cx, &w);
        assert_eq!(settings(cx, &w), Settings::from_config(&Config::default()));
        assert!(is_open(cx, &w), "Defaults keeps the hint open");
    }
}

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
