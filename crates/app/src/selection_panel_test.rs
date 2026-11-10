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
        let view = cx.new(|cx| Shell::new(Config::default(), None, Session::default(), window, cx));
        shell = Some(view.clone());
        gpui_kit::component::Root::new(view, window, cx)
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
fn the_panel_s_own_keys_close_it_keeping_the_selection(cx: &mut TestAppContext) {
    let (shell, cx) = open(cx);
    shell.update_in(cx, |shell, _, _| shell.selection = span(100, 200));
    toggle(cx, &shell);
    assert!(panel_open(cx, &shell));
    keys(cx, "secondary-a 1 2 0 secondary-e");
    assert!(!panel_open(cx, &shell), "the keys that opened it close it");
    assert_eq!(
        selected(cx, &shell).time,
        span(120, 200),
        "keeping what was typed"
    );
}

#[gpui_kit::test]
fn the_panel_and_the_analysis_hint_never_stand_together(cx: &mut TestAppContext) {
    let (shell, cx) = open(cx);
    shell.update_in(cx, |shell, _, _| shell.selection = span(100, 200));
    let analysis = |cx: &mut gpui_kit::VisualTestContext| {
        shell.read_with(cx, |shell, cx| shell.analysis_hint.read(cx).is_open())
    };
    toggle(cx, &shell);
    keys(cx, "secondary-,");
    assert!(analysis(cx), "the settings open from the panel");
    assert!(!panel_open(cx, &shell), "and close it");
    toggle(cx, &shell);
    assert!(panel_open(cx, &shell), "the panel opens from the settings");
    assert!(!analysis(cx), "and closes them");
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
    keys(cx, "up");
    assert_eq!(
        selected(cx, &shell).time,
        span(122, 200),
        "the stepped field shows its step, so the next one goes on from it"
    );
    keys(cx, "enter");
    assert_eq!(
        selected(cx, &shell).time,
        span(122, 200),
        "and Enter keeps it"
    );
}

#[gpui_kit::test]
fn a_segment_selects_its_kind_from_the_visible_range_and_opens_the_panel(cx: &mut TestAppContext) {
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
