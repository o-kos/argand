use super::*;
use crate::analysis::{FileInfo, Update};
use argand_core::SampleSpan;
use argand_core::{Domain, SampleFormat, SampleType, SignalMeta};
use gpui_kit::TestAppContext;
use std::path::PathBuf;

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
    cx.run_until_parked();
    (shell, cx)
}

/// A capture that has described itself, with its edits started as an opened file's are.
fn described(cx: &mut gpui_kit::VisualTestContext, shell: &Entity<Shell>, path: &str) {
    shell.update_in(cx, |shell, window, cx| {
        shell.open(Origin::new(PathBuf::from(path)), window, cx);
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
                source: PathBuf::from(path),
            },
            FileInfo::default(),
        ));
        shell.start_editing(cx);
        shell.view = Some(crate::navigation::View::full(48_000));
    });
    cx.run_until_parked();
}

fn length(cx: &mut gpui_kit::VisualTestContext, shell: &Entity<Shell>) -> Option<u64> {
    shell.read_with(cx, |shell, _| shell.sample_count())
}

fn select(cx: &mut gpui_kit::VisualTestContext, shell: &Entity<Shell>, a: u64, b: u64) {
    shell.update_in(cx, |shell, _, _| {
        shell.selection = SampleSpan::between(a, b)
    });
}

#[gpui_kit::test]
fn delete_undo_and_redo_follow_the_length_selection_and_title(cx: &mut TestAppContext) {
    let (shell, cx) = open_window(cx);
    described(cx, &shell, "/captures/a.iqw");
    select(cx, &shell, 1000, 3000);
    let can = shell.read_with(cx, |shell, _| shell.edit_commands_available());
    assert!(can.cut && !can.undo && !can.paste);
    shell.update_in(cx, |shell, window, cx| shell.delete_selection(window, cx));
    assert_eq!(length(cx, &shell), Some(46_000));
    assert!(shell.read_with(cx, |shell, _| shell.selection.is_none()));
    assert!(shell.read_with(cx, |shell, _| shell.file_name().starts_with('•')));
    shell.update_in(cx, |shell, window, cx| shell.undo(window, cx));
    assert_eq!(length(cx, &shell), Some(48_000));
    assert!(!shell.read_with(cx, |shell, _| shell.file_name().starts_with('•')));
    shell.update_in(cx, |shell, window, cx| shell.redo(window, cx));
    assert_eq!(length(cx, &shell), Some(46_000));
}

#[gpui_kit::test]
fn copy_and_replace_selection_paste_and_select_the_pasted_samples(cx: &mut TestAppContext) {
    let (shell, cx) = open_window(cx);
    described(cx, &shell, "/captures/a.iqw");
    select(cx, &shell, 0, 100);
    shell.update_in(cx, |shell, _, cx| shell.copy_selection(cx));
    select(cx, &shell, 1000, 3000);
    assert!(shell.read_with(cx, |shell, _| shell.edit_commands_available().replace));
    shell.update_in(cx, |shell, window, cx| {
        shell.paste(
            Placement::Replace(SampleSpan::between(1000, 3000).unwrap()),
            window,
            cx,
        );
    });
    assert_eq!(length(cx, &shell), Some(46_100));
    assert_eq!(
        shell.read_with(cx, |shell, _| shell.selection),
        SampleSpan::between(1000, 1100)
    );
}

