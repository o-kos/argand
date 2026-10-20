//! The selection panel, every value of the selection typed in a pinned hint under the toolbar.
//!
//! It applies each value at once and follows the selection made on the plot
//! while it is open, since the plot stays live under it.

use super::badge_entry::on_steps;
use super::*;
use crate::selection_entry::{Field, Scale};
use argand_core::{FrequencyBand, SampleSpan, Selection};
use gpui_kit::Focusable as _;
use gpui_kit::base::actions::Confirm;
use gpui_kit::component::input::{Input, InputEvent, InputState};

/// The fields in the order they are laid out and kept, time's first.
const FIELDS: [Field; 6] = [
    Field::TimeStart,
    Field::TimeEnd,
    Field::TimeLength,
    Field::BandLow,
    Field::BandHigh,
    Field::BandWidth,
];

/// What the toolbar's segments select, in their order, as the time and band they keep.
pub(super) const KINDS: [(&str, &str, [bool; 2]); 3] = [
    ("select-time", "Time", [true, false]),
    ("select-band", "Band", [false, true]),
    ("select-both", "Both", [true, true]),
];

/// Every value of the selection, for the axes the toolbar's segment selects.
pub(super) struct SelectionPanel {
    shell: WeakEntity<Shell>,
    /// The document it was opened on, which alone it selects in.
    document: u64,
    scale: Scale,
    /// Each axis's values, kept while the axis is not selected.
    time: SampleSpan,
    band: FrequencyBand,
    /// Whether time and the band are selected.
    checked: [bool; 2],
    inputs: [Entity<InputState>; 6],
    /// The text each field was last given, which is not read again while it stands.
    shown: [String; 6],
    /// The text each field last refused, marked while it stands.
    refused: [Option<String>; 6],
    /// Set once it closed, so the fields leaving focus read nothing.
    closed: bool,
    /// Whether its first frame has put the keyboard in a field.
    placed: bool,
    _events: Vec<Subscription>,
}

impl SelectionPanel {
    fn new(
        shell: &Entity<Shell>,
        document: u64,
        scale: Scale,
        opened: Selection,
        (time, band): (SampleSpan, FrequencyBand),
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let shown = texts(&scale, time, band);
        let inputs = shown
            .clone()
            .map(|text| cx.new(|cx| InputState::new(window, cx).default_value(text)));
        let mut events: Vec<Subscription> = FIELDS
            .iter()
            .zip(&inputs)
            .map(|(&field, input)| {
                cx.subscribe_in(input, window, move |dialog, _, event, window, cx| {
                    if matches!(event, InputEvent::PressEnter { .. } | InputEvent::Blur) {
                        dialog.commit(field, window, cx);
                    }
                })
            })
            .collect();
        // The plot stays live, so a selection made there shows here, and another document closes the panel.
        events.push(cx.observe_in(shell, window, |panel, shell, window, cx| {
            panel.follow(&shell, window, cx)
        }));
        Self {
            shell: shell.downgrade(),
            document,
            scale,
            time,
            band,
            checked: [opened.time.is_some(), opened.band.is_some()],
            inputs,
            shown,
            refused: Default::default(),
            closed: false,
            placed: false,
            _events: events,
        }
    }

    fn slot(field: Field) -> usize {
        FIELDS.iter().position(|&each| each == field).unwrap_or(0)
    }

    /// Read a field whose text changed, show the result on the plot, or mark it.
    fn commit(&mut self, field: Field, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let slot = Self::slot(field);
        let text = self.inputs[slot].read(cx).value().to_string();
        if self.closed || text == self.shown[slot] {
            return true;
        }
        let read = match field.is_time() {
            true => self
                .scale
                .time(self.time, field, &text)
                .map(|time| self.time = time),
            false => self
                .scale
                .band(self.band, field, &text)
                .map(|band| self.band = band),
        };
        if read.is_none() {
            self.refused[slot] = Some(text);
            cx.notify();
            return false;
        }
        self.refused[slot] = None;
        // A value typed for an axis not selected selects it.
        self.checked[usize::from(!field.is_time())] = true;
        self.refresh(window, cx);
        self.apply(cx);
        true
    }

