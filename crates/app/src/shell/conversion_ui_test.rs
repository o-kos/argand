use super::super::replace_ui::tests::wait_for_save_chooser;
use super::*;
use argand_core::{Domain, SampleFormat, SampleType, SignalMeta};
use argand_edit::{Capture, SourceId};
use gpui_kit::component::input::InputEvent;
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
        carrier: None,
        target: "output.wav".into(),
        protected: Vec::new(),
        container: argand_io::write::Container::Wave,
        level: Level::Preserve,
        full_scale: 1.0,
        copy: None,
    }
}

#[gpui_kit::test]
fn domain_switches_derive_carrier_and_rate_from_source_and_encoding_preserves_drafts(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    let mut editor = None;
    let window = cx.add_window(|window, cx| {
        let form = cx.new(|cx| Editor::new(request(), window, cx));
        editor = Some(form.clone());
        Root::new(form, window, cx)
    });
    let editor = editor.unwrap();
    window
        .update(cx, |_, window, cx| {
            let form = editor.read(cx);
            let rate = form.rate.clone();
            let reference = form.reference.clone();
            let kind = form.kind.clone();
            rate.update(cx, |input, cx| input.set_value("12345", window, cx));
            reference.update(cx, |input, cx| {
                input.set_value("not a frequency", window, cx)
            });
            kind.update(cx, |kind, cx| {
                kind.set_selected_value(&kind_label("iq_i16"), window, cx);
                cx.emit(SelectEvent::Confirm(kind.selected_value().cloned()));
            });
            assert_eq!(
                editor.read(cx).request(cx).unwrap().output.center_freq,
                100_000.,
                "request guard also ignores a hidden invalid draft"
            );
        })
        .unwrap();
    cx.run_until_parked();
    window
        .update(cx, |_, window, cx| {
            editor.read(cx).kind.clone().update(cx, |kind, cx| {
                kind.set_selected_value(&kind_label("rl_i16"), window, cx);
                cx.emit(SelectEvent::Confirm(kind.selected_value().cloned()));
            });
        })
        .unwrap();
    cx.run_until_parked();
    window
        .update(cx, |_, window, cx| {
            let form = editor.read(cx);
            assert_eq!(form.request(cx).unwrap().output.sample_rate, 16000.);
            assert_eq!(form.request(cx).unwrap().output.center_freq, 0.);
            assert!(form.summary(cx).unwrap().is_empty());
            let rate = form.rate.clone();
            let reference = form.reference.clone();
            rate.update(cx, |input, cx| input.set_value("18000", window, cx));
            assert_eq!(
                editor.read(cx).reference.read(cx).value().as_ref(),
                crate::numbers::number(4000.)
            );
            reference.update(cx, |input, cx| input.set_value("3000", window, cx));
            assert_eq!(editor.read(cx).request(cx).unwrap().output.center_freq, 0.);
            editor.read(cx).kind.clone().update(cx, |kind, cx| {
                kind.set_selected_value(&kind_label("rl_f32"), window, cx);
                cx.emit(SelectEvent::Confirm(kind.selected_value().cloned()));
            });
        })
        .unwrap();
    cx.run_until_parked();
    window
        .update(cx, |_, window, cx| {
            assert_eq!(
                editor.read(cx).reference.read(cx).value().as_ref(),
                "3000",
                "encoding-only changes preserve the active draft"
            );
            assert_eq!(
                editor.read(cx).request(cx).unwrap().output.sample_rate,
                18000.
            );
            assert_eq!(
                editor.read(cx).summary(cx).unwrap(),
                format!(
                    "Input frequencies outside {} … {} Hz will be discarded",
                    crate::numbers::number(-3000.),
                    crate::numbers::number(4000.),
                )
            );
            editor.read(cx).kind.clone().update(cx, |kind, cx| {
                kind.set_selected_value(&kind_label("iq_f32"), window, cx);
                cx.emit(SelectEvent::Confirm(kind.selected_value().cloned()));
            });
        })
        .unwrap();
    cx.run_until_parked();
    editor.read_with(cx, |editor, cx| {
        assert_eq!(
            editor.reference.read(cx).value().as_ref(),
            crate::numbers::number(100_000.)
        );
        assert_eq!(editor.request(cx).unwrap().output.center_freq, 100_000.);
        assert_eq!(editor.request(cx).unwrap().output.sample_rate, 8000.);
    });
}

#[gpui_kit::test]
fn only_valid_changed_targets_enable_conversion(cx: &mut TestAppContext) {
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
                assert_eq!(editor.request(cx).unwrap().level, Level::Preserve);
                let rate = editor.rate.clone();
                rate.update(cx, |rate, cx| rate.set_value("0", window, cx));
                assert!(editor.request(cx).is_err());
                assert_eq!(editor.request.input.sample_rate, 8000.0);
            });
        })
        .unwrap();
}

#[gpui_kit::test]
fn numeric_commit_groups_drafts_and_container_validation_never_changes_type(
    cx: &mut TestAppContext,
) {
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
            let editor = form.read(cx);
            let rate = editor.rate.clone();
            assert_eq!(
                rate.read(cx).value().as_ref(),
                crate::numbers::number(8000.0)
            );
            rate.update(cx, |state, cx| state.set_value("12345.5", window, cx));
            assert_eq!(rate.read(cx).value().as_ref(), "12345.5");
            rate.update(cx, |_, cx| {
                cx.emit(InputEvent::PressEnter {
                    secondary: false,
                    shift: false,
                })
            });
        })
        .unwrap();
    cx.run_until_parked();
    window
        .update(cx, |_, window, cx| {
            let editor = form.read(cx);
            let rate = editor.rate.clone();
            let container = editor.container.clone();
            let kind = editor.kind.clone();
            assert_eq!(
                rate.read(cx).value().as_ref(),
                crate::numbers::number(12345.5)
            );
            container.update(cx, |state, cx| {
                state.set_selected_value(&"FLAC".to_owned(), window, cx)
            });
            assert!(
                form.read(cx)
                    .request(cx)
                    .err()
                    .unwrap()
                    .contains("8-bit or 16-bit")
            );
            assert_eq!(
                kind.read(cx).selected_value().unwrap(),
                &kind_label("iq_f32")
            );
            kind.update(cx, |state, cx| {
                state.set_selected_value(&kind_label("iq_i16"), window, cx)
            });
            let request = form.read(cx).request(cx).unwrap();
            assert_eq!(request.output.sample_rate, 12345.5);
            assert_eq!(request.output.center_freq, 100000.0);
            assert_eq!(request.input.sample_rate, 8000.0);
            assert_eq!(request.container, argand_io::write::Container::Flac);
            assert_eq!(request.level, Level::Preserve);
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
        assert_save_parent(shell, &path);
        editing.file().meta.source.clone()
    });
    assert_ne!(first_backing, path);
    assert_eq!(std::fs::read(&path).unwrap(), original);
    shell.update_in(cx, |shell, window, cx| {
        let mut request = shell.conversion_request().unwrap();
        request.output.sample_type = SampleType::new(Domain::Iq, SampleFormat::I16);
        request.container = argand_io::write::Container::Flac;
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
        assert!(shell.file_name().contains("recording-converted.flac"));
        assert_save_parent(shell, &path);
        assert_eq!(
            shell.conversion_request().unwrap().container,
            argand_io::write::Container::Flac
        );
        assert_eq!(shell.editing().unwrap().file().meta.container, "wav");
        assert!(!shell.file_name().contains("converted-converted"));
        assert!(shell.editing().unwrap().is_dirty());
        shell.save_over(window, cx);
    });
    let target = dir.join("chosen.flac");
    let chosen = target.clone();
    wait_for_save_chooser(cx);
    cx.simulate_new_path_selection(move |_| Some(chosen));
    wait_until(
        cx,
        &shell,
        "saved ordinary document",
        saved_ordinary_document,
    );
    assert_eq!(&std::fs::read(&target).unwrap()[..4], b"fLaC");
    assert_saved_flac_metadata(&shell, cx);
    assert_saved_samples(&target, 1200);
    assert_saved_flac_band(&shell, cx, &dir);
    assert_eq!(std::fs::read(&path).unwrap(), original);
    std::fs::remove_dir_all(dir).unwrap();
}

