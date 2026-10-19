//! Edit → Selection…, every value of the selection typed in a dialog and shown on the plot at once.

use super::*;
use crate::selection_entry::{Field, Scale};
use argand_core::{FrequencyBand, SampleSpan, Selection};
use gpui_kit::component::checkbox::Checkbox;
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::{WindowExt as _, h_flex, v_flex};

/// The fields in the order they are laid out and kept, time's first.
const FIELDS: [Field; 6] = [
    Field::TimeStart,
    Field::TimeEnd,
    Field::TimeLength,
    Field::BandLow,
    Field::BandHigh,
    Field::BandWidth,
];

/// Every value of the selection, each axis selected by its check box.
pub(super) struct SelectionDialog {
    shell: WeakEntity<Shell>,
    scale: Scale,
    /// The selection the dialog opened on, which Cancel restores.
    opened: Selection,
    /// Each axis's values, kept while its box is unchecked.
    time: SampleSpan,
    band: FrequencyBand,
    /// Whether time and the band are selected.
    checked: [bool; 2],
    inputs: [Entity<InputState>; 6],
    /// The text each field was last given, which is not read again while it stands.
    shown: [String; 6],
    /// The text each field last refused, marked while it stands.
    refused: [Option<String>; 6],
    /// Set once OK or Cancel closed it, so the fields leaving focus read nothing.
    closed: bool,
    /// Whether its first frame has put the keyboard in a field.
    placed: bool,
    _events: Vec<Subscription>,
}

