use super::*;
use crate::analysis::{FileInfo, Update};
use crate::session;
use argand_core::{Domain, SampleFormat, SampleType, SignalMeta};
use gpui_kit::test::TestWindowExt;
use gpui_kit::{TestAppContext, WindowHandle};
use std::path::{Path, PathBuf};

fn open(cx: &mut TestAppContext) -> WindowHandle<Shell> {
    cx.update(|cx| {
        gpui_kit::init(cx);
        settings_ui::init(cx);
        crate::shell::plot::navigation_ui::init(cx);
        hints::init(cx);
        app_menu_ui::init(cx);
        window_keys(cx);
    });
    let handle = cx.add_window(|window, cx| {
        Shell::new(Config::default(), None, Session::default(), window, cx)
    });
    frame(cx, handle);
    handle
}

fn frame(cx: &mut TestAppContext, handle: WindowHandle<Shell>) {
    cx.update_window(handle.into(), |_, window, cx| window.render_frame(cx))
        .unwrap();
    cx.run_until_parked();
}

/// The window as a person would find it, which is a range to navigate.
fn open_document(cx: &mut TestAppContext, handle: WindowHandle<Shell>) {
    handle
        .update(cx, |shell, _, cx| {
            shell.view = Some(crate::navigation::View::full(1000));
            cx.notify();
        })
        .unwrap();
    frame(cx, handle);
}

fn vertical(cx: &mut TestAppContext, handle: WindowHandle<Shell>) -> bool {
    handle
        .read_with(cx, |shell, _| shell.session.orientation.vertical())
        .unwrap()
}

fn owner_focused(cx: &mut TestAppContext, handle: WindowHandle<Shell>) -> bool {
    handle
        .update(cx, |shell, window, _| shell.focus.is_focused(window))
        .unwrap()
}

#[gpui_kit::test]
fn only_the_other_orientation_segment_switches(cx: &mut TestAppContext) {
    let handle = open(cx);
    open_document(cx, handle);
    assert!(!vertical(cx, handle), "horizontal is the default");
    // Another control held the keyboard, as it would after a toolbar click.
    handle
        .update(cx, |_, window, cx| {
            let elsewhere = cx.focus_handle();
            window.focus(&elsewhere, cx);
        })
        .unwrap();
    assert!(!owner_focused(cx, handle), "the keyboard left the owner");
    cx.update_window(handle.into(), |_, window, cx| {
        window.click("orientation-horizontal", cx);
    })
    .unwrap();
    assert!(
        owner_focused(cx, handle),
        "a click on either segment hands the keyboard back"
    );
    assert!(!vertical(cx, handle), "the mode in force is not an action");
    cx.update_window(handle.into(), |_, window, cx| {
        window.click("orientation-vertical", cx);
    })
    .unwrap();
    assert!(vertical(cx, handle), "one click is one switch");
    cx.update_window(handle.into(), |_, window, cx| {
        window.click("orientation-vertical", cx);
    })
    .unwrap();
    assert!(vertical(cx, handle), "and it stays put");
    cx.update_window(handle.into(), |_, window, cx| {
        window.click("orientation-horizontal", cx);
    })
    .unwrap();
    assert!(!vertical(cx, handle));
}

/// The window as a person finds it, with the keys the window itself binds.
fn open_window(cx: &mut TestAppContext) -> (Entity<Shell>, &mut gpui_kit::VisualTestContext) {
    cx.update(|cx| {
        gpui_kit::init(cx);
        settings_ui::init(cx);
        crate::shell::plot::navigation_ui::init(cx);
        hints::init(cx);
        app_menu_ui::init(cx);
        window_keys(cx);
        crate::theme::install(ThemeMode::Dark, cx);
    });
    let (shell, cx) = cx.add_window_view(|window, cx| {
        Shell::new(Config::default(), None, Session::default(), window, cx)
    });
    cx.simulate_resize(gpui_kit::size(px(800.), px(600.)));
    draw(cx);
    (shell, cx)
}

fn draw(cx: &mut gpui_kit::VisualTestContext) {
    cx.run_until_parked();
    cx.update(|window, cx| window.render_frame(cx));
    cx.run_until_parked();
}

/// A document, which is what binds the window's own keys to a focus path.
///
/// The capture is nowhere near a real file and nothing analyses it, because
/// only the document's existence matters to a key context.
fn open_capture(cx: &mut gpui_kit::VisualTestContext, shell: &Entity<Shell>) {
    shell.update_in(cx, |shell, window, cx| {
        shell.open(
            Origin::new(PathBuf::from("/captures/session.iqw")),
            window,
            cx,
        );
    });
    draw(cx);
    shell.update_in(cx, |shell, _, cx| {
        shell.view = Some(crate::navigation::View::full(1000));
        cx.notify();
    });
    draw(cx);
}