fn assert_saved_flac_metadata(shell: &Entity<Shell>, cx: &mut gpui_kit::VisualTestContext) {
    assert_eq!(
        shell.read_with(cx, |shell, _| shell
            .editing()
            .unwrap()
            .file()
            .meta
            .container),
        "flac"
    );
}

fn assert_save_parent(shell: &Shell, path: &std::path::Path) {
    assert_eq!(
        shell
            .save_presentation(&shell.editing().unwrap().file().meta)
            .1,
        path.parent().unwrap()
    );
}

fn assert_saved_flac_band(
    shell: &Entity<Shell>,
    cx: &mut gpui_kit::VisualTestContext,
    dir: &std::path::Path,
) {
    use super::super::replace_ui::tests::wait_until;
    shell.update_in(cx, |shell, window, cx| {
        shell.band = argand_core::FrequencyBand::between(100.0, 1000.0);
        shell.save_as(true, window, cx);
    });
    let target = dir.join("saved-band.flac");
    let chosen = target.clone();
    wait_for_save_chooser(cx);
    cx.simulate_new_path_selection(move |_| Some(chosen));
    wait_until(cx, shell, "saved FLAC band", |shell| {
        shell.saving.is_none() && shell.save_notice.is_some()
    });
    assert_eq!(&std::fs::read(target).unwrap()[..4], b"fLaC");
    assert_saved_container_overrides(shell, cx, dir);
}

fn assert_saved_container_overrides(
    shell: &Entity<Shell>,
    cx: &mut gpui_kit::VisualTestContext,
    dir: &std::path::Path,
) {
    use super::super::replace_ui::tests::wait_until;
    shell.update_in(cx, |shell, window, cx| {
        shell
            .editing_mut()
            .unwrap()
            .delete(argand_core::SampleSpan::between(0, 100).unwrap());
        shell.save_as(false, window, cx);
    });
    let wave = dir.join("override.wav");
    let chosen = wave.clone();
    wait_for_save_chooser(cx);
    cx.simulate_new_path_selection(move |_| Some(chosen));
    wait_until(cx, shell, "FLAC to WAV Save as", saved_ordinary_document);
    assert_eq!(&std::fs::read(&wave).unwrap()[..4], b"RIFF");
    assert_eq!(
        shell.read_with(cx, |shell, _| shell
            .editing()
            .unwrap()
            .file()
            .meta
            .container),
        "wav"
    );
    for band in [false, true] {
        shell.update_in(cx, |shell, window, cx| {
            shell.selection = argand_core::SampleSpan::between(50, 200);
            shell.band = band.then(|| argand_core::FrequencyBand::between(100.0, 1000.0).unwrap());
            shell.save_as(true, window, cx);
        });
        let target = dir.join(if band {
            "wave-band.flac"
        } else {
            "wave-time.flac"
        });
        let chosen = target.clone();
        wait_for_save_chooser(cx);
        cx.simulate_new_path_selection(move |_| Some(chosen));
        wait_until(cx, shell, "WAV selection to FLAC Save as", |shell| {
            shell.saving.is_none() && target.exists()
        });
        assert_eq!(&std::fs::read(target).unwrap()[..4], b"fLaC");
        assert_eq!(
            shell.read_with(cx, |shell, _| shell
                .editing()
                .unwrap()
                .file()
                .meta
                .container),
            "wav"
        );
    }
}

fn assert_saved_samples(target: &std::path::Path, count: u64) {
    assert_eq!(
        argand_io::open(target, &argand_io::OpenHints::default())
            .unwrap()
            .meta()
            .len_samples,
        count
    );
}

