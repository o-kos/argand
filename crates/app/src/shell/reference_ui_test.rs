use super::*;
use crate::shell::plot::navigation_ui::ToggleOrientation;
use crate::shell::replace_ui::tests::{open_window, wait_until, write_wave};
use gpui_kit::Focusable;
use gpui_kit::{TestAppContext, test::TestWindowExt};

fn frame(cx: &mut gpui_kit::VisualTestContext) {
    cx.run_until_parked();
    cx.update(|window, cx| {
        window.render_frame(cx);
        window.simulate_next_frame(cx);
    });
    cx.run_until_parked();
}

fn opened(
    cx: &mut TestAppContext,
) -> (
    Entity<Shell>,
    argand_io::testutil::TempDir,
    &mut gpui_kit::VisualTestContext,
) {
    let (shell, cx) = open_window(cx);
    cx.update(|window, _| window.activate_window());
    let dir = argand_io::testutil::TempDir::new("status-reference");
    let path = dir.join("capture.wav");
    write_wave(&path, 2400);
    shell.update_in(cx, |shell, window, cx| {
        shell.open(Origin::new(path), window, cx)
    });
    wait_until(cx, &shell, "reference fixture", |shell| {
        shell.is_reference_editable()
    });
    cx.update(|window, cx| Shell::bind_choose_file(shell.downgrade(), window.window_handle(), cx));
    frame(cx);
    (shell, dir, cx)
}

fn editor(shell: &Entity<Shell>, cx: &gpui_kit::VisualTestContext) -> Entity<Editor> {
    shell.read_with(cx, |shell, cx| {
        shell
            .reference
            .hint
            .read(cx)
            .view()
            .unwrap()
            .downcast::<Editor>()
            .unwrap()
    })
}

#[gpui_kit::test]
fn status_reference_edits_and_saves_without_changing_samples(cx: &mut TestAppContext) {
    let _cleanup = crate::test_context::cleanup(cx);
    let (shell, dir, cx) = opened(cx);
    let path = dir.join("capture.wav");
    let before = std::fs::read(&path).unwrap()[44..].to_vec();
    cx.update(|window, cx| {
        assert!(window.try_find("reference-frequency-control").is_none());
        let reference = window.find("reference-frequency").bounds();
        let plot = shell
            .read(cx)
            .plot_view()
            .unwrap()
            .read(cx)
            .panel_bounds
            .unwrap();
        assert!(reference.left() >= plot.left());
        assert!(reference.right() <= plot.right());
        assert!(reference.top() < window.find("analysis-settings").bounds().top());
        window.click("reference-frequency", cx);
    });
    frame(cx);
    let editor = editor(&shell, cx);
    let input = editor.read_with(cx, |editor, _| editor.input.clone());
    input.update_in(cx, |input, window, cx| {
        input.set_value("10 100 00.8", window, cx)
    });
    assert_eq!(
        input.read_with(cx, |input, _| input.value().to_string()),
        "10 100 00.8"
    );
    cx.simulate_keystrokes("enter");
    frame(cx);
    shell.read_with(cx, |shell, _| {
        assert_eq!(shell.editing().unwrap().reference(), 1_010_001.);
        assert!(shell.editing().unwrap().is_dirty());
    });
    assert!(editor.read_with(cx, |editor, _| !editor.refused));
    cx.simulate_keystrokes("escape");
    frame(cx);
    cx.simulate_keystrokes(if cfg!(target_os = "macos") {
        "cmd-s"
    } else {
        "ctrl-s"
    });
    wait_until(cx, &shell, "reference save", |shell| {
        shell.saving.is_none() && !shell.editing().unwrap().is_dirty()
    });
    let source = argand_io::open(&path, &Default::default()).unwrap();
    assert_eq!(source.meta().center_freq, 1_010_001.);
    let bytes = std::fs::read(&path).unwrap();
    let data = bytes.windows(4).position(|part| part == b"data").unwrap() + 8;
    assert_eq!(&bytes[data..], before);
}