    /// Write every field from the values, except one still holding a refused text.
    fn refresh(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let texts = texts(&self.scale, self.time, self.band);
        for (slot, text) in texts.into_iter().enumerate() {
            // A field leaving focus is read after the next one took it, which may already be typed in.
            let typing = self.inputs[slot].focus_handle(cx).is_focused(window)
                && *self.inputs[slot].read(cx).value() != *self.shown[slot];
            if self.marked(slot, cx) || typing {
                continue;
            }
            if *self.inputs[slot].read(cx).value() != *text {
                self.inputs[slot].update(cx, |input, cx| input.set_value(text.clone(), window, cx));
            }
            self.shown[slot] = text;
        }
        cx.notify();
    }

    fn marked(&self, slot: usize, cx: &gpui_kit::App) -> bool {
        self.refused[slot]
            .as_deref()
            .is_some_and(|refused| *self.inputs[slot].read(cx).value() == *refused)
    }

    /// The selection the checked axes make.
    fn selection(&self) -> Selection {
        Selection {
            time: self.checked[0].then_some(self.time),
            band: self.checked[1].then_some(self.band),
        }
    }

    fn apply(&self, cx: &mut Context<Self>) {
        self.select(self.selection(), cx);
    }

    /// Take up the selection the shell holds where it is not the panel's own, closing on another document.
    fn follow(&mut self, shell: &Entity<Shell>, window: &mut Window, cx: &mut Context<Self>) {
        let shell = shell.read(cx);
        if self.closed {
            return;
        }
        if shell.file.as_ref().map(|file| file.id) != Some(self.document) {
            self.closed = true;
            let hint = shell.selection_hint.clone();
            cx.defer(move |cx| hint.update(cx, |hint, cx| hint.close(false, cx)));
            return;
        }
        let selected = Selection {
            time: shell.selection,
            band: shell.band,
        };
        let scale = shell.entry_scale();
        if selected == self.selection() && scale.is_none_or(|scale| scale == self.scale) {
            return;
        }
        self.scale = scale.unwrap_or(self.scale);
        self.checked = [selected.time.is_some(), selected.band.is_some()];
        self.time = selected.time.unwrap_or(self.time);
        self.band = selected.band.unwrap_or(self.band);
        self.refresh(window, cx);
    }

    /// Select in the document the panel was opened on, and nowhere else.
    fn select(&self, selection: Selection, cx: &mut Context<Self>) {
        let document = self.document;
        let _ = self.shell.update(cx, |shell, cx| {
            if shell.file.as_ref().is_some_and(|file| file.id == document) {
                shell.select(selection, cx);
            }
        });
    }

    /// Step the field holding the keyboard by its last digit, reading a changed text first.
    fn step(&mut self, steps: i64, window: &mut Window, cx: &mut Context<Self>) {
        let Some(slot) = self
            .inputs
            .iter()
            .position(|input| input.focus_handle(cx).is_focused(window))
        else {
            return;
        };
        let field = FIELDS[slot];
        if !self.commit(field, window, cx) {
            return;
        }
        let text = self.shown[slot].clone();
        let stepped = match field.is_time() {
            true => self
                .scale
                .time_step(self.time, field, &text, steps)
                .map(|time| self.time = time),
            false => self
                .scale
                .band_step(self.band, field, &text, steps)
                .map(|band| self.band = band),
        };
        if stepped.is_some() {
            self.refresh(window, cx);
            self.apply(cx);
        }
    }

    /// Read every changed field, answering whether all of them were usable.
    fn confirm(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let mut usable = true;
        for field in FIELDS {
            usable &= self.commit(field, window, cx);
        }
        self.closed = usable;
        usable
    }

    /// Put the keyboard in the first field of the first selected axis, or of time with none.
    fn focus_first(&self, window: &mut Window, cx: &mut Context<Self>) {
        let axis = self
            .checked
            .iter()
            .position(|&checked| checked)
            .unwrap_or(0);
        self.focus_field(axis * 3, window, cx);
    }

    /// Move the keyboard `by` fields on from the one holding it, round the panel.
    fn traverse(&self, by: usize, window: &mut Window, cx: &mut Context<Self>) {
        let at = self
            .inputs
            .iter()
            .position(|input| input.focus_handle(cx).is_focused(window))
            .unwrap_or(FIELDS.len() - 1);
        self.focus_field((at + by) % FIELDS.len(), window, cx);
    }

    fn focus_field(&self, slot: usize, window: &mut Window, cx: &mut Context<Self>) {
        self.inputs[slot].update(cx, |input, cx| {
            input.focus(window, cx);
            input.select_all(window, cx);
        });
    }