/// The middle of the window, which is over the content and not the menu.
fn outside() -> gpui_kit::Point<gpui_kit::Pixels> {
    gpui_kit::point(px(400.), px(400.))
}

/// Where a drawn control is, so a click lands where a person would put it.
fn at(cx: &mut gpui_kit::VisualTestContext, id: &'static str) -> gpui_kit::Point<gpui_kit::Pixels> {
    cx.update(|window, _| window.find(id).bounds().center())
}

fn click(cx: &mut gpui_kit::VisualTestContext, id: &'static str) {
    let at = at(cx, id);
    cx.simulate_click(at, gpui_kit::Modifiers::default());
    draw(cx);
}

fn click_outside(cx: &mut gpui_kit::VisualTestContext) {
    cx.simulate_click(outside(), gpui_kit::Modifiers::default());
    draw(cx);
}

fn press(cx: &mut gpui_kit::VisualTestContext, keys: &str) {
    cx.simulate_keystrokes(keys);
    draw(cx);
}

/// Whether the shell has built the menu, which is its own answer.
fn menu_open(cx: &mut gpui_kit::VisualTestContext, shell: &Entity<Shell>) -> bool {
    shell.read_with(cx, |shell, _| shell.application_menu.is_some())
}

/// Whether the last frame drew the menu itself.
fn menu_drawn(cx: &mut gpui_kit::VisualTestContext) -> bool {
    cx.update(|window, _| window.try_find("popup-menu").is_some())
}

/// Whether the keyboard is in the menu, which is where it belongs.
fn menu_focused(cx: &mut gpui_kit::VisualTestContext, shell: &Entity<Shell>) -> bool {
    shell.update_in(cx, |shell, window, cx| {
        shell
            .application_menu
            .as_ref()
            .is_some_and(|menu| menu.read(cx).focus_handle(cx).is_focused(window))
    })
}

/// Whether the keyboard is where the menu found it, which is what dismissal
/// hands it back to.
fn keyboard_at_owner(cx: &mut gpui_kit::VisualTestContext, shell: &Entity<Shell>) -> bool {
    shell.update_in(cx, |shell, window, _| shell.focus.is_focused(window))
}

/// Whether the File branch holds the keyboard, which a digit row needs.
fn file_focused(cx: &mut gpui_kit::VisualTestContext, shell: &Entity<Shell>) -> bool {
    shell.update_in(cx, |shell, window, cx| {
        shell
            .application_file_menu
            .as_ref()
            .is_some_and(|file| file.read(cx).focus_handle(cx).is_focused(window))
    })
}

/// One available capture in the recent list, with a digit waiting for it.
fn recent(cx: &mut gpui_kit::VisualTestContext, shell: &Entity<Shell>, path: &str) {
    available_recent(cx, shell, std::slice::from_ref(&path.to_owned()), true);
}

/// Recent captures in a given availability, newest first.
fn available_recent(
    cx: &mut gpui_kit::VisualTestContext,
    shell: &Entity<Shell>,
    paths: &[String],
    available: bool,
) {
    shell.update_in(cx, |shell, _, cx| {
        shell.session.recent = paths
            .iter()
            .map(|path| session::Recent {
                path: PathBuf::from(path),
                hints: Default::default(),
            })
            .collect();
        shell.recent_files.refresh(&shell.session.recent);
        for path in paths {
            shell.recent_files.apply(PathBuf::from(path), available);
        }
        cx.notify();
    });
    draw(cx);
}

/// A capture a probe has just re-checked while the menu is open.
fn recheck(cx: &mut gpui_kit::VisualTestContext, shell: &Entity<Shell>, path: &str) {
    shell.update_in(cx, |shell, _, cx| {
        shell.recent_files.apply(PathBuf::from(path), false);
        cx.notify();
    });
    draw(cx);
}

/// Where a stock menu row is, which the toolkit records under its index.
fn row(cx: &mut gpui_kit::VisualTestContext, index: u64) -> gpui_kit::Point<gpui_kit::Pixels> {
    cx.update(|window, _| {
        window
            .within("submenu")
            .find(gpui_kit::ElementId::Integer(index))
            .bounds()
            .center()
    })
}

