use super::*;
use argand_core::{Domain, SampleFormat, SampleType, SignalMeta};
use argand_edit::{Capture, SourceId};
use gpui_kit::test::TestWindowExt;
use gpui_kit::{AppContext, TestAppContext};

fn request() -> Request {
    let meta = SignalMeta {
        sample_rate: 8000.0,
        center_freq: 100_000.0,
        sample_type: SampleType::new(Domain::Iq, SampleFormat::F32),
        len_samples: 1000,
        container: "wav",
        divisor: 1.0,
        source: "input.wav".into(),
    };
    Request {
        capture: Capture::whole(SourceId(0), 1000),
        sources: Vec::new(),
        input: meta.clone(),
        output: meta,
        target: "output.wav".into(),
        protected: Vec::new(),
        level: Level::Preserve,
        full_scale: 1.0,
        copy: None,
    }
}

#[gpui_kit::test]
fn changing_kind_keeps_both_explicit_frequencies_and_revalidates_the_band(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let mut editor = None;
    let window = cx.add_window(|window, cx| {
        let form = cx.new(|cx| Editor::new(request(), window, cx));
        editor = Some(form.clone());
        Root::new(form, window, cx)
    });
    cx.run_until_parked();
    let editor = editor.unwrap();
    window
        .update(cx, |_, window, cx| {
            let kind = editor.read(cx).kind.clone();
            kind.update(cx, |kind, cx| {
                kind.set_selected_value(&kind_label("rl_i16"), window, cx)
            });
            let converted = editor.read(cx).request(cx).unwrap();
            assert_eq!(converted.output.sample_rate, 8000.0);
            assert_eq!(converted.output.center_freq, 100_000.0);
            assert_eq!(converted.output.sample_type.to_string(), "rl_i16");
            assert!(
                editor
                    .read(cx)
                    .summary(cx)
                    .unwrap()
                    .contains("will be discarded")
            );
            let reference = editor.read(cx).reference.clone();
            reference.update(cx, |state, cx| state.set_value("200000", window, cx));
            assert!(editor.read(cx).request(cx).is_err());
            let rate = editor.read(cx).rate.clone();
            rate.update(cx, |state, cx| state.set_value("NaN", window, cx));
            assert!(editor.read(cx).request(cx).is_err());
        })
        .unwrap();
}

#[test]
fn repeated_conversion_names_do_not_accumulate_suffixes() {
    assert_eq!(converted_name("recording.wav"), "recording-converted.wav");
    assert_eq!(
        converted_name("recording-converted.wav"),
        "recording-converted.wav"
    );
    assert_eq!(
        converted_name("recording-converted-converted.wav"),
        "recording-converted.wav"
    );
}

#[gpui_kit::test]
fn only_valid_changed_targets_or_normalization_enable_conversion(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let mut form = None;
    let window = cx.add_window(|window, cx| {
        let editor = cx.new(|cx| Editor::new(request(), window, cx));
        form = Some(editor.clone());
        Root::new(editor, window, cx)
    });
    let form = form.unwrap();
    window
        .update(cx, |_, window, cx| {
            form.update(cx, |editor, cx| {
                assert!(!editor.changed(&editor.request(cx).unwrap()));
                let level = editor.level.clone();
                level.update(cx, |level, cx| {
                    level.set_selected_value(&"Normalize peak".into(), window, cx)
                });
                assert!(editor.changed(&editor.request(cx).unwrap()));
                let rate = editor.rate.clone();
                rate.update(cx, |rate, cx| rate.set_value("0", window, cx));
                assert!(editor.request(cx).is_err());
                assert_eq!(editor.request.input.sample_rate, 8000.0);
            });
        })
        .unwrap();
}

