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
        crate::shell::plot::navigation_ui::init(cx);
        hints::init(cx);
        // The window's own Tab binding, which the shell swallows while the hint is open.
        window_keys(cx);
    });
    let mut shell = None;
    let handle = cx.add_window(|window, cx| {
        let view = cx.new(|cx| Shell::new(Config::default(), None, Session::default(), window, cx));
        shell = Some(view.clone());
        Root::new(view, window, cx)
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
    let committed = bounds(cx);
    assert_eq!(
        committed.right(),
        resting.right(),
        "the value stays right-aligned"
    );
    assert_eq!(committed.top(), resting.top());
    assert_eq!(committed.size.height, resting.size.height);
}

#[gpui_kit::test]
fn a_held_stepper_repeats_until_the_pointer_leaves_it(cx: &mut TestAppContext) {
    let w = open(cx);
    open_hint(cx, &w);
    let opening = settings(cx, &w).overlap;
    let button = cx
        .update_window(w.handle.into(), |_, window, _| {
            window.find("overlap-up").bounds().center()
        })
        .expect("the window is open");
    let none = gpui_kit::Modifiers::default();
    let mut input = gpui_kit::VisualTestContext::from_window(w.handle.into(), cx);
    input.simulate_mouse_move(button, None, none);
    input.simulate_mouse_down(button, MouseButton::Left, none);
    input.run_until_parked();
    assert_eq!(
        settings(&mut input, &w).overlap,
        opening + 1,
        "the press steps"
    );
    input
        .executor()
        .advance_clock(REPEAT_DELAY + REPEAT_EVERY * 3);
    input.run_until_parked();
    let held = settings(&mut input, &w).overlap;
    assert!(held > opening + 1, "holding repeats");
    input.simulate_mouse_move(
        gpui_kit::point(px(5.), px(5.)),
        Some(MouseButton::Left),
        none,
    );
    input.run_until_parked();
    input.executor().advance_clock(REPEAT_EVERY * 10);
    input.run_until_parked();
    assert_eq!(
        settings(&mut input, &w).overlap,
        held,
        "leaving stops the repeat"
    );
}

/// Which value holds the keyboard, or `None` while Reset to defaults or nothing does.
fn focused(cx: &mut TestAppContext, w: &Window_) -> Option<Stop> {
    let editor = editor(cx, w);
    cx.update_window(w.handle.into(), |_, window, cx| {
        editor.read(cx).focused(window, cx)
    })
    .expect("the window is open")
}

fn on_reset(cx: &mut TestAppContext, w: &Window_) -> bool {
    let editor = editor(cx, w);
    cx.update_window(w.handle.into(), |_, window, cx| {
        editor.read(cx).defaults_focus.is_focused(window)
    })
    .expect("the window is open")
}

#[gpui_kit::test]
fn the_keyboard_starts_on_the_first_value_and_tab_goes_round_the_hint(cx: &mut TestAppContext) {
    let w = open(cx);
    open_hint(cx, &w);
    frame(cx, &w);
    assert_eq!(
        focused(cx, &w),
        Some(Stop::Choice(Choice::Fft)),
        "Ctrl+, puts the keyboard on FFT size"
    );
    for stop in [
        Stop::Choice(Choice::Window),
        Stop::Number(Number::Overlap),
        Stop::Choice(Choice::Aggregation),
        Stop::Choice(Choice::Mode),
        Stop::Choice(Choice::Palette),
    ] {
        press(cx, &w, "tab");
        assert_eq!(
            focused(cx, &w),
            Some(stop),
            "Tab reaches every value in order"
        );
    }
    press(cx, &w, "tab");
    assert!(on_reset(cx, &w), "then Reset to defaults");
    press(cx, &w, "tab");
    assert_eq!(
        focused(cx, &w),
        Some(Stop::Choice(Choice::Fft)),
        "and goes round"
    );
    press(cx, &w, "shift-tab");
    assert!(on_reset(cx, &w), "Shift+Tab goes back round");
    press(cx, &w, "shift-tab");
    assert_eq!(
        focused(cx, &w),
        Some(Stop::Choice(Choice::Palette)),
        "and back"
    );
    assert!(is_open(cx, &w), "and none of it closes the hint");
}

#[gpui_kit::test]
fn a_fixed_range_is_a_stop_of_its_own(cx: &mut TestAppContext) {
    let w = open(cx);
    open_hint(cx, &w);
    frame(cx, &w);
    for _ in 0..4 {
        press(cx, &w, "tab");
    }
    assert_eq!(focused(cx, &w), Some(Stop::Choice(Choice::Mode)));
    // Below measured peak is the second mode, which makes Range a number.
    press(cx, &w, "enter");
    press(cx, &w, "down");
    press(cx, &w, "enter");
    assert!(matches!(
        settings(cx, &w).dynamic_range,
        DynamicRange::Fixed(_)
    ));
    press(cx, &w, "tab");
    assert_eq!(
        focused(cx, &w),
        Some(Stop::Number(Number::Range)),
        "Tab reaches Range"
    );
    press(cx, &w, "tab");
    assert_eq!(focused(cx, &w), Some(Stop::Choice(Choice::Palette)));
    press(cx, &w, "shift-tab");
    assert_eq!(
        focused(cx, &w),
        Some(Stop::Number(Number::Range)),
        "and Shift+Tab"
    );
}

#[gpui_kit::test]
fn the_keyboard_chooses_in_a_list_and_steps_a_number(cx: &mut TestAppContext) {
    let w = open(cx);
    open_hint(cx, &w);
    frame(cx, &w);
    let opening = settings(cx, &w);
    let editor = editor(cx, &w);
    let list_open = |cx: &mut TestAppContext| editor.read_with(cx, |e, cx| e.list_open(cx));
    press(cx, &w, "enter");
    assert!(list_open(cx), "Enter opens the focused list");
    press(cx, &w, "escape");
    assert!(!list_open(cx), "Escape closes the list");
    assert!(is_open(cx, &w), "and only the list");
    press(cx, &w, "down");
    assert!(list_open(cx), "Down opens it too");
    press(cx, &w, "down");
    press(cx, &w, "enter");
    assert!(!list_open(cx), "Enter chooses");
    assert_eq!(
        settings(cx, &w).fft_size,
        opening.fft_size * 2,
        "the next size"
    );
    assert_eq!(
        focused(cx, &w),
        Some(Stop::Choice(Choice::Fft)),
        "and the keyboard stays"
    );
    press(cx, &w, "tab");
    press(cx, &w, "tab");
    assert_eq!(focused(cx, &w), Some(Stop::Number(Number::Overlap)));
    press(cx, &w, "up");
    assert_eq!(
        settings(cx, &w).overlap,
        opening.overlap + 1,
        "Up steps a number"
    );
    press(cx, &w, "down");
    press(cx, &w, "down");
    assert_eq!(
        settings(cx, &w).overlap,
        opening.overlap - 1,
        "Down steps it back"
    );
    assert!(is_open(cx, &w));
}

#[gpui_kit::test]
fn enter_on_reset_to_defaults_resets_and_keeps_the_hint(cx: &mut TestAppContext) {
    let w = open(cx);
    open_hint(cx, &w);
    frame(cx, &w);
    type_overlap(cx, &w, "10");
    press(cx, &w, "enter");
    assert_eq!(settings(cx, &w).overlap, 10);
    press(cx, &w, "tab");
    press(cx, &w, "tab");
    press(cx, &w, "tab");
    press(cx, &w, "tab");
    assert!(on_reset(cx, &w));
    press(cx, &w, "enter");
    assert_eq!(settings(cx, &w), Settings::from_config(&Config::default()));
    assert!(is_open(cx, &w), "Enter on Reset keeps the hint");
}

/// Gives the open document a warned range, as a quiet capture's first picture does.
fn warn(cx: &mut TestAppContext, w: &Window_) {
    w.shell.update(cx, |shell, cx| {
        if let Some(file) = shell.file.as_mut() {
            file.state.document.show_a_warned_range();
        }
        cx.notify();
    });
    frame(cx, w);
}

fn balloon_drawn(cx: &mut TestAppContext, w: &Window_) -> bool {
    cx.update_window(w.handle.into(), |_, window, _| {
        window.try_find("analysis-range-balloon").is_some()
    })
    .expect("the window is open")
}

#[gpui_kit::test]
fn a_warned_range_shows_its_balloon_until_a_list_opens_or_its_cross(cx: &mut TestAppContext) {
    let w = open(cx);
    warn(cx, &w);
    open_hint(cx, &w);
    // The first frame measures where the balloon goes, the next one draws it.
    frame(cx, &w);
    assert!(
        balloon_drawn(cx, &w),
        "the warning is explained beside the hint"
    );
    cx.update_window(w.handle.into(), |_, window, cx| {
        window.within("analysis-palette").click("input", cx);
    })
    .expect("the window is open");
    cx.run_until_parked();
    frame(cx, &w);
    assert!(!balloon_drawn(cx, &w), "an open list is not covered");
    press(cx, &w, "escape");
    assert!(
        balloon_drawn(cx, &w),
        "the balloon comes back with the list closed"
    );
    let opening = settings(cx, &w);
    cx.update_window(w.handle.into(), |_, window, cx| {
        window.click("analysis-range-balloon-close", cx);
    })
    .expect("the window is open");
    cx.run_until_parked();
    frame(cx, &w);
    assert!(!balloon_drawn(cx, &w), "the cross hides the balloon");
    assert_eq!(settings(cx, &w), opening, "without applying the range");
    assert!(is_open(cx, &w), "and keeps the hint");
}

fn acknowledge_range(cx: &mut TestAppContext, w: &Window_) {
    w.shell.update(cx, |shell, cx| {
        let settings = shell.settings;
        let file = shell.file.as_mut().expect("the test document is open");
        file.state.document.show_a_range(settings.dynamic_range);
        file.state.record_for_test(&settings);
        cx.notify();
    });
    frame(cx, w);
}

#[gpui_kit::test]
fn hidden_range_advice_does_not_retain_transient_hover(cx: &mut TestAppContext) {
    let w = open(cx);
    acknowledge_range(cx, &w);
    open_hint(cx, &w);
    frame(cx, &w);
    cx.update_window(w.handle.into(), |_, window, cx| {
        window.click("analysis-range-balloon-close", cx);
    })
    .expect("the window is open");
    frame(cx, &w);
    let anchor = cx
        .update_window(w.handle.into(), |_, window, _| {
            window.find("analysis-range-advice").bounds().center()
        })
        .expect("the window is open");
    let mut input = gpui_kit::VisualTestContext::from_window(w.handle.into(), cx);
    let modifiers = gpui_kit::Modifiers::default();
    input.simulate_mouse_move(gpui_kit::point(px(5.), px(5.)), None, modifiers);
    frame(&mut input, &w);
    input.simulate_mouse_move(anchor, None, modifiers);
    frame(&mut input, &w);
    assert!(balloon_drawn(&mut input, &w));
    input
        .update_window(w.handle.into(), |_, window, cx| {
            window.click("analysis-range-balloon", cx);
        })
        .expect("the window is open");
    frame(&mut input, &w);
    assert!(matches!(
        settings(&mut input, &w).dynamic_range,
        DynamicRange::Fixed(_)
    ));
    acknowledge_range(&mut input, &w);
    assert!(!balloon_drawn(&mut input, &w));
    input.simulate_mouse_move(gpui_kit::point(px(5.), px(5.)), None, modifiers);
    frame(&mut input, &w);
    press(&mut input, &w, "secondary-r");
    assert_eq!(
        settings(&mut input, &w).dynamic_range,
        DynamicRange::Default
    );
    acknowledge_range(&mut input, &w);
    frame(&mut input, &w);
    assert!(
        !balloon_drawn(&mut input, &w),
        "hidden advice cannot retain its former hover"
    );
    input.simulate_mouse_move(anchor, None, modifiers);
    frame(&mut input, &w);
    assert!(
        balloon_drawn(&mut input, &w),
        "a fresh entry still reopens it"
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
fn a_kept_range_alone_does_not_claim_the_saved_settings(cx: &mut TestAppContext) {
    let w = open(cx);
    open_hint(cx, &w);
    let opening = settings(cx, &w);
    for _ in 0..4 {
        press(cx, &w, "tab");
    }
    press(cx, &w, "enter");
    press(cx, &w, "down");
    press(cx, &w, "enter");
    open_hint(cx, &w);
    assert!(!is_open(cx, &w));
    assert_ne!(settings(cx, &w).dynamic_range, opening.dynamic_range);
    let saved = w
        .shell
        .read_with(cx, |shell, _| shell.session.analysis_settings);
    assert_eq!(
        saved, None,
        "a range the file never keeps was saved as a change"
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
        crate::numbers::number(overlap),
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
