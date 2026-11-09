use super::*;
use gpui_kit::test::TestWindowExt;
use gpui_kit::{InputEvent, TestAppContext, WindowHandle};

struct Harness {
    window: WindowHandle<Root>,
    shell: Entity<Shell>,
}

fn frame(cx: &mut TestAppContext, h: &Harness) {
    cx.run_until_parked();
    cx.update_window(h.window.into(), |_, window, cx| window.render_frame(cx))
        .expect("window");
    cx.run_until_parked();
}

fn open(cx: &mut TestAppContext) -> Harness {
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
    let window = cx.add_window(|window, cx| {
        let view = cx.new(|cx| Shell::new(Config::default(), None, Session::default(), window, cx));
        shell = Some(view.clone());
        window.activate_window();
        Root::new(view, window, cx).bordered(false)
    });
    let h = Harness {
        window,
        shell: shell.expect("shell"),
    };
    frame(cx, &h);
    h
}

fn click(cx: &mut TestAppContext, h: &Harness, id: &'static str) {
    cx.update_window(h.window.into(), |_, window, cx| window.click(id, cx))
        .expect("window");
    frame(cx, h);
}

fn keys(cx: &mut TestAppContext, h: &Harness, keys: &str) {
    cx.update_window(h.window.into(), |_, window, cx| {
        for key in keys.split_whitespace() {
            window.press(key, cx);
            window.dispatch_event(
                gpui_kit::KeyUpEvent {
                    keystroke: gpui_kit::Keystroke::parse(key).expect("key"),
                }
                .to_platform_input(),
                cx,
            );
        }
    })
    .expect("window");
    frame(cx, h);
}

fn shown(cx: &mut TestAppContext, h: &Harness) -> bool {
    cx.update_window(h.window.into(), |_, window, _| {
        window.try_find("about-panel").is_some()
    })
    .expect("window")
}

#[gpui_kit::test]
fn toolbar_and_main_menu_open_one_panel_and_escape_restores_focus(cx: &mut TestAppContext) {
    let h = open(cx);
    click(cx, &h, "about-button");
    assert!(shown(cx, &h));
    let first = h.shell.read_with(cx, |shell, cx| {
        shell.about.hint.read(cx).view().expect("About").entity_id()
    });
    cx.update_window(h.window.into(), |_, window, cx| {
        h.shell
            .update(cx, |shell, cx| shell.show_about(&ShowAbout, window, cx));
    })
    .expect("window");
    frame(cx, &h);
    assert_eq!(
        h.shell.read_with(cx, |shell, cx| shell
            .about
            .hint
            .read(cx)
            .view()
            .expect("About")
            .entity_id()),
        first
    );
    keys(cx, &h, "escape");
    assert!(!shown(cx, &h));
    assert!(
        cx.update_window(h.window.into(), |_, window, cx| h
            .shell
            .read(cx)
            .focus
            .is_focused(window))
            .expect("window")
    );
    keys(cx, &h, "f10 down down down down enter");
    assert!(shown(cx, &h), "the direct main-menu item opens About");
    assert!(
        h.shell
            .read_with(cx, |shell, _| shell.application_menu.is_none())
    );
}

#[gpui_kit::test]
fn panel_clicks_and_repository_activation_dismiss_without_reaching_the_workspace(
    cx: &mut TestAppContext,
) {
    let h = open(cx);
    click(cx, &h, "about-button");
    click(cx, &h, "about-panel");
    assert!(!shown(cx, &h));
    click(cx, &h, "about-button");
    click(cx, &h, "about-repository");
    assert!(!shown(cx, &h));
    assert_eq!(cx.opened_url().as_deref(), Some(about::REPOSITORY));
    assert!(h.shell.read_with(cx, |shell, _| !shell.title_drag_pending));
}

#[gpui_kit::test]
fn focus_within_the_link_is_retained_and_keyboard_activation_dismisses(cx: &mut TestAppContext) {
    let h = open(cx);
    for key in ["enter", "space"] {
        click(cx, &h, "about-button");
        keys(cx, &h, "tab");
        assert!(shown(cx, &h), "Tab to the repository stays inside About");
        keys(cx, &h, key);
        assert_eq!(cx.opened_url().as_deref(), Some(about::REPOSITORY));
        assert!(!shown(cx, &h));
        assert!(
            cx.update_window(h.window.into(), |_, window, cx| h
                .shell
                .read(cx)
                .focus
                .is_focused(window))
                .expect("window")
        );
    }
}

#[gpui_kit::test]
fn leaving_the_focus_subtree_or_activating_another_window_dismisses(cx: &mut TestAppContext) {
    let h = open(cx);
    click(cx, &h, "about-button");
    cx.update_window(h.window.into(), |_, window, cx| {
        let focus = h.shell.read(cx).focus.clone();
        window.focus(&focus, cx);
    })
    .expect("window");
    frame(cx, &h);
    assert!(!shown(cx, &h), "focus left the whole About subtree");
    click(cx, &h, "about-button");
    let other = cx.add_window(|window, _| {
        window.activate_window();
        Empty
    });
    frame(cx, &h);
    assert!(
        !shown(cx, &h),
        "a different active native test window closes About"
    );
    cx.update_window(other.into(), |_, window, _| window.remove_window())
        .expect("other window");
}

struct Empty;
impl Render for Empty {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

#[gpui_kit::test]
fn themes_and_menu_transitions_keep_the_same_standard_panel(cx: &mut TestAppContext) {
    let h = open(cx);
    for mode in [ThemeMode::Light, ThemeMode::Dark] {
        cx.update(|cx| gpui_kit::component::Theme::change(mode, None, cx));
        click(cx, &h, "about-button");
        assert!(shown(cx, &h));
        let (panel, link) = cx
            .update_window(h.window.into(), |_, window, _| {
                (
                    window.find("about-panel").bounds(),
                    window.find("about-repository").bounds(),
                )
            })
            .expect("window");
        assert!(
            panel.contains(&link.origin) && panel.contains(&link.bottom_right()),
            "repository fits the panel"
        );
        keys(cx, &h, "escape");
    }
    click(cx, &h, "about-button");
    click(cx, &h, "application-menu-button");
    assert!(!shown(cx, &h));
    assert!(
        h.shell
            .read_with(cx, |shell, _| shell.application_menu.is_some())
    );
}

#[gpui_kit::test]
fn an_outside_press_only_closes_the_panel(cx: &mut TestAppContext) {
    let h = open(cx);
    click(cx, &h, "about-button");
    cx.update_window(h.window.into(), |_, window, cx| {
        let position = gpui_kit::point(
            window.viewport_size().width - px(8.),
            window.viewport_size().height - px(8.),
        );
        window.dispatch_event(
            gpui_kit::MouseDownEvent {
                position,
                button: MouseButton::Left,
                click_count: 1,
                ..Default::default()
            }
            .to_platform_input(),
            cx,
        );
        window.dispatch_event(
            gpui_kit::MouseUpEvent {
                position,
                button: MouseButton::Left,
                click_count: 1,
                ..Default::default()
            }
            .to_platform_input(),
            cx,
        );
    })
    .expect("window");
    frame(cx, &h);
    assert!(!shown(cx, &h));
    assert!(h.shell.read_with(cx, |shell, _| !shell.title_drag_pending
        && shell.file.is_none()));
}
