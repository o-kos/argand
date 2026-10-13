//! Noticing that the open capture changed on disk, and opening it again.

use super::*;
use crate::disk_change::Watched;
use gpui_kit::AnyElement;
use std::path::PathBuf;

impl Shell {
    /// The files the shown capture reads, with the stamps they were opened with.
    ///
    /// A file without a stamp is left out: reading it again is refused on its own.
    fn watched(&self) -> Option<Watched> {
        let file = self.file.as_ref()?;
        let document = &file.state.document;
        let files: Vec<_> = match &file.editing {
            Some(editing) => editing
                .source_files()
                .into_iter()
                .flatten()
                .filter_map(|source| Some((source.meta.source, source.stamp?)))
                .collect(),
            None => document
                .stamp()
                .map(|stamp| (document.origin().path.clone(), stamp))
                .into_iter()
                .collect(),
        };
        (!files.is_empty()).then_some(Watched(files))
    }

    /// Stamp the capture's files again off the window's thread, once at a time.
    pub(super) fn check_disk(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.replacement_active {
            return;
        }
        let Some(watched) = self.watched() else {
            return;
        };
        let Some(file) = self.file.as_mut() else {
            return;
        };
        // While new readers open a saved file, the document's stamp is still the old file's.
        if file.state.document.changed().is_some()
            || file.disk_check.is_some()
            || file.rebinding.is_some()
        {
            return;
        }
        let document = file.id;
        let (found, result) = async_channel::bounded(1);
        let checked = watched.clone();
        // Never joined: a check blocked on a dead mount must not hold up anything.
        let spawned = std::thread::Builder::new()
            .name("argand-disk-check".into())
            .spawn(move || {
                let _ = found.try_send(checked.first_changed());
            });
        if let Err(error) = spawned {
            tracing::warn!(%error, "cannot check the capture's files");
            return;
        }
        file.disk_check = Some(cx.spawn_in(window, async move |shell, cx| {
            let Ok(changed) = result.recv().await else {
                return;
            };
            let _ = shell.update(cx, |shell, cx| {
                shell.disk_checked(document, &watched, changed, cx);
            });
        }));
    }

    /// Take a check's result, when it is about the document and the files still shown.
    fn disk_checked(
        &mut self,
        document: u64,
        watched: &Watched,
        changed: Option<PathBuf>,
        cx: &mut Context<Self>,
    ) {
        let current = self.watched();
        let Some(file) = self.file.as_mut().filter(|file| file.id == document) else {
            return;
        };
        file.disk_check = None;
        // A save that rebound the file, or an edit that read another, makes the check stale.
        if self.replacement_active || file.rebinding.is_some() || current.as_ref() != Some(watched)
        {
            return;
        }
        if let Some(path) = changed {
            tracing::warn!(path = %path.display(), "the capture changed on disk");
            file.state.document.mark_changed(path);
            cx.notify();
        }
    }

    /// Whether a file the shown capture reads changed on disk.
    pub(super) fn capture_changed(&self) -> bool {
        self.file
            .as_ref()
            .is_some_and(|file| file.state.document.changed().is_some())
    }

    /// Open the changed capture afresh, asking first about unsaved edits.
    fn reopen_changed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(origin) = self
            .file
            .as_ref()
            .map(|file| file.state.document.origin().clone())
        else {
            return;
        };
        self.open(origin, window, cx);
    }

    /// The status-bar notice of a changed file, with the command that opens the capture again.
    pub(super) fn changed_item(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let path = self.file.as_ref()?.state.document.changed()?;
        let name = path
            .file_name()
            .unwrap_or(path.as_os_str())
            .to_string_lossy()
            .into_owned();
        let hint = MetadataHint {
            title: "Changed on disk",
            value: path.display().to_string(),
            explanation: "The picture shows the file as it was opened".into(),
            rows: Vec::new(),
        };
        Some(
            div()
                .id("changed-on-disk")
                .flex()
                .items_center()
                .gap_1()
                .min_w_0()
                .text_color(cx.theme().danger)
                .tooltip(move |_, cx| metadata_tooltip(hint.clone(), cx))
                .child(
                    div()
                        .min_w_0()
                        .overflow_hidden()
                        .text_ellipsis()
                        .whitespace_nowrap()
                        .child(format!("{name} changed on disk")),
                )
                .child(
                    Button::new("reopen-changed")
                        .ghost()
                        .xsmall()
                        .tab_stop(false)
                        .label("Reopen")
                        .on_click(cx.listener(|shell, _, window, cx| {
                            cx.stop_propagation();
                            shell.reopen_changed(window, cx);
                        })),
                )
                .into_any_element(),
        )
    }
}

#[cfg(test)]
mod tests {
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

        write_wave(&path, 48_000);
        let file = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
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
                    .changed()
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
}