#[gpui_kit::test]
fn reference_draft_rejects_invalid_and_stale_values_and_isolates_plot_keys(
    cx: &mut TestAppContext,
) {
    let _cleanup = crate::test_context::cleanup(cx);
    let (shell, _dir, cx) = opened(cx);
    cx.update(|window, cx| window.click("reference-frequency", cx));
    frame(cx);
    let editor = editor(&shell, cx);
    let input = editor.read_with(cx, |editor, _| editor.input.clone());
    input.update_in(cx, |input, window, cx| input.set_value("bad", window, cx));
    cx.simulate_keystrokes("enter");
    frame(cx);
    assert!(editor.read_with(cx, |editor, _| editor.refused));
    assert_eq!(
        shell.read_with(cx, |shell, _| shell.editing().unwrap().reference()),
        0.
    );
    let grid = shell.read_with(cx, |shell, _| shell.session.show_grid);
    cx.simulate_keystrokes("ctrl-g");
    cx.simulate_keystrokes("space");
    frame(cx);
    assert_eq!(
        shell.read_with(cx, |shell, _| shell.session.show_grid),
        grid
    );
    input.update_in(cx, |input, window, cx| input.set_value("200", window, cx));
    shell.update_in(cx, |shell, window, cx| {
        shell.set_reference(100., window, cx);
    });
    editor.update_in(cx, |editor, window, cx| {
        assert!(!editor.commit(window, cx));
    });
    assert_eq!(
        shell.read_with(cx, |shell, _| shell.editing().unwrap().reference()),
        100.
    );
    cx.simulate_keystrokes("escape");
    frame(cx);
    shell.read_with(cx, |shell, cx| {
        assert!(!shell.reference.hint.read(cx).is_open())
    });
}

#[gpui_kit::test]
fn closing_reference_commits_outside_draft_but_escape_discards_it(cx: &mut TestAppContext) {
    let _cleanup = crate::test_context::cleanup(cx);
    let (shell, _dir, cx) = opened(cx);
    cx.update(|window, cx| window.click("reference-frequency", cx));
    frame(cx);
    let input = editor(&shell, cx).read_with(cx, |editor, _| editor.input.clone());
    input.update_in(cx, |input, window, cx| input.set_value("123", window, cx));
    cx.simulate_mouse_down(
        gpui_kit::point(px(40.), px(300.)),
        MouseButton::Left,
        Default::default(),
    );
    cx.simulate_mouse_up(
        gpui_kit::point(px(40.), px(300.)),
        MouseButton::Left,
        Default::default(),
    );
    frame(cx);
    assert_eq!(
        shell.read_with(cx, |shell, _| shell.editing().unwrap().reference()),
        123.
    );
    cx.update(|window, cx| window.click("reference-frequency", cx));
    frame(cx);
    let input = editor(&shell, cx).read_with(cx, |editor, _| editor.input.clone());
    input.update_in(cx, |input, window, cx| input.set_value("456", window, cx));
    cx.simulate_keystrokes("escape");
    frame(cx);
    assert_eq!(
        shell.read_with(cx, |shell, _| shell.editing().unwrap().reference()),
        123.
    );
}

#[gpui_kit::test]
fn edit_menu_opens_reference_and_uses_guarded_command_availability(cx: &mut TestAppContext) {
    let _cleanup = crate::test_context::cleanup(cx);
    let (shell, _dir, cx) = opened(cx);
    // F10, Edit, then Selection panel followed by Reference frequency.
    cx.simulate_keystrokes("f10 down down right");
    frame(cx);
    let at = cx.update(|window, _| {
        window
            .find_all(gpui_kit::ElementId::Integer(9))
            .into_iter()
            .find(|row| {
                row.path()
                    .iter()
                    .filter(|id| **id == gpui_kit::ElementId::Name("submenu".into()))
                    .count()
                    == 1
            })
            .expect("Reference menu row")
            .bounds()
            .center()
    });
    cx.simulate_click(at, Default::default());
    frame(cx);
    cx.update(|window, _| {
        assert!(window.try_find("reference-editor").is_some());
        assert!(window.try_find("save-reference").is_none());
    });
    shell.read_with(cx, |shell, cx| {
        assert_eq!(shell.information_panel, Some(hints::Panel::Reference));
        assert!(shell.command_availability().is_enabled::<EditReference>());
        assert!(shell.reference.hint.read(cx).is_open());
    });
    cx.simulate_keystrokes("escape");
    frame(cx);
    shell.update(cx, |shell, cx| {
        shell.replacement_active = true;
        cx.notify();
    });
    cx.update(|window, cx| window.dispatch_action(Box::new(EditReference), cx));
    frame(cx);
    shell.read_with(cx, |shell, cx| {
        assert!(!shell.command_availability().is_enabled::<EditReference>());
        assert!(!shell.reference.hint.read(cx).is_open());
    });
}