/// The analysis settings hint the menu's Settings row opens.
fn settings_open(cx: &mut gpui_kit::VisualTestContext, shell: &Entity<Shell>) -> bool {
    shell.read_with(cx, |shell, cx| shell.analysis_hint.read(cx).is_open())
}

/// The document the open menu replaced, once one of its rows opened something.
fn opened(cx: &mut gpui_kit::VisualTestContext, shell: &Entity<Shell>) -> Option<PathBuf> {
    shell.read_with(cx, |shell, _| {
        shell
            .file
            .as_ref()
            .map(|file| file.state.document.origin().path.clone())
    })
}

fn grid(cx: &mut gpui_kit::VisualTestContext, shell: &Entity<Shell>) -> bool {
    shell.read_with(cx, |shell, _| shell.session.show_grid)
}

#[gpui_kit::test]
fn the_button_and_the_key_open_the_menu_with_the_keyboard_in_it(cx: &mut TestAppContext) {
    let (shell, cx) = open_window(cx);
    open_capture(cx, &shell);
    click(cx, "application-menu-button");
    assert!(menu_open(cx, &shell), "the button opens the menu");
    assert!(menu_drawn(cx), "and the menu is drawn");
    assert!(menu_focused(cx, &shell), "with the keyboard inside it");
    click(cx, "application-menu-button");
    assert!(!menu_open(cx, &shell), "a second click closes it");
    press(cx, "f10");
    assert!(menu_open(cx, &shell), "the key opens it too");
    assert!(menu_focused(cx, &shell));
}

#[gpui_kit::test]
fn a_click_on_the_button_closes_the_menu_and_leaves_it_closed(cx: &mut TestAppContext) {
    let (shell, cx) = open_window(cx);
    open_capture(cx, &shell);
    press(cx, "f10");
    assert!(menu_open(cx, &shell));
    click(cx, "application-menu-button");
    assert!(!menu_open(cx, &shell));
    draw(cx);
    draw(cx);
    assert!(!menu_drawn(cx), "and nothing opens it again");
}

#[gpui_kit::test]
fn the_key_escape_and_an_outside_click_each_close_the_menu(cx: &mut TestAppContext) {
    let (shell, cx) = open_window(cx);
    open_capture(cx, &shell);
    for key in ["f10", "tab", "escape"] {
        press(cx, "f10");
        press(cx, key);
        assert!(!menu_open(cx, &shell), "{key} closes the menu");
        assert!(!menu_drawn(cx), "{key} leaves nothing drawn");
        assert!(
            keyboard_at_owner(cx, &shell),
            "{key} hands the keyboard back"
        );
    }
    press(cx, "f10");
    assert!(menu_open(cx, &shell));
    click_outside(cx);
    assert!(!menu_open(cx, &shell), "an outside click closes it");
    assert!(keyboard_at_owner(cx, &shell));
    // The next key reaches the window again, which is what dismissal promised.
    press(cx, "f10");
    assert!(menu_open(cx, &shell));
}

#[gpui_kit::test]
fn an_outside_click_and_wheel_reach_nothing_under_the_menu(cx: &mut TestAppContext) {
    let (shell, cx) = open_window(cx);
    open_capture(cx, &shell);
    let view = shell.read_with(cx, |shell, _| shell.view);
    press(cx, "f10");
    assert!(menu_open(cx, &shell));
    click_outside(cx);
    assert!(!menu_open(cx, &shell), "the click closed the menu");
    assert_eq!(
        shell.read_with(cx, |shell, _| shell.view),
        view,
        "and moved nothing under it"
    );
    assert!(
        !shell.read_with(cx, |shell, _| shell.title_drag_pending),
        "nor began a title drag"
    );
    press(cx, "f10");
    cx.simulate_event(gpui_kit::ScrollEvent {
        position: outside(),
        delta: gpui_kit::ScrollDelta::Pixels(gpui_kit::point(px(0.), px(80.))),
        ..Default::default()
    });
    draw(cx);
    assert!(menu_open(cx, &shell), "the wheel leaves the menu open");
    assert_eq!(
        shell.read_with(cx, |shell, _| shell.view),
        view,
        "and changes no view"
    );
}

