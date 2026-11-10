use super::replace_ui::tests::{open_window, wait_until, write_wave};
use super::*;
use gpui_kit::TestAppContext;

/// A WAVE capture opened from a fresh folder, described, with one edit unsaved.
fn opened_and_edited(
    cx: &mut gpui_kit::VisualTestContext,
    shell: &Entity<Shell>,
    label: &str,
) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("argand-{label}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("capture.wav");
    write_wave(&path, 48_000);
    shell.update_in(cx, |shell, window, cx| {
        shell.open(Origin::new(path.clone()), window, cx);
    });
    wait_until(cx, shell, "the file to describe itself", |shell| {
        shell
            .editing()
            .is_some_and(crate::editing::Editing::is_described)
            && shell.waveform.is_some()
    });
    shell.update_in(cx, |shell, window, cx| {
        shell.selection = argand_core::SampleSpan::between(0, 100);
        window.dispatch_action(Box::new(editing_ui::DeleteSelection), cx);
    });
    cx.run_until_parked();
    assert!(shell.read_with(cx, |shell, _| {
        shell
            .editing()
            .is_some_and(crate::editing::Editing::is_dirty)
    }));
    path
}

fn document_id(cx: &mut gpui_kit::VisualTestContext, shell: &Entity<Shell>) -> Option<u64> {
    shell.read_with(cx, |shell, _| shell.file.as_ref().map(|file| file.id))
}

#[gpui_kit::test]
fn a_file_changed_while_away_is_noticed_and_reopened(cx: &mut TestAppContext) {
    let (shell, cx) = open_window(cx);
    let path = opened_and_edited(cx, &shell, "changed-away");
    shell.update_in(cx, |shell, window, cx| shell.check_disk(window, cx));
    wait_until(cx, &shell, "the unchanged check", |shell| {
        shell
            .file
            .as_ref()
            .is_some_and(|file| file.disk_check.is_none())
    });
    assert!(!shell.read_with(cx, |shell, _| shell.capture_changed()));

    // Rewritten in place without truncation, which Windows allows for a mapped file.
    let bytes = std::fs::read(&path).unwrap();
    let mut file = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
    std::io::Write::write_all(&mut file, &bytes).unwrap();
    file.set_modified(std::time::SystemTime::now() + std::time::Duration::from_secs(5))
        .unwrap();
    drop(file);
    shell.update_in(cx, |shell, window, cx| shell.check_disk(window, cx));
    wait_until(
        cx,
        &shell,
        "the change to be noticed",
        Shell::capture_changed,
    );
    shell.read_with(cx, |shell, _| {
        assert_eq!(
            shell.edit_commands_available(),
            editing_ui::EditCommands::default(),
            "a changed capture takes no edits"
        );
    });

    let first = document_id(cx, &shell);
    shell.update_in(cx, |shell, window, cx| shell.reopen_changed(window, cx));
    assert!(cx.has_pending_prompt(), "unsaved edits are asked about");
    cx.simulate_prompt_answer("Cancel");
    cx.run_until_parked();
    assert_eq!(document_id(cx, &shell), first);
    shell.update_in(cx, |shell, window, cx| shell.reopen_changed(window, cx));
    cx.simulate_prompt_answer("Discard");
    wait_until(cx, &shell, "the file to open again", |shell| {
        shell
            .file
            .as_ref()
            .is_some_and(|file| Some(file.id) != first)
            && shell
                .editing()
                .is_some_and(crate::editing::Editing::is_described)
    });
    assert!(!shell.read_with(cx, |shell, _| shell.capture_changed()));
}

#[gpui_kit::test]
fn a_stale_check_is_not_applied(cx: &mut TestAppContext) {
    let (shell, cx) = open_window(cx);
    let path = opened_and_edited(cx, &shell, "changed-stale");
    let (document, watched) = shell.read_with(cx, |shell, _| {
        (shell.file.as_ref().map(|file| file.id), shell.watched())
    });
    let (Some(document), Some(watched)) = (document, watched) else {
        panic!("the capture is not watched");
    };
    shell.update(cx, |shell, cx| {
        shell.disk_checked(document + 1, &watched, Some(path.clone()), cx);
        let other = Watched(
            vec![(path.clone(), watched.0[0].1)]
                .into_iter()
                .chain([(PathBuf::from("/elsewhere.wav"), watched.0[0].1)])
                .collect(),
        );
        shell.disk_checked(document, &other, Some(path.clone()), cx);
    });
    assert!(!shell.read_with(cx, |shell, _| shell.capture_changed()));
    shell.update(cx, |shell, cx| {
        shell.disk_checked(document, &watched, Some(path), cx);
    });
    assert!(shell.read_with(cx, |shell, _| shell.capture_changed()));
}

#[gpui_kit::test]
fn a_reader_refused_by_a_changed_file_marks_the_capture(cx: &mut TestAppContext) {
    let (shell, cx) = open_window(cx);
    let path = opened_and_edited(cx, &shell, "changed-reader");
    std::fs::remove_file(&path).unwrap();
    write_wave(&path, 48_000);
    // A fresh minimap scan reopens the file, which is now another one.
    shell.update_in(cx, |shell, window, cx| {
        shell.start_minimap(true, window, cx)
    });
    wait_until(
        cx,
        &shell,
        "the minimap to be refused",
        Shell::capture_changed,
    );
    assert!(
        shell.read_with(cx, |shell, _| shell.waveform.is_some()),
        "the minimap shown stays"
    );
    let changed = shell.read_with(cx, |shell, _| {
        shell.file.as_ref().and_then(|file| {
            file.state
                .document
                .changed_file()
                .map(std::path::Path::to_path_buf)
        })
    });
    assert_eq!(changed, Some(path));
}

