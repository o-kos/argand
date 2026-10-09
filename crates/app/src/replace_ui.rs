//! Save, which writes the edited capture over the file it was opened from.
//!
//! The capture is written to a temporary file while the original is still
//! read. Then every thread reading the original is waited for, the temporary
//! file replaces it, and new readers open it under the picture already shown,
//! which stays. Windows refuses to replace a file that is still mapped, which
//! is why nothing reading it may be left when the rename happens.

use super::*;
use crate::editing::Editing;
use crate::saving;
use argand_io::write::{Output, Replacing, Staged};

/// Why Save cannot act now, said where a person asks for it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Unsavable {
    Unchanged,
    Busy,
    Empty,
    Unchecked,
    /// The file is FLAC at a rate its encoder cannot state, so only Save as can write it.
    FlacRate,
}

impl Unsavable {
    pub(super) fn reason(self) -> &'static str {
        match self {
            Self::Unchanged => "there are no edits to save",
            Self::Busy => "another save is running",
            Self::Empty => "the capture is empty",
            Self::Unchecked => "the file is still being checked",
            Self::FlacRate => "the FLAC encoder cannot write this sample rate, use Save as",
        }
    }
}

/// What waits while the open file is let go of and replaced.
struct Replacement {
    /// Starts the readers of the file once it is replaced, or once it is not.
    start: crate::analysis::Start,
    then: Option<editing_ui::Pending>,
}

/// What the readers started under a picture already shown are to make of the file they open.
pub(super) struct Rebinding {
    to: Rebind,
    shown: crate::document::Shown,
}

/// The file new readers open under the shown picture.
enum Rebind {
    /// A new file of the samples these edits showed.
    Fresh(Box<Editing>),
    /// The same file with the same edits, the replacement having failed.
    Keep,
}

/// Readers of one file: its analysis worker, the task carrying its deliveries, and its lease.
pub(super) struct Readers {
    pub(super) analyst: crate::analysis::Analyst,
    pub(super) pump: Task<()>,
    pub(super) start: crate::analysis::Start,
    pub(super) lease: crate::release::Lease,
    pub(super) released: crate::release::Released,
}

impl Shell {
    /// Whether Save can write the edits over the open file now.
    pub(super) fn can_save_over(&self) -> Result<(), Unsavable> {
        let Some(editing) = self.editing() else {
            return Err(Unsavable::Unchanged);
        };
        let file = editing.file();
        if !editing.is_dirty() {
            Err(Unsavable::Unchanged)
        } else if self.saving.is_some() {
            Err(Unsavable::Busy)
        } else if editing.capture().is_empty() {
            Err(Unsavable::Empty)
        } else if !editing.is_described() || file.stamp.is_none() {
            Err(Unsavable::Unchecked)
        } else if file.meta.container == "flac"
            && argand_io::write::writes_as_wave(&file.meta, &file.hints)
        {
            Err(Unsavable::FlacRate)
        } else {
            Ok(())
        }
    }

    /// Refuse what would open or close a file while the open one is being replaced.
    pub(super) fn refuse_while_replacing(&mut self, cx: &mut Context<Self>) -> bool {
        if self.replacement_active {
            self.save_notice = Some(saving_ui::Notice::Failed(
                "Please wait",
                "the file is being saved".into(),
            ));
            cx.notify();
        }
        self.replacement_active
    }

    pub(super) fn save_over(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.save_over_then(None, window, cx);
    }

    /// Write the edits over the open file, then do `then` once it is replaced.
    pub(super) fn save_over_then(
        &mut self,
        then: Option<editing_ui::Pending>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Err(why) = self.can_save_over() {
            self.save_notice = Some(saving_ui::Notice::Failed(
                "Cannot save",
                why.reason().into(),
            ));
            cx.notify();
            return;
        }
        let Some(file) = &self.file else { return };
        let Some(editing) = &file.editing else { return };
        let source = editing.file();
        let Some(stamp) = source.stamp else { return };
        let path = source.meta.source.clone();
        let Some(mut request) = editing.save_request(None, path.clone()) else {
            return;
        };
        request.replacing = Some(Replacing { path, stamp });
        if source.hints.raw.is_some() {
            request.output = Output::Headerless {
                preamble: source.hints.byte_offset,
            };
        }
        let of = saving_ui::SaveOf {
            document: file.id,
            version: editing.version(),
            whole: true,
            rebind: true,
            then,
            over: true,
        };
        self.interrupt_plot(cx);
        self.start_save(request, of, window, cx);
    }