#[gpui_kit::test]
fn the_backdrop_keeps_the_window_beneath_the_menu(cx: &mut TestAppContext) {
    let (shell, cx) = open_window(cx);
    open_capture(cx, &shell);
    let before = grid(cx, &shell);
    press(cx, "f10");
    assert!(menu_open(cx, &shell));
    // A control under the menu must not answer the click that closed it.
    click(cx, "toggle-grid");
    assert!(
        !menu_open(cx, &shell),
        "the control's click dismissed the menu"
    );
    assert_eq!(grid(cx, &shell), before, "and did not reach the control");
    // A press on the title bar must not become a window drag either.
    press(cx, "f10");
    let bar = gpui_kit::point(px(400.), px(14.));
    let none = gpui_kit::Modifiers::default();
    cx.simulate_mouse_down(bar, gpui_kit::MouseButton::Left, none);
    draw(cx);
    assert!(!menu_open(cx, &shell), "the press dismissed the menu");
    assert!(
        !shell.read_with(cx, |shell, _| shell.title_drag_pending),
        "and the title bar never saw it"
    );
    cx.simulate_mouse_up(bar, gpui_kit::MouseButton::Left, none);
    draw(cx);
}

// The arming this observes lives in the Linux title bar alone, since the
// other platforms leave moving and zooming the window to the native one.
#[cfg(target_os = "linux")]
#[gpui_kit::test]
fn the_toolbar_gaps_carry_the_title_drag_and_its_controls_do_not(cx: &mut TestAppContext) {
    let (shell, cx) = open_window(cx);
    open_capture(cx, &shell);
    let none = gpui_kit::Modifiers::default();
    let pending = |cx: &mut gpui_kit::VisualTestContext| {
        shell.read_with(cx, |shell, _| shell.title_drag_pending)
    };

    // The pixels between the application button and the orientation group
    // hold a gap, the separator and its margins, all bare title-bar pixels.
    let gap = cx.update(|window, _| {
        let first = window.find("application-menu-button").bounds();
        let second = window.find("orientation-horizontal").bounds();
        gpui_kit::point((first.right() + second.origin.x) / 2., first.center().y)
    });
    cx.simulate_mouse_down(gap, gpui_kit::MouseButton::Left, none);
    draw(cx);
    assert!(pending(cx), "a press in a toolbar gap arms the title drag");
    cx.simulate_mouse_up(gap, gpui_kit::MouseButton::Left, none);
    draw(cx);
    assert!(!pending(cx), "and releasing it clears the arming");

    // A press on a segment, on the group's own frame between the segments,
    // on Grid and on the application button never arms the drag.
    let frame = cx.update(|window, _| {
        let first = window.find("orientation-horizontal").bounds().center();
        let second = window.find("orientation-vertical").bounds().center();
        gpui_kit::point((first.x + second.x) / 2., first.y)
    });
    for at in [
        at(cx, "orientation-horizontal"),
        frame,
        at(cx, "toggle-grid"),
        at(cx, "application-menu-button"),
    ] {
        cx.simulate_mouse_down(at, gpui_kit::MouseButton::Left, none);
        draw(cx);
        assert!(!pending(cx), "a control press never arms the title drag");
        cx.simulate_mouse_up(at, gpui_kit::MouseButton::Left, none);
        draw(cx);
    }
    assert!(
        menu_open(cx, &shell),
        "the application button still opens its menu on its own press"
    );
    press(cx, "escape");
    assert!(!menu_open(cx, &shell));
}

