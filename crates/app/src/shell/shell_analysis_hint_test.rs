use super::*;
use argand_dsp::DynamicRange;
use gpui_kit::{TestAppContext, WindowHandle};

fn open(cx: &mut TestAppContext) -> WindowHandle<Shell> {
    cx.update(|cx| {
        gpui_kit::init(cx);
        settings_ui::init(cx);
        crate::shell::plot::navigation_ui::init(cx);
        hints::init(cx);
    });
    let handle = cx.add_window(|window, cx| {
        Shell::new(Config::default(), None, Session::default(), window, cx)
    });
    cx.run_until_parked();
    handle
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