#[gpui_kit::test]
fn recording_panel_commits_start_and_modes_keep_history_and_analysis(cx: &mut TestAppContext) {
    let _cleanup = crate::test_context::cleanup(cx);
    let (shell, _dir, cx) = opened(cx);
    cx.update(|window, cx| window.click("reference-frequency", cx));
    frame(cx);
    let editor = editor(&shell, cx);
    let input = editor.read_with(cx, |editor, _| editor.start_input.clone());
    input.update_in(cx, |input, window, cx| {
        let value = DateTime::Single("2026-11-23T23:59:59".parse().ok());
        input.set_date_time(value, window, cx);
        cx.emit(DatePickerEvent::Change(input.date_time()));
    });
    cx.simulate_keystrokes("enter");
    frame(cx);
    let (version, generation) = shell.read_with(cx, |shell, _| {
        assert_eq!(
            shell
                .editing()
                .unwrap()
                .recording_start()
                .unwrap()
                .format(3),
            "2026-11-23T23:59:59.000Z"
        );
        (
            shell.editing().unwrap().version(),
            shell
                .file
                .as_ref()
                .unwrap()
                .state
                .analyst
                .generation_for_test(),
        )
    });
    cx.update(|window, cx| window.click("time-absolute", cx));
    frame(cx);
    cx.update(|window, cx| window.click("frequency-relative", cx));
    frame(cx);
    shell.read_with(cx, |shell, _| {
        assert!(shell.absolute_start().is_some());
        assert_eq!(shell.session.frequency_mode, FrequencyMode::Baseband);
        assert_eq!(shell.editing().unwrap().version(), version);
        assert_eq!(
            shell
                .file
                .as_ref()
                .unwrap()
                .state
                .analyst
                .generation_for_test(),
            generation
        );
    });
    input.update_in(cx, |input, window, cx| {
        input.set_date_time(DateTime::Single(None), window, cx);
        cx.emit(DatePickerEvent::Change(input.date_time()));
    });
    cx.simulate_keystrokes("enter");
    frame(cx);
    shell.read_with(cx, |shell, _| {
        assert_eq!(shell.editing().unwrap().recording_start(), None);
        assert_eq!(shell.session.time_origin, TimeOrigin::Absolute);
        assert!(shell.absolute_start().is_none());
    });
}

#[gpui_kit::test]
fn untouched_start_draft_preserves_precise_trim_origin_on_blur_and_escape(cx: &mut TestAppContext) {
    let _cleanup = crate::test_context::cleanup(cx);
    let (shell, _dir, cx) = opened(cx);
    shell.update_in(cx, |shell, window, cx| {
        let start = argand_core::RecordingTime::parse("2026-11-23T12:34:56.123Z");
        shell.set_recording_start(start, window, cx);
        let editing = shell.editing_mut().unwrap();
        editing.delete(argand_core::SampleSpan::between(0, 1).unwrap());
        editing.mark_saved();
        shell.selection = argand_core::SampleSpan::between(10, 20);
    });
    let snapshot = |shell: &Shell| {
        let editing = shell.editing().unwrap();
        (
            editing.recording_start(),
            editing.version(),
            editing.is_dirty(),
            shell.selection,
            shell
                .file
                .as_ref()
                .unwrap()
                .state
                .analyst
                .generation_for_test(),
        )
    };
    let before = shell.read_with(cx, |shell, _| snapshot(shell));
    assert_ne!(before.0.unwrap(), before.0.unwrap().rounded(3));
    for escape in [false, true] {
        cx.update(|window, cx| window.click("reference-frequency", cx));
        frame(cx);
        let input = editor(&shell, cx).read_with(cx, |editor, _| editor.start_input.clone());
        input.update_in(cx, |input, window, cx| {
            window.focus(&input.focus_handle(cx), cx)
        });
        frame(cx);
        if escape {
            cx.simulate_keystrokes("escape");
        } else {
            cx.simulate_click(gpui_kit::point(px(40.), px(300.)), Default::default());
        }
        frame(cx);
        assert_eq!(shell.read_with(cx, |shell, _| snapshot(shell)), before);
    }
}

#[gpui_kit::test]
fn corner_button_is_centered_in_the_real_ruler_minimap_cell(cx: &mut TestAppContext) {
    let _cleanup = crate::test_context::cleanup(cx);
    let (shell, _dir, cx) = opened(cx);
    for width in [800., 640.] {
        cx.simulate_resize(gpui_kit::size(px(width), px(600.)));
        for _ in 0..2 {
            frame(cx);
            cx.update(|window, cx| {
                let plot = shell.read(cx).plot_view().unwrap().read(cx);
                let corner = plot.geometry.unwrap().reference_corner();
                let button = window.find("reference-frequency").bounds();
                let delta = button.center() - corner.center();
                assert!(f32::from(delta.x).abs() <= 1.);
                assert!(f32::from(delta.y).abs() <= 1.);
                assert!(button.left() >= corner.left() && button.right() <= corner.right());
                assert!(button.top() >= corner.top() && button.bottom() <= corner.bottom());
                window.dispatch_action(Box::new(ToggleOrientation), cx);
            });
        }
    }
}

fn scale_menu_row(
    cx: &mut gpui_kit::VisualTestContext,
    index: u64,
    depth: usize,
) -> gpui_kit::Point<Pixels> {
    cx.update(|window, _| {
        window
            .find_all(gpui_kit::ElementId::Integer(index))
            .into_iter()
            .find(|row| {
                row.path()
                    .iter()
                    .filter(|id| **id == gpui_kit::ElementId::Name("submenu".into()))
                    .count()
                    == depth
            })
            .unwrap()
            .bounds()
            .center()
    })
}

