use super::*;
use gpui_kit::TestAppContext;
use gpui_kit::test::TestWindowExt;
use std::path::{Path, PathBuf};

pub(in crate::shell) fn open_window(
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
    let (shell, cx) = cx.add_window_view(|window, cx| {
        Shell::new(Config::default(), None, Session::default(), window, cx)
    });
    cx.simulate_resize(gpui_kit::size(px(800.), px(600.)));
    (shell, cx)
}

/// A 16-bit I/Q WAVE file whose sample `n` holds `n` in both channels.
pub(in crate::shell) fn write_wave(path: &Path, samples: u32) {
    let data_len = samples * 4;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_len).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&2u16.to_le_bytes());
    bytes.extend_from_slice(&24_000u32.to_le_bytes());
    bytes.extend_from_slice(&96_000u32.to_le_bytes());
    bytes.extend_from_slice(&4u16.to_le_bytes());
    bytes.extend_from_slice(&16u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_len.to_le_bytes());
    for n in 0..samples {
        let value = (n % 30_000) as i16;
        bytes.extend_from_slice(&value.to_le_bytes());
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    std::fs::write(path, bytes).unwrap();
}

/// Draw and let the real threads work until `done` holds, or fail after a while.
pub(in crate::shell) fn wait_for_save_chooser(cx: &mut gpui_kit::VisualTestContext) {
    for _ in 0..1000 {
        cx.run_until_parked();
        if cx.did_prompt_for_new_path() {
            return;
        }
    }
    panic!("save chooser did not open");
}