    /// The selection drawn with its fields around it and its dimension lines.
    fn drawing(&self, cx: &mut Context<Self>) -> gpui_kit::Div {
        let (left, top, width, height) = AREA;
        let (right, bottom) = (left + width, top + height);
        let above = top - LINE_GAP;
        // The width's line runs through its field, which stands clear right of the area.
        let beside = right + FIELD_GAP + FIELD.0 / 2.;
        let ink = |checked: bool| match checked {
            true => cx.theme().foreground,
            false => cx.theme().muted_foreground.opacity(0.5),
        };
        let lines = [
            ((left, above), (right, above), ink(self.checked[0])),
            ((beside, top), (beside, bottom), ink(self.checked[1])),
        ];
        let below = bottom + FIELD_GAP + FIELD.1 / 2.;
        let beside_area = left - FIELD_GAP - FIELD.0;
        let places = [
            // Start and end under the area's edges, the length on its line above.
            (left, below),
            (right - FIELD.0, below),
            ((left + right - FIELD.0) / 2., above),
            // Low and high beside the area's bottom and top, the width on its line at the right.
            (beside_area, bottom - FIELD.1 / 2.),
            (beside_area, top + FIELD.1 / 2.),
            (beside - FIELD.0 / 2., (top + bottom) / 2.),
        ];
        div()
            .relative()
            .w(px(DRAWING.0))
            .h(px(DRAWING.1))
            .child(
                div()
                    .absolute()
                    .left(px(left))
                    .top(px(top))
                    .w(px(width))
                    .h(px(height))
                    .bg(cx.theme().blue_light.opacity(0.3)),
            )
            .child(
                canvas(
                    |_, _, _| (),
                    move |bounds, _, window, _| {
                        for (from, to, color) in lines {
                            paint_dimension(bounds.origin, from, to, color, window);
                        }
                    },
                )
                .absolute()
                .size_full(),
            )
            .children(
                places
                    .into_iter()
                    .enumerate()
                    .map(|(slot, place)| self.field(slot, place, cx)),
            )
            // Each line names its unit where a ruler would, past its end.
            .child(self.unit(
                self.scale.mode.caption(),
                (right + FIELD_GAP + 12., above),
                cx,
            ))
            .child(self.unit(self.scale.unit, (beside, top - FIELD.1 / 2. - 4.), cx))
    }

    /// A unit caption in the rulers' muted type, centred on `(x, y)`.
    fn unit(
        &self,
        text: &'static str,
        (x, y): (f32, f32),
        cx: &mut Context<Self>,
    ) -> gpui_kit::Div {
        div()
            .absolute()
            .left(px(x - FIELD.0 / 2.))
            .top(px(y - FIELD.1 / 2.))
            .w(px(FIELD.0))
            .h(px(FIELD.1))
            .flex()
            .items_center()
            .justify_center()
            .text_xs()
            .text_color(cx.theme().muted_foreground)
            .child(text)
    }

    /// The field of `slot` with its left edge at `x`, centred on `y`.
    fn field(&self, slot: usize, (x, y): (f32, f32), cx: &mut Context<Self>) -> gpui_kit::Div {
        let input = Input::new(&self.inputs[slot])
            .small()
            .w_full()
            .text_align(gpui_kit::TextAlign::Center)
            // An axis not selected shows the visible range muted, and a value typed there selects it.
            .when(!self.checked[slot / 3], |input| {
                input.text_color(cx.theme().muted_foreground)
            })
            // The focus ring would hide the refusal on the very field that holds the keyboard.
            .when(self.marked(slot, cx), |input| {
                input.focus_bordered(false).border_color(cx.theme().danger)
            });
        div()
            .absolute()
            .left(px(x))
            .top(px(y - FIELD.1 / 2.))
            .w(px(FIELD.0))
            .h(px(FIELD.1))
            .flex()
            .items_center()
            // The dimension lines run under the fields, so a field must hide the line it stands on.
            .bg(crate::theme::sheet(cx))
            .rounded(cx.theme().radius)
            .child(input)
    }
}

/// The panel's surroundings, where Up and Down step the focused field.
const CONTEXT: &str = "SelectionPanel";

actions!(selection_panel, [NextField, PreviousField]);

