use super::*;
use argand_dsp::DynamicRange;
use gpui_kit::{TestAppContext, WindowHandle};

fn open(cx: &mut TestAppContext) -> WindowHandle<Shell> {
    cx.update(|cx| {
        gpui_kit::init(cx);
        settings_ui::init(cx);
        navigation_ui::init(cx);
        hints::init(cx);
    });
    let handle = cx.add_window(|window, cx| {
        Shell::new(Config::default(), None, Session::default(), window, cx)
    });
    cx.run_until_parked();
    handle
}

/// Opens the hint, changes the range as its recommendation would, then closes it.
fn change_and_close(cx: &mut TestAppContext, revert: bool) -> (Settings, Settings) {
    let handle = open(cx);
    handle
        .update(cx, |shell, window, cx| {
            shell.open(Origin::new("/captures/a.iqw".into()), window, cx);
        })
        .unwrap();
    cx.run_until_parked();
    let (hint, opening) = handle
        .read_with(cx, |shell, _| (shell.analysis_hint.clone(), shell.settings))
        .unwrap();
    cx.update_window(handle.into(), |_, window, cx| {
        hint.update(cx, |hint, cx| hint.open(window, cx));
    })
    .unwrap();
    cx.run_until_parked();
    handle
        .update(cx, |shell, _, cx| {
            shell.settings.dynamic_range = DynamicRange::Fixed(42.);
            let hint = shell.analysis_hint.clone();
            hint.update(cx, |hint, cx| hint.close(revert, cx));
        })
        .unwrap();
    cx.run_until_parked();
    let closed = handle.read_with(cx, |shell, _| shell.settings).unwrap();
    (opening, closed)
}

#[gpui_kit::test]
fn the_shell_follows_the_mouse_into_and_out_of_the_window(cx: &mut TestAppContext) {
    let handle = open(cx);
    let inside = |cx: &mut TestAppContext| {
        handle
            .read_with(cx, |shell, _| shell.pointer_in_window)
            .unwrap()
    };
    assert!(!inside(cx), "nothing is known before the first move");
    let mut input = gpui_kit::VisualTestContext::from_window(handle.into(), cx);
    let position = gpui_kit::point(gpui_kit::px(100.), gpui_kit::px(100.));
    input.simulate_mouse_move(position, None, gpui_kit::Modifiers::default());
    assert!(inside(cx));
    input.simulate_event(gpui_kit::MouseExitEvent {
        position,
        pressed_button: None,
        modifiers: gpui_kit::Modifiers::default(),
    });
    assert!(!inside(cx));
}

#[gpui_kit::test]
fn edit_analysis_opens_the_hint_and_closes_it_keeping_the_values(cx: &mut TestAppContext) {
    let handle = open(cx);
    let toggled = |cx: &mut TestAppContext| {
        handle
            .update(cx, |shell, window, cx| {
                shell.edit_analysis(&EditAnalysis, window, cx);
            })
            .unwrap();
        cx.run_until_parked();
        handle
            .read_with(cx, |shell, cx| shell.analysis_hint.read(cx).is_open())
            .unwrap()
    };
    assert!(!toggled(cx), "without a document there is nothing to edit");
    handle
        .update(cx, |shell, window, cx| {
            shell.open(
                Origin::new(std::path::PathBuf::from("/captures/a.iqw")),
                window,
                cx,
            );
        })
        .unwrap();
    cx.run_until_parked();
    let toggle = |cx: &mut TestAppContext| {
        handle
            .update(cx, |shell, window, cx| {
                shell.edit_analysis(&EditAnalysis, window, cx);
            })
            .unwrap();
        cx.run_until_parked();
        handle
            .read_with(cx, |shell, cx| shell.analysis_hint.read(cx).is_open())
            .unwrap()
    };
    assert!(toggle(cx), "the first press opens the hint");
    handle
        .update(cx, |shell, _, _| {
            shell.settings.dynamic_range = DynamicRange::Fixed(42.);
        })
        .unwrap();
    assert!(!toggle(cx), "the second press closes it");
    let kept = handle.read_with(cx, |shell, _| shell.settings).unwrap();
    assert_eq!(
        kept.dynamic_range,
        DynamicRange::Fixed(42.),
        "keeping the values"
    );
}

#[gpui_kit::test]
fn escape_restores_the_settings_the_hint_opened_with(cx: &mut TestAppContext) {
    let (opening, closed) = change_and_close(cx, true);
    assert_ne!(opening.dynamic_range, DynamicRange::Fixed(42.));
    assert_eq!(closed, opening);
}

#[gpui_kit::test]
fn other_closes_keep_what_changed(cx: &mut TestAppContext) {
    let (_, closed) = change_and_close(cx, false);
    assert_eq!(closed.dynamic_range, DynamicRange::Fixed(42.));
}