#[gpui_kit::test]
fn a_right_or_middle_press_on_a_chrome_button_keeps_the_plot_focused(cx: &mut TestAppContext) {
    let (shell, cx) = open_window(cx);
    open_capture(cx, &shell);
    // A warned analysis makes the range item actionable, which is the only
    // state its wrapper exists in.
    shell.update_in(cx, |shell, _, cx| {
        let Some(file) = shell.file.as_mut() else {
            panic!("the capture never opened");
        };
        file.state.document.apply(Update::Opened(
            SignalMeta {
                sample_rate: 24_000.0,
                center_freq: 0.0,
                sample_type: SampleType::new(Domain::Iq, SampleFormat::I16),
                len_samples: 48_000,
                container: "raw",
                divisor: 32_768.0,
                source: PathBuf::from("/captures/session.iqw"),
            },
            FileInfo::default(),
        ));
        file.state.document.apply(Update::Ready {
            analysis: settings_ui::warned_analysis(),
            elapsed: std::time::Duration::ZERO,
        });
        file.state.record_for_test(&shell.settings);
        cx.notify();
    });
    draw(cx);
    let none = gpui_kit::Modifiers::default();
    let plot_focused = |cx: &mut gpui_kit::VisualTestContext| {
        shell.update_in(cx, |shell, window, cx| {
            shell.focus_target(cx).is_focused(window)
        })
    };
    assert!(plot_focused(cx), "the plot holds the keyboard first");

    // The middle and the right press on every chrome button leave the plot
    // holding the keyboard. The summary's right press is handled below,
    // because it opens the pinned hint, which takes the keyboard while it
    // is open by design.
    for (name, button) in [
        ("middle", gpui_kit::MouseButton::Middle),
        ("right", gpui_kit::MouseButton::Right),
    ] {
        for id in [
            "application-menu-button",
            "orientation-horizontal",
            "toggle-grid",
            "analysis-range",
            "analysis-settings",
        ] {
            if id == "analysis-settings" && name == "right" {
                continue;
            }
            let at = at(cx, id);
            cx.simulate_mouse_down(at, button, none);
            draw(cx);
            assert!(plot_focused(cx), "{name} on {id} keeps the plot focused");
            cx.simulate_mouse_up(at, button, none);
            draw(cx);
        }
    }

    // The left press still works through the wrapper. Grid toggles the
    // grid, which is its own action, and the plot keeps the keyboard, and
    // the pressed surface itself is paint the harness cannot see, which
    // the plan records instead.
    let grid_before = grid(cx, &shell);
    let grid_at = at(cx, "toggle-grid");
    cx.simulate_mouse_down(grid_at, gpui_kit::MouseButton::Left, none);
    draw(cx);
    assert!(
        plot_focused(cx),
        "a left press on Grid keeps the plot focused"
    );
    cx.simulate_mouse_up(grid_at, gpui_kit::MouseButton::Left, none);
    draw(cx);
    assert_ne!(
        grid(cx, &shell),
        grid_before,
        "Grid still acts on a left click"
    );

    // The summary's right press still toggles the pinned hint through the
    // wrapper, and the plot has the keyboard back once the hint closes.
    let hint_open = |cx: &mut gpui_kit::VisualTestContext| {
        shell.read_with(cx, |shell, cx| shell.analysis_hint.read(cx).is_open())
    };
    let summary_at = at(cx, "analysis-settings");
    cx.simulate_mouse_down(summary_at, gpui_kit::MouseButton::Right, none);
    draw(cx);
    assert!(
        hint_open(cx),
        "the right press still toggles the pinned hint through the wrapper"
    );
    press(cx, "escape");
    draw(cx);
    assert!(!hint_open(cx), "and escape closes it again");

    // The harness has no PlotView, so the focus target here is the shell
    // handle itself rather than a plot surface; what the assertions prove
    // is that no press moves the keyboard off the target that holds it,
    // which is what the plot keys depend on.
    assert!(plot_focused(cx), "the plot has the keyboard at the end");
}

#[gpui_kit::test]
fn enter_on_a_view_row_dispatches_its_action_once(cx: &mut TestAppContext) {
    let (shell, cx) = open_window(cx);
    open_capture(cx, &shell);
    let before = grid(cx, &shell);
    press(cx, "f10");
    // Down to the View branch, right into it, then confirm its first row.
    press(cx, "down down down right enter");
    assert!(!menu_open(cx, &shell), "the menu closed");
    assert_ne!(grid(cx, &shell), before, "one confirmation is one toggle");
}

#[gpui_kit::test]
fn view_offers_the_theme_without_a_document(cx: &mut TestAppContext) {
    let (shell, cx) = open_window(cx);
    let palette = shell.read_with(cx, |shell, _| shell.settings.colormap);
    let chosen = |cx: &mut gpui_kit::VisualTestContext| {
        let mode = cx.update(|_, cx| (cx.theme().mode, cx.theme().status_bar));
        let session = shell.read_with(cx, |shell, _| shell.session.theme);
        (mode, session)
    };
    // View is the third branch, and its last row is the Theme branch.
    press(cx, "f10");
    press(cx, "down down down right up right down enter");
    assert!(!menu_open(cx, &shell), "the menu closed");
    let light_bar = gpui_kit::rgb(0xf3f3f3).into();
    let dark_bar = gpui_kit::rgb(0x171717).into();
    assert_eq!(
        chosen(cx),
        ((ThemeMode::Light, light_bar), Some(Theme::Light))
    );
    press(cx, "f10");
    press(cx, "down down down right up right down down enter");
    assert_eq!(chosen(cx), ((ThemeMode::Dark, dark_bar), Some(Theme::Dark)));
    assert_eq!(
        shell.read_with(cx, |shell, _| shell.settings.colormap),
        palette,
        "the interface theme leaves the spectrogram palette alone"
    );
}

