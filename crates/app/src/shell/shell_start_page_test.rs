use super::*;
use gpui_kit::TestAppContext;
use gpui_kit::test::TestWindowExt;

fn open_window(cx: &mut TestAppContext) -> (Entity<Shell>, &mut gpui_kit::VisualTestContext) {
    cx.update(|cx| {
        gpui_kit::init(cx);
        settings_ui::init(cx);
        editing_ui::init(cx);
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

fn available_recent(
    cx: &mut gpui_kit::VisualTestContext,
    shell: &Entity<Shell>,
    paths: &[String],
    available: bool,
) {
    shell.update_in(cx, |shell, _, cx| {
        shell.session.recent = paths
            .iter()
            .map(|path| crate::session::Recent {
                path: std::path::PathBuf::from(path),
                hints: Default::default(),
            })
            .collect();
        shell.recent_files.refresh(&shell.session.recent);
        for path in paths {
            shell
                .recent_files
                .apply(std::path::PathBuf::from(path), available);
        }
        cx.notify();
    });
    draw(cx);
}

#[gpui_kit::test]
fn start_page_centers_the_visible_group_and_keeps_its_columns(cx: &mut TestAppContext) {
    let _cleanup = crate::test_context::cleanup(cx);
    let (shell, cx) = open_window(cx);
    let directory = std::env::temp_dir().join(format!(
        "argand-start-page-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&directory).unwrap();
    let paths: Vec<_> = (0..10)
        .map(|index| {
            let path = directory.join(format!("capture-{index}.wav"));
            std::fs::write(&path, []).unwrap();
            path.to_string_lossy().into_owned()
        })
        .collect();
    for count in [0, 1, 10] {
        available_recent(cx, &shell, &paths[..count], true);
        for dimensions in [(800., 600.), (420., 260.), (800., 220.)] {
            cx.simulate_resize(size(px(dimensions.0), px(dimensions.1)));
            draw(cx);
            cx.update(|window, _| {
                let page = window.find("start-page").bounds();
                let block = window.find("start-block").bounds();
                let chooser = window.find("start-open").bounds();
                assert!((block.center().x - page.center().x).abs() <= px(1.));
                assert!((block.center().y - page.center().y).abs() <= px(1.));
                assert!(block.top() >= page.top());
                assert!(block.bottom() <= page.bottom());
                assert!(chooser.bottom() <= page.bottom());
                assert!(block.size.width < px(400.), "short rows size to content");
                if count > 0 {
                    let first = window.find(("start-recent", 0usize)).bounds();
                    assert_eq!(first.left(), block.left());
                    assert_eq!(chooser.left() - first.left(), window.rem_size() * 2.);
                    assert!(first.top() < chooser.top());
                }
            });
        }
    }
    let long_paths: Vec<_> = (0..10)
        .map(|index| {
            let parent = directory.join(format!(
                "a-very-long-recording-directory-number-{index}-with-a-shared-suffix"
            ));
            std::fs::create_dir(&parent).unwrap();
            let path = parent.join("a-long-capture-filename-with-a-shared-extension.wav");
            std::fs::write(&path, []).unwrap();
            path.to_string_lossy().into_owned()
        })
        .collect();
    available_recent(cx, &shell, &long_paths, true);
    cx.simulate_resize(size(px(420.), px(400.)));
    draw(cx);
    cx.update(|window, _| {
        let page = window.find("start-page").bounds();
        let block = window.find("start-block").bounds();
        assert!((block.center().x - page.center().x).abs() <= px(1.));
        assert!(block.size.width <= px(324.));
        for index in 0..10usize {
            let row = window.find(("start-recent", index)).bounds();
            assert_eq!(row.left(), block.left());
            assert!(row.right() <= block.right());
        }
    });
    std::fs::remove_dir_all(directory).unwrap();
}

#[gpui_kit::test]
fn wheel_scrolling_retires_the_recent_hint_at_a_stationary_pointer(cx: &mut TestAppContext) {
    let _cleanup = crate::test_context::cleanup(cx);
    let (shell, cx) = open_window(cx);
    let directory = argand_io::testutil::TempDir::new("recent-tooltip-scroll");
    let paths: Vec<_> = (0..10)
        .map(|ix| {
            let path = directory.join(&format!("capture-{ix}.wav"));
            std::fs::write(&path, []).unwrap();
            path.to_string_lossy().into_owned()
        })
        .collect();
    available_recent(cx, &shell, &paths, true);
    cx.simulate_resize(size(px(640.), px(400.)));
    draw(cx);
    cx.update(|window, _| window.activate_window());
    draw(cx);
    cx.simulate_mouse_move(point(px(5.), px(5.)), None, Default::default());
    draw(cx);
    let at = cx.update(|window, _| window.find(("start-recent", 3usize)).bounds().center());
    for wait in [1000, 200] {
        cx.simulate_mouse_move(point(px(5.), px(5.)), None, Default::default());
        draw(cx);
        cx.simulate_mouse_move(at, None, Default::default());
        draw(cx);
        cx.executor()
            .advance_clock(std::time::Duration::from_millis(wait));
        draw(cx);
        cx.update(|window, _| {
            assert_eq!(window.try_find("recent-tooltip").is_some(), wait == 1000)
        });
        cx.simulate_event(gpui_kit::ScrollEvent {
            position: at,
            delta: gpui_kit::ScrollDelta::Pixels(point(
                px(0.),
                px(if wait == 1000 { -112. } else { 112. }),
            )),
            ..Default::default()
        });
        draw(cx);
        cx.executor()
            .advance_clock(std::time::Duration::from_secs(1));
        draw(cx);
        cx.update(|window, _| {
            assert!(
                window.try_find("recent-tooltip").is_none(),
                "old row hint survives wheel with {wait} ms hover"
            )
        });
    }
    cx.simulate_mouse_move(point(px(5.), px(5.)), None, Default::default());
    draw(cx);
    cx.simulate_mouse_move(at, None, Default::default());
    draw(cx);
    cx.executor()
        .advance_clock(std::time::Duration::from_millis(200));
    draw(cx);
    cx.update(|window, _| {
        assert!(
            window.try_find("recent-tooltip").is_none(),
            "normal hover delay remains"
        )
    });
    cx.executor()
        .advance_clock(std::time::Duration::from_secs(1));
    draw(cx);
    cx.update(|window, _| {
        assert!(
            window.try_find("recent-tooltip").is_some(),
            "fresh hover restores hints"
        )
    });
}

#[gpui_kit::test]
fn recent_rows_use_available_width_before_truncating(cx: &mut TestAppContext) {
    let _cleanup = crate::test_context::cleanup(cx);
    let (shell, cx) = open_window(cx);
    let directory = argand_io::testutil::TempDir::new("recent-width");
    let name = "a-long-recording-with-frequency-123456789-and-timestamp-20261124T123456-and-receiver-location.wav";
    let path = directory.join(name);
    std::fs::write(&path, []).unwrap();
    available_recent(cx, &shell, &[path.to_string_lossy().into_owned()], true);
    for width in [1200., 420., 1200.] {
        cx.simulate_resize(size(px(width), px(600.)));
        draw(cx);
        cx.update(|window, _| {
            let page = window.find("start-page").bounds();
            let block = window.find("start-block").bounds();
            let row = window.find(("start-recent", 0usize)).bounds();
            assert!((block.center().x - page.center().x).abs() <= px(1.));
            assert!(row.size.width <= px(width - 96.));
            if width == 1200. {
                assert!(
                    row.size.width > px(560.),
                    "wide rows recover text beyond the old cap"
                );
                assert!(
                    row.size.width < px(width - 96.),
                    "rows still size to their contents"
                );
            } else {
                assert!(row.size.width <= px(324.));
            }
        });
    }
}
