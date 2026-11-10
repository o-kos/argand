mod context_menu_tests {
    use super::*;
    use gpui_kit::test::TestEventExt;
    use gpui_kit::{Focusable, WeakEntity};

    #[derive(Clone, Copy)]
    enum Target {
        TimeRuler,
        Spectrum,
    }

    fn open_menu(
        cx: &mut TestAppContext,
        handle: WindowHandle<Harness>,
        target: Target,
    ) -> WeakEntity<PopupMenu> {
        cx.update_window(handle.into(), |_, window, _| window.activate_window())
            .unwrap();
        frame(cx, handle);
        let at = handle
            .update(cx, |harness, _, cx| {
                let geometry = harness.plot.as_ref().unwrap().read(cx).geometry.unwrap();
                match target {
                    Target::TimeRuler => geometry.time_ruler.center(),
                    Target::Spectrum => geometry.spectrum.center(),
                }
            })
            .unwrap();
        cx.update_window(handle.into(), |_, window, cx| {
            window.pointer_down(at, MouseButton::Right, cx);
            window.pointer_up(at, MouseButton::Right, cx);
        })
        .unwrap();
        frame(cx, handle);
        handle
            .update(cx, |harness, window, cx| {
                let plot = harness.plot.as_ref().unwrap().read(cx);
                let weak = plot
                    .open_menu
                    .clone()
                    .expect("the real context menu opened");
                let menu = weak.upgrade().expect("the open menu is alive");
                assert!(menu.read(cx).focus_handle(cx).is_focused(window));
                assert!(!plot.focus.is_focused(window));
                weak
            })
            .unwrap()
    }

    fn remove_menu_window(cx: &mut TestAppContext, handle: WindowHandle<Harness>) {
        cx.update_window(handle.into(), |_, window, _| window.remove_window())
            .unwrap();
        cx.run_until_parked();
        cx.update(|_| {});
    }

    #[gpui_kit::test]
    fn ruler_menu_isolates_keys_and_escape_restores_plot_commands(cx: &mut TestAppContext) {
        let handle = open(cx);
        let menu = open_menu(cx, handle, Target::TimeRuler);
        navigation(cx, handle);
        for key in ["ctrl-g", "ctrl-=", "ctrl-shift-=", "home"] {
            press(cx, handle, key);
            assert!(navigation(cx, handle).is_empty(), "{key} reached the plot");
            handle
                .update(cx, |harness, _, cx| {
                    assert_eq!(harness.grid, 0, "{key} reached the session command");
                    assert_eq!(
                        harness
                            .plot
                            .as_ref()
                            .unwrap()
                            .read(cx)
                            .open_menu
                            .as_ref()
                            .map(|menu| menu.entity_id()),
                        Some(menu.entity_id()),
                        "{key} dismissed the menu"
                    );
                })
                .unwrap();
        }
        press(cx, handle, "escape");
        handle
            .update(cx, |harness, _, cx| {
                assert!(harness.plot.as_ref().unwrap().read(cx).open_menu.is_none());
            })
            .unwrap();
        assert!(plot_focused(cx, handle));
        press(cx, handle, "ctrl-g");
        assert_eq!(handle.update(cx, |harness, _, _| harness.grid).unwrap(), 1);
        press(cx, handle, "ctrl-=");
        assert_eq!(
            navigation(cx, handle),
            vec![PlotIntent::Time(TimeIntent::Zoom {
                factor: 0.5,
                anchor: 0.5,
            })]
        );
        let reopened = open_menu(cx, handle, Target::TimeRuler);
        assert_ne!(menu.entity_id(), reopened.entity_id());
        remove_menu_window(cx, handle);
        assert!(menu.upgrade().is_none());
        assert!(reopened.upgrade().is_none());
    }

    fn closing_with_open_menu_releases_popup(cx: &mut TestAppContext, target: Target) {
        let handle = open(cx);
        let menu = open_menu(cx, handle, target);
        assert!(menu.upgrade().is_some());
        remove_menu_window(cx, handle);
        assert!(menu.upgrade().is_none(), "the menu outlived its window");
    }

    #[gpui_kit::test]
    fn open_ruler_menu_is_released_when_its_window_closes(cx: &mut TestAppContext) {
        closing_with_open_menu_releases_popup(cx, Target::TimeRuler);
    }

    #[gpui_kit::test]
    fn open_spectrum_menu_is_released_when_its_window_closes(cx: &mut TestAppContext) {
        closing_with_open_menu_releases_popup(cx, Target::Spectrum);
    }
}