#[gpui_kit::test]
fn a_digit_answers_only_in_the_file_branch(cx: &mut TestAppContext) {
    let (shell, cx) = open_window(cx);
    open_capture(cx, &shell);
    recent(cx, &shell, "/captures/hfdl.iqw");
    press(cx, "f10");
    press(cx, "1");
    assert!(menu_open(cx, &shell), "a digit in the menu bar is no row");
    assert_eq!(
        opened(cx, &shell).as_deref(),
        Some(Path::new("/captures/session.iqw")),
        "and opened nothing"
    );
    press(cx, "down right");
    assert!(file_focused(cx, &shell), "Right enters the File branch");
    press(cx, "1");
    assert!(!menu_open(cx, &shell), "the capture replaced the menu");
    assert_eq!(
        opened(cx, &shell).as_deref(),
        Some(Path::new("/captures/hfdl.iqw"))
    );
}

#[gpui_kit::test]
fn a_recent_row_opens_its_capture(cx: &mut TestAppContext) {
    let (shell, cx) = open_window(cx);
    open_capture(cx, &shell);
    recent(cx, &shell, "/captures/beacon.iqw");
    press(cx, "f10");
    // Into File, past the command and its separator, onto the capture.
    press(cx, "down right");
    assert!(file_focused(cx, &shell));
    press(cx, "down enter");
    draw(cx);
    assert!(!menu_open(cx, &shell), "the menu closed");
    assert_eq!(
        opened(cx, &shell).as_deref(),
        Some(Path::new("/captures/beacon.iqw"))
    );
}

#[gpui_kit::test]
fn the_settings_hint_opens_from_the_menu_itself(cx: &mut TestAppContext) {
    let (shell, cx) = open_window(cx);
    open_capture(cx, &shell);
    press(cx, "f10");
    // Down to File, right into it, then past the command to Settings.
    press(cx, "down right");
    assert!(file_focused(cx, &shell));
    press(cx, "down enter");
    draw(cx);
    assert!(!menu_open(cx, &shell), "the menu closed");
    assert!(settings_open(cx, &shell), "and the settings hint opened");
}

#[gpui_kit::test]
fn a_click_on_a_recent_row_opens_that_capture(cx: &mut TestAppContext) {
    let (shell, cx) = open_window(cx);
    open_capture(cx, &shell);
    recent(cx, &shell, "/captures/beacon.iqw");
    press(cx, "f10");
    // The File branch is on screen, and its sixth row is the first capture.
    press(cx, "down right");
    assert!(file_focused(cx, &shell));
    let capture = row(cx, 5);
    cx.simulate_click(capture, gpui_kit::Modifiers::default());
    draw(cx);
    assert!(!menu_open(cx, &shell), "the menu closed");
    assert_eq!(
        opened(cx, &shell).as_deref(),
        Some(Path::new("/captures/beacon.iqw"))
    );
}

#[gpui_kit::test]
fn a_digit_opens_the_row_that_was_drawn(cx: &mut TestAppContext) {
    let (shell, cx) = open_window(cx);
    open_capture(cx, &shell);
    let paths = [
        "/captures/hfdl.iqw".to_owned(),
        "/captures/beacon.iqw".to_owned(),
    ];
    available_recent(cx, &shell, &paths, true);
    press(cx, "f10");
    press(cx, "down right");
    assert!(file_focused(cx, &shell));
    // A probe finds the first capture gone while the menu stands open.
    recheck(cx, &shell, "/captures/hfdl.iqw");
    assert!(
        menu_open(cx, &shell),
        "the menu keeps the rows it was drawn with"
    );
    press(cx, "1");
    assert_eq!(
        opened(cx, &shell).as_deref(),
        Some(Path::new("/captures/hfdl.iqw")),
        "the digit opened the row that was drawn, not the first available"
    );
}

#[gpui_kit::test]
fn enter_and_right_each_enter_a_submenu_before_running_its_row(cx: &mut TestAppContext) {
    let (shell, cx) = open_window(cx);
    open_capture(cx, &shell);
    for key in ["enter", "right"] {
        let before = grid(cx, &shell);
        press(cx, "f10");
        press(cx, "down down down");
        press(cx, key);
        assert!(menu_open(cx, &shell), "the menu stays open");
        assert!(!menu_focused(cx, &shell), "the branch holds the keyboard");
        assert_eq!(grid(cx, &shell), before, "opening a branch runs no row");
        press(cx, "enter");
        assert_ne!(grid(cx, &shell), before, "Enter runs the branch's row");
    }
}