#[gpui_kit::test]
fn opening_another_file_asks_about_unsaved_edits_first(cx: &mut TestAppContext) {
    let (shell, cx) = open_window(cx);
    described(cx, &shell, "/captures/a.iqw");
    select(cx, &shell, 0, 100);
    shell.update_in(cx, |shell, window, cx| shell.delete_selection(window, cx));
    let opened = |cx: &mut gpui_kit::VisualTestContext| {
        shell.read_with(cx, |shell, _| {
            shell
                .file
                .as_ref()
                .map(|file| file.state.document.origin().path.clone())
        })
    };
    shell.update_in(cx, |shell, window, cx| {
        shell.open(Origin::new(PathBuf::from("/captures/b.iqw")), window, cx);
    });
    assert!(cx.has_pending_prompt());
    cx.simulate_prompt_answer("Cancel");
    cx.run_until_parked();
    assert_eq!(opened(cx), Some(PathBuf::from("/captures/a.iqw")));
    shell.update_in(cx, |shell, window, cx| {
        shell.open(Origin::new(PathBuf::from("/captures/b.iqw")), window, cx);
    });
    cx.simulate_prompt_answer("Discard");
    cx.run_until_parked();
    assert_eq!(opened(cx), Some(PathBuf::from("/captures/b.iqw")));
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
#[gpui_kit::test]
fn without_a_system_dialog_the_toolkit_dialog_asks(cx: &mut TestAppContext) {
    open_window(cx);
    let mut shell = None;
    let handle = cx.add_window(|window, cx| {
        let view = cx.new(|cx| Shell::new(Config::default(), None, Session::default(), window, cx));
        shell = Some(view.clone());
        gpui_kit::component::Root::new(view, window, cx)
    });
    let shell = shell.unwrap();
    let cx = &mut gpui_kit::VisualTestContext::from_window(handle.into(), cx);
    described(cx, &shell, "/captures/a.iqw");
    select(cx, &shell, 0, 100);
    shell.update_in(cx, |shell, window, cx| shell.delete_selection(window, cx));
    shell.update_in(cx, |shell, window, cx| {
        shell.open(Origin::new(PathBuf::from("/captures/b.iqw")), window, cx);
    });
    cx.run_until_parked();
    assert!(!cx.has_pending_prompt());
    assert!(cx.update(|window, cx| window.has_active_dialog(cx)));
    cx.simulate_keystrokes("escape");
    cx.run_until_parked();
    assert!(!cx.update(|window, cx| window.has_active_dialog(cx)));
    let opened = shell.read_with(cx, |shell, _| {
        shell
            .file
            .as_ref()
            .map(|file| file.state.document.origin().path.clone())
    });
    assert_eq!(
        opened,
        Some(PathBuf::from("/captures/a.iqw")),
        "Escape cancels"
    );
}

#[gpui_kit::test]
fn a_band_is_clamped_to_the_capture_and_leaves_nothing_to_edit(cx: &mut TestAppContext) {
    let (shell, cx) = open_window(cx);
    described(cx, &shell, "/captures/a.iqw");
    shell.update_in(cx, |shell, _, cx| {
        shell.select(
            argand_core::Selection {
                time: SampleSpan::between(0, 100),
                band: argand_core::FrequencyBand::between(-1e9, 1_000.),
            },
            cx,
        );
    });
    shell.read_with(cx, |shell, _| {
        assert_eq!(
            shell.band,
            argand_core::FrequencyBand::between(-12_000., 1_000.)
        );
        assert_eq!(
            shell.time_selection(),
            None,
            "a rectangle is not a time selection"
        );
        assert!(!shell.edit_commands_available().cut);
    });
    shell.update_in(cx, |shell, _, cx| {
        shell.select(
            argand_core::Selection {
                time: SampleSpan::between(0, 100),
                band: argand_core::FrequencyBand::between(20_000., 30_000.),
            },
            cx,
        );
    });
    shell.read_with(cx, |shell, _| {
        assert_eq!(
            (shell.selection, shell.band),
            (None, None),
            "a band beyond the capture selects nothing"
        );
    });
}

#[gpui_kit::test]
fn the_close_button_asks_about_unsaved_edits(cx: &mut TestAppContext) {
    let (shell, cx) = open_window(cx);
    described(cx, &shell, "/captures/a.iqw");
    select(cx, &shell, 0, 100);
    shell.update_in(cx, |shell, window, cx| shell.delete_selection(window, cx));
    cx.update(|window, cx| window.dispatch_action(Box::new(chrome::CloseWindow), cx));
    cx.run_until_parked();
    assert!(cx.has_pending_prompt(), "the window asks before closing");
    cx.simulate_prompt_answer("Cancel");
}

#[gpui_kit::test]
fn a_save_finished_after_more_edits_keeps_the_document_and_its_unsaved_state(
    cx: &mut TestAppContext,
) {
    let (shell, cx) = open_window(cx);
    described(cx, &shell, "/captures/a.iqw");
    select(cx, &shell, 0, 100);
    shell.update_in(cx, |shell, window, cx| shell.delete_selection(window, cx));
    let (document, version) = shell.read_with(cx, |shell, _| {
        let file = shell.file.as_ref().map_or(0, |file| file.id);
        (file, shell.editing().map_or(0, Editing::version))
    });
    select(cx, &shell, 0, 100);
    shell.update_in(cx, |shell, window, cx| shell.delete_selection(window, cx));
    shell.update_in(cx, |shell, window, cx| {
        shell.saved(
            PathBuf::from("/captures/saved.iqw"),
            saving_ui::SaveOf::whole_for_test(document, version),
            window,
            cx,
        );
    });
    cx.run_until_parked();
    let (path, dirty) = shell.read_with(cx, |shell, _| {
        (
            shell
                .file
                .as_ref()
                .map(|file| file.state.document.origin().path.clone()),
            shell.editing().is_some_and(Editing::is_dirty),
        )
    });
    assert_eq!(
        path,
        Some(PathBuf::from("/captures/a.iqw")),
        "the window stays on its capture"
    );
    assert!(dirty, "the later edit is still unsaved");
}

#[gpui_kit::test]
fn undo_reaches_a_capture_edited_down_to_nothing(cx: &mut TestAppContext) {
    let (shell, cx) = open_window(cx);
    described(cx, &shell, "/captures/a.iqw");
    select(cx, &shell, 0, 48_000);
    shell.update_in(cx, |shell, window, cx| {
        shell.delete_selection(window, cx);
        window.focus(&shell.focus, cx);
    });
    assert_eq!(length(cx, &shell), Some(0));
    cx.simulate_keystrokes(if cfg!(target_os = "macos") {
        "cmd-z"
    } else {
        "ctrl-z"
    });
    cx.run_until_parked();
    assert_eq!(length(cx, &shell), Some(48_000));
}

fn quit_shortcut() -> &'static str {
    if cfg!(target_os = "macos") {
        "cmd-q"
    } else {
        "ctrl-q"
    }
}