    /// The edits are written to a temporary file, so the open file can now be let go of and replaced.
    pub(super) fn staged(
        &mut self,
        staged: Staged,
        mut saving: saving_ui::Saving,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let of = &saving.of;
        let current = self
            .file
            .as_ref()
            .filter(|file| file.id == of.document)
            .and_then(|file| file.editing.as_ref())
            .is_some_and(|editing| editing.version() == of.version);
        if !current {
            // Dropping the staged file removes it, the document having moved on.
            self.save_notice = Some(saving_ui::Notice::Failed(
                "Not saved",
                "the capture changed while it was being saved".into(),
            ));
            return;
        }
        let Some(origin) = self
            .file
            .as_ref()
            .map(|file| file.state.document.origin().clone())
        else {
            return;
        };
        let Some((start, old)) = self.swap_readers(&origin, window, cx) else {
            return;
        };
        saving.progress = None;
        self.replacement_active = true;
        let replacement = Replacement {
            start,
            then: saving.of.then.take(),
        };
        self.saving = Some(saving);
        cx.notify();
        cx.spawn_in(window, async move |shell, cx| {
            old.wait().await;
            let committed = cx
                .background_spawn(async move {
                    staged.commit().map_err(|refused| {
                        let error = saving::message(&refused.error);
                        // The written edits are kept beside the file rather than lost with the refusal.
                        (error, refused.keep_beside())
                    })
                })
                .await;
            let _ = shell.update_in(cx, |shell, window, cx| {
                shell.replaced(committed, replacement, window, cx);
            });
        })
        .detach();
    }