/// The View row of the menu bar, found while it is the only menu on screen.
fn view_row(cx: &mut gpui_kit::VisualTestContext) -> gpui_kit::Point<gpui_kit::Pixels> {
    cx.update(|window, _| {
        let menu = window.find("popup-menu").bounds();
        menu.center() + gpui_kit::point(px(0.), menu.size.height / 4.)
    })
}

#[gpui_kit::test]
fn the_keys_reach_the_menu_after_a_hover_drops_the_focused_branch(cx: &mut TestAppContext) {
    let (shell, cx) = open_window(cx);
    open_capture(cx, &shell);
    for key in ["f10", "tab", "escape"] {
        press(cx, "f10");
        let row = view_row(cx);
        press(cx, "down right");
        assert!(file_focused(cx, &shell), "{key} found the File branch");
        // Crossing to View stops drawing File while its handle keeps the focus.
        cx.simulate_mouse_move(row, None, gpui_kit::Modifiers::default());
        draw(cx);
        press(cx, "down right");
        assert!(
            file_focused(cx, &shell),
            "{key} found the keys on the menu again, and Down moved the selection"
        );
        press(cx, key);
        assert!(!menu_open(cx, &shell), "{key} closes the menu");
        assert!(
            keyboard_at_owner(cx, &shell),
            "{key} hands the keyboard back"
        );
    }
}

/// How wide a stock menu row is, which changes with what the row carries.
fn row_width(cx: &mut gpui_kit::VisualTestContext, index: u64) -> gpui_kit::Pixels {
    cx.update(|window, _| {
        window
            .within("submenu")
            .find(gpui_kit::ElementId::Integer(index))
            .bounds()
            .size
            .width
    })
}

/// How tall a stock menu row is, which grows when a label wraps.
fn row_height(cx: &mut gpui_kit::VisualTestContext, index: u64) -> gpui_kit::Pixels {
    cx.update(|window, _| {
        window
            .within("submenu")
            .find(gpui_kit::ElementId::Integer(index))
            .bounds()
            .size
            .height
    })
}

/// The View submenu's "Vertical orientation" row, the first index only the
/// View branch has.
const ORIENTATION_ROW: u64 = 2;

/// The File submenu's separator, which spans the whole menu.
const FILE_SEPARATOR: u64 = 4;

#[gpui_kit::test]
fn an_action_row_draws_the_shared_keycap_when_its_binding_resolves(cx: &mut TestAppContext) {
    let (shell, cx) = open_window(cx);
    open_capture(cx, &shell);
    let before = grid(cx, &shell);
    press(cx, "f10");
    press(cx, "down down down right");
    assert!(menu_open(cx, &shell), "the View branch is on screen");
    // Without a plot the row's own binding is out of reach, so it has no keycap.
    let bare = row_width(cx, ORIENTATION_ROW);
    press(cx, "escape");
    cx.update(|_, cx| {
        cx.bind_keys([gpui_kit::KeyBinding::new(
            "ctrl-alt-v",
            ToggleOrientation,
            Some("Shell"),
        )]);
    });
    press(cx, "f10");
    press(cx, "down down down right");
    let capped = row_width(cx, ORIENTATION_ROW);
    assert!(
        capped > bare,
        "the row grew to hold the framed keycap, {bare} then {capped}"
    );
    press(cx, "enter");
    assert!(!menu_open(cx, &shell), "and the row still dispatches once");
    assert_ne!(grid(cx, &shell), before, "one Enter ran one row");
}

#[gpui_kit::test]
fn a_long_recent_name_keeps_the_file_branch_within_its_width(cx: &mut TestAppContext) {
    let (shell, cx) = open_window(cx);
    open_capture(cx, &shell);
    let long = format!(
        "{}.iqw",
        "capture-2026-09-28-145233-1440.000000-MHz-".repeat(4)
    );
    recent(cx, &shell, &long);
    press(cx, "f10");
    press(cx, "down right");
    assert!(file_focused(cx, &shell), "the File branch is on screen");
    assert!(
        row_width(cx, FILE_SEPARATOR) <= px(420.),
        "a name too long for its row leaves the menu at its own maximum"
    );

    assert!(
        row_height(cx, 2) <= px(26.),
        "and the long name truncates in one line instead of wrapping"
    );
}