#[cfg(target_os = "linux")]
fn dirty_close_fixture(
    cx: &mut gpui_kit::VisualTestContext,
    shell: &Entity<Shell>,
) -> (argand_io::testutil::TempDir, PathBuf) {
    let dir = argand_io::testutil::TempDir::new("close-guard");
    let path = dir.join("capture.wav");
    argand_io::testutil::write_wav(
        &path,
        SampleType::new(Domain::Iq, SampleFormat::I16),
        24_000,
        &vec![0.0; 96_000],
        1.0,
    );
    let recent = dir.join("recent.wav");
    std::fs::copy(&path, &recent).unwrap();
    described(cx, shell, path.to_str().unwrap());
    shell.update_in(cx, |shell, _, cx| {
        shell
            .file
            .as_mut()
            .unwrap()
            .state
            .document
            .apply(Update::Opened(
                SignalMeta {
                    sample_rate: 24_000.0,
                    center_freq: 0.0,
                    sample_type: SampleType::new(Domain::Iq, SampleFormat::I16),
                    len_samples: 48_000,
                    container: "wav",
                    divisor: 32_768.0,
                    source: path.clone(),
                },
                FileInfo {
                    stamp: argand_io::write::SourceStamp::of(&path).ok(),
                    ..FileInfo::default()
                },
            ));
        shell.start_editing(cx);
    });
    select(cx, shell, 0, 100);
    shell.update_in(cx, |shell, window, cx| shell.delete_selection(window, cx));
    (dir, recent)
}

#[cfg(target_os = "linux")]
fn assert_close_overlay(
    cx: &mut gpui_kit::VisualTestContext,
    shell: &Entity<Shell>,
    overlay: &str,
) {
    use gpui_kit::Focusable;
    shell.update_in(cx, |shell, window, cx| match overlay {
        "Analysis" => assert!(shell.analysis_hint.read(cx).is_open()),
        "About" => assert!(shell.about.hint.read(cx).is_open()),
        "Recent menu" => assert!(
            shell
                .application_recent_focus
                .as_ref()
                .unwrap()
                .is_focused(window)
        ),
        "File menu" => assert!(
            shell
                .application_file_menu
                .as_ref()
                .unwrap()
                .read(cx)
                .focus_handle(cx)
                .is_focused(window)
        ),
        _ => assert!(shell.application_menu.is_some()),
    });
}