impl SelectionDialog {
    fn new(
        shell: WeakEntity<Shell>,
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
        let events = FIELDS
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
        Self {
            shell,
            scale,
            opened,
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
        self.refresh(window, cx);
        self.apply(cx);
        true
    }

    /// Write every field from the values, except one still holding a refused text.
    fn refresh(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let texts = texts(&self.scale, self.time, self.band);
        for (slot, text) in texts.into_iter().enumerate() {
            if self.marked(slot, cx) {
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
        let selection = self.selection();
        let _ = self
            .shell
            .update(cx, |shell, cx| shell.select(selection, cx));
    }

    fn check(&mut self, axis: usize, checked: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.checked[axis] = checked;
        if checked {
            self.focus_field(axis * 3, window, cx);
        }
        self.apply(cx);
        cx.notify();
    }

    /// Read every changed field of the checked axes, answering whether all of them were usable.
    fn confirm(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let mut usable = true;
        for field in FIELDS {
            if self.checked[usize::from(!field.is_time())] {
                usable &= self.commit(field, window, cx);
            }
        }
        self.closed = usable;
        usable
    }

    fn cancel(&mut self, cx: &mut Context<Self>) {
        self.closed = true;
        let opened = self.opened;
        let _ = self.shell.update(cx, |shell, cx| shell.select(opened, cx));
    }

    /// Put the keyboard in the first field of the first checked axis, if any.
    fn focus_first(&self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(axis) = self.checked.iter().position(|&checked| checked) {
            self.focus_field(axis * 3, window, cx);
        }
    }

    fn focus_field(&self, slot: usize, window: &mut Window, cx: &mut Context<Self>) {
        self.inputs[slot].update(cx, |input, cx| {
            input.focus(window, cx);
            input.select_all(window, cx);
        });
    }

    fn axis(&self, axis: usize, cx: &mut Context<Self>) -> gpui_kit::Div {
        let (id, title, unit, names) = match axis {
            0 => (
                "time-axis",
                "Time",
                self.scale.mode.caption(),
                ["Start", "End", "Length"],
            ),
            _ => (
                "band-axis",
                "Band",
                self.scale.unit,
                ["Low", "High", "Width"],
            ),
        };
        let checked = self.checked[axis];
        let rows = names.into_iter().enumerate().map(|(index, name)| {
            let slot = axis * 3 + index;
            h_flex()
                .gap_2()
                .child(div().w(px(64.)).pl_6().child(name))
                .child(
                    Input::new(&self.inputs[slot])
                        .small()
                        .flex_1()
                        .disabled(!checked)
                        .when(self.marked(slot, cx), |input| {
                            input.border_color(cx.theme().danger)
                        }),
                )
                .child(
                    div()
                        .w(px(32.))
                        .text_color(cx.theme().muted_foreground)
                        .child(unit),
                )
        });
        v_flex()
            .gap_1()
            .child(
                Checkbox::new(id)
                    .label(title)
                    .checked(checked)
                    .on_click(cx.listener(move |dialog, checked: &bool, window, cx| {
                        dialog.check(axis, *checked, window, cx)
                    })),
            )
            .children(rows)
    }
}

impl Render for SelectionDialog {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // The dialog takes the keyboard as it opens, so the field takes it after the first frame.
        if !self.placed {
            self.placed = true;
            let dialog = cx.entity();
            window.defer(cx, move |window, cx| {
                dialog.update(cx, |dialog, cx| dialog.focus_first(window, cx))
            });
        }
        v_flex()
            .gap_3()
            .child(self.axis(0, cx))
            .child(self.axis(1, cx))
    }
}

/// The texts of every field, time's first.
fn texts(scale: &Scale, time: SampleSpan, band: FrequencyBand) -> [String; 6] {
    let [start, end, length] = scale.time_texts(time);
    let [low, high, width] = scale.band_texts(band);
    [start, end, length, low, high, width]
}

impl Shell {
    /// Open the dialog on the selection, or on the visible range where an axis is not selected.
    pub(super) fn open_selection_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let (Some(scale), Some(view), Some((low, high))) = (
            self.entry_scale(),
            self.view,
            self.extents().map(|extents| extents.hertz),
        ) else {
            return;
        };
        let has_root = window
            .root::<gpui_kit::component::Root>()
            .flatten()
            .is_some();
        if !has_root || window.has_active_dialog(cx) {
            return;
        }
        let (floor, ceiling) = scale.band;
        let visible_band = FrequencyBand::between(low.max(floor), high.min(ceiling));
        let visible_time = SampleSpan::between(view.start, view.start + view.len);
        let opened = Selection {
            time: self.selection,
            band: self.band,
        };
        let (Some(time), Some(band)) = (opened.time.or(visible_time), opened.band.or(visible_band))
        else {
            return;
        };
        if let Some(plot) = self.plot_view() {
            plot.update(cx, |plot, cx| plot.interrupt(cx));
        }
        let shell = cx.entity().downgrade();
        let dialog =
            cx.new(|cx| SelectionDialog::new(shell, scale, opened, (time, band), window, cx));
        window.open_dialog(cx, move |modal, _, _| {
            let confirm = dialog.clone();
            let ok = dialog.clone();
            let cancel = dialog.clone();
            let escape = dialog.clone();
            modal
                .title("Selection")
                .w(px(360.))
                .close_button(false)
                .overlay_closable(false)
                .child(dialog.clone())
                .footer(
                    h_flex()
                        .w_full()
                        .justify_end()
                        .gap_2()
                        .child(Button::new("cancel").label("Cancel").on_click(
                            move |_, window, cx| {
                                cancel.update(cx, |dialog, cx| dialog.cancel(cx));
                                window.close_dialog(cx);
                            },
                        ))
                        .child(Button::new("ok").primary().label("OK").on_click(
                            move |_, window, cx| {
                                if ok.update(cx, |dialog, cx| dialog.confirm(window, cx)) {
                                    window.close_dialog(cx);
                                }
                            },
                        )),
                )
                .on_ok(move |_, window, cx| {
                    confirm.update(cx, |dialog, cx| dialog.confirm(window, cx))
                })
                .on_cancel(move |_, _, cx| {
                    escape.update(cx, |dialog, cx| dialog.cancel(cx));
                    true
                })
        });
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

    /// A shell under the toolkit's root, which the dialog layer needs, with a described capture.
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

    fn open_dialog(cx: &mut gpui_kit::VisualTestContext, shell: &Entity<Shell>) {
        shell.update_in(cx, |shell, window, cx| {
            shell.open_selection_dialog(window, cx)
        });
        // The opening animation runs on the wall clock, and a click lands where it ends.
        std::thread::sleep(std::time::Duration::from_millis(400));
        settle(cx);
    }

    /// Let the dialog's opening animation end and a frame report focus changes.
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

    fn dialog_open(cx: &mut gpui_kit::VisualTestContext) -> bool {
        cx.update(|window, cx| window.has_active_dialog(cx))
    }

    fn span(a: u64, b: u64) -> Option<SampleSpan> {
        SampleSpan::between(a, b)
    }

    #[gpui_kit::test]
    fn a_typed_value_moves_the_selection_live_and_enter_keeps_it(cx: &mut TestAppContext) {
        let (shell, cx) = open(cx);
        shell.update_in(cx, |shell, _, _| shell.selection = span(100, 200));
        open_dialog(cx, &shell);
        assert!(dialog_open(cx));
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
        assert!(!dialog_open(cx), "Enter keeps it and closes");
    }

    #[gpui_kit::test]
    fn escape_restores_the_selection_the_dialog_opened_on(cx: &mut TestAppContext) {
        let (shell, cx) = open(cx);
        shell.update_in(cx, |shell, _, _| shell.selection = span(100, 200));
        open_dialog(cx, &shell);
        keys(cx, "5 0 tab");
        assert_eq!(selected(cx, &shell).time, span(50, 200));
        keys(cx, "7 escape");
        assert!(!dialog_open(cx));
        assert_eq!(selected(cx, &shell).time, span(100, 200));
    }

    #[gpui_kit::test]
    fn a_bad_value_keeps_the_dialog_open_and_changes_nothing(cx: &mut TestAppContext) {
        let (shell, cx) = open(cx);
        shell.update_in(cx, |shell, _, _| shell.selection = span(100, 200));
        open_dialog(cx, &shell);
        keys(cx, "x enter");
        assert!(dialog_open(cx), "OK waits for a usable value");
        assert_eq!(selected(cx, &shell).time, span(100, 200));
        keys(cx, "backspace 1 5 0 enter");
        assert!(!dialog_open(cx));
        assert_eq!(selected(cx, &shell).time, span(150, 200));
    }

    #[gpui_kit::test]
    fn with_nothing_selected_a_checked_axis_selects_the_visible_range(cx: &mut TestAppContext) {
        let (shell, cx) = open(cx);
        open_dialog(cx, &shell);
        assert_eq!(selected(cx, &shell), Selection::default());
        cx.update(|window, cx| window.click("band-axis", cx));
        settle(cx);
        let band = selected(cx, &shell).band.expect("a band");
        assert_eq!(
            (band.low(), band.high()),
            (-500., 500.),
            "the whole visible band"
        );
        assert_eq!(selected(cx, &shell).time, None);
        keys(cx, "- 1 0 0 enter");
        assert!(!dialog_open(cx));
        let band = selected(cx, &shell).band.expect("a band");
        assert_eq!((band.low(), band.high()), (-100., 500.));
    }
}