/// Give the open capture the description its file would have reported, from a real file.
fn describe(
    cx: &mut gpui_kit::VisualTestContext,
    shell: &Entity<Shell>,
) -> argand_io::testutil::TempDir {
    let dir = argand_io::testutil::TempDir::new("app-menu-describe");
    let fixture = dir.join("capture.wav");
    argand_io::testutil::write_wav(
        &fixture,
        SampleType::new(Domain::Iq, SampleFormat::I16),
        24_000,
        &vec![0.0; 96_000],
        1.0,
    );
    shell.update_in(cx, |shell, _, cx| {
        let Some(file) = shell.file.as_mut() else {
            panic!("the capture never opened");
        };
        file.state.document.apply(Update::Opened(
            SignalMeta {
                sample_rate: 24_000.0,
                center_freq: 0.0,
                sample_type: SampleType::new(Domain::Iq, SampleFormat::I16),
                len_samples: 48_000,
                container: "wav",
                divisor: 32_768.0,
                source: fixture.clone(),
            },
            FileInfo {
                stamp: argand_io::write::SourceStamp::of(&fixture).ok(),
                ..FileInfo::default()
            },
        ));
        shell.start_editing(cx);
        cx.notify();
    });
    draw(cx);
    dir
}

fn can_save(cx: &mut gpui_kit::VisualTestContext, shell: &Entity<Shell>) -> (bool, bool) {
    shell.read_with(cx, |shell, _| (shell.can_save(false), shell.can_save(true)))
}

#[gpui_kit::test]
fn saving_needs_a_described_capture_and_saving_a_selection_needs_one(cx: &mut TestAppContext) {
    let (shell, cx) = open_window(cx);
    assert_eq!(can_save(cx, &shell), (false, false));
    open_capture(cx, &shell);
    assert_eq!(can_save(cx, &shell), (false, false), "not described yet");
    let _fixture = describe(cx, &shell);
    assert_eq!(can_save(cx, &shell), (true, false));
    shell.update_in(cx, |shell, _, cx| {
        shell.selection = argand_core::SampleSpan::between(100, 200);
        cx.notify();
    });
    assert_eq!(can_save(cx, &shell), (true, true));
    // The selection key does nothing without a dialog to answer, and opens no menu.
    press(cx, "ctrl-alt-s");
    assert!(!menu_open(cx, &shell));
}

#[gpui_kit::test]
fn a_failed_save_stays_in_the_status_bar_until_closed(cx: &mut TestAppContext) {
    let (shell, cx) = open_window(cx);
    open_capture(cx, &shell);
    shell.update_in(cx, |shell, _, cx| {
        shell.save_notice = Some(saving_ui::Notice::Failed("Save failed", "disk full".into()));
        cx.notify();
    });
    draw(cx);
    assert!(cx.update(|window, _| window.try_find("save-close").is_some()));
    click(cx, "save-close");
    assert!(shell.read_with(cx, |shell, _| shell.save_notice.is_none()));
    assert!(cx.update(|window, _| window.try_find("save-close").is_none()));
}

#[gpui_kit::test]
fn a_saved_notice_goes_with_the_next_press_or_move_and_a_failure_stays(cx: &mut TestAppContext) {
    let (shell, cx) = open_window(cx);
    open_capture(cx, &shell);
    let notice = |cx: &mut gpui_kit::VisualTestContext| {
        shell.read_with(cx, |shell, _| match &shell.save_notice {
            Some(saving_ui::Notice::Saved(_)) => "saved",
            Some(saving_ui::Notice::Failed(..)) => "failed",
            None => "none",
        })
    };
    shell.update_in(cx, |shell, _, cx| {
        shell.save_notice = Some(saving_ui::Notice::Saved("b.wav".into()));
        cx.notify();
    });
    draw(cx);
    assert!(
        cx.update(|window, _| window.try_find("save-close").is_none()),
        "a saved notice has nothing to close"
    );
    click_outside(cx);
    assert_eq!(notice(cx), "none");
    shell.update_in(cx, |shell, _, cx| {
        shell.save_notice = Some(saving_ui::Notice::Saved("b.wav".into()));
        cx.notify();
    });
    draw(cx);
    cx.simulate_mouse_move(outside(), None, gpui_kit::Modifiers::default());
    draw(cx);
    assert_eq!(notice(cx), "none", "a move takes it away too");
    shell.update_in(cx, |shell, _, cx| {
        shell.save_notice = Some(saving_ui::Notice::Failed("Save failed", "disk full".into()));
        cx.notify();
    });
    draw(cx);
    click_outside(cx);
    cx.simulate_mouse_move(outside(), None, gpui_kit::Modifiers::default());
    draw(cx);
    assert_eq!(notice(cx), "failed");
}