pub(super) fn init(cx: &mut gpui_kit::App) {
    super::badge_entry::step_keys(CONTEXT, cx);
    // Bound here rather than left to the window, whose Tab would walk out of the panel.
    cx.bind_keys([
        KeyBinding::new("tab", NextField, Some(CONTEXT)),
        KeyBinding::new("shift-tab", PreviousField, Some(CONTEXT)),
    ]);
}

/// A field's width and the height it is centred in.
const FIELD: (f32, f32) = (104., 24.);
/// The selection drawn in the panel, as left, top, width and height.
const AREA: (f32, f32, f32, f32) = (124., 44., 220., 104.);
/// How far the length's line stands above the area.
const LINE_GAP: f32 = 24.;
/// Room between the area and the fields beside and under it.
const FIELD_GAP: f32 = 8.;
/// The drawing's whole size.
const DRAWING: (f32, f32) = (
    AREA.0 + AREA.2 + FIELD_GAP + FIELD.0,
    AREA.1 + AREA.3 + FIELD_GAP + FIELD.1,
);

impl Render for SelectionPanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // The popover takes the keyboard as it opens, so the field takes it after the first frame.
        if !self.placed {
            self.placed = true;
            let dialog = cx.entity();
            window.defer(cx, move |window, cx| {
                dialog.update(cx, |dialog, cx| dialog.focus_first(window, cx))
            });
        }
        let drawing = self.drawing(cx);
        // Enter keeps the values and closes the panel, but only once every changed field reads.
        let confirm = cx.listener(|panel: &mut Self, _: &Confirm, window, cx| {
            if panel.confirm(window, cx) {
                cx.propagate();
            }
        });
        on_steps(div().id("selection-panel"), Self::step, cx)
            .key_context(CONTEXT)
            .on_action(confirm)
            // Tab goes round the panel's own fields, which hold the keyboard while it is open.
            .on_action(
                cx.listener(|panel, _: &NextField, window, cx| panel.traverse(1, window, cx)),
            )
            .on_action(cx.listener(|panel, _: &PreviousField, window, cx| {
                panel.traverse(FIELDS.len() - 1, window, cx)
            }))
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
            .child(
                div()
                    .h_6()
                    .flex()
                    .items_center()
                    .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                    .child("Selection"),
            )
            .child(drawing)
    }
}

/// A dimension line from `from` to `to`, straight across or down, with an arrowhead at each end.
fn paint_dimension(
    origin: gpui_kit::Point<Pixels>,
    from: (f32, f32),
    to: (f32, f32),
    color: gpui_kit::Hsla,
    window: &mut Window,
) {
    const HEAD: f32 = 6.;
    const HALF: f32 = 3.;
    let at = |x: f32, y: f32| origin + gpui_kit::point(px(x), px(y));
    let across = from.1 == to.1;
    let line = match across {
        true => Bounds::new(at(from.0, from.1 - 0.5), size(px(to.0 - from.0), px(1.))),
        false => Bounds::new(at(from.0 - 0.5, from.1), size(px(1.), px(to.1 - from.1))),
    };
    window.paint_quad(gpui_kit::fill(line, color));
    for (tip, back) in [(from, HEAD), (to, -HEAD)] {
        let points = match across {
            true => [
                at(tip.0, tip.1),
                at(tip.0 + back, tip.1 - HALF),
                at(tip.0 + back, tip.1 + HALF),
            ],
            false => [
                at(tip.0, tip.1),
                at(tip.0 - HALF, tip.1 + back),
                at(tip.0 + HALF, tip.1 + back),
            ],
        };
        let mut path = gpui_kit::PathBuilder::fill();
        path.add_polygon(&points, true);
        if let Ok(path) = path.build() {
            window.paint_path(path, color);
        }
    }
}

/// The texts of every field, time's first.
fn texts(scale: &Scale, time: SampleSpan, band: FrequencyBand) -> [String; 6] {
    let [start, end, length] = scale.time_texts(time);
    let [low, high, width] = scale.band_texts(band);
    [start, end, length, low, high, width]
}