#[gpui_kit::test]
fn scale_menu_keycaps_keep_explicit_choices_idempotent(cx: &mut TestAppContext) {
    let _cleanup = crate::test_context::cleanup(cx);
    let (shell, _dir, cx) = opened(cx);
    shell.update_in(cx, |shell, window, cx| {
        shell.set_recording_start(
            argand_core::RecordingTime::parse("2026-11-24T00:00:00.123Z"),
            window,
            cx,
        );
    });
    let version = shell.read_with(cx, |shell, _| shell.editing().unwrap().version());
    for (branch, choice) in [(7, 1), (7, 1), (8, 0), (8, 0)] {
        cx.simulate_keystrokes("f10 down down down right");
        frame(cx);
        let at = scale_menu_row(cx, branch, 1);
        cx.simulate_click(at, Default::default());
        frame(cx);
        cx.update(|window, _| {
            let keycaps = window
                .find_all("command-shortcut")
                .into_iter()
                .filter(|row| {
                    row.path()
                        .iter()
                        .filter(|id| **id == gpui_kit::ElementId::Name("submenu".into()))
                        .count()
                        == 2
                })
                .count();
            assert_eq!(
                keycaps, 2,
                "both explicit choices expose the canonical toggle key"
            );
        });
        let at = scale_menu_row(cx, choice, 2);
        cx.simulate_click(at, Default::default());
        frame(cx);
        shell.read_with(cx, |shell, _| {
            assert_eq!(shell.editing().unwrap().version(), version);
            if branch == 7 {
                assert_eq!(shell.session.time_origin, TimeOrigin::Absolute);
            } else {
                assert_eq!(shell.session.frequency_mode, FrequencyMode::Baseband);
            }
        });
    }
}

#[gpui_kit::test]
fn metadata_field_frames_align_and_reference_steps_keep_the_existing_contract(
    cx: &mut TestAppContext,
) {
    let _cleanup = crate::test_context::cleanup(cx);
    let (shell, _dir, cx) = opened(cx);
    cx.update(|window, cx| window.click("reference-frequency", cx));
    frame(cx);
    let number = editor(&shell, cx).read_with(cx, |editor, _| editor.number.clone());
    cx.update(|window, cx| {
        let frequency = window.find(("numeric-field", number.entity_id())).bounds();
        let start = window.find("recording-start-input").bounds();
        assert_eq!(frequency.left(), start.left());
        assert_eq!(frequency.right(), start.right());
        assert_eq!(frequency.size.height, start.size.height);
        window.click("reference-up", cx);
    });
    frame(cx);
    assert_eq!(
        shell.read_with(cx, |shell, _| shell.editing().unwrap().reference()),
        1.
    );
    cx.simulate_keystrokes("up");
    frame(cx);
    assert_eq!(
        shell.read_with(cx, |shell, _| shell.editing().unwrap().reference()),
        2.
    );
}

#[gpui_kit::test]
fn creation_default_is_clean_and_saved_clear_does_not_resurrect(cx: &mut TestAppContext) {
    let _cleanup = crate::test_context::cleanup(cx);
    let (shell, dir, cx) = opened(cx);
    let path = dir.join("capture.wav");
    let expected = std::fs::metadata(&path)
        .unwrap()
        .created()
        .ok()
        .and_then(argand_core::RecordingTime::from_timestamp);
    shell.read_with(cx, |shell, _| {
        assert_eq!(shell.editing().unwrap().recording_start(), expected);
        assert!(!shell.editing().unwrap().is_dirty());
        assert_eq!(shell.editing().unwrap().file().meta.recording_start, None);
    });
    shell.update_in(cx, |shell, window, cx| {
        // Also exercise clear deterministically on filesystems without birthtime.
        if shell.editing().unwrap().recording_start().is_none() {
            shell.set_recording_start(
                argand_core::RecordingTime::parse("2026-11-23T12:00:00Z"),
                window,
                cx,
            );
        }
        shell.set_recording_start(None, window, cx);
    });
    cx.simulate_keystrokes(if cfg!(target_os = "macos") {
        "cmd-s"
    } else {
        "ctrl-s"
    });
    wait_until(cx, &shell, "clear save", |shell| {
        shell.saving.is_none() && !shell.editing().unwrap().is_dirty()
    });
    shell.read_with(cx, |shell, _| {
        assert_eq!(shell.editing().unwrap().recording_start(), None)
    });
    assert_eq!(
        argand_io::open(&path, &argand_io::OpenHints::default())
            .unwrap()
            .meta()
            .recording_start,
        None
    );
}