pub(in crate::shell) fn wait_until(
    cx: &mut gpui_kit::VisualTestContext,
    shell: &Entity<Shell>,
    what: &str,
    done: impl Fn(&Shell) -> bool,
) {
    for _ in 0..1000 {
        cx.run_until_parked();
        cx.update(|window, cx| {
            window.render_frame(cx);
            window.simulate_next_frame(cx);
        });
        if shell.read_with(cx, |shell, _| done(shell)) {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let state = shell.read_with(cx, |shell, _| {
        let file = shell.file.as_ref();
        format!(
            "status {:?}, editing {}, stamp {:?}",
            file.map(|file| file.state.document.status().clone()),
            file.is_some_and(|file| file.editing.is_some()),
            file.and_then(|file| file.state.document.stamp()).is_some()
        )
    });
    panic!("timed out waiting for {what}: {state}");
}

#[gpui_kit::test]
fn save_writes_the_edits_over_the_open_file_and_keeps_showing_it(cx: &mut TestAppContext) {
    let (shell, cx) = open_window(cx);
    let dir = std::env::temp_dir().join(format!("argand-save-over-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path: PathBuf = dir.join("capture.wav");
    write_wave(&path, 48_000);
    shell.update_in(cx, |shell, window, cx| {
        shell.open(Origin::new(path.clone()), window, cx);
    });
    wait_until(cx, &shell, "the file to describe itself", |shell| {
        shell.editing().is_some_and(Editing::is_described)
    });
    assert_eq!(
        shell.read_with(cx, |shell, _| shell.can_save_over()),
        Err(Unsavable::Unchanged)
    );
    shell.update_in(cx, |shell, window, cx| {
        shell.selection = argand_core::SampleSpan::between(0, 24_000);
        window.dispatch_action(Box::new(editing_ui::DeleteSelection), cx);
    });
    cx.run_until_parked();
    assert_eq!(
        shell.read_with(cx, |shell, _| shell.can_save_over()),
        Ok(())
    );
    wait_until(cx, &shell, "the edited picture", |shell| {
        shell.file.as_ref().is_some_and(|file| {
            matches!(file.state.document.status(), Status::Ready { .. })
                && file.state.document.analysis().is_some()
        })
    });
    let plot = shell.read_with(cx, |shell, _| shell.plot_view().map(Entity::entity_id));
    shell.update_in(cx, |shell, window, cx| shell.save_over(window, cx));
    wait_until(cx, &shell, "the saved file to open again", |shell| {
        shell.saving.is_none()
            && shell
                .editing()
                .is_some_and(|editing| !editing.is_dirty() && editing.len() == 24_000)
    });
    wait_until(cx, &shell, "the saved file to be described", |shell| {
        shell.editing().is_some_and(Editing::is_described)
    });
    shell.read_with(cx, |shell, _| {
        assert_eq!(
            shell.plot_view().map(Entity::entity_id),
            plot,
            "the plot stays"
        );
        let document = &shell.file.as_ref().unwrap().state.document;
        assert!(document.analysis().is_some(), "the picture stays");
        assert!(matches!(document.status(), Status::Ready { .. }));
        assert_eq!(shell.sample_count(), Some(24_000));
    });
    let reopened = argand_io::open(&path, &argand_io::OpenHints::default()).unwrap();
    assert_eq!(reopened.meta().len_samples, 24_000);
    let names: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert_eq!(names.len(), 1, "no temporary file is left");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[gpui_kit::test]
fn a_failed_replacement_keeps_the_edits_on_the_file(cx: &mut TestAppContext) {
    let (shell, cx) = open_window(cx);
    let dir = std::env::temp_dir().join(format!("argand-restore-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path: PathBuf = dir.join("capture.wav");
    write_wave(&path, 48_000);
    shell.update_in(cx, |shell, window, cx| {
        shell.open(Origin::new(path.clone()), window, cx);
    });
    wait_until(cx, &shell, "the file to describe itself", |shell| {
        shell.editing().is_some_and(Editing::is_described)
    });
    shell.update_in(cx, |shell, window, cx| {
        shell.selection = argand_core::SampleSpan::between(0, 24_000);
        window.dispatch_action(Box::new(editing_ui::DeleteSelection), cx);
    });
    cx.run_until_parked();
    let plot = shell.read_with(cx, |shell, _| shell.plot_view().map(Entity::entity_id));
    // What a refused commit does, the file itself left as it was.
    shell.update_in(cx, |shell, window, cx| {
        let origin = Origin::new(path.clone());
        let (start, _old) = shell.swap_readers(&origin, window, cx).unwrap();
        let replacement = Replacement { start, then: None };
        let kept = dir.join("capture.unsaved-1.wav");
        shell.replaced(Err(("disk full".into(), kept)), replacement, window, cx);
    });
    wait_until(cx, &shell, "the file to open under the edits", |shell| {
        shell
            .file
            .as_ref()
            .is_some_and(|file| file.rebinding.is_none())
    });
    shell.read_with(cx, |shell, _| {
        let editing = shell.editing().unwrap();
        assert!(
            editing.is_dirty() && editing.len() == 24_000,
            "the edits stay"
        );
        assert_eq!(
            shell.plot_view().map(Entity::entity_id),
            plot,
            "the plot stays"
        );
        assert!(matches!(
            shell.save_notice,
            Some(saving_ui::Notice::Failed("Save failed", _))
        ));
    });
    assert_eq!(
        shell.read_with(cx, |shell, _| shell.sample_count()),
        Some(24_000)
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[gpui_kit::test]
fn a_selected_band_saves_as_a_complex_capture_at_a_lower_rate(cx: &mut TestAppContext) {
    let (shell, cx) = open_window(cx);
    let dir = std::env::temp_dir().join(format!("argand-save-band-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path: PathBuf = dir.join("capture.wav");
    write_wave(&path, 48_000);
    shell.update_in(cx, |shell, window, cx| {
        shell.open(Origin::new(path.clone()), window, cx);
    });
    wait_until(cx, &shell, "the file to describe itself", |shell| {
        shell.editing().is_some_and(Editing::is_described)
    });
    shell.update_in(cx, |shell, window, cx| {
        shell.select(
            argand_core::Selection {
                time: None,
                band: argand_core::FrequencyBand::between(1_000., 3_000.),
            },
            cx,
        );
        assert!(shell.set_reference(14_074_000., window, cx));
        assert!(shell.can_save(true));
        shell.save_as(true, window, cx);
    });
    let target = dir.join("band.wav");
    let chosen = target.clone();
    wait_for_save_chooser(cx);
    cx.simulate_new_path_selection(move |_| Some(chosen));
    wait_until(cx, &shell, "the band to be saved", |shell| {
        shell.saving.is_none() && matches!(shell.save_notice, Some(saving_ui::Notice::Saved(_)))
    });
    let saved = argand_io::open(&target, &argand_io::OpenHints::default()).unwrap();
    let meta = saved.meta();
    assert_eq!(meta.sample_rate, 3_000.);
    assert_eq!(meta.center_freq, 0.);
    assert_eq!(meta.len_samples, 6_000);
    assert!(meta.is_iq());
    std::fs::remove_dir_all(&dir).unwrap();
}

#[gpui_kit::test]
fn nothing_opens_or_closes_while_the_file_is_replaced(cx: &mut TestAppContext) {
    let (shell, cx) = open_window(cx);
    shell.update_in(cx, |shell, window, cx| {
        shell.replacement_active = true;
        shell.open(
            Origin::new(PathBuf::from("/captures/other.iqw")),
            window,
            cx,
        );
    });
    assert!(
        shell.read_with(cx, |shell, _| shell.file.is_none()),
        "the open was refused"
    );
    cx.update(|window, cx| window.dispatch_action(Box::new(chrome::CloseWindow), cx));
    cx.run_until_parked();
    assert!(!cx.has_pending_prompt());
    assert!(shell.read_with(cx, |shell, _| matches!(
        shell.save_notice,
        Some(saving_ui::Notice::Failed("Please wait", _))
    )));
}

#[gpui_kit::test]
fn reference_edit_and_save_rebase_the_picture_and_band_without_changing_samples(
    cx: &mut TestAppContext,
) {
    let (shell, cx) = open_window(cx);
    let dir = argand_io::testutil::TempDir::new("reference-save-rebind");
    let path = dir.join("capture.wav");
    write_wave(&path, 48_000);
    shell.update_in(cx, |shell, window, cx| {
        let mut origin = Origin::new(path.clone());
        origin.hints.center_freq = Some(0.);
        shell.open(origin, window, cx);
    });
    wait_until(cx, &shell, "ready reference fixture", |shell| {
        shell.editing().is_some_and(Editing::is_described)
            && shell
                .file
                .as_ref()
                .is_some_and(|file| matches!(file.state.document.status(), Status::Ready { .. }))
    });
    shell.update_in(cx, |shell, window, cx| {
        shell.band = argand_core::FrequencyBand::between(-100., 100.);
        let before = shell
            .file
            .as_ref()
            .unwrap()
            .state
            .document
            .analysis()
            .unwrap()
            .db
            .values
            .as_ptr();
        let view = shell.frequency;
        assert!(shell.set_reference(1_000_000., window, cx));
        assert_eq!(
            shell
                .file
                .as_ref()
                .unwrap()
                .state
                .document
                .analysis()
                .unwrap()
                .db
                .values
                .as_ptr(),
            before
        );
        assert_eq!(shell.frequency, view);
        assert_eq!(shell.extents().unwrap().frequency_offset, 1_000_000.);
        assert_eq!(shell.band.unwrap().low(), -100.);
        shell.save_over(window, cx);
    });
    wait_until(cx, &shell, "saved reference rebound", |shell| {
        shell.saving.is_none()
            && shell
                .editing()
                .is_some_and(|editing| !editing.is_dirty() && editing.reference() == 1_000_000.)
    });
    shell.read_with(cx, |shell, _| {
        let document = &shell.file.as_ref().unwrap().state.document;
        assert_eq!(document.meta().unwrap().center_freq, 1_000_000.);
        assert_eq!(document.analysis().unwrap().db.f0, 988_000.);
        assert_eq!(shell.band.unwrap().low(), 999_900.);
        assert_eq!(shell.extents().unwrap().frequency_offset, 0.);
        assert_eq!(
            shell.editing().unwrap().written_source().hints.center_freq,
            None
        );
    });
    let mut source = argand_io::open(&path, &argand_io::OpenHints::default()).unwrap();
    assert_eq!(source.meta().center_freq, 1_000_000.);
    assert_eq!(source.meta().len_samples, 48_000);
    let mut samples = vec![0.; 96_000];
    assert_eq!(source.read(&mut samples).unwrap(), samples.len());
    for (n, pair) in samples.as_chunks::<2>().0.iter().enumerate() {
        assert_eq!(pair, &[(n % 30_000) as f32 / 32768.; 2]);
    }
}

#[gpui_kit::test]
fn reference_edits_do_not_change_recent_and_saved_zero_comes_from_the_file(
    cx: &mut TestAppContext,
) {
    let (shell, cx) = open_window(cx);
    let dir = argand_io::testutil::TempDir::new("reference-recent");
    let path = dir.join("capture.wav");
    write_wave(&path, 2400);
    shell.update_in(cx, |shell, window, cx| {
        shell.open(Origin::new(path.clone()), window, cx)
    });
    wait_until(cx, &shell, "reference Recent fixture", |shell| {
        shell.editing().is_some_and(Editing::is_described)
    });
    shell.update_in(cx, |shell, window, cx| {
        let recent = shell.session.recent.clone();
        assert!(shell.set_reference(1_000_000., window, cx));
        assert_eq!(shell.session.recent, recent);
        let entry = &recent[0];
        shell.open(
            Origin {
                path: entry.path.clone(),
                hints: entry.hints.to_open_hints(),
            },
            window,
            cx,
        );
    });
    assert!(cx.has_pending_prompt());
    cx.simulate_prompt_answer("Discard");
    wait_until(cx, &shell, "discarded reference reopening", |shell| {
        shell.editing().is_some_and(|editing| {
            editing.is_described() && !editing.is_dirty() && editing.reference() == 0.
        })
    });
    for reference in [1_000_000., 0.] {
        shell.update_in(cx, |shell, window, cx| {
            let recent = shell.session.recent.clone();
            assert!(shell.set_reference(reference, window, cx));
            assert_eq!(shell.session.recent, recent);
            shell.save_over(window, cx);
        });
        wait_until(
            cx,
            &shell,
            "saved reference without Recent override",
            |shell| {
                shell.saving.is_none()
                    && shell.editing().is_some_and(|editing| {
                        editing.is_described()
                            && !editing.is_dirty()
                            && editing.reference() == reference
                    })
            },
        );
        shell.read_with(cx, |shell, _| {
            let origin = shell.file.as_ref().unwrap().state.document.origin();
            assert_eq!(origin.hints.center_freq, None);
            assert_eq!(shell.editing().unwrap().file().hints.center_freq, None);
            let entry = shell
                .session
                .recent
                .iter()
                .find(|entry| entry.path == path)
                .unwrap();
            let hints = entry.hints.to_open_hints();
            assert_eq!(hints.center_freq, None);
            let reopened = argand_io::open(&entry.path, &hints).unwrap();
            assert_eq!(reopened.meta().center_freq, reference);
        });
    }
}

#[gpui_kit::test]
fn saving_before_open_clears_the_published_files_recent_reference_override(
    cx: &mut TestAppContext,
) {
    let (shell, cx) = open_window(cx);
    let dir = argand_io::testutil::TempDir::new("reference-pending-open");
    let path = dir.join("first.wav");
    let second = dir.join("second.wav");
    write_wave(&path, 2400);
    write_wave(&second, 4800);
    shell.update_in(cx, |shell, window, cx| {
        let mut origin = Origin::new(path.clone());
        origin.hints.center_freq = Some(1_000_000.);
        shell.open(origin, window, cx);
    });
    wait_until(cx, &shell, "CLI reference fixture", |shell| {
        shell.editing().is_some_and(Editing::is_described)
    });
    shell.update_in(cx, |shell, window, cx| {
        assert!(shell.set_reference(0., window, cx));
        shell.open(Origin::new(second.clone()), window, cx);
    });
    assert!(cx.has_pending_prompt());
    cx.simulate_prompt_answer("Save");
    wait_until(cx, &shell, "save then open", |shell| {
        shell
            .editing()
            .is_some_and(|editing| editing.is_described() && editing.file().meta.source == second)
    });
    shell.read_with(cx, |shell, _| {
        let entry = shell
            .session
            .recent
            .iter()
            .find(|entry| entry.path == path)
            .unwrap();
        let hints = entry.hints.to_open_hints();
        assert_eq!(hints.center_freq, None);
        assert_eq!(
            argand_io::open(&path, &hints).unwrap().meta().center_freq,
            0.
        );
    });
}

#[gpui_kit::test]
fn metadata_undo_keeps_analysis_and_selected_bins(cx: &mut TestAppContext) {
    let (shell, cx) = open_window(cx);
    let dir = argand_io::testutil::TempDir::new("reference-undo");
    let path = dir.join("capture.wav");
    write_wave(&path, 48_000);
    shell.update_in(cx, |shell, window, cx| {
        shell.open(Origin::new(path), window, cx)
    });
    wait_until(cx, &shell, "ready reference fixture", |shell| {
        shell
            .file
            .as_ref()
            .is_some_and(|file| matches!(file.state.document.status(), Status::Ready { .. }))
    });
    let generation = shell.read_with(cx, |shell, _| {
        shell
            .file
            .as_ref()
            .unwrap()
            .state
            .analyst
            .generation_for_test()
    });
    shell.update_in(cx, |shell, window, cx| {
        shell.band = argand_core::FrequencyBand::between(-200., 300.);
        assert!(shell.set_reference(100_000., window, cx));
    });
    cx.run_until_parked();
    shell.read_with(cx, |shell, _| {
        assert_eq!(shell.extents().unwrap().frequency_offset, 100_000.);
        assert_eq!(shell.editing().unwrap().reference(), 100_000.);
    });
    shell.update_in(cx, |_, window, cx| {
        window.dispatch_action(Box::new(editing_ui::Undo), cx)
    });
    cx.run_until_parked();
    shell.read_with(cx, |shell, _| {
        assert_eq!(shell.editing().unwrap().reference(), 0.);
        assert_eq!(shell.band.unwrap().low(), -200.);
        assert!(!shell.editing().unwrap().is_dirty());
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
}

#[gpui_kit::test]
fn failed_reference_publication_keeps_original_reader_basis_for_navigation_and_suppression(
    cx: &mut TestAppContext,
) {
    let (shell, cx) = open_window(cx);
    let dir = argand_io::testutil::TempDir::new("reference-failed-publication");
    let path = dir.join("capture.wav");
    write_wave(&path, 48_000);
    shell.update_in(cx, |shell, window, cx| {
        let mut origin = Origin::new(path.clone());
        origin.hints.center_freq = Some(0.);
        shell.open(origin, window, cx);
    });
    wait_until(cx, &shell, "original reference analysis", |shell| {
        shell
            .editing()
            .is_some_and(|editing| editing.is_described() && editing.can_compute())
            && shell
                .file
                .as_ref()
                .is_some_and(|file| matches!(file.state.document.status(), Status::Ready { .. }))
    });
    shell.update_in(cx, |shell, window, cx| {
        shell.band = argand_core::FrequencyBand::between(1000., 3000.);
        assert!(shell.set_reference(1_000_000., window, cx));
        let origin = shell.file.as_ref().unwrap().state.document.origin().clone();
        assert_eq!(
            origin.hints.center_freq,
            Some(0.),
            "reader hints describe the unchanged source"
        );
        let (start, _old) = shell.swap_readers(&origin, window, cx).unwrap();
        shell.replaced(
            Err(("publication refused".into(), dir.join("kept.wav"))),
            Replacement { start, then: None },
            window,
            cx,
        );
    });
    wait_until(cx, &shell, "original file rebound after refusal", |shell| {
        shell
            .file
            .as_ref()
            .is_some_and(|file| file.rebinding.is_none())
    });
    shell.update_in(cx, |shell, _, cx| {
        assert_eq!(
            shell
                .file
                .as_ref()
                .unwrap()
                .state
                .document
                .meta()
                .unwrap()
                .center_freq,
            0.
        );
        assert_eq!(shell.editing().unwrap().reference(), 1_000_000.);
        assert_eq!(shell.band.unwrap().low(), 1000.);
        shell.frequency = shell.frequency.zoom(0.5, 0.5, 2048);
        shell.ask_for_a_picture();
        cx.notify();
    });
    wait_until(
        cx,
        &shell,
        "navigation on original frequency basis",
        |shell| {
            shell
                .file
                .as_ref()
                .and_then(|file| file.state.document.analysis())
                .is_some_and(|analysis| analysis.db.f0 == -6000.)
        },
    );
    shell.update_in(cx, |shell, window, cx| shell.suppress_band(window, cx));
    wait_until(
        cx,
        &shell,
        "suppression after publication refusal",
        |shell| {
            shell.suppressing.is_none()
                && shell.editing().is_some_and(|editing| {
                    editing.capture().sources() != [argand_edit::SourceId(0)]
                })
        },
    );
    shell.read_with(cx, |shell, _| {
        assert_eq!(shell.editing().unwrap().reference(), 1_000_000.);
        assert_eq!(shell.band.unwrap().low(), 1000.);
        assert!(shell.editing().unwrap().is_dirty());
    });
}

fn raw_fixture<'a>(
    cx: &'a mut TestAppContext,
    name: &str,
) -> (
    Entity<Shell>,
    &'a mut gpui_kit::VisualTestContext,
    argand_io::testutil::TempDir,
) {
    let (shell, cx) = open_window(cx);
    let dir = argand_io::testutil::TempDir::new(name);
    let path = dir.join("capture.raw");
    let samples: Vec<u8> = (0..9600i16).flat_map(i16::to_le_bytes).collect();
    std::fs::write(&path, samples).unwrap();
    shell.update_in(cx, |shell, window, cx| {
        let mut origin = Origin::new(path);
        origin.hints.raw = Some("iq_i16@24k".parse().unwrap());
        origin.hints.center_freq = Some(0.);
        shell.open(origin, window, cx);
    });
    wait_until(cx, &shell, "RAW description", |shell| {
        shell.editing().is_some_and(Editing::is_described)
    });
    (shell, cx, dir)
}

#[gpui_kit::test]
fn raw_reference_save_cancel_and_wave_rebind_preserve_source(cx: &mut TestAppContext) {
    let (shell, cx, dir) = raw_fixture(cx, "raw-reference-wave");
    let raw = dir.join("capture.raw");
    let original = std::fs::read(&raw).unwrap();
    shell.update_in(cx, |shell, window, cx| {
        shell.band = argand_core::FrequencyBand::between(-100., 100.);
        assert!(shell.set_reference(1_000_000., window, cx));
        assert_eq!(shell.can_save_over(), Err(Unsavable::RawReference));
        assert!(shell.save_needs_new_path() && shell.can_save(false));
        let source = shell.editing().unwrap().file();
        assert_eq!(
            saving::suggested_name(&source.meta, true, None),
            "capture.wav"
        );
        shell.save_over(window, cx);
    });
    wait_for_save_chooser(cx);
    cx.simulate_new_path_selection(|_| None);
    cx.run_until_parked();
    shell.read_with(cx, |shell, _| {
        assert!(shell.editing().unwrap().is_dirty());
        assert_eq!(
            shell.file.as_ref().unwrap().state.document.origin().path,
            raw
        );
        assert!(shell.saving.is_none());
    });
    assert_eq!(std::fs::read(&raw).unwrap(), original);
    shell.update_in(cx, |shell, window, cx| shell.save_over(window, cx));
    wait_for_save_chooser(cx);
    let wave = dir.join("capture.wav");
    cx.simulate_new_path_selection(|_| Some(wave.clone()));
    wait_until(cx, &shell, "RAW to saved WAVE", |shell| {
        shell.saving.is_none()
            && shell.editing().is_some_and(|editing| {
                editing.file().meta.source == wave && !editing.is_dirty() && editing.is_described()
            })
    });
    shell.read_with(cx, |shell, _| {
        let editing = shell.editing().unwrap();
        assert_eq!(editing.reference(), 1_000_000.);
        assert!(editing.file().hints.raw.is_none());
        assert_eq!(shell.band.unwrap().low(), 999_900.);
        let recent = shell
            .session
            .recent
            .iter()
            .find(|entry| entry.path == wave)
            .unwrap();
        let reopened = argand_io::open(&wave, &recent.hints.to_open_hints()).unwrap();
        assert_eq!(reopened.meta().center_freq, 1_000_000.);
    });
    assert_eq!(std::fs::read(&raw).unwrap(), original);
    let mut reopened = argand_io::open(&wave, &argand_io::OpenHints::default()).unwrap();
    assert_eq!(reopened.meta().center_freq, 1_000_000.);
    let mut samples = vec![0.; 9600];
    assert_eq!(reopened.read(&mut samples).unwrap(), samples.len());
    for (index, sample) in samples.into_iter().enumerate() {
        assert_eq!(sample, index as f32 / 32768.);
    }
    shell.update_in(cx, |shell, window, cx| {
        assert!(shell.set_reference(2_000_000., window, cx));
        assert!(!shell.save_needs_new_path());
        assert_eq!(shell.can_save_over(), Ok(()));
    });
}

#[gpui_kit::test]
fn raw_reference_undo_retains_headerless_sample_save_and_prompt_cancel(cx: &mut TestAppContext) {
    let (shell, cx, dir) = raw_fixture(cx, "raw-reference-undo");
    let raw = dir.join("capture.raw");
    shell.update_in(cx, |shell, window, cx| {
        shell.selection = argand_core::SampleSpan::between(0, 2400);
        window.dispatch_action(Box::new(editing_ui::DeleteSelection), cx);
    });
    cx.run_until_parked();
    shell.update_in(cx, |shell, window, cx| {
        assert!(shell.set_reference(100., window, cx));
        window.dispatch_action(Box::new(editing_ui::Undo), cx);
    });
    cx.run_until_parked();
    shell.update_in(cx, |shell, window, cx| {
        assert_eq!(shell.editing().unwrap().reference(), 0.);
        assert_eq!(shell.can_save_over(), Ok(()));
        assert!(!shell.save_needs_new_path());
        shell.save_over(window, cx);
    });
    wait_until(cx, &shell, "ordinary headerless save", |shell| {
        shell.saving.is_none()
            && shell.editing().is_some_and(|editing| {
                !editing.is_dirty() && editing.is_described() && editing.len() == 2400
            })
    });
    assert_eq!(std::fs::read(&raw).unwrap().len(), 9600);
    let second = dir.join("second.wav");
    write_wave(&second, 4800);
    shell.update_in(cx, |shell, window, cx| {
        assert!(shell.set_reference(100., window, cx));
        shell.open(Origin::new(second.clone()), window, cx);
    });
    cx.simulate_prompt_answer("Save");
    wait_for_save_chooser(cx);
    cx.simulate_new_path_selection(|_| None);
    cx.run_until_parked();
    shell.read_with(cx, |shell, _| {
        assert_eq!(
            shell.file.as_ref().unwrap().state.document.origin().path,
            raw
        );
        assert!(shell.editing().unwrap().is_dirty());
    });
    assert!(!cx.has_pending_prompt());
}

#[gpui_kit::test]
fn wave_save_rebind_uses_the_exported_whole_hertz_rate(cx: &mut TestAppContext) {
    let (shell, cx) = open_window(cx);
    let dir = argand_io::testutil::TempDir::new("whole-rate-rebind");
    let path = dir.join("capture.iqw");
    write_wave(&path, 48_000);
    shell.update_in(cx, |shell, window, cx| {
        let mut origin = Origin::new(path.clone());
        origin.hints.sample_rate = Some(24_000.25);
        shell.open(origin, window, cx);
    });
    wait_until(cx, &shell, "fractional-rate source", |shell| {
        shell.editing().is_some_and(Editing::is_described)
            && shell
                .file
                .as_ref()
                .is_some_and(|file| matches!(file.state.document.status(), Status::Ready { .. }))
    });
    shell.update_in(cx, |shell, window, cx| {
        shell.clipboard = Some(
            shell
                .editing()
                .unwrap()
                .copy(argand_core::SampleSpan::between(0, 100).unwrap()),
        );
        assert!(shell.set_reference(1_000_000., window, cx));
        shell.save_over(window, cx);
    });
    wait_until(
        cx,
        &shell,
        "whole-rate saved source and refreshed picture",
        |shell| {
            shell.saving.is_none()
                && shell.editing().is_some_and(|editing| {
                    !editing.is_dirty() && editing.file().meta.sample_rate == 24_000.
                })
                && shell
                    .file
                    .as_ref()
                    .and_then(|file| file.state.document.analysis())
                    .is_some_and(|analysis| analysis.db.f1 == 1_012_000.)
        },
    );
    let reopened = argand_io::open(&path, &argand_io::OpenHints::default()).unwrap();
    assert_eq!(reopened.meta().sample_rate, 24_000.);
    assert_eq!(reopened.meta().center_freq, 1_000_000.);
    assert_eq!(reopened.meta().len_samples, 48_000);
    cx.update(|window, _| assert!(window.try_find("save-status").is_none()));
    shell.update_in(cx, |shell, _, _| {
        let clipboard = shell.clipboard.clone().unwrap();
        assert!(
            shell
                .editing_mut()
                .unwrap()
                .paste(&clipboard, crate::editing::Placement::At(0))
                .is_ok()
        );
    });
}

#[gpui_kit::test]
fn headerless_save_preserves_an_unchanged_fractional_reference_hint(cx: &mut TestAppContext) {
    let (shell, cx, dir) = raw_fixture(cx, "raw-fractional-reference");
    shell.update_in(cx, |shell, window, cx| {
        let mut origin = Origin::new(dir.join("capture.raw"));
        origin.hints.raw = Some("iq_i16@24k".parse().unwrap());
        origin.hints.center_freq = Some(123.25);
        shell.open(origin, window, cx);
    });
    wait_until(cx, &shell, "fractional RAW reference", |shell| {
        shell.editing().is_some_and(Editing::is_described)
    });
    shell.update_in(cx, |shell, window, cx| {
        shell.selection = argand_core::SampleSpan::between(0, 2400);
        window.dispatch_action(Box::new(editing_ui::DeleteSelection), cx);
    });
    cx.run_until_parked();
    shell.update_in(cx, |shell, window, cx| {
        assert_eq!(shell.editing().unwrap().len(), 2400);
        assert_eq!(shell.can_save_over(), Ok(()));
        shell.save_over(window, cx);
    });
    wait_until(cx, &shell, "saved RAW", |shell| {
        shell.saving.is_none()
            && shell
                .editing()
                .is_some_and(|editing| !editing.is_dirty() && editing.is_described())
    });
    shell.read_with(cx, |shell, _| {
        assert_eq!(shell.editing().unwrap().reference(), 123.25);
        assert_eq!(
            shell
                .file
                .as_ref()
                .unwrap()
                .state
                .document
                .origin()
                .hints
                .center_freq,
            Some(123.25)
        );
    });
}

#[gpui_kit::test]
fn disabled_save_shortcut_is_inert_and_reenables_after_a_reference_edit(cx: &mut TestAppContext) {
    let (shell, cx) = open_window(cx);
    cx.update(|_, cx| Shell::bind_choose_file(shell.downgrade(), cx));
    let dir = argand_io::testutil::TempDir::new("save-shortcut-availability");
    let path = dir.join("capture.wav");
    write_wave(&path, 2400);
    shell.update_in(cx, |shell, window, cx| {
        shell.open(Origin::new(path.clone()), window, cx)
    });
    wait_until(cx, &shell, "save shortcut fixture", |shell| {
        shell.editing().is_some_and(Editing::is_described)
    });
    cx.simulate_keystrokes(if cfg!(target_os = "macos") {
        "cmd-s"
    } else {
        "ctrl-s"
    });
    cx.run_until_parked();
    shell.read_with(cx, |shell, _| {
        assert!(!shell.can_save_edits());
        assert!(shell.save_notice.is_none());
        assert!(shell.saving.is_none());
    });
    shell.update_in(cx, |shell, window, cx| {
        shell.set_reference(12345., window, cx);
    });
    assert!(shell.read_with(cx, |shell, _| shell.can_save_edits()));
    cx.simulate_keystrokes(if cfg!(target_os = "macos") {
        "cmd-s"
    } else {
        "ctrl-s"
    });
    wait_until(cx, &shell, "shortcut reference save", |shell| {
        shell.saving.is_none()
            && shell
                .editing()
                .is_some_and(|e| !e.is_dirty() && e.reference() == 12345.)
    });
    assert_eq!(
        argand_io::open(&path, &Default::default())
            .unwrap()
            .meta()
            .center_freq,
        12345.
    );
}