#[gpui_kit::test]
fn conversion_of_unsaved_edits_is_unsaved_until_first_save_and_can_repeat(cx: &mut TestAppContext) {
    use super::super::replace_ui::tests::{open_window, wait_until, write_wave};
    use crate::editing::Editing;
    let (shell, cx) = open_window(cx);
    let dir = std::env::temp_dir().join(format!(
        "argand-conversion-lifecycle-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("recording.wav");
    write_wave(&path, 2400);
    let original = std::fs::read(&path).unwrap();
    shell.update_in(cx, |shell, window, cx| {
        shell.open(Origin::new(path.clone()), window, cx)
    });
    wait_until(cx, &shell, "source description", |shell| {
        shell.editing().is_some_and(Editing::is_described)
    });
    shell.update_in(cx, |shell, window, cx| {
        shell
            .editing_mut()
            .unwrap()
            .delete(argand_core::SampleSpan::between(0, 1200).unwrap());
        assert!(shell.editing().unwrap().is_dirty());
        let mut request = shell.conversion_request().unwrap();
        request.output.sample_type = SampleType::new(Domain::Iq, SampleFormat::F32);
        shell.start_conversion(request, window, cx);
        assert!(
            shell.conversion_request().is_none(),
            "a second job is unavailable"
        );
    });
    wait_until(cx, &shell, "converted unsaved document", |shell| {
        shell
            .file
            .as_ref()
            .is_some_and(|file| file.unsaved.is_some())
            && shell.editing().is_some_and(Editing::is_described)
    });
    let first_backing = shell.read_with(cx, |shell, _| {
        let editing = shell.editing().unwrap();
        assert_eq!(editing.len(), 1200);
        assert!(editing.is_dirty());
        assert!(!editing.can_undo());
        assert!(shell.file_name().contains("recording-converted.wav"));
        assert!(shell.can_save_over().is_err());
        assert!(shell.session.recent.iter().all(|entry| entry.path == path));
        editing.file().meta.source.clone()
    });
    assert_ne!(first_backing, path);
    assert_eq!(std::fs::read(&path).unwrap(), original);
    shell.update_in(cx, |shell, window, cx| {
        let mut request = shell.conversion_request().unwrap();
        request.output.sample_type = SampleType::new(Domain::Iq, SampleFormat::I16);
        shell.start_conversion(request, window, cx);
        assert!(first_backing.exists());
    });
    wait_until(cx, &shell, "second conversion", |shell| {
        shell.conversion.running.is_none()
            && shell.editing().is_some_and(|editing| {
                editing.is_described()
                    && editing.file().meta.sample_type.format == SampleFormat::I16
            })
    });
    shell.update_in(cx, |shell, window, cx| {
        assert!(shell.file_name().contains("recording-converted.wav"));
        assert!(!shell.file_name().contains("converted-converted"));
        assert!(shell.editing().unwrap().is_dirty());
        shell.save_over(window, cx);
    });
    let target = dir.join("chosen.wav");
    let chosen = target.clone();
    cx.simulate_new_path_selection(move |_| Some(chosen));
    wait_until(cx, &shell, "saved ordinary document", |shell| {
        shell.saving.is_none()
            && shell
                .file
                .as_ref()
                .is_some_and(|file| file.unsaved.is_none())
            && shell
                .editing()
                .is_some_and(|editing| editing.is_described() && !editing.is_dirty())
    });
    assert_eq!(
        argand_io::open(&target, &argand_io::OpenHints::default())
            .unwrap()
            .meta()
            .len_samples,
        1200
    );
    assert_eq!(std::fs::read(&path).unwrap(), original);
    std::fs::remove_dir_all(dir).unwrap();
}

#[gpui_kit::test]
fn conversion_failure_and_cancel_preserve_current_unsaved_edits(cx: &mut TestAppContext) {
    use super::super::replace_ui::tests::{open_window, wait_until, write_wave};
    use crate::editing::Editing;
    let (shell, cx) = open_window(cx);
    let dir = std::env::temp_dir().join(format!("argand-conversion-cancel-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("input.wav");
    write_wave(&path, 24_000);
    let bytes = std::fs::read(&path).unwrap();
    shell.update_in(cx, |shell, window, cx| {
        shell.open(Origin::new(path.clone()), window, cx)
    });
    wait_until(cx, &shell, "source", |shell| {
        shell.editing().is_some_and(Editing::is_described)
    });
    let state = shell.update_in(cx, |shell, _, _| {
        shell
            .editing_mut()
            .unwrap()
            .delete(argand_core::SampleSpan::between(0, 100).unwrap());
        (
            shell.file.as_ref().unwrap().id,
            shell.editing().unwrap().version(),
            shell.editing().unwrap().capture().clone(),
        )
    });
    for fail in [true, false] {
        shell.update_in(cx, |shell, window, cx| {
            let mut request = shell.conversion_request().unwrap();
            request.output.sample_rate = 12_000.;
            if fail {
                request.full_scale = f64::NAN;
            }
            shell.start_conversion(request, window, cx);
            if !fail {
                shell.conversion.running.as_ref().unwrap().job.cancel();
            }
        });
        wait_until(cx, &shell, "job end", |shell| {
            shell.conversion.running.is_none()
        });
        shell.read_with(cx, |shell, _| {
            assert_eq!(shell.file.as_ref().unwrap().id, state.0);
            let editing = shell.editing().unwrap();
            assert_eq!(editing.version(), state.1);
            assert_eq!(editing.capture(), &state.2);
            assert!(editing.is_dirty());
            assert!(shell.file.as_ref().unwrap().unsaved.is_none());
        });
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
    }
    std::fs::remove_dir_all(dir).unwrap();
}

#[gpui_kit::test]
fn file_panel_reuses_standard_controls_and_escape_discards_the_draft(cx: &mut TestAppContext) {
    use super::super::replace_ui::tests::{wait_until, write_wave};
    use crate::editing::Editing;
    let (shell, cx) = panel_window(cx);
    let path = std::env::temp_dir().join(format!("argand-panel-{}.wav", std::process::id()));
    write_wave(&path, 2400);
    shell.update_in(cx, |shell, window, cx| {
        shell.open(Origin::new(path.clone()), window, cx)
    });
    wait_until(cx, &shell, "source", |shell| {
        shell.editing().is_some_and(Editing::is_described)
    });
    cx.simulate_keystrokes("f11");
    frame(cx);
    let editor = shell.read_with(cx, |shell, cx| {
        shell
            .conversion
            .hint
            .read(cx)
            .view()
            .unwrap()
            .downcast::<Editor>()
            .unwrap()
    });
    cx.simulate_keystrokes("tab");
    frame(cx);
    editor.update_in(cx, |editor, window, cx| {
        assert!(
            editor
                .reference
                .read(cx)
                .focus_handle(cx)
                .is_focused(window)
        );
    });
    cx.simulate_keystrokes("shift-tab");
    frame(cx);
    editor.update_in(cx, |editor, window, cx| {
        assert!(editor.rate.read(cx).focus_handle(cx).is_focused(window));
    });
    editor.update_in(cx, |editor, window, cx| {
        editor
            .rate
            .update(cx, |rate, cx| rate.set_value("12000", window, cx))
    });
    frame(cx);
    cx.simulate_mouse_move(
        point(px(900.), px(100.)),
        None,
        gpui_kit::Modifiers::default(),
    );
    frame(cx);
    assert!(shell.read_with(cx, |shell, cx| shell.conversion.hint.read(cx).is_open()));
    assert_eq!(
        shell.read_with(cx, |shell, _| shell
            .editing()
            .unwrap()
            .file()
            .meta
            .sample_rate),
        24_000.
    );
    editor.update_in(cx, |editor, window, cx| {
        window.focus(&editor.kind.read(cx).focus_handle(cx), cx)
    });
    frame(cx);
    cx.simulate_keystrokes("enter");
    frame(cx);
    cx.simulate_keystrokes("escape");
    frame(cx);
    assert!(
        shell.read_with(cx, |shell, cx| shell.conversion.hint.read(cx).is_open()),
        "first Escape belongs to Select"
    );
    cx.simulate_keystrokes("escape");
    frame(cx);
    assert!(!shell.read_with(cx, |shell, cx| shell.conversion.hint.read(cx).is_open()));
    cx.simulate_keystrokes("f11");
    frame(cx);
    let restored = shell.read_with(cx, |shell, cx| {
        shell
            .conversion
            .hint
            .read(cx)
            .view()
            .unwrap()
            .downcast::<Editor>()
            .unwrap()
    });
    assert_eq!(
        restored.read_with(cx, |editor, cx| editor
            .request(cx)
            .unwrap()
            .output
            .sample_rate),
        24_000.
    );
    std::fs::remove_file(path).unwrap();
}

fn frame(cx: &mut gpui_kit::VisualTestContext) {
    cx.run_until_parked();
    cx.update(|window, cx| window.render_frame(cx));
    cx.run_until_parked();
}

fn panel_window(cx: &mut TestAppContext) -> (Entity<Shell>, &mut gpui_kit::VisualTestContext) {
    cx.executor().allow_parking();
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
    let (_, cx) = cx.add_window_view(|window, cx| {
        let view = cx.new(|cx| Shell::new(Config::default(), None, Session::default(), window, cx));
        Shell::bind_choose_file(view.downgrade(), cx);
        shell = Some(view.clone());
        Root::new(view, window, cx)
    });
    let shell = shell.unwrap();
    cx.simulate_resize(size(px(1000.), px(900.)));
    (shell, cx)
}

fn converted_fixture<'a>(
    cx: &'a mut TestAppContext,
    tag: &str,
) -> (
    Entity<Shell>,
    &'a mut gpui_kit::VisualTestContext,
    std::path::PathBuf,
) {
    use super::super::replace_ui::tests::{open_window, wait_until, write_wave};
    use crate::editing::Editing;
    let (shell, cx) = open_window(cx);
    let dir = std::env::temp_dir().join(format!("argand-converted-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("recording.wav");
    write_wave(&path, 2400);
    shell.update_in(cx, |shell, window, cx| {
        shell.open(Origin::new(path), window, cx)
    });
    wait_until(cx, &shell, "described input", |shell| {
        shell.editing().is_some_and(Editing::is_described)
    });
    shell.update_in(cx, |shell, window, cx| {
        let mut request = shell.conversion_request().unwrap();
        request.output.sample_type.format = SampleFormat::F32;
        shell.start_conversion(request, window, cx);
    });
    wait_until(
        cx,
        &shell,
        "adopted reader delivers complete analysis",
        |shell| {
            shell.file.as_ref().is_some_and(|file| {
                file.unsaved.is_some()
                    && matches!(
                        file.state.document.status(),
                        crate::document::Status::Ready { .. }
                    )
            }) && shell.editing().is_some_and(Editing::is_described)
        },
    );
    (shell, cx, dir)
}

#[gpui_kit::test]
fn converted_document_open_discard_and_save_each_answer_only_once(cx: &mut TestAppContext) {
    use super::super::replace_ui::tests::{wait_until, write_wave};
    let (shell, cx, dir) = converted_fixture(cx, "open");
    let second = dir.join("second.wav");
    write_wave(&second, 4800);
    shell.update_in(cx, |shell, window, cx| {
        shell.open(Origin::new(second.clone()), window, cx)
    });
    assert!(cx.has_pending_prompt());
    cx.simulate_prompt_answer("Discard");
    wait_until(cx, &shell, "discard then open", |shell| {
        shell
            .file
            .as_ref()
            .is_some_and(|file| file.state.document.origin().path == second)
            && shell
                .editing()
                .is_some_and(|editing| editing.is_described())
    });
    assert!(!cx.has_pending_prompt());
    shell.update_in(cx, |shell, window, cx| {
        let mut request = shell.conversion_request().unwrap();
        request.output.sample_type.format = SampleFormat::F32;
        shell.start_conversion(request, window, cx);
    });
    wait_until(cx, &shell, "second converted document", |shell| {
        shell
            .file
            .as_ref()
            .is_some_and(|file| file.unsaved.is_some())
            && shell
                .editing()
                .is_some_and(|editing| editing.is_described())
    });
    let first = dir.join("recording.wav");
    shell.update_in(cx, |shell, window, cx| {
        shell.open(Origin::new(first.clone()), window, cx)
    });
    cx.simulate_prompt_answer("Save");
    cx.run_until_parked();
    let saved = dir.join("saved.wav");
    cx.simulate_new_path_selection(|_| Some(saved.clone()));
    wait_until(cx, &shell, "save then open", |shell| {
        shell
            .file
            .as_ref()
            .is_some_and(|file| file.state.document.origin().path == first)
    });
    assert!(!cx.has_pending_prompt());
    assert!(saved.exists());
    std::fs::remove_dir_all(dir).unwrap();
}

#[gpui_kit::test]
fn published_result_reader_failure_preserves_dirty_capture(cx: &mut TestAppContext) {
    use super::super::replace_ui::tests::wait_until;
    let (shell, cx, dir) = converted_fixture(cx, "reader-failure");
    let before = shell.update_in(cx, |shell, window, cx| {
        shell
            .editing_mut()
            .unwrap()
            .delete(argand_core::SampleSpan::between(0, 100).unwrap());
        let before = (
            shell.file.as_ref().unwrap().id,
            shell.editing().unwrap().version(),
            shell.editing().unwrap().capture().clone(),
        );
        shell.conversion.fail_reader_open = true;
        let mut request = shell.conversion_request().unwrap();
        request.output.sample_type.format = SampleFormat::I16;
        shell.start_conversion(request, window, cx);
        before
    });
    wait_until(
        cx,
        &shell,
        "reader open failure after publication",
        |shell| {
            shell.conversion.running.is_none()
                && shell.conversion.pending.is_none()
                && matches!(
                    shell.save_notice,
                    Some(saving_ui::Notice::Failed("Conversion failed", _))
                )
        },
    );
    shell.read_with(cx, |shell, _| {
        assert_eq!(shell.file.as_ref().unwrap().id, before.0);
        let editing = shell.editing().unwrap();
        assert_eq!(editing.version(), before.1);
        assert_eq!(editing.capture(), &before.2);
        assert!(editing.is_dirty());
        assert!(shell.file.as_ref().unwrap().unsaved.is_some());
    });
    std::fs::remove_dir_all(dir).unwrap();
}

fn stage_reader(shell: &mut Shell, window: &mut Window, cx: &mut Context<Shell>) {
    use super::super::replace_ui::tests::write_wave;
    let folder = crate::scratch::folder().unwrap();
    let path = folder.new_path("wav");
    write_wave(&path, 1200);
    let (job, _) = crate::saving::spawn("prepared-conversion-test", |_, _| (), |_| ());
    let file = shell.file.as_ref().unwrap();
    let job = Converting {
        job,
        progress: None,
        document: file.id,
        version: shell.editing().unwrap().version(),
        name: "staged-converted.wav".into(),
        backing: Arc::new(crate::scratch::Scratch::new(path.clone())),
    };
    shell.prepare_conversion_reader(job, Origin::new(path), window, cx);
}

#[gpui_kit::test]
fn cancelled_and_stale_prepared_readers_cannot_replace_or_stop_current_reader(
    cx: &mut TestAppContext,
) {
    use super::super::replace_ui::tests::wait_until;
    let (shell, cx, dir) = converted_fixture(cx, "reader-routing");
    let document = shell.read_with(cx, |shell, _| shell.file.as_ref().unwrap().id);
    for stale in [false, true] {
        shell.update_in(cx, |shell, window, cx| {
            stage_reader(shell, window, cx);
            if stale {
                shell
                    .file
                    .as_mut()
                    .unwrap()
                    .editing
                    .as_mut()
                    .unwrap()
                    .delete(argand_core::SampleSpan::between(0, 100).unwrap());
            } else {
                shell.conversion.pending = None;
            }
            shell.settings.fft_size = if stale { 512 } else { 256 };
            shell.ask_for_a_picture();
        });
        wait_until(
            cx,
            &shell,
            "current reader still delivers after rejected preparation",
            |shell| {
                shell.conversion.pending.is_none()
                    && shell.file.as_ref().is_some_and(|file| {
                        file.id == document
                            && file.state.document.analysis().is_some_and(|analysis| {
                                analysis.psd.db.len() == if stale { 512 } else { 256 }
                            })
                    })
            },
        );
        shell.read_with(cx, |shell, _| assert!(shell.editing().unwrap().is_dirty()));
    }
    std::fs::remove_dir_all(dir).unwrap();
}

#[gpui_kit::test]
fn unsaved_selection_exports_never_suggest_private_scratch_directories(cx: &mut TestAppContext) {
    let (shell, cx, dir) = converted_fixture(cx, "export");
    for (time, band) in [(true, false), (false, true), (true, true)] {
        shell.update_in(cx, |shell, window, cx| {
            shell.selection = time.then(|| argand_core::SampleSpan::between(100, 1000).unwrap());
            shell.band = band.then(|| argand_core::FrequencyBand::between(1000., 3000.).unwrap());
            let (display, directory) =
                shell.save_presentation(&shell.editing().unwrap().file().meta);
            assert!(directory.as_os_str().is_empty());
            assert_eq!(
                display.source,
                std::path::PathBuf::from("recording-converted.wav")
            );
            shell.save_as(true, window, cx);
        });
        cx.simulate_new_path_selection(|directory| {
            assert!(directory.as_os_str().is_empty());
            None
        });
        cx.run_until_parked();
    }
    shell.read_with(cx, |shell, _| {
        assert!(shell.file.as_ref().unwrap().unsaved.is_some())
    });
    std::fs::remove_dir_all(dir).unwrap();
}
