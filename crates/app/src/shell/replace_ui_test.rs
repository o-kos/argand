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
        assert!(shell.set_reference(1_000_000.25, window, cx));
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
        assert_eq!(shell.extents().unwrap().frequency_offset, 1_000_000.25);
        assert_eq!(shell.band.unwrap().low(), -100.);
        shell.save_over(window, cx);
    });
    wait_until(cx, &shell, "saved reference rebound", |shell| {
        shell.saving.is_none()
            && shell
                .editing()
                .is_some_and(|editing| !editing.is_dirty() && editing.reference() == 1_000_000.25)
    });
    shell.read_with(cx, |shell, _| {
        let document = &shell.file.as_ref().unwrap().state.document;
        assert_eq!(document.meta().unwrap().center_freq, 1_000_000.25);
        assert_eq!(document.analysis().unwrap().db.f0, 988_000.25);
        assert_eq!(shell.band.unwrap().low(), 999_900.25);
        assert_eq!(shell.extents().unwrap().frequency_offset, 0.);
        assert_eq!(
            shell.editing().unwrap().written_source().hints.center_freq,
            Some(1_000_000.25)
        );
    });
    let mut source = argand_io::open(&path, &argand_io::OpenHints::default()).unwrap();
    assert_eq!(source.meta().center_freq, 1_000_000.25);
    assert_eq!(source.meta().len_samples, 48_000);
    let mut samples = vec![0.; 96_000];
    assert_eq!(source.read(&mut samples).unwrap(), samples.len());
    for (n, pair) in samples.as_chunks::<2>().0.iter().enumerate() {
        assert_eq!(pair, &[(n % 30_000) as f32 / 32768.; 2]);
    }
}

#[gpui_kit::test]
fn baseband_and_metadata_undo_keep_analysis_and_selected_bins(cx: &mut TestAppContext) {
    let (shell, cx) = open_window(cx);
    let dir = argand_io::testutil::TempDir::new("reference-baseband");
    let path = dir.join("capture.wav");
    write_wave(&path, 48_000);
    shell.update_in(cx, |shell, window, cx| {
        shell.open(Origin::new(path), window, cx)
    });
    wait_until(cx, &shell, "ready baseband fixture", |shell| {
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
        window.dispatch_action(
            Box::new(crate::shell::plot::navigation_ui::ToggleBaseband),
            cx,
        );
    });
    cx.run_until_parked();
    shell.read_with(cx, |shell, _| {
        assert!(shell.session.baseband_frequencies);
        assert_eq!(shell.extents().unwrap().frequency_offset, 0.);
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
