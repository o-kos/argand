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