    /// Put unstarted readers of `origin` under the shown picture, answering their start and how the old ones go.
    fn swap_readers(
        &mut self,
        origin: &crate::document::Origin,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<(crate::analysis::Start, crate::release::Released)> {
        let readers = self.readers(origin, window, cx);
        readers.analyst.keep_picture();
        let file = self.file.as_mut()?;
        let old = std::mem::replace(&mut file.released, readers.released);
        file.state.analyst = readers.analyst;
        file._updates = readers.pump;
        file.lease = readers.lease;
        file._minimap_updates = None;
        file._edit_tasks.clear();
        Some((readers.start, old))
    }

    /// Read the shown capture from the copy saved at `path` from now on, the picture staying.
    pub(super) fn rebind_saved_as(
        &mut self,
        path: std::path::PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let origin = crate::document::Origin::new(path);
        let Some((start, _)) = self.swap_readers(&origin, window, cx) else {
            return;
        };
        let Some(file) = self.file.as_mut() else {
            return;
        };
        file.state.document.set_origin(origin.clone());
        let shown = file.state.document.shown();
        file.rebinding = file.editing.take().map(|editing| Rebinding {
            to: Rebind::Fresh(Box::new(editing)),
            shown,
        });
        start.start();
        self.remember_file(&origin);
        self.refresh_titles(window);
        cx.notify();
    }

    /// Move the clipboard onto the replaced file, answering whether it survived.
    fn move_clipboard(&mut self, editing: &Editing, saved: &argand_io::write::Saved) -> bool {
        let Some(clipboard) = &self.clipboard else {
            return true;
        };
        let mut written = editing.written_source();
        written.stamp = saved.stamp;
        let moved = clipboard.moved_onto(editing.file(), editing.capture(), &written);
        let kept = moved.is_some();
        self.clipboard = moved;
        kept
    }

    /// Start the readers again on the file as it now is, the document and its picture staying.
    fn replaced(
        &mut self,
        committed: Result<argand_io::write::Saved, (String, std::path::PathBuf)>,
        replacement: Replacement,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Replacement { start, then } = replacement;
        self.saving = None;
        self.replacement_active = false;
        let to = match committed {
            Ok(saved) => self.adopted(&saved),
            Err((error, kept)) => self.kept_edits(&error, &kept, window, cx),
        };
        if let Some(file) = self.file.as_mut() {
            let shown = file.state.document.shown();
            file.rebinding = Some(Rebinding { to, shown });
        }
        start.start();
        self.refresh_titles(window);
        if let Some(pending) = then {
            self.carry_out(pending, window, cx);
        }
        cx.notify();
    }

    /// The file now holds the edits, so they are saved and the clipboard follows its samples.
    fn adopted(&mut self, saved: &argand_io::write::Saved) -> Rebind {
        let Some(editing) = self.file.as_mut().and_then(|file| file.editing.take()) else {
            return Rebind::Keep;
        };
        let kept_clipboard = self.move_clipboard(&editing, saved);
        let name = editing
            .file()
            .meta
            .source
            .file_name()
            .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
        self.save_notice = Some(if kept_clipboard {
            saving_ui::Notice::Saved(name)
        } else {
            saving_ui::Notice::Saved(format!("{name}, the clipboard was cleared"))
        });
        tracing::info!(path = %saved.path.display(), "saved over the open file");
        Rebind::Fresh(Box::new(editing))
    }

    /// The file was not replaced, so the edits stay on it and their envelopes are built again.
    fn kept_edits(
        &mut self,
        error: &str,
        kept: &std::path::Path,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Rebind {
        let name = kept.file_name().map_or_else(
            || kept.display().to_string(),
            |name| name.to_string_lossy().into_owned(),
        );
        self.save_notice = Some(saving_ui::Notice::Failed(
            "Save failed",
            format!("{error}, the edits were written to {name}"),
        ));
        let mut missing = Vec::new();
        if let Some(file) = self.file.as_mut()
            && let Some(editing) = file.editing.as_mut()
        {
            editing.restart_envelopes();
            missing = editing.missing_envelopes();
            file.state.analyst.set_edit(editing.edit_state());
        }
        for (id, source) in missing {
            self.build_envelope(id, source, window, cx);
        }
        Rebind::Keep
    }

    /// The readers started under the shown picture have opened their file.
    pub(super) fn rebound(
        &mut self,
        rebinding: Rebinding,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Rebinding { to, shown } = rebinding;
        let ready = shown.is_ready();
        if let Some(file) = self.file.as_mut() {
            file.state.document.keep_shown(shown);
        }
        match to {
            Rebind::Fresh(previous) => {
                self.start_editing(cx);
                let same = self
                    .file
                    .as_ref()
                    .and_then(|file| file.state.document.meta())
                    .is_some_and(|meta| previous.scaled_as(meta));
                let envelope = previous.minimap().filter(|_| same).map(Arc::new);
                let complete = envelope.as_ref().is_some_and(|envelope| envelope.complete);
                if let (Some(editing), Some(envelope)) = (self.editing_mut(), envelope) {
                    editing.set_envelope(argand_edit::SourceId(0), envelope);
                }
                if !complete {
                    self.start_minimap(true, window, cx);
                }
                // Samples stored on another scale make another picture, drawn before it replaces this one.
                if !same {
                    self.ask_for_a_picture();
                }
            }
            Rebind::Keep => self.kept_on_file(window, cx),
        }
        self.refresh_titles(window);
        // A picture finished before the save is the picture of these samples, so it is not drawn again.
        if !ready {
            self.ask_for_a_picture();
        }
        cx.notify();
    }

    /// The file opened again under edits that failed to replace it, whose length is the edited one.
    fn kept_on_file(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(file) = self.file.as_mut() else {
            return;
        };
        let stamp = file.state.document.stamp();
        let Some(editing) = file.editing.as_mut() else {
            return;
        };
        if editing.file().stamp != stamp {
            // The file changed under the edits, which no longer say where its samples are.
            let origin = file.state.document.origin().clone();
            tracing::warn!(path = %origin.path.display(), "the file changed while it was being replaced");
            file.editing = None;
            self.open(origin, window, cx);
            return;
        }
        let complete = editing.file_envelope_complete();
        file.state.document.set_len(editing.len());
        if !complete {
            self.start_minimap(true, window, cx);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::TestAppContext;
    use gpui_kit::test::TestWindowExt;
    use std::path::{Path, PathBuf};

    fn open_window(cx: &mut TestAppContext) -> (Entity<Shell>, &mut gpui_kit::VisualTestContext) {
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
        let (shell, cx) = cx.add_window_view(|window, cx| {
            Shell::new(Config::default(), None, Session::default(), window, cx)
        });
        cx.simulate_resize(gpui_kit::size(px(800.), px(600.)));
        (shell, cx)
    }

    /// A 16-bit I/Q WAVE file whose sample `n` holds `n` in both channels.
    fn write_wave(path: &Path, samples: u32) {
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
    fn wait_until(
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
        cx.simulate_new_path_selection(move |_| Some(chosen));
        wait_until(cx, &shell, "the band to be saved", |shell| {
            shell.saving.is_none() && matches!(shell.save_notice, Some(saving_ui::Notice::Saved(_)))
        });
        let saved = argand_io::open(&target, &argand_io::OpenHints::default()).unwrap();
        let meta = saved.meta();
        assert_eq!(meta.sample_rate, 24_000. / 9.);
        assert_eq!(meta.center_freq, 2_000.);
        assert_eq!(meta.len_samples, 48_000u64.div_ceil(9));
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
}