#[gpui_kit::test]
fn a_late_envelope_failure_counts_only_for_a_file_still_read(cx: &mut TestAppContext) {
    let (shell, cx) = open_window(cx);
    opened_and_edited(cx, &shell, "changed-envelope");
    let error = anyhow::Error::from(argand_core::SourceError::Changed);
    let stamp = shell.read_with(cx, |shell, _| shell.editing().and_then(|e| e.file().stamp));
    shell.update(cx, |shell, cx| {
        // A pasted file undone since, and the open file under another stamp.
        shell.envelope_failed(argand_edit::SourceId(1), stamp, &error, cx);
        shell.envelope_failed(argand_edit::SourceId(0), None, &error, cx);
    });
    assert!(!shell.read_with(cx, |shell, _| shell.capture_changed()));
    shell.update(cx, |shell, cx| {
        shell.envelope_failed(argand_edit::SourceId(0), stamp, &error, cx);
    });
    assert!(shell.read_with(cx, |shell, _| shell.capture_changed()));
}

#[gpui_kit::test]
fn nothing_is_checked_while_a_saved_file_is_rebound(cx: &mut TestAppContext) {
    let (shell, cx) = open_window(cx);
    let path = opened_and_edited(cx, &shell, "changed-rebound");
    let (document, watched) = shell.read_with(cx, |shell, _| {
        (shell.file.as_ref().map(|file| file.id), shell.watched())
    });
    let (Some(document), Some(watched)) = (document, watched) else {
        panic!("the capture is not watched");
    };
    // The file was just saved over, and the document still holds the old stamp.
    std::fs::remove_file(&path).unwrap();
    write_wave(&path, 48_000);
    shell.update(cx, |shell, _| {
        let file = shell.file.as_mut().unwrap();
        let shown = file.state.document.shown();
        file.rebinding = Some(replace_ui::Rebinding::kept(shown));
    });
    shell.update_in(cx, |shell, window, cx| shell.check_disk(window, cx));
    assert!(
        shell.read_with(cx, |shell, _| shell
            .file
            .as_ref()
            .unwrap()
            .disk_check
            .is_none()),
        "no check starts"
    );
    shell.update(cx, |shell, cx| {
        shell.disk_checked(document, &watched, Some(path.clone()), cx);
    });
    assert!(
        !shell.read_with(cx, |shell, _| shell.capture_changed()),
        "a check started before is not applied"
    );
}

#[gpui_kit::test]
fn save_as_of_a_changed_capture_goes_on_from_the_saved_file(cx: &mut TestAppContext) {
    let (shell, cx) = open_window(cx);
    let path = opened_and_edited(cx, &shell, "changed-save-as");
    shell.update(cx, |shell, cx| {
        shell.envelope_failed(
            argand_edit::SourceId(0),
            shell.editing().and_then(|editing| editing.file().stamp),
            &anyhow::Error::from(argand_core::SourceError::Changed),
            cx,
        );
    });
    assert!(shell.read_with(cx, |shell, _| shell.capture_changed()));
    // What Save as wrote, read in place of the changed file from now on.
    let saved = path.with_file_name("saved.wav");
    write_wave(&saved, 47_900);
    shell.update_in(cx, |shell, window, cx| {
        shell.rebind_saved_as(saved.clone(), window, cx);
    });
    wait_until(cx, &shell, "the saved file to be read", |shell| {
        shell
            .file
            .as_ref()
            .is_some_and(|file| file.rebinding.is_none())
            && shell
                .editing()
                .is_some_and(crate::editing::Editing::is_described)
    });
    shell.read_with(cx, |shell, _| {
        assert!(!shell.capture_changed());
        assert_eq!(
            shell
                .editing()
                .map(|editing| editing.file().meta.source.clone()),
            Some(saved)
        );
    });
}

#[gpui_kit::test]
fn a_late_description_of_another_opening_keeps_the_stamp(cx: &mut TestAppContext) {
    let (shell, cx) = open_window(cx);
    let path = opened_and_edited(cx, &shell, "changed-described");
    let (document, stamp) = shell.read_with(cx, |shell, _| {
        (
            shell.file.as_ref().map(|file| file.id),
            shell.editing().and_then(|editing| editing.file().stamp),
        )
    });
    let Some(document) = document else {
        panic!("no document");
    };
    let mut earlier = shell.read_with(cx, |shell, _| shell.editing().unwrap().file().clone());
    earlier.stamp = None;
    shell.update(cx, |shell, cx| {
        shell.file_stored(document, &earlier, None, cx)
    });
    assert_eq!(
        shell.read_with(cx, |shell, _| shell.editing().and_then(|e| e.file().stamp)),
        stamp
    );
    assert!(stamp.is_some(), "{}", path.display());
}