fn saved_ordinary_document(shell: &Shell) -> bool {
    shell.saving.is_none()
        && shell
            .file
            .as_ref()
            .is_some_and(|file| file.unsaved.is_none())
        && shell
            .editing()
            .is_some_and(|editing| editing.is_described() && !editing.is_dirty())
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
    wait_for_metrics(&shell, cx);
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
                .container
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
    assert_live_metrics_preserve_draft(&shell, &editor, cx);
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

fn wait_for_metrics(shell: &Entity<Shell>, cx: &mut gpui_kit::VisualTestContext) {
    super::super::replace_ui::tests::wait_until(
        cx,
        shell,
        "initial amplitude statistics",
        |shell| {
            shell
                .file
                .as_ref()
                .and_then(|file| file.state.document.file_summary())
                .is_some_and(|summary| !summary.hint.value.contains("Calculating…"))
        },
    );
}

fn assert_live_metrics_preserve_draft(
    shell: &Entity<Shell>,
    editor: &Entity<Editor>,
    cx: &mut gpui_kit::VisualTestContext,
) {
    shell.update_in(cx, |shell, _, cx| {
        shell
            .file
            .as_mut()
            .unwrap()
            .state
            .document
            .set_statistics(Some(Ok(crate::sample_statistics::Metrics {
                peak: 5.,
                rms: 3.,
            })));
        cx.notify();
    });
    frame(cx);
    editor.update_in(cx, |editor, window, cx| {
        assert_eq!(editor.rate.read(cx).value().as_ref(), "12000");
        assert!(editor.rate.read(cx).focus_handle(cx).is_focused(window));
        assert!(
            editor
                .details
                .iter()
                .any(|(label, value)| label == "RMS" && value == "3")
        );
    });
}

fn frame(cx: &mut gpui_kit::VisualTestContext) {
    cx.run_until_parked();
    cx.update(|window, cx| window.render_frame(cx));
    cx.run_until_parked();
}

pub(in crate::shell) fn panel_window(
    cx: &mut TestAppContext,
) -> (Entity<Shell>, &mut gpui_kit::VisualTestContext) {
    cx.executor().allow_parking();
    cx.update(|cx| {
        gpui_kit::init(cx);
        settings_ui::init(cx);
        crate::shell::plot::navigation_ui::init(cx);
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
    wait_for_save_chooser(cx);
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
    let (job, _) = crate::processing::saving::spawn("prepared-conversion-test", |_, _| (), |_| ());
    let file = shell.file.as_ref().unwrap();
    let job = Converting {
        job,
        progress: None,
        document: file.id,
        version: shell.editing().unwrap().version(),
        name: "staged-converted.wav".into(),
        directory: shell
            .save_presentation(&shell.editing().unwrap().file().meta)
            .1,
        output_is_iq: false,
        container: argand_io::write::Container::Wave,
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
            assert_eq!(directory, dir);

            assert_eq!(
                display.source,
                std::path::PathBuf::from("recording-converted.wav")
            );
            shell.save_as(true, window, cx);
        });
        wait_for_save_chooser(cx);
        cx.simulate_new_path_selection(|directory| {
            assert_eq!(directory, dir);
            None
        });
        cx.run_until_parked();
    }
    shell.read_with(cx, |shell, _| {
        assert!(shell.file.as_ref().unwrap().unsaved.is_some())
    });
    std::fs::remove_dir_all(dir).unwrap();
}

fn hover_about(cx: &mut gpui_kit::VisualTestContext) {
    let position = cx.update(|window, _| window.find("about-button").bounds().center());
    cx.simulate_mouse_move(position, None, gpui_kit::Modifiers::default());
    cx.executor()
        .advance_clock(std::time::Duration::from_secs(1));
    frame(cx);
}

#[gpui_kit::test]
fn property_panels_are_mutually_exclusive_for_explicit_commands(cx: &mut TestAppContext) {
    use super::super::replace_ui::tests::{wait_until, write_wave};
    let (shell, cx) = panel_window(cx);
    let path = std::env::temp_dir().join(format!("argand-panel-switch-{}.wav", std::process::id()));
    write_wave(&path, 2400);
    shell.update_in(cx, |shell, window, cx| {
        shell.open(Origin::new(path.clone()), window, cx)
    });
    wait_until(cx, &shell, "switching source", |shell| {
        shell
            .editing()
            .is_some_and(|editing| editing.is_described())
    });
    hover_about(cx);
    assert!(cx.update(|window, _| window.try_find("about-tooltip").is_some()));
    cx.simulate_keystrokes("f11");
    frame(cx);
    {
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
        editor.update_in(cx, |editor, window, cx| {
            editor
                .rate
                .update(cx, |rate, cx| rate.set_value("12000", window, cx));
            window.focus(&editor.kind.read(cx).focus_handle(cx), cx);
        });
        frame(cx);
        cx.simulate_keystrokes("enter");
        frame(cx);
    }
    shell.update_in(cx, |shell, window, cx| {
        shell.open_status_panel(
            super::super::settings_ui::StatusPanel::Analysis,
            false,
            window,
            cx,
        )
    });
    frame(cx);
    assert_property_panels(cx, &shell, false, true);
    hover_about(cx);
    assert!(cx.update(|window, _| window.try_find("about-tooltip").is_none()));
    assert_property_panels(cx, &shell, false, true);
    shell.update(cx, |shell, cx| {
        shell.status_panel_changed(
            super::super::settings_ui::StatusPanel::File,
            hints::Pinned::Closed { revert: false },
            cx,
        );
    });
    assert_property_panels(cx, &shell, false, true);
    shell.update_in(cx, |shell, window, cx| {
        shell.open_status_panel(
            super::super::settings_ui::StatusPanel::File,
            false,
            window,
            cx,
        )
    });
    frame(cx);
    assert_property_panels(cx, &shell, true, false);
    hover_about(cx);
    assert!(cx.update(|window, _| window.try_find("about-tooltip").is_none()));
    assert_property_panels(cx, &shell, true, false);
    shell.read_with(cx, |shell, cx| {
        let editor = shell
            .conversion
            .hint
            .read(cx)
            .view()
            .unwrap()
            .downcast::<Editor>()
            .unwrap();
        assert_eq!(
            editor.read(cx).request(cx).unwrap().output.sample_rate,
            24_000.
        );
        assert_eq!(shell.editing().unwrap().file().meta.sample_rate, 24_000.);
        assert!(shell.conversion.running.is_none());
    });
    cx.simulate_keystrokes(if cfg!(target_os = "macos") {
        "cmd-,"
    } else {
        "ctrl-,"
    });
    frame(cx);
    assert_property_panels(cx, &shell, false, true);
    cx.simulate_keystrokes("f11");
    frame(cx);
    assert_property_panels(cx, &shell, true, false);
    newer_open_supersedes_queued_file_command(cx, &shell);
    std::fs::remove_file(path).unwrap();
}

#[gpui_kit::test]
fn status_triggers_show_help_without_opening_then_click_opens(cx: &mut TestAppContext) {
    use super::super::replace_ui::tests::{wait_until, write_wave};
    let (shell, cx) = panel_window(cx);
    let path = std::env::temp_dir().join(format!("argand-panel-hover-{}.wav", std::process::id()));
    write_wave(&path, 2400);
    shell.update_in(cx, |shell, window, cx| {
        shell.open(Origin::new(path.clone()), window, cx)
    });
    wait_until(cx, &shell, "hover source", |shell| {
        shell
            .editing()
            .is_some_and(|editing| editing.is_described())
    });
    frame(cx);
    let file = cx.update(|window, _| window.find("file-information").bounds().center());
    let analysis = cx.update(|window, _| window.find("analysis-settings").bounds().center());
    for position in [file, analysis, file] {
        cx.simulate_mouse_move(position, None, gpui_kit::Modifiers::default());
        cx.executor()
            .advance_clock(std::time::Duration::from_secs(2));
        frame(cx);
        assert_property_panels(cx, &shell, false, false);
    }
    cx.simulate_mouse_down(analysis, MouseButton::Left, gpui_kit::Modifiers::default());
    cx.simulate_mouse_up(analysis, MouseButton::Left, gpui_kit::Modifiers::default());
    frame(cx);
    assert_property_panels(cx, &shell, false, true);
    for (position, file_open, analysis_open) in [(file, true, false), (analysis, false, true)] {
        cx.simulate_mouse_down(position, MouseButton::Left, gpui_kit::Modifiers::default());
        cx.simulate_mouse_up(position, MouseButton::Left, gpui_kit::Modifiers::default());
        frame(cx);
        assert_property_panels(cx, &shell, file_open, analysis_open);
    }
    let plot = point(px(500.), px(400.));
    cx.simulate_mouse_down(plot, MouseButton::Left, gpui_kit::Modifiers::default());
    cx.simulate_mouse_up(plot, MouseButton::Left, gpui_kit::Modifiers::default());
    frame(cx);
    assert_property_panels(cx, &shell, false, false);
    shell.read_with(cx, |shell, _| {
        assert!(
            shell.selection.is_none(),
            "the backdrop consumed the plot click"
        )
    });
    std::fs::remove_file(path).unwrap();
}

#[gpui_kit::test]
fn shared_frequency_editor_steps_drafts_and_tab_visits_each_value_once(cx: &mut TestAppContext) {
    use super::super::replace_ui::tests::{wait_until, write_wave};
    let (shell, cx) = panel_window(cx);
    let path =
        std::env::temp_dir().join(format!("argand-numeric-draft-{}.wav", std::process::id()));
    write_wave(&path, 2400);
    shell.update_in(cx, |shell, window, cx| {
        shell.open(Origin::new(path.clone()), window, cx)
    });
    wait_until(cx, &shell, "numeric source", |shell| {
        shell
            .editing()
            .is_some_and(|editing| editing.is_described())
    });
    cx.update(|window, _| window.activate_window());
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
    cx.update(|window, cx| {
        let kind = editor.read(cx).kind.clone();
        kind.update(cx, |kind, cx| {
            kind.set_selected_value(&kind_label("rl_f32"), window, cx);
            cx.emit(SelectEvent::Confirm(kind.selected_value().cloned()));
        });
    });
    frame(cx);
    cx.simulate_keystrokes("up");
    frame(cx);
    editor.read_with(cx, |editor, cx| {
        assert_eq!(editor.request(cx).unwrap().output.sample_rate, 48_001.)
    });
    cx.simulate_keystrokes("tab");
    frame(cx);
    cx.update(|window, cx| {
        assert!(
            editor
                .read(cx)
                .reference
                .read(cx)
                .focus_handle(cx)
                .is_focused(window)
        )
    });
    cx.simulate_keystrokes("down");
    frame(cx);
    editor.read_with(cx, |editor, cx| {
        assert_eq!(editor.request(cx).unwrap().carrier, Some(11999.))
    });
    cx.simulate_keystrokes("shift-tab");
    frame(cx);
    cx.update(|window, cx| {
        assert!(
            editor
                .read(cx)
                .rate
                .read(cx)
                .focus_handle(cx)
                .is_focused(window)
        )
    });
    shell.read_with(cx, |shell, _| {
        assert_eq!(shell.editing().unwrap().file().meta.sample_rate, 24_000.);
        assert_eq!(shell.editing().unwrap().file().meta.center_freq, 0.);
        assert!(shell.conversion.running.is_none());
    });
    std::fs::remove_file(path).unwrap();
}

fn newer_open_supersedes_queued_file_command(
    cx: &mut gpui_kit::VisualTestContext,
    shell: &Entity<Shell>,
) {
    cx.update(|window, cx| {
        shell
            .read(cx)
            .conversion
            .hint
            .clone()
            .update(cx, |hint, cx| hint.close(false, cx));
        shell.update(cx, |shell, cx| shell.convert_dialog(window, cx));
        let analysis = shell.read(cx).analysis_hint.clone();
        analysis.update(cx, |hint, cx| hint.open(window, cx));
    });
    frame(cx);
    assert_property_panels(cx, shell, false, true);
}

fn assert_property_panels(
    cx: &gpui_kit::VisualTestContext,
    shell: &Entity<Shell>,
    file: bool,
    analysis: bool,
) {
    shell.read_with(cx, |shell, cx| {
        let owner = if file {
            Some(super::super::settings_ui::StatusPanel::File)
        } else if analysis {
            Some(super::super::settings_ui::StatusPanel::Analysis)
        } else {
            None
        };
        assert_eq!(shell.status_panel, owner, "property panel owner");
        assert_eq!(shell.conversion.hint.read(cx).is_open(), file, "File panel");
        assert_eq!(
            shell.analysis_hint.read(cx).is_open(),
            analysis,
            "Analysis panel"
        );
    });
}

#[gpui_kit::test]
fn an_edit_during_save_directory_preparation_does_not_open_a_stale_chooser(
    cx: &mut TestAppContext,
) {
    let (shell, cx, dir) = converted_fixture(cx, "stale-chooser");
    shell.update_in(cx, |shell, window, cx| {
        shell.save_as(false, window, cx);
        shell
            .editing_mut()
            .unwrap()
            .delete(argand_core::SampleSpan::between(0, 100).unwrap());
    });
    cx.run_until_parked();
    assert!(!cx.did_prompt_for_new_path());
    shell.update_in(cx, |shell, window, cx| shell.save_as(false, window, cx));
    wait_for_save_chooser(cx);
    cx.simulate_new_path_selection(|_| None);
    shell.read_with(cx, |shell, _| {
        assert!(shell.editing().unwrap().is_dirty());
        assert!(shell.file.as_ref().unwrap().unsaved.is_some());
    });
    std::fs::remove_dir_all(dir).unwrap();
}

#[gpui_kit::test]
fn opening_a_real_source_discards_old_carrier_drafts_and_derives_reverse_defaults(
    cx: &mut TestAppContext,
) {
    use super::super::replace_ui::tests::{wait_until, write_wave};
    let (shell, cx) = panel_window(cx);
    let dir = argand_io::testutil::TempDir::new("carrier-new-source");
    let iq_path = dir.join("iq.wav");
    write_wave(&iq_path, 2400);
    shell.update_in(cx, |shell, window, cx| {
        shell.open(Origin::new(iq_path), window, cx)
    });
    wait_until(cx, &shell, "IQ source", |shell| {
        shell
            .editing()
            .is_some_and(|editing| editing.is_described())
    });
    cx.simulate_keystrokes("f11");
    frame(cx);
    let old = panel_editor(&shell, cx);
    old.update_in(cx, |editor, window, cx| {
        editor.kind.update(cx, |kind, cx| {
            kind.set_selected_value(&kind_label("rl_f32"), window, cx);
            cx.emit(SelectEvent::Confirm(kind.selected_value().cloned()));
        });
    });
    frame(cx);
    old.update_in(cx, |editor, window, cx| {
        editor
            .reference
            .update(cx, |input, cx| input.set_value("1000", window, cx))
    });
    let path = argand_io::testutil::write_wav(
        &dir.join("real.wav"),
        "rl_f32".parse().unwrap(),
        48000,
        &vec![0.25; 2400],
        1.,
    );
    shell.update_in(cx, |shell, window, cx| {
        shell.open(Origin::new(path), window, cx)
    });
    wait_until(cx, &shell, "real source", |shell| {
        shell
            .editing()
            .is_some_and(|editing| editing.is_described() && !editing.file().meta.is_iq())
    });
    assert!(!shell.read_with(cx, |shell, cx| shell.conversion.hint.read(cx).is_open()));
    cx.simulate_keystrokes("f11");
    frame(cx);
    let current = panel_editor(&shell, cx);
    current.update_in(cx, |editor, window, cx| {
        editor.kind.update(cx, |kind, cx| {
            kind.set_selected_value(&kind_label("iq_f32"), window, cx);
            cx.emit(SelectEvent::Confirm(kind.selected_value().cloned()));
        });
    });
    frame(cx);
    current.read_with(cx, |editor, cx| {
        let request = editor.request(cx).unwrap();
        assert_eq!(request.output.sample_rate, 24000.);
        assert_eq!(request.output.center_freq, 0.);
        assert_eq!(
            editor.reference.read(cx).value().as_ref(),
            crate::numbers::number(12000.)
        );
        assert!(editor.summary(cx).unwrap().is_empty());
    });
}

fn panel_editor(shell: &Entity<Shell>, cx: &gpui_kit::VisualTestContext) -> Entity<Editor> {
    shell.read_with(cx, |shell, cx| {
        shell
            .conversion
            .hint
            .read(cx)
            .view()
            .unwrap()
            .downcast::<Editor>()
            .unwrap()
    })
}

#[gpui_kit::test]
fn exact_carrier_defaults_do_not_warn_from_rounded_physical_references(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    for (rate, reference) in [(24000., 1e20), (8000.15, 1e10)] {
        let mut source = request();
        source.input.sample_rate = rate;
        source.input.center_freq = reference;
        source.output = source.input.clone();
        let mut editor = None;
        let window = cx.add_window(|window, cx| {
            let form = cx.new(|cx| Editor::new(source, window, cx));
            editor = Some(form.clone());
            Root::new(form, window, cx)
        });
        let editor = editor.unwrap();
        window
            .update(cx, |_, window, cx| {
                editor.read(cx).kind.clone().update(cx, |kind, cx| {
                    kind.set_selected_value(&kind_label("rl_f32"), window, cx);
                    cx.emit(SelectEvent::Confirm(kind.selected_value().cloned()));
                });
            })
            .unwrap();
        cx.run_until_parked();
        editor.read_with(cx, |editor, cx| {
            let draft = editor.request(cx).unwrap();
            assert_eq!(draft.carrier, Some(rate / 2.));
            assert_eq!(draft.output.sample_rate, rate * 2.);
            assert!(draft.plan().unwrap().retains_input_band());
            assert!(editor.summary(cx).unwrap().is_empty());
        });
    }
}

#[cfg(target_os = "linux")]
#[gpui_kit::test]
fn caption_close_with_open_sample_list_reaches_unsaved_workflow(cx: &mut TestAppContext) {
    use super::super::replace_ui::tests::{wait_until, write_wave};
    use gpui_kit::component::WindowExt;
    let (shell, cx) = panel_window(cx);
    let dir = argand_io::testutil::TempDir::new("caption-select-close");
    let path = dir.join("input.wav");
    write_wave(&path, 2400);
    shell.update_in(cx, |shell, window, cx| {
        shell.open(Origin::new(path), window, cx)
    });
    wait_until(cx, &shell, "caption source", |shell| {
        shell
            .editing()
            .is_some_and(|editing| editing.is_described())
    });
    shell.update_in(cx, |shell, window, cx| {
        shell.invert_spectrum(window, cx);
        shell.convert_dialog(window, cx);
    });
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
    let trigger = editor.read_with(cx, |editor, cx| editor.kind.read(cx).focus_handle(cx));
    cx.update(|window, cx| window.focus(&trigger, cx));
    cx.simulate_keystrokes("enter");
    frame(cx);
    assert_ne!(
        editor.read_with(cx, |editor, cx| editor.kind.read(cx).focus_handle(cx)),
        trigger
    );
    let close = cx.update(|window, _| {
        let bounds = chrome::overlay_area(window);
        let half = gpui_kit::component::TITLE_BAR_HEIGHT / 2.;
        point(bounds.right() - half, bounds.top() + half)
    });
    let outside = point(close.x - px(100.), close.y + px(80.));
    cx.simulate_mouse_move(close, None, gpui_kit::Modifiers::default());
    frame(cx);
    cx.simulate_mouse_down(close, MouseButton::Left, gpui_kit::Modifiers::default());
    cx.simulate_mouse_move(
        outside,
        Some(MouseButton::Left),
        gpui_kit::Modifiers::default(),
    );
    cx.simulate_mouse_up(outside, MouseButton::Left, gpui_kit::Modifiers::default());
    frame(cx);
    assert!(!cx.update(|window, cx| window.has_active_dialog(cx)));
    assert!(shell.read_with(cx, |shell, _| shell.editing().unwrap().is_dirty()));
    shell.update_in(cx, |shell, window, cx| shell.convert_dialog(window, cx));
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
    let trigger = editor.read_with(cx, |editor, cx| editor.kind.read(cx).focus_handle(cx));
    cx.update(|window, cx| window.focus(&trigger, cx));
    cx.simulate_keystrokes("enter");
    frame(cx);
    assert_ne!(
        editor.read_with(cx, |editor, cx| editor.kind.read(cx).focus_handle(cx)),
        trigger
    );
    cx.simulate_mouse_move(close, None, gpui_kit::Modifiers::default());
    frame(cx);
    cx.simulate_mouse_down(close, MouseButton::Left, gpui_kit::Modifiers::default());
    cx.simulate_mouse_up(close, MouseButton::Left, gpui_kit::Modifiers::default());
    frame(cx);
    assert!(
        cx.update(|window, cx| window.has_active_dialog(cx)),
        "one caption click must ask about the unsaved capture"
    );
    cx.simulate_keystrokes("escape");
    frame(cx);
    assert!(shell.read_with(cx, |shell, _| shell.editing().unwrap().is_dirty()));
}

#[gpui_kit::test]
fn cancellation_rejects_success_already_queued_by_the_worker(cx: &mut TestAppContext) {
    let (shell, cx, dir) = converted_fixture(cx, "cancel-queued-success");
    shell.update_in(cx, |shell, window, cx| {
        let file = shell.file.as_ref().unwrap();
        let document = file.id;
        let editing = shell.editing().unwrap();
        let version = editing.version();
        let capture = editing.capture().clone();
        let backing = file.unsaved.as_ref().unwrap().backing.clone();
        let path = editing.file().meta.source.clone();
        let samples = editing.len();
        let (job, _) = crate::processing::saving::spawn("queued-success-test", |_, _| (), |_| ());
        shell.conversion.running = Some(Converting {
            job,
            progress: None,
            document,
            version,
            name: "must-not-adopt.wav".into(),
            directory: dir.clone(),
            output_is_iq: true,
            container: argand_io::write::Container::Wave,
            backing,
        });
        let queued = Update::Finished(Ended::Done(crate::processing::conversion::Converted {
            saved: argand_io::write::Saved {
                path,
                samples,
                container: "wav",
                stamp: None,
            },
            clipped: 0,
        }));
        shell.cancel_conversion(window, cx);
        shell.receive_conversion(queued, window, cx);
        assert!(!shell.conversion_active());
        assert!(shell.conversion.after_cancel.is_none());
        assert_eq!(shell.file.as_ref().unwrap().id, document);
        let editing = shell.editing().unwrap();
        assert_eq!(editing.version(), version);
        assert_eq!(editing.capture(), &capture);
        assert!(editing.is_dirty());
        assert!(shell.file.as_ref().unwrap().unsaved.is_some());
    });
    std::fs::remove_dir_all(dir).unwrap();
}

#[gpui_kit::test]
fn conversion_blocks_document_actions_and_close_waits_before_unsaved_prompt(
    cx: &mut TestAppContext,
) {
    let (shell, cx, dir) = converted_fixture(cx, "busy-actions");
    shell.update_in(cx, |shell, window, cx| {
        shell.save_as(false, window, cx);
        let file = shell.file.as_ref().unwrap();
        let document = file.id;
        let version = shell.editing().unwrap().version();
        let (job, _) = crate::processing::saving::spawn("busy-conversion-test", |_, _| (), |_| ());
        shell.conversion.running = Some(Converting {
            job,
            progress: None,
            document,
            version,
            name: "busy-converted.wav".into(),
            directory: dir.clone(),
            output_is_iq: true,
            container: argand_io::write::Container::Wave,
            backing: file.unsaved.as_ref().unwrap().backing.clone(),
        });
        shell.open(Origin::new(dir.join("other.wav")), window, cx);
        shell.choose_file(window, cx);
        shell.invert_spectrum(window, cx);
        shell.normalize(window, cx);
        window.dispatch_action(Box::new(editing_ui::Undo), cx);
        window.dispatch_action(Box::new(editing_ui::DeleteSelection), cx);
        window.dispatch_action(Box::new(editing_ui::PasteHere), cx);
        shell.save_as(false, window, cx);
        shell.open_application_menu(window, cx);
        shell.convert_dialog(window, cx);
        shell.edit_analysis(&EditAnalysis, window, cx);
        shell.toggle_selection_panel(window, cx);
        assert_eq!(shell.file.as_ref().unwrap().id, document);
        assert_eq!(shell.editing().unwrap().version(), version);
        assert!(!shell.can_save(false));
        assert!(shell.editing_mut().is_none());
        assert!(shell.cursor_readout(cx).is_none());
        assert!(shell.application_menu.is_none());
        assert!(!shell.conversion.hint.read(cx).is_open());
        assert!(!shell.analysis_hint.read(cx).is_open());
        assert!(!shell.selection_hint.read(cx).is_open());
        shell.request_close(window, cx);
        assert!(shell.conversion.after_cancel == Some(AfterCancel::Close));
        assert!(shell.file.as_ref().unwrap().unsaved.is_some());
    });
    cx.run_until_parked();
    assert!(
        !cx.did_prompt_for_new_path(),
        "queued chooser must not open during conversion"
    );
    assert!(
        !cx.has_pending_prompt(),
        "closing waits for the conversion worker"
    );
    shell.update_in(cx, |shell, window, cx| {
        shell.receive_conversion(Update::Finished(Ended::Cancelled), window, cx);
    });
    cx.run_until_parked();
    assert!(
        cx.has_pending_prompt(),
        "one close request resumes normal unsaved protection"
    );
    cx.simulate_prompt_answer("Cancel");
    cx.run_until_parked();
    shell.read_with(cx, |shell, _| {
        assert!(shell.file.as_ref().unwrap().unsaved.is_some());
        assert!(shell.editing().unwrap().is_dirty());
    });
    std::fs::remove_dir_all(dir).unwrap();
}

fn sample_row(token: &str) -> gpui_kit::SharedString {
    format!("sample-format-{}", kind_label(token)).into()
}

fn assert_flac_trigger_fits(cx: &mut gpui_kit::VisualTestContext) {
    cx.update(|window, cx| {
        let input = window.within("conversion-container").find("input").bounds();
        let text = "FLAC";
        let style = gpui_kit::TextStyle {
            font_family: cx.theme().font_family.clone(),
            ..Default::default()
        };
        let glyphs = window
            .text_system()
            .shape_line(
                text.into(),
                window.rem_size() * 0.875,
                &[style.to_run(text.len())],
                None,
            )
            .width;
        let reserved = px(18.) + window.rem_size() * (0.25 + 0.875);
        assert!(
            input.size.width >= glyphs + reserved,
            "the rendered trigger must fit FLAC plus stock padding, borders, gap and caret"
        );
    });
}

fn choose_container(editor: &Entity<Editor>, cx: &mut gpui_kit::VisualTestContext, name: &str) {
    editor.update_in(cx, |editor, window, cx| {
        editor.container.update(cx, |state, cx| {
            state.set_selected_value(&name.to_owned(), window, cx);
            cx.emit(SelectEvent::Confirm(state.selected_value().cloned()));
        });
    });
    frame(cx);
}

#[gpui_kit::test]
fn sample_choices_fit_and_flac_filters_without_resetting_drafts(cx: &mut TestAppContext) {
    cx.update(|cx| {
        gpui_kit::init(cx);
        settings_ui::init(cx);
    });
    for rem in [16., 24.] {
        let mut editor = None;
        let (_, cx) = cx.add_window_view(|window, cx| {
            window.set_rem_size(px(rem));
            let form = cx.new(|cx| Editor::new(request(), window, cx));
            editor = Some(form.clone());
            Root::new(form, window, cx)
        });
        cx.simulate_resize(size(px(1000.), px(1000.)));
        let editor = editor.unwrap();
        editor.update_in(cx, |editor, window, cx| {
            editor.focus_on_open = false;
            window.focus(&editor.kind.read(cx).focus_handle(cx), cx);
        });
        frame(cx);
        cx.simulate_keystrokes("enter");
        frame(cx);
        let last = cx.update(|window, _| {
            let first = window.find(sample_row("rl_u8")).bounds();
            let last = window.find(sample_row("iq_f16x8")).bounds();
            assert!(first.top() >= px(0.) && last.bottom() <= window.viewport_size().height);
            last.center()
        });
        cx.simulate_mouse_move(last, None, gpui_kit::Modifiers::default());
        cx.simulate_mouse_down(last, MouseButton::Left, gpui_kit::Modifiers::default());
        cx.simulate_mouse_up(last, MouseButton::Left, gpui_kit::Modifiers::default());
        frame(cx);
        assert_eq!(
            editor.read_with(cx, |editor, cx| editor.kind_value(cx).map(str::to_owned)),
            Some(kind_label("iq_f16x8"))
        );
        editor.update_in(cx, |editor, window, cx| {
            editor.kind.update(cx, |state, cx| {
                state.set_selected_value(&kind_label("iq_f32"), window, cx);
                cx.emit(SelectEvent::Confirm(state.selected_value().cloned()));
            });
            editor.container.update(cx, |state, cx| {
                state.set_selected_value(&"FLAC".into(), window, cx);
                cx.emit(SelectEvent::Confirm(state.selected_value().cloned()));
            });
            editor
                .rate
                .update(cx, |state, cx| state.set_value("12345", window, cx));
        });
        frame(cx);
        assert_flac_trigger_fits(cx);
        assert_eq!(
            editor.read_with(cx, |editor, cx| editor.kind_value(cx).map(str::to_owned)),
            Some(kind_label("iq_i16"))
        );
        assert!(editor.read_with(cx, |editor, cx| editor.request(cx).is_ok()));
        choose_container(&editor, cx, "WAV");
        assert_eq!(
            editor.read_with(cx, |editor, cx| editor
                .kind
                .read(cx)
                .selected_value()
                .cloned()),
            Some(kind_label("iq_i16"))
        );
        choose_container(&editor, cx, "FLAC");
        cx.update(|window, cx| window.focus(&editor.read(cx).kind.read(cx).focus_handle(cx), cx));
        cx.simulate_keystrokes("enter");
        frame(cx);
        let target = cx.update(|window, _| {
            for token in [
                "rl_i32", "rl_f32", "rl_f16x8", "iq_i32", "iq_f32", "iq_f16x8",
            ] {
                assert!(window.try_find(sample_row(token)).is_none());
            }
            window.find(sample_row("iq_i16")).bounds().center()
        });
        cx.simulate_mouse_move(target, None, gpui_kit::Modifiers::default());
        cx.simulate_mouse_down(target, MouseButton::Left, gpui_kit::Modifiers::default());
        cx.simulate_mouse_up(target, MouseButton::Left, gpui_kit::Modifiers::default());
        frame(cx);
        editor.read_with(cx, |editor, cx| {
            assert_eq!(editor.kind_value(cx), Some(kind_label("iq_i16").as_str()));
            assert_eq!(editor.rate.read(cx).value().as_ref(), "12345");
            assert_eq!(
                editor.reference.read(cx).value().as_ref(),
                crate::numbers::number(100_000.)
            );
        });
        cx.simulate_keystrokes("enter up enter");
        frame(cx);
        assert_eq!(
            editor.read_with(cx, |editor, cx| editor.kind_value(cx).map(str::to_owned)),
            Some(kind_label("iq_u8"))
        );
    }
}

#[gpui_kit::test]
fn format_tooltip_disappears_for_open_select_and_returns_after_close(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let mut editor = None;
    let (_, cx) = cx.add_window_view(|window, cx| {
        let form = cx.new(|cx| Editor::new(request(), window, cx));
        editor = Some(form.clone());
        Root::new(form, window, cx)
    });
    cx.simulate_resize(size(px(1000.), px(900.)));
    let editor = editor.unwrap();
    editor.update_in(cx, |editor, _, _| editor.focus_on_open = false);
    frame(cx);
    let long = cx.update(|window, _| window.within("conversion-kind").find("input").bounds());
    let at = long.center();
    let modifiers = gpui_kit::Modifiers::default();
    cx.simulate_mouse_move(at, None, modifiers);
    cx.executor()
        .advance_clock(std::time::Duration::from_secs(1));
    frame(cx);
    cx.update(|window, _| {
        let tip = window.find("metadata-tooltip-content").bounds();
        assert!(
            tip.right() <= window.viewport_size().width
                && tip.bottom() <= window.viewport_size().height
        );
    });
    cx.simulate_mouse_down(at, MouseButton::Left, modifiers);
    cx.simulate_mouse_up(at, MouseButton::Left, modifiers);
    frame(cx);
    assert!(editor.read_with(cx, |editor, cx| presentation::select_open(
        &editor.kind,
        &editor.closed_selects[1],
        cx
    )));
    cx.update(|window, _| {
        assert!(
            window.try_find("metadata-tooltip-content").is_none(),
            "already visible tooltip must disappear"
        );
        assert!(window.find(sample_row("iq_i16")).visible());
    });
    let selected = cx.update(|window, _| window.find(sample_row("iq_i16")).bounds().center());
    cx.simulate_mouse_move(selected, None, modifiers);
    cx.simulate_mouse_down(selected, MouseButton::Left, modifiers);
    cx.simulate_mouse_up(selected, MouseButton::Left, modifiers);
    frame(cx);
    let at = cx.update(|window, _| {
        let short = window.within("conversion-kind").find("input").bounds();
        assert!(
            short.size.width < long.size.width,
            "short selected titles must not reserve the longest option width"
        );
        assert!(
            (short.right() - long.right()).abs() < px(1.),
            "selected values keep their right-column anchor"
        );
        short.center()
    });
    cx.simulate_mouse_move(gpui_kit::point(px(900.), px(800.)), None, modifiers);
    cx.simulate_mouse_move(at, None, modifiers);
    cx.executor()
        .advance_clock(std::time::Duration::from_secs(1));
    frame(cx);
    cx.update(|window, _| assert!(window.find("metadata-tooltip-content").visible()));
}

#[gpui_kit::test]
fn flac_fallback_preserves_domain_and_frequency_draft(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    for token in argand_core::SAMPLE_TYPE_TOKENS {
        let mut form = None;
        let (_, cx) = cx.add_window_view(|window, cx| {
            let editor = cx.new(|cx| Editor::new(request(), window, cx));
            form = Some(editor.clone());
            Root::new(editor, window, cx)
        });
        let form = form.unwrap();
        form.update_in(cx, |editor, window, cx| {
            editor.kind.update(cx, |state, cx| {
                state.set_selected_value(&kind_label(token), window, cx);
                cx.emit(SelectEvent::Confirm(state.selected_value().cloned()));
            });
        });
        frame(cx);
        form.update_in(cx, |editor, window, cx| {
            editor
                .rate
                .update(cx, |input, cx| input.set_value("12345", window, cx));
            editor
                .reference
                .update(cx, |input, cx| input.set_value("1800", window, cx));
        });
        choose_container(&form, cx, "FLAC");
        form.read_with(cx, |editor, cx| {
            assert_eq!(editor.rate.read(cx).value().as_ref(), "12345");
            assert_eq!(editor.reference.read(cx).value().as_ref(), "1800");
            let supported = token.ends_with("_u8") || token.ends_with("_i16");
            let expected = if supported {
                token
            } else if token.starts_with("iq") {
                "iq_i16"
            } else {
                "rl_i16"
            };
            assert_eq!(editor.kind_value(cx), Some(kind_label(expected).as_str()));
            assert_eq!(editor.format_warning, !supported);
            assert!(editor.request(cx).is_ok());
        });
        choose_container(&form, cx, "FLAC");
        form.read_with(cx, |editor, cx| {
            assert!(!editor.format_warning);
            assert!(editor.kind.read(cx).selected_value().is_some());
            assert!(editor.request(cx).is_ok());
        });
        choose_container(&form, cx, "WAV");
        form.read_with(cx, |editor, cx| {
            assert!(!editor.format_warning);
            assert!(!editor.kind_value(cx).unwrap().contains("Float"));
        });
    }
}

fn choose_kind(form: &Entity<Editor>, cx: &mut gpui_kit::VisualTestContext, token: &str) {
    form.update_in(cx, |editor, window, cx| {
        editor.kind.update(cx, |state, cx| {
            state.set_selected_value(&kind_label(token), window, cx);
            cx.emit(SelectEvent::Confirm(state.selected_value().cloned()));
        });
    });
    frame(cx);
}

fn assert_advice_fallback(window: &Window) {
    let tip = window.find("metadata-tooltip-content").bounds();
    assert!(tip.right() <= window.viewport_size().width);
    assert!(tip.bottom() <= window.viewport_size().height);
    assert!(window.try_find("conversion-format-balloon").is_none());
}

fn exercise_reopened_advice(
    cx: &mut gpui_kit::VisualTestContext,
    anchor: gpui_kit::Point<gpui_kit::Pixels>,
) {
    let modifiers = gpui_kit::Modifiers::default();
    let (close, geometry, pointer_geometry) = cx.update(|window, _| {
        let balloon = window.find("conversion-format-balloon").bounds();
        assert!(balloon.right() <= window.viewport_size().width);
        let pointer = window.find("conversion-format-balloon-pointer").bounds();
        let trigger = window.find("conversion-format-advice").bounds();
        assert_eq!(pointer.left(), trigger.right() + px(3.));
        assert_eq!(balloon.left(), pointer.right() - px(1.));
        assert!(window.try_find("metadata-tooltip-content").is_none());
        (
            window
                .find("conversion-format-balloon-close")
                .bounds()
                .center(),
            balloon,
            pointer,
        )
    });
    cx.simulate_mouse_move(point(anchor.x + px(4.), anchor.y), None, modifiers);
    frame(cx);
    cx.update(|window, _| assert!(window.find("conversion-format-balloon").visible()));
    cx.simulate_mouse_move(close, None, modifiers);
    frame(cx);
    cx.update(|window, _| assert!(window.find("conversion-format-balloon").visible()));
    cx.simulate_mouse_down(close, MouseButton::Left, modifiers);
    cx.simulate_mouse_up(close, MouseButton::Left, modifiers);
    frame(cx);
    cx.update(|window, _| assert!(window.try_find("conversion-format-balloon").is_none()));
    cx.simulate_mouse_move(point(px(10.), px(10.)), None, modifiers);
    frame(cx);
    cx.simulate_mouse_move(anchor, None, modifiers);
    frame(cx);
    cx.update(|window, _| {
        assert_eq!(window.find("conversion-format-balloon").bounds(), geometry);
        assert_eq!(
            window.find("conversion-format-balloon-pointer").bounds(),
            pointer_geometry
        );
    });
    cx.simulate_mouse_move(point(px(10.), px(10.)), None, modifiers);
    frame(cx);
    cx.update(|window, _| assert!(window.try_find("conversion-format-balloon").is_none()));
    cx.simulate_mouse_move(anchor, None, modifiers);
    frame(cx);
    cx.update(|window, _| {
        assert_eq!(window.find("conversion-format-balloon").bounds(), geometry);
        assert_eq!(
            window.find("conversion-format-balloon-pointer").bounds(),
            pointer_geometry
        );
    });
}

#[gpui_kit::test]
fn format_advice_reuses_range_close_hover_and_list_suppression(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    for width in [1000., 600.] {
        let mut form = None;
        let (_, cx) = cx.add_window_view(|window, cx| {
            let editor = cx.new(|cx| Editor::new(request(), window, cx));
            form = Some(editor.clone());
            Root::new(editor, window, cx)
        });
        cx.simulate_resize(size(px(width), px(900.)));
        let form = form.unwrap();
        form.update_in(cx, |editor, _, _| editor.focus_on_open = false);
        frame(cx);
        let before = cx.update(|window, _| window.find("convert-capture").bounds().top());
        choose_container(&form, cx, "FLAC");
        frame(cx);
        let anchor = cx.update(|window, _| {
            assert_eq!(
                window.find("convert-capture").bounds().top(),
                before,
                "advice adds no inline row"
            );
            let balloon = window.try_find("conversion-format-balloon");
            assert_eq!(
                balloon.is_some(),
                width == 1000.,
                "automatic advice needs a readable side"
            );
            if let Some(balloon) = balloon {
                assert!(balloon.bounds().right() <= window.viewport_size().width);
            }
            {
                let bounds = window.find("conversion-format-advice").bounds();
                point(bounds.right() - px(3.), bounds.center().y)
            }
        });
        if width == 1000. {
            cx.update(|window, cx| window.click("conversion-format-balloon-close", cx));
            frame(cx);
            frame(cx);
            cx.update(|window, _| {
                assert!(
                    window.try_find("conversion-format-balloon").is_none(),
                    "close must not immediately reopen"
                );
                assert!(window.find("conversion-format-advice").visible());
            });
        }
        let modifiers = gpui_kit::Modifiers::default();
        cx.simulate_mouse_move(point(px(10.), px(10.)), None, modifiers);
        frame(cx);
        cx.simulate_mouse_move(anchor, None, modifiers);
        cx.executor()
            .advance_clock(std::time::Duration::from_secs(1));
        frame(cx);
        if width == 1000. {
            exercise_reopened_advice(cx, anchor);
        } else {
            cx.update(|window, _| assert_advice_fallback(window));
        }
        let input = cx.update(|window, _| {
            window
                .within("conversion-kind")
                .find("input")
                .bounds()
                .center()
        });
        cx.simulate_mouse_move(input, None, modifiers);
        cx.simulate_mouse_down(input, MouseButton::Left, modifiers);
        cx.simulate_mouse_up(input, MouseButton::Left, modifiers);
        frame(cx);
        cx.update(|window, _| {
            assert!(window.try_find("metadata-tooltip-content").is_none());
            assert!(window.try_find("conversion-format-balloon").is_none());
            assert!(window.find(sample_row("iq_u8")).visible());
        });
        cx.simulate_keystrokes("escape");
        frame(cx);
        cx.update(|window, _| assert!(window.try_find("conversion-format-balloon").is_none()));
        choose_kind(&form, cx, "iq_u8");
        cx.update(|window, _| {
            assert!(
                window.try_find("conversion-format-advice").is_none(),
                "explicit choice clears the marker"
            )
        });
        choose_container(&form, cx, "WAV");
        choose_kind(&form, cx, "iq_i32");
        choose_container(&form, cx, "FLAC");
        frame(cx);
        cx.update(|window, _| {
            assert_eq!(
                window.try_find("conversion-format-balloon").is_some(),
                width == 1000.,
                "a new fallback resets dismissal"
            )
        });
    }
}

#[gpui_kit::test]
fn external_tooltips_are_cancelled_on_panel_open_and_do_not_resurrect(cx: &mut TestAppContext) {
    let (shell, cx) = panel_window(cx);
    cx.update(|window, _| window.activate_window());
    frame(cx);
    let position = cx.update(|window, _| window.find("about-button").bounds().center());
    for wait in [200, 1000] {
        cx.simulate_mouse_move(
            point(px(500.), px(400.)),
            None,
            gpui_kit::Modifiers::default(),
        );
        frame(cx);
        cx.simulate_mouse_move(position, None, gpui_kit::Modifiers::default());
        cx.executor()
            .advance_clock(std::time::Duration::from_millis(wait));
        frame(cx);
        if wait == 1000 {
            assert!(cx.update(|window, _| window.try_find("about-tooltip").is_some()));
        }
        shell.update_in(cx, |shell, window, cx| {
            shell.show_about(&super::super::about_ui::ShowAbout, window, cx);
        });
        frame(cx);
        assert!(cx.update(|window, _| window.try_find("about-tooltip").is_none()));
        shell.update(cx, |shell, cx| shell.close_about(cx));
        frame(cx);
        cx.executor()
            .advance_clock(std::time::Duration::from_secs(2));
        frame(cx);
        assert!(
            cx.update(|window, _| window.try_find("about-tooltip").is_none()),
            "closing under a resting pointer must not resurrect visible or pending help"
        );
        cx.simulate_mouse_move(position, None, gpui_kit::Modifiers::default());
        cx.executor()
            .advance_clock(std::time::Duration::from_secs(1));
        frame(cx);
        assert!(
            cx.update(|window, _| window.try_find("about-tooltip").is_some()),
            "the first fresh motion after panel close can show help again"
        );
    }
    cx.update(|window, _| window.activate_window());
    frame(cx);
    cx.deactivate_window();
    frame(cx);
    assert!(cx.update(|window, _| window.try_find("about-tooltip").is_none()));
    cx.update(|window, _| window.activate_window());
    frame(cx);
    cx.executor()
        .advance_clock(std::time::Duration::from_secs(2));
    frame(cx);
    assert!(cx.update(|window, _| window.try_find("about-tooltip").is_none()));
}

#[gpui_kit::test]
fn deactivation_closes_both_property_panels_without_reopening(cx: &mut TestAppContext) {
    use super::super::replace_ui::tests::{wait_until, write_wave};
    let (shell, cx) = panel_window(cx);
    let path = std::env::temp_dir().join(format!(
        "argand-panel-deactivate-{}.wav",
        std::process::id()
    ));
    write_wave(&path, 2400);
    shell.update_in(cx, |shell, window, cx| {
        shell.open(Origin::new(path.clone()), window, cx)
    });
    wait_until(cx, &shell, "deactivation source", |shell| {
        shell
            .editing()
            .is_some_and(|editing| editing.is_described())
    });
    let analysis_key = if cfg!(target_os = "macos") {
        "cmd-,"
    } else {
        "ctrl-,"
    };
    for (key, file, analysis) in [("f11", true, false), (analysis_key, false, true)] {
        cx.update(|window, _| window.activate_window());
        cx.simulate_keystrokes(key);
        frame(cx);
        assert_property_panels(cx, &shell, file, analysis);
        cx.deactivate_window();
        frame(cx);
        assert_property_panels(cx, &shell, false, false);
        cx.update(|window, _| window.activate_window());
        cx.executor()
            .advance_clock(std::time::Duration::from_secs(2));
        frame(cx);
        assert_property_panels(cx, &shell, false, false);
    }
    std::fs::remove_file(path).unwrap();
}

#[gpui_kit::test]
fn rapid_reactivation_cancels_pending_and_visible_help_before_fresh_motion(
    cx: &mut TestAppContext,
) {
    let (shell, cx) = panel_window(cx);
    cx.update(|window, _| window.activate_window());
    frame(cx);
    let position = cx.update(|window, _| window.find("about-button").bounds().center());
    for wait in [200, 1000] {
        cx.simulate_mouse_move(
            point(px(500.), px(400.)),
            None,
            gpui_kit::Modifiers::default(),
        );
        frame(cx);
        cx.simulate_mouse_move(position, None, gpui_kit::Modifiers::default());
        cx.executor()
            .advance_clock(std::time::Duration::from_millis(wait));
        frame(cx);
        if wait == 1000 {
            assert!(cx.update(|window, _| window.try_find("about-tooltip").is_some()));
        }
        // deactivate_window drains scheduled draws. Exercise the same suspension
        // and the active callback atomically, before any prepaint can retire help.
        shell.update_in(cx, |shell, window, cx| {
            shell.suppress_external_tooltips();
            shell.activation_changed(window, cx);
            assert_eq!(shell.external_help_cleared_epoch, None);
            shell.pointer_moved(cx);
            assert!(
                shell.external_help_suppressed,
                "motion before teardown must not release stale native state"
            );
        });
        frame(cx);
        cx.executor()
            .advance_clock(std::time::Duration::from_secs(2));
        frame(cx);
        assert!(cx.update(|window, _| window.try_find("about-tooltip").is_none()));
        cx.simulate_mouse_move(position, None, gpui_kit::Modifiers::default());
        cx.executor()
            .advance_clock(std::time::Duration::from_secs(1));
        frame(cx);
        assert!(
            cx.update(|window, _| window.try_find("about-tooltip").is_some()),
            "the first fresh motion after teardown restores normal help"
        );
    }
}

#[gpui_kit::test]
fn an_old_teardown_acknowledgement_cannot_clear_a_new_suppression(cx: &mut TestAppContext) {
    let (shell, cx) = panel_window(cx);
    frame(cx);
    cx.update(|window, cx| {
        shell.update(cx, |shell, _| shell.suppress_external_tooltips());
        Shell::acknowledge_tooltip_teardown(shell.downgrade(), window, cx);
        shell.update(cx, |shell, _| shell.suppress_external_tooltips());
    });
    assert_eq!(
        shell.read_with(cx, |shell, _| shell.external_help_cleared_epoch),
        None
    );
    frame(cx);
    shell.read_with(cx, |shell, _| {
        assert_eq!(
            shell.external_help_cleared_epoch,
            Some(shell.external_help_epoch)
        )
    });
}