/// The panel for the shell's selection, or for the visible range where an axis is not selected.
pub(super) fn selection_panel(
    owner: WeakEntity<Shell>,
    window: &mut Window,
    cx: &mut gpui_kit::App,
) -> gpui_kit::AnyView {
    let Some(shell) = owner.upgrade() else {
        return cx.new(|_| gpui_kit::Empty).into();
    };
    let read = shell.read(cx);
    let opened = Selection {
        time: read.selection,
        band: read.band,
    };
    let start = (
        read.file.as_ref().map(|file| file.id),
        read.entry_scale(),
        read.view,
        read.extents().map(|extents| extents.hertz),
    );
    let (Some(document), Some(scale), Some(view), Some((low, high))) = start else {
        return cx.new(|_| gpui_kit::Empty).into();
    };
    let (floor, ceiling) = scale.band;
    let visible_band = FrequencyBand::between(low.max(floor), high.min(ceiling));
    let visible_time = SampleSpan::between(view.start, view.start + view.len);
    let (Some(time), Some(band)) = (opened.time.or(visible_time), opened.band.or(visible_band))
    else {
        return cx.new(|_| gpui_kit::Empty).into();
    };
    cx.new(|cx| SelectionPanel::new(&shell, document, scale, opened, (time, band), window, cx))
        .into()
}

impl Shell {
    /// The axes the selection holds, time's and the band's.
    pub(super) fn selection_kind(&self) -> [bool; 2] {
        [self.selection.is_some(), self.band.is_some()]
    }

    /// Close the selection panel, keeping the selection.
    pub(super) fn close_selection_hint(&mut self, cx: &mut Context<Self>) {
        self.selection_hint
            .update(cx, |hint, cx| hint.close(false, cx));
    }

    /// Ctrl+E and Edit → Selection…, which open the panel or close it keeping the selection.
    pub(super) fn toggle_selection_panel(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.dismiss_application_menu(window, cx);
        if self.entry_scale().is_none() {
            return;
        }
        let hint = self.selection_hint.clone();
        if hint.read(cx).is_open() {
            hint.update(cx, |hint, cx| hint.close(false, cx));
            return;
        }
        // With nothing selected the panel opens on time and band, both from the visible range.
        if self.selection_kind() == [false, false] {
            self.remember_opening();
            self.select_visible([true, true], cx);
        }
        // The panel reads the shell as it is built, so it waits until this update ends.
        window.defer(cx, move |window, cx| {
            hint.update(cx, |hint, cx| hint.open(window, cx));
        });
    }

    /// A segment chose what to select: the same kind toggles the panel, another one selects it and opens it.
    pub(super) fn choose_selection_kind(
        &mut self,
        kind: [bool; 2],
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if kind == self.selection_kind() {
            // The press on the title bar has just closed the panel, which is all it asked.
            if std::mem::take(&mut self.press_closed_selection) {
                return;
            }
            return self.toggle_selection_panel(window, cx);
        }
        if !self.selection_hint.read(cx).is_open() {
            self.remember_opening();
        }
        self.select_visible(kind, cx);
        if !self.selection_hint.read(cx).is_open() {
            self.toggle_selection_panel(window, cx);
        }
    }

    /// Keep the selection as it stands before the panel changes it, for Escape to restore.
    fn remember_opening(&mut self) {
        let current = Selection {
            time: self.selection,
            band: self.band,
        };
        self.selection_opening.get_or_insert(current);
    }

    /// Select `kind`, keeping an axis already selected and taking a missing one from the visible range.
    fn select_visible(&mut self, kind: [bool; 2], cx: &mut Context<Self>) {
        let Some(view) = self.view else { return };
        let visible_time = argand_core::SampleSpan::between(view.start, view.start + view.len);
        let visible_band = self.extents().and_then(|extents| {
            let (low, high) = extents.hertz;
            argand_core::FrequencyBand::between(low, high)
        });
        let selection = Selection {
            time: kind[0].then(|| self.selection.or(visible_time)).flatten(),
            band: kind[1].then(|| self.band.or(visible_band)).flatten(),
        };
        self.select(selection, cx);
    }

