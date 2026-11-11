//! Save, which writes the edited capture over the file it was opened from.
//!
//! The capture is written to a temporary file while the original is still
//! read. Then every thread reading the original is waited for, the temporary
//! file replaces it, and new readers open it under the picture already shown,
//! which stays. Windows refuses to replace a file that is still mapped, which
//! is why nothing reading it may be left when the rename happens.

use super::*;
use crate::editing::Editing;
use crate::processing::saving;
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

impl Rebinding {}

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
        if self
            .file
            .as_ref()
            .is_some_and(|file| file.unsaved.is_some())
        {
            return Err(Unsavable::Unchecked);
        }
        if !editing.is_dirty() {
            Err(Unsavable::Unchanged)
        } else if self.saving.is_some() || self.computing() {
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
        if self
            .file
            .as_ref()
            .is_some_and(|file| file.unsaved.is_some())
        {
            self.save_as_then(false, then, window, cx);
            return;
        }
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
            _scratch: editing.scratches(),
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
        // A check started on the old readers would compare the saved file with the old stamp.
        file.disk_check = None;
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
                // The samples now come from the saved file, not from the files that changed.
                if let Some(file) = self.file.as_mut() {
                    file.state.document.clear_changed();
                    file.unsaved = None;
                }
                self.start_editing(cx);
                if let Some(origin) = self
                    .file
                    .as_ref()
                    .map(|file| file.state.document.origin().clone())
                {
                    self.remember_file(&origin);
                }
                let same = self
                    .file
                    .as_ref()
                    .and_then(|file| file.state.document.meta())
                    .is_some_and(|meta| previous.scaled_as(meta));
                let envelope = previous.minimap().filter(|_| same).map(Arc::new);
                if let (Some(editing), Some(envelope)) = (self.editing_mut(), envelope) {
                    editing.set_envelope(argand_edit::SourceId(0), envelope);
                }
                // A reused envelope does not index the newly encoded saved source.
                self.start_minimap(true, window, cx);
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
#[path = "replace_ui_test.rs"]
pub(super) mod tests;

#[cfg(test)]
include!("replace_ui_support_test.rs");