/// Caption dispatch must survive both overlay focus and an outside-dismiss frame.
#[cfg(target_os = "linux")]
#[gpui_kit::test]
fn first_caption_close_and_quit_preserve_dirty_documents_under_supported_overlays(
    cx: &mut TestAppContext,
) {
    use gpui_kit::test::TestWindowExt;
    let (shell, cx) = open_window(cx);
    let (_dir, recent) = dirty_close_fixture(cx, &shell);
    cx.update(|window, cx| {
        window.activate_window();
        Shell::bind_choose_file(shell.downgrade(), window.window_handle(), cx);
    });
    let identity = shell.read_with(cx, |shell, _| {
        (
            shell.file.as_ref().unwrap().id,
            shell.editing().unwrap().version(),
            shell.sample_count(),
        )
    });
    // File information's editor requires the production reader fixture; its
    // first-click Close and dirty Cancel are verified in release acceptance.
    for overlay in [
        "Analysis",
        "About",
        "application",
        "File menu",
        "Edit menu",
        "View menu",
        "Recent menu",
    ] {
        for quit_key in [false, true] {
            shell.update_in(cx, |shell, window, cx| {
                shell.close_hints(cx);
                shell.dismiss_application_menu(window, cx);
                match overlay {
                    "Analysis" => shell.edit_analysis(&EditAnalysis, window, cx),
                    "About" => shell.show_about(&about_ui::ShowAbout, window, cx),
                    _ => {
                        shell.session.recent = vec![crate::session::Recent {
                            path: recent.clone(),
                            hints: Default::default(),
                        }];
                        shell.recent_files.refresh(&shell.session.recent);
                        shell.recent_files.apply(recent.clone(), true);
                        shell.open_application_menu(window, cx);
                    }
                }
            });
            cx.run_until_parked();
            cx.update(|window, cx| window.render_frame(cx));
            let keys = match overlay {
                "File menu" => "down right",
                "Edit menu" => "down down right",
                "View menu" => "down down down right",
                "Recent menu" => "down right down right",
                _ => "",
            };
            cx.simulate_keystrokes(keys);
            cx.run_until_parked();
            cx.update(|window, cx| window.render_frame(cx));
            assert_close_overlay(cx, &shell, overlay);
            if quit_key {
                cx.simulate_keystrokes(quit_shortcut());
            } else {
                let at = cx.update(|window, _| {
                    let half = gpui_kit::component::TITLE_BAR_HEIGHT / 2.;
                    gpui_kit::point(window.viewport_size().width - half, half)
                });
                cx.simulate_click(at, gpui_kit::Modifiers::default());
            }
            cx.run_until_parked();
            assert!(
                cx.has_pending_prompt(),
                "first close reaches dirty guard: {overlay}, key={quit_key}"
            );
            shell.read_with(cx, |shell, cx| {
                assert!(
                    shell.application_menu.is_none(),
                    "menu must not cover close confirmation"
                );
                for panel in hints::Panel::ALL {
                    assert!(
                        !shell.information_hint(panel).read(cx).is_open(),
                        "panel must not cover close confirmation"
                    );
                }
            });
            cx.simulate_prompt_answer("Cancel");
            cx.run_until_parked();
            assert!(
                !cx.has_pending_prompt(),
                "one close request, not duplicate delivery: {overlay}"
            );
            assert_eq!(
                identity,
                shell.read_with(cx, |shell, _| (
                    shell.file.as_ref().unwrap().id,
                    shell.editing().unwrap().version(),
                    shell.sample_count()
                )),
                "Cancel preserves document: {overlay}"
            );
            assert!(shell.read_with(cx, |shell, _| shell.editing().unwrap().is_dirty()));
        }
    }
}

#[cfg(target_os = "linux")]
#[gpui_kit::test]
fn clean_caption_shortcut_and_menu_quit_remove_the_window(cx: &mut TestAppContext) {
    use gpui_kit::test::TestWindowExt;
    for route in ["caption", "shortcut", "menu"] {
        let (shell, cx) = open_window(cx);
        cx.update(|window, cx| {
            window.activate_window();
            Shell::bind_choose_file(shell.downgrade(), window.window_handle(), cx);
        });
        shell.update_in(cx, |shell, window, cx| {
            if route == "menu" {
                shell.open_application_menu(window, cx);
            } else {
                shell.show_about(&about_ui::ShowAbout, window, cx);
            }
        });
        cx.run_until_parked();
        cx.update(|window, cx| window.render_frame(cx));
        match route {
            "caption" => {
                let at = cx.update(|window, _| {
                    let half = gpui_kit::component::TITLE_BAR_HEIGHT / 2.;
                    gpui_kit::point(window.viewport_size().width - half, half)
                });
                cx.simulate_click(at, gpui_kit::Modifiers::default());
            }
            "shortcut" => cx.simulate_keystrokes(quit_shortcut()),
            _ => cx.simulate_keystrokes("down down down down down enter"),
        }
        cx.run_until_parked();
        assert!(
            cx.cx.windows().is_empty(),
            "clean {route} closes its window"
        );
    }
}

#[gpui_kit::test]
fn overlay_quit_fallback_only_closes_its_active_owning_window(cx: &mut TestAppContext) {
    use gpui_kit::test::TestWindowExt;
    let (first, first_cx) = open_window(cx);
    let owner = first_cx.update(|window, cx| {
        Shell::bind_choose_file(first.downgrade(), window.window_handle(), cx);
        window.window_handle()
    });
    let mut another_context = first_cx.cx.clone();
    let (second, cx) = open_window(&mut another_context);
    cx.update(|window, cx| {
        window.activate_window();
        Shell::bind_choose_file(second.downgrade(), window.window_handle(), cx);
    });
    second.update_in(cx, |shell, window, cx| {
        shell.show_about(&about_ui::ShowAbout, window, cx)
    });
    cx.run_until_parked();
    cx.update(|window, cx| window.render_frame(cx));
    cx.simulate_keystrokes(quit_shortcut());
    cx.run_until_parked();
    assert_eq!(
        cx.cx.windows(),
        vec![owner],
        "another window's fallback propagates without closing it"
    );
}