    /// What the panel opening and closing means for the selection.
    pub(super) fn selection_hint_changed(
        &mut self,
        _: Entity<hints::PinnedHint>,
        event: &hints::Pinned,
        cx: &mut Context<Self>,
    ) {
        match *event {
            hints::Pinned::Opened => {
                // The panel freezes the plot as the analysis hint does.
                self.interrupt_plot(cx);
                self.remember_opening();
            }
            hints::Pinned::Closed { revert } => {
                let opening = self.selection_opening.take();
                if let (true, Some(opening)) = (revert, opening) {
                    self.select(opening, cx);
                }
            }
        }
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::{FileInfo, Update};
    use argand_core::{Domain, SampleFormat, SampleType, SignalMeta};
    use gpui_kit::TestAppContext;
    use gpui_kit::test::TestWindowExt;
    use std::path::PathBuf;

    /// A shell under the toolkit's root, which the popover layer needs, with a described capture.
    fn open(cx: &mut TestAppContext) -> (Entity<Shell>, &mut gpui_kit::VisualTestContext) {
        cx.update(|cx| {
            gpui_kit::init(cx);
            settings_ui::init(cx);
            navigation_ui::init(cx);
            hints::init(cx);
            app_menu_ui::init(cx);
            window_keys(cx);
            crate::theme::install(ThemeMode::Dark, cx);
        });
        let mut shell = None;
        let handle = cx.add_window(|window, cx| {
            let view =
                cx.new(|cx| Shell::new(Config::default(), None, Session::default(), window, cx));
            shell = Some(view.clone());
            gpui_kit::component::Root::new(view, window, cx).bordered(false)
        });
        let shell = shell.unwrap();
        let cx = gpui_kit::VisualTestContext::from_window(handle.into(), cx).into_mut();
        cx.simulate_resize(gpui_kit::size(px(800.), px(600.)));
        // An inactive test window reports no focus path, so no field would ever blur.
        cx.update(|window, _| window.activate_window());
        shell.update_in(cx, |shell, window, cx| {
            let path = PathBuf::from("/captures/a.iqw");
            shell.open(Origin::new(path.clone()), window, cx);
            let Some(file) = shell.file.as_mut() else {
                panic!("the capture never opened");
            };
            file.state.document.apply(Update::Opened(
                SignalMeta {
                    sample_rate: 1_000.0,
                    center_freq: 0.0,
                    sample_type: SampleType::new(Domain::Iq, SampleFormat::I16),
                    len_samples: 10_000,
                    container: "raw",
                    divisor: 32_768.0,
                    source: path,
                },
                FileInfo::default(),
            ));
            shell.start_editing(cx);
            shell.view = Some(crate::navigation::View::full(10_000));
            shell.session.time_ruler = crate::time_ruler::Mode::Samples;
        });
        cx.run_until_parked();
        (shell, cx)
    }

    fn selected(cx: &mut gpui_kit::VisualTestContext, shell: &Entity<Shell>) -> Selection {
        shell.read_with(cx, |shell, _| Selection {
            time: shell.selection,
            band: shell.band,
        })
    }

    /// Ctrl+E, as the plot binds it.
    fn toggle(cx: &mut gpui_kit::VisualTestContext, shell: &Entity<Shell>) {
        shell.update_in(cx, |shell, window, cx| {
            shell.toggle_selection_panel(window, cx)
        });
        settle(cx);
    }

    /// Let the popover open and a frame report focus changes.
    fn settle(cx: &mut gpui_kit::VisualTestContext) {
        cx.executor()
            .advance_clock(std::time::Duration::from_secs(1));
        cx.run_until_parked();
        cx.update(|window, cx| window.render_frame(cx));
        cx.run_until_parked();
    }

    fn keys(cx: &mut gpui_kit::VisualTestContext, keys: &str) {
        cx.simulate_keystrokes(keys);
        settle(cx);
    }

    fn panel_open(cx: &mut gpui_kit::VisualTestContext, shell: &Entity<Shell>) -> bool {
        shell.read_with(cx, |shell, cx| shell.selection_hint.read(cx).is_open())
    }

    /// The text each field of the open panel holds.
    fn texts(cx: &mut gpui_kit::VisualTestContext, shell: &Entity<Shell>) -> Vec<String> {
        shell.read_with(cx, |shell, cx| {
            let view = shell
                .selection_hint
                .read(cx)
                .view()
                .expect("the panel is open");
            let panel = view
                .downcast::<SelectionPanel>()
                .expect("a selection panel");
            panel
                .read(cx)
                .inputs
                .iter()
                .map(|input| input.read(cx).value().to_string())
                .collect()
        })
    }

    fn span(a: u64, b: u64) -> Option<SampleSpan> {
        SampleSpan::between(a, b)
    }

    #[gpui_kit::test]
    fn a_typed_value_moves_the_selection_live_and_enter_keeps_it(cx: &mut TestAppContext) {
        let (shell, cx) = open(cx);
        shell.update_in(cx, |shell, _, _| shell.selection = span(100, 200));
        toggle(cx, &shell);
        assert!(panel_open(cx, &shell));
        keys(cx, "5 0 tab");
        assert_eq!(
            selected(cx, &shell).time,
            span(50, 200),
            "applied on leaving the field"
        );
        keys(cx, "secondary-a # 4 9 9 enter");
        assert_eq!(
            selected(cx, &shell).time,
            span(50, 500),
            "the end names the last sample"
        );
        assert!(!panel_open(cx, &shell), "Enter keeps it and closes");
    }

    #[gpui_kit::test]
    fn tab_goes_round_the_panel_s_own_fields(cx: &mut TestAppContext) {
        let (shell, cx) = open(cx);
        shell.update_in(cx, |shell, _, _| shell.selection = span(100, 200));
        toggle(cx, &shell);
        keys(cx, "tab tab tab tab tab tab");
        assert!(panel_open(cx, &shell));
        keys(cx, "5 0 enter");
        assert_eq!(
            selected(cx, &shell).time,
            span(50, 200),
            "back on the start, six fields on"
        );
        toggle(cx, &shell);
        keys(cx, "shift-tab 3 0 0 enter");
        let band = selected(cx, &shell)
            .band
            .expect("the width selects the band");
        assert_eq!(
            (band.low(), band.high()),
            (-500., -200.),
            "the width is the last field"
        );
    }

    #[gpui_kit::test]
    fn a_field_read_late_leaves_the_next_one_being_typed_in(cx: &mut TestAppContext) {
        let (shell, cx) = open(cx);
        shell.update_in(cx, |shell, _, _| shell.selection = span(100, 200));
        toggle(cx, &shell);
        // Tab and the next value arrive before the frame that reports the first field left.
        cx.simulate_keystrokes("5 0 tab 3 0 0");
        settle(cx);
        keys(cx, "enter");
        assert_eq!(
            selected(cx, &shell).time,
            span(50, 301),
            "300 is the last sample"
        );
    }

    #[gpui_kit::test]
    fn escape_restores_the_selection_the_panel_opened_on(cx: &mut TestAppContext) {
        let (shell, cx) = open(cx);
        shell.update_in(cx, |shell, _, _| shell.selection = span(100, 200));
        toggle(cx, &shell);
        keys(cx, "5 0 tab");
        assert_eq!(selected(cx, &shell).time, span(50, 200));
        keys(cx, "7 escape");
        assert!(!panel_open(cx, &shell));
        assert_eq!(selected(cx, &shell).time, span(100, 200));
    }

    #[gpui_kit::test]
    fn a_bad_value_keeps_the_panel_open_and_changes_nothing(cx: &mut TestAppContext) {
        let (shell, cx) = open(cx);
        shell.update_in(cx, |shell, _, _| shell.selection = span(100, 200));
        toggle(cx, &shell);
        keys(cx, "x enter");
        assert!(panel_open(cx, &shell), "Enter waits for a usable value");
        assert_eq!(selected(cx, &shell).time, span(100, 200));
        keys(cx, "backspace 1 5 0 enter");
        assert!(!panel_open(cx, &shell));
        assert_eq!(selected(cx, &shell).time, span(150, 200));
    }

    #[gpui_kit::test]
    fn up_and_down_step_the_focused_field_by_its_last_digit(cx: &mut TestAppContext) {
        let (shell, cx) = open(cx);
        shell.update_in(cx, |shell, _, _| shell.selection = span(100, 200));
        toggle(cx, &shell);
        keys(cx, "up");
        assert_eq!(selected(cx, &shell).time, span(101, 200));
        keys(cx, "shift-down");
        assert_eq!(selected(cx, &shell).time, span(91, 200), "ten with Shift");
        keys(cx, "secondary-a 1 2 0 up");
        assert_eq!(
            selected(cx, &shell).time,
            span(121, 200),
            "a changed text is read before the step"
        );
    }

    #[gpui_kit::test]
    fn a_segment_selects_its_kind_from_the_visible_range_and_opens_the_panel(
        cx: &mut TestAppContext,
    ) {
        let (shell, cx) = open(cx);
        shell.update_in(cx, |shell, window, cx| {
            shell.choose_selection_kind([false, true], window, cx)
        });
        settle(cx);
        assert!(panel_open(cx, &shell));
        let band = selected(cx, &shell).band.expect("a band");
        assert_eq!(
            (band.low(), band.high()),
            (-500., 500.),
            "the whole visible band"
        );
        assert_eq!(selected(cx, &shell).time, None);
        keys(cx, "secondary-a - 1 0 0 enter");
        let band = selected(cx, &shell).band.expect("a band");
        assert_eq!((band.low(), band.high()), (-100., 500.));
        shell.update_in(cx, |shell, window, cx| {
            shell.choose_selection_kind([false, true], window, cx)
        });
        settle(cx);
        assert!(
            panel_open(cx, &shell),
            "the kind already selected toggles the panel"
        );
    }

    #[gpui_kit::test]
    fn the_panel_follows_a_selection_made_on_the_plot(cx: &mut TestAppContext) {
        let (shell, cx) = open(cx);
        shell.update_in(cx, |shell, _, _| shell.selection = span(100, 200));
        toggle(cx, &shell);
        shell.update_in(cx, |shell, _, cx| {
            shell.select(Selection::time(SampleSpan::between(300, 700).unwrap()), cx)
        });
        settle(cx);
        assert_eq!(&texts(cx, &shell)[..3], ["#300", "#699", "400"]);
        assert!(panel_open(cx, &shell));
    }

    #[gpui_kit::test]
    fn from_the_menu_the_panel_keeps_the_keyboard(cx: &mut TestAppContext) {
        let (shell, cx) = open(cx);
        shell.update_in(cx, |shell, window, cx| {
            window.focus(&shell.focus_target(cx), cx)
        });
        settle(cx);
        // Edit is the second branch, and its first usable row with nothing to edit is the panel.
        keys(cx, "f10 down down right down enter");
        assert!(panel_open(cx, &shell));
        keys(cx, "escape");
        assert!(
            !panel_open(cx, &shell),
            "Escape reaches the panel, not the plot"
        );
        assert_eq!(selected(cx, &shell), Selection::default());
    }

    #[gpui_kit::test]
    fn with_nothing_selected_the_panel_opens_on_time_and_band(cx: &mut TestAppContext) {
        let (shell, cx) = open(cx);
        toggle(cx, &shell);
        let opened = selected(cx, &shell);
        assert_eq!(opened.time, span(0, 10_000), "the visible time");
        let band = opened.band.expect("the visible band");
        assert_eq!((band.low(), band.high()), (-500., 500.));
        keys(cx, "escape");
        assert_eq!(
            selected(cx, &shell),
            Selection::default(),
            "Escape restores nothing selected"
        );
    }

    #[gpui_kit::test]
    fn a_value_typed_for_an_axis_not_selected_selects_it(cx: &mut TestAppContext) {
        let (shell, cx) = open(cx);
        shell.update_in(cx, |shell, window, cx| {
            shell.choose_selection_kind([false, true], window, cx)
        });
        settle(cx);
        assert_eq!(selected(cx, &shell).time, None);
        shell.update_in(cx, |shell, window, cx| {
            let view = shell.selection_hint.read(cx).view().expect("open");
            let panel = view
                .downcast::<SelectionPanel>()
                .expect("a selection panel");
            panel.update(cx, |panel, cx| panel.focus_field(0, window, cx));
        });
        keys(cx, "secondary-a 2 0 0 enter");
        assert_eq!(
            selected(cx, &shell).time,
            span(200, 10_000),
            "the visible end kept"
        );
        assert!(selected(cx, &shell).band.is_some());
    }

    #[gpui_kit::test]
    fn choosing_a_file_closes_the_panel_keeping_what_it_applied(cx: &mut TestAppContext) {
        let (shell, cx) = open(cx);
        shell.update_in(cx, |shell, _, _| shell.selection = span(100, 200));
        toggle(cx, &shell);
        keys(cx, "5 0 tab");
        shell.update_in(cx, |shell, window, cx| shell.choose_file(window, cx));
        settle(cx);
        assert!(!panel_open(cx, &shell));
        assert_eq!(selected(cx, &shell).time, span(50, 200));
    }

    #[gpui_kit::test]
    fn another_document_opened_under_the_panel_closes_it(cx: &mut TestAppContext) {
        let (shell, cx) = open(cx);
        shell.update_in(cx, |shell, _, _| shell.selection = span(100, 200));
        toggle(cx, &shell);
        keys(cx, "5 0");
        shell.update_in(cx, |shell, window, cx| {
            shell.open(Origin::new(PathBuf::from("/captures/b.iqw")), window, cx);
        });
        settle(cx);
        assert!(!panel_open(cx, &shell));
        assert_eq!(
            selected(cx, &shell),
            Selection::default(),
            "nothing carried over"
        );
    }
}
