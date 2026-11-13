//! Edit commands, the unsaved state and what the window does about it.

use super::*;
use crate::editing::{Editing, Placement};
use argand_edit::SourceId;
use gpui_kit::SharedString;
use gpui_kit::component::{WindowExt as _, h_flex};

actions!(
    edit,
    [
        Undo,
        Redo,
        CutSelection,
        CopySelection,
        PasteClipboard,
        PasteHere,
        DeleteSelection,
        SuppressBand,
        InvertSpectrum,
        NormalizeLevel,
        EditSelection
    ]
);

pub(super) fn init(cx: &mut gpui_kit::App) {
    macro_rules! command {
        ($key:literal) => {
            if cfg!(target_os = "macos") {
                concat!("cmd-", $key)
            } else {
                concat!("ctrl-", $key)
            }
        };
    }
    let mut keys = vec![
        KeyBinding::new(command!("z"), Undo, Some("Plot")),
        KeyBinding::new(command!("shift-z"), Redo, Some("Plot")),
        // A capture edited down to nothing shows no plot, and its edits must still be undone.
        KeyBinding::new(command!("z"), Undo, Some("Shell")),
        KeyBinding::new(command!("shift-z"), Redo, Some("Shell")),
        KeyBinding::new(command!("x"), CutSelection, Some("Plot")),
        KeyBinding::new(command!("c"), CopySelection, Some("Plot")),
        KeyBinding::new(command!("v"), PasteClipboard, Some("Plot")),
        KeyBinding::new("delete", DeleteSelection, Some("Plot")),
        KeyBinding::new(command!("shift-m"), SuppressBand, Some("Plot")),
        KeyBinding::new(command!("shift-i"), InvertSpectrum, Some("Plot")),
        KeyBinding::new(command!("shift-n"), NormalizeLevel, Some("Plot")),
        KeyBinding::new(command!("e"), EditSelection, Some("Plot")),
        // The panel holds the keyboard while it is open, and the same keys close it.
        KeyBinding::new(command!("e"), EditSelection, Some("SelectionPanel")),
    ];
    if cfg!(target_os = "windows") {
        keys.push(KeyBinding::new("ctrl-y", Redo, Some("Plot")));
        keys.push(KeyBinding::new("ctrl-y", Redo, Some("Shell")));
    }
    cx.bind_keys(keys);
}

/// The file a capture was opened from, as the edits and the clipboard know it.
fn editing_source(file: &argand_io::write::SourceFile) -> crate::editing::Source {
    let mut source = crate::editing::Source::new(file.meta.clone(), file.hints.clone());
    source.stamp = file.stamp;
    source
}

/// What waits for an answer about unsaved edits.
#[derive(Clone)]
pub(super) enum Pending {
    Open(Origin),
    Close,
}

/// Which edit commands can act now, for the menus.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct EditCommands {
    pub undo: bool,
    pub redo: bool,
    pub cut: bool,
    pub paste: bool,
    pub replace: bool,
    pub suppress: bool,
    pub invert: bool,
    pub normalize: bool,
    /// Whether the spectrum is read inverted, which the menu row is checked by.
    pub inverted: bool,
}

/// What the unsaved-edits question asks.
const UNSAVED_QUESTION: &str = "Save the edits?";
/// What it asks when the edits cannot be saved, the file under them having changed.
const CHANGED_QUESTION: &str = "Discard the edits?";

impl Shell {
    pub(super) fn editing(&self) -> Option<&Editing> {
        self.file.as_ref()?.editing.as_ref()
    }

    /// The edits to change, none while the file under them is being replaced or has changed.
    pub(super) fn editing_mut(&mut self) -> Option<&mut Editing> {
        if self.replacement_active || self.capture_changed() || self.computing() {
            return None;
        }
        self.file.as_mut()?.editing.as_mut()
    }

    pub(super) fn edit_commands_available(&self) -> EditCommands {
        let idle = !self.capture_changed() && !self.computing();
        let Some(editing) = self.editing().filter(|_| idle) else {
            return EditCommands::default();
        };
        let selected = self.time_selection().is_some();
        let clip = self.clipboard.is_some();
        EditCommands {
            undo: editing.can_undo(),
            redo: editing.can_redo(),
            cut: selected,
            paste: clip,
            replace: clip && selected,
            suppress: self.can_suppress(),
            invert: self.can_invert(),
            normalize: self.can_normalize(),
            inverted: editing.adjustment().invert(),
        }
    }

    /// Start keeping edits for the file that has just described itself.
    pub(super) fn start_editing(&mut self, cx: &mut Context<Self>) {
        let Some(file) = &mut self.file else { return };
        let Some(meta) = file.state.document.meta().cloned() else {
            return;
        };
        let hints = file.state.document.origin().hints.clone();
        let stamp = file.state.document.stamp();
        let source = argand_io::write::SourceFile {
            meta: meta.clone(),
            hints: hints.clone(),
            stamp,
            encoding: None,
        };
        if let Some(clipboard) = &mut self.clipboard {
            clipboard.adopt(&editing_source(&source));
        }
        let mut editing = match &file.unsaved {
            Some(unsaved) => Editing::unsaved(meta, hints, unsaved.backing.clone()),
            None => Editing::new(meta, hints),
        };
        editing.describe_format(stamp, None);
        editing.describe_units(file.state.document.sample_units());
        file.editing = Some(editing);
        let document = file.id;
        let opened = editing_source(&source);
        // Reading the header may touch a network mount, so it happens away from the window.
        let format = cx.background_spawn(async move { argand_io::write::format(&source).ok() });
        cx.spawn(async move |shell, cx| {
            let format = format.await;
            let _ = shell.update(cx, |shell, cx| {
                shell.file_stored(document, &opened, format, cx);
            });
        })
        .detach();
    }

    /// Record how the file of `document` stores its samples, in its edits and in a clipboard copied from it.
    pub(super) fn file_stored(
        &mut self,
        document: u64,
        opened: &crate::editing::Source,
        format: Option<argand_io::write::Format>,
        cx: &mut Context<Self>,
    ) {
        // A clipboard copied from the file outlives its document, so it learns either way.
        if let Some(clipboard) = &mut self.clipboard {
            clipboard.describe(
                opened,
                format.as_ref().map(argand_io::write::Format::storage),
            );
        }
        if let Some(editing) = self
            .file
            .as_mut()
            .filter(|file| file.id == document)
            .and_then(|file| file.editing.as_mut())
            // A save rebinds the document to its saved file, whose stamp this must not undo.
            .filter(|editing| {
                editing.file().meta.source == opened.meta.source
                    && editing.file().stamp == opened.stamp
            })
        {
            editing.describe_format(opened.stamp, format);
        }
        cx.notify();
    }

    /// The minimap of what is shown, composed when the capture was edited.
    pub(super) fn refresh_waveform(&mut self) {
        let Some(editing) = self.editing() else {
            return;
        };
        self.waveform = editing
            .minimap()
            .map(|snapshot| Arc::new(waveform::Waveform::new(Arc::new(snapshot))));
    }

    fn delete_selection(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(span) = self.time_selection() else {
            return;
        };
        let Some(editing) = self.editing_mut() else {
            return;
        };
        editing.delete(span);
        self.edited(window, cx);
    }

    fn copy_selection(&mut self, cx: &mut Context<Self>) {
        let Some(span) = self.time_selection() else {
            return;
        };
        let Some(editing) = self.editing() else {
            return;
        };
        self.clipboard = Some(editing.copy(span));
        cx.notify();
    }

    fn cut_selection(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.copy_selection(cx);
        self.delete_selection(window, cx);
    }

    fn paste(&mut self, placement: Placement, window: &mut Window, cx: &mut Context<Self>) {
        let Some(clipboard) = self.clipboard.clone() else {
            return;
        };
        let Some(editing) = self.editing_mut() else {
            return;
        };
        match editing.paste(&clipboard, placement) {
            Ok(_) => self.edited(window, cx),
            Err(error) => {
                self.save_notice =
                    Some(saving_ui::Notice::Failed("Cannot paste", error.to_string()));
                cx.notify();
            }
        }
    }

    fn paste_here(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(at) = self.plot_view().and_then(|plot| plot.read(cx).paste_point) else {
            return;
        };
        self.paste(Placement::At(at), window, cx);
    }

    fn undo(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.history_step(false, window, cx);
    }

    fn redo(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.history_step(true, window, cx);
    }

    fn history_step(&mut self, redo: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(editing) = self.editing_mut() else {
            return;
        };
        let before = editing.capture().clone();
        let changed = if redo { editing.redo() } else { editing.undo() };
        if !changed {
            return;
        }
        if *editing.capture() == before {
            let marked = editing.marked();
            self.selection = marked.time;
            self.band = marked.band;
        } else {
            self.edited(window, cx);
        }
        self.reference_changed(window, cx);
    }

    pub(super) fn set_reference(
        &mut self,
        reference: f64,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let selection = argand_core::Selection {
            time: self.selection,
            band: self.band,
        };
        let changed = self
            .editing_mut()
            .is_some_and(|editing| editing.set_reference(reference, selection));
        if changed {
            self.reference_changed(window, cx);
        }
        changed
    }

    fn reference_changed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.frequency_scheme = None;
        self.refresh_titles(window);
        cx.notify();
    }

    /// Show the current version: its length, picture, minimap, selection and titles.
    pub(super) fn edited(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(file) = &mut self.file else { return };
        let Some(editing) = &mut file.editing else {
            return;
        };
        let len = editing.len();
        let marked = editing.marked();
        let missing = editing.missing_envelopes();
        let state = editing.edit_state();
        let before = editing.repicture();
        file.state.document.set_len(len);
        file.state.document.follow_edit(before);
        file.state.analyst.set_edit(state);
        self.selection = marked.time.and_then(|span| span.within(len));
        self.band = marked.band;
        // The wider picture is not moved with the edit, so it goes until the new one is drawn.
        self.release_backdrop(window, cx);
        self.upload_pending = true;
        self.time_scheme = None;
        self.tick_pan = None;
        self.bound_view(cx);
        self.refresh_waveform();
        for (id, source) in missing {
            self.build_envelope(id, source, window, cx);
        }
        self.refresh_titles(window);
        self.ask_for_a_picture();
        cx.notify();
    }

    /// Build the minimap envelope of a file pasted into the capture.
    pub(super) fn build_envelope(
        &mut self,
        id: SourceId,
        source: crate::editing::Source,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(lease) = self.file.as_ref().map(|file| file.lease.clone()) else {
            return;
        };
        let stamp = source.stamp;
        let updates = crate::minimap::start(source.source_file(), lease);
        let task = cx.spawn_in(window, async move |shell, cx| {
            while let Ok(update) = updates.recv().await {
                let more =
                    shell.update(cx, |shell, cx| shell.envelope_update(id, stamp, update, cx));
                if !more.is_ok_and(|more| more) {
                    break;
                }
            }
        });
        if let Some(file) = &mut self.file {
            file._edit_tasks.push(task);
        }
    }

    /// Take one update of a pasted file's envelope, answering whether more can follow.
    fn envelope_update(
        &mut self,
        id: SourceId,
        stamp: Option<argand_io::write::SourceStamp>,
        update: anyhow::Result<Arc<crate::minimap::Snapshot>>,
        cx: &mut Context<Self>,
    ) -> bool {
        match update {
            Ok(snapshot) => {
                self.envelope_ready(id, snapshot, cx);
                true
            }
            Err(error) => {
                tracing::warn!(%error, "pasted file's envelope unavailable");
                self.envelope_failed(id, stamp, &error, cx);
                false
            }
        }
    }

    /// Note a pasted file's failed envelope, while the capture still reads that file.
    pub(super) fn envelope_failed(
        &mut self,
        id: SourceId,
        stamp: Option<argand_io::write::SourceStamp>,
        error: &anyhow::Error,
        cx: &mut Context<Self>,
    ) {
        let Some(file) = &mut self.file else { return };
        if let Some(editing) = &mut file.editing
            && editing
                .retained_sources()
                .get(id.0 as usize)
                .is_some_and(|source| source.stamp == stamp)
        {
            editing.fail_statistics(id);
        }
        let read = file.editing.as_ref().is_some_and(|editing| {
            editing
                .source_files()
                .get(id.0 as usize)
                .and_then(Option::as_ref)
                .is_some_and(|source| source.stamp == stamp)
        });
        if read {
            file.state.document.note_failure(error);
            file.state
                .document
                .set_statistics(Some(Err("Unavailable".into())));
            cx.notify();
        }
    }

    fn envelope_ready(
        &mut self,
        id: SourceId,
        snapshot: Arc<crate::minimap::Snapshot>,
        cx: &mut Context<Self>,
    ) {
        if let Some(editing) = self.editing_mut() {
            editing.set_envelope(id, snapshot);
        }
        self.refresh_waveform();
        cx.notify();
    }

    /// Name the window after its file, marking unsaved edits.
    pub(super) fn refresh_titles(&self, window: &mut Window) {
        let name = self.file_name();
        if name.is_empty() {
            window.set_window_title(TITLE);
        } else {
            window.set_window_title(&format!("{name} – {TITLE}"));
        }
    }

    pub(super) fn request_close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.settle_unsaved(Pending::Close, window, cx) {
            window.remove_window();
        }
    }

    /// Whether `pending` can go ahead now; otherwise ask about the unsaved edits first.
    pub(super) fn settle_unsaved(
        &mut self,
        pending: Pending,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if matches!(pending, Pending::Close) {
            // Deferred popovers must not cover the unsaved-edits question.
            // Dismiss with their usual keep semantics, without bypassing guards.
            self.cancel_status_select(window, cx);
            self.dismiss_application_menu(window, cx);
            self.close_hints(cx);
            if self.close_after_conversion(window, cx) {
                return false;
            }
        }
        if self.conversion_active() || self.refuse_while_replacing(cx) {
            return false;
        }
        if !self
            .file
            .as_ref()
            .is_some_and(|file| file.unsaved.is_some())
            && !self.editing().is_some_and(Editing::is_dirty)
        {
            return true;
        }
        let name = self
            .file
            .as_ref()
            .map(|file| {
                file.unsaved.as_ref().map_or_else(
                    || file.state.document.origin().name(),
                    |unsaved| unsaved.name.clone(),
                )
            })
            .unwrap_or_default();
        // The writer refuses a changed source, so Save is not offered for one.
        let answer = Self::ask_unsaved(&name, !self.capture_changed(), window, cx);
        cx.spawn_in(window, async move |shell, cx| {
            let Ok(answer) = answer.recv().await else {
                return;
            };
            let _ = shell.update_in(cx, |shell, window, cx| match answer {
                // Save writes over the file where it can, and asks where otherwise.
                0 if shell.can_save_over().is_ok() => {
                    shell.save_over_then(Some(pending), window, cx)
                }
                0 => shell.save_as_then(false, Some(pending), window, cx),
                1 => shell.carry_out(pending, window, cx),
                _ => {}
            });
        })
        .detach();
        false
    }

    /// Ask whether to save the edits, answering 0 to save, 1 to discard and 2 to cancel.
    ///
    /// The system's own dialog asks where there is one, and the toolkit's dialog elsewhere.
    /// Without `savable` only Discard and Cancel are offered.
    fn ask_unsaved(
        name: &str,
        savable: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> async_channel::Receiver<usize> {
        let (answer, answered) = async_channel::bounded(1);
        let (question, detail) = match savable {
            true => (
                UNSAVED_QUESTION,
                format!("The edits to {name} are lost otherwise."),
            ),
            false => (
                CHANGED_QUESTION,
                format!("{name} changed on disk, so its edits cannot be saved."),
            ),
        };
        let native = cfg!(any(target_os = "macos", target_os = "windows"));
        // A window without the toolkit's root, as in a headless test, has no dialog layer.
        if native
            || window
                .root::<gpui_kit::component::Root>()
                .flatten()
                .is_none()
        {
            let (buttons, first): (&[&str], usize) = match savable {
                true => (&["Save", "Discard", "Cancel"], 0),
                false => (&["Discard", "Cancel"], 1),
            };
            let prompt = window.prompt(
                gpui_kit::PromptLevel::Warning,
                question,
                Some(&detail),
                buttons,
                cx,
            );
            cx.spawn(async move |_, _| {
                if let Ok(index) = prompt.await {
                    let _ = answer.send(index + first).await;
                }
            })
            .detach();
            return answered;
        }
        let detail = SharedString::from(detail);
        window.open_dialog(cx, move |dialog, _, _| {
            let reply = |index: usize| {
                let answer = answer.clone();
                move |_: &gpui_kit::ClickEvent, window: &mut Window, cx: &mut gpui_kit::App| {
                    let _ = answer.try_send(index);
                    window.close_dialog(cx);
                }
            };
            let keyed = |index: usize| {
                let answer = answer.clone();
                move |_: &gpui_kit::ClickEvent, _: &mut Window, _: &mut gpui_kit::App| {
                    answer.try_send(index).is_ok()
                }
            };
            let save = savable.then(|| {
                Button::new("save")
                    .primary()
                    .label("Save")
                    .on_click(reply(0))
            });
            dialog
                .title(question)
                .w(px(400.))
                .close_button(false)
                .overlay_closable(false)
                .child(detail.clone())
                .footer(
                    h_flex()
                        .w_full()
                        .justify_between()
                        .child(Button::new("discard").label("Discard").on_click(reply(1)))
                        .child(
                            h_flex()
                                .gap_2()
                                .child(Button::new("cancel").label("Cancel").on_click(reply(2)))
                                .children(save),
                        ),
                )
                .on_ok(keyed(if savable { 0 } else { 1 }))
                .on_cancel(keyed(2))
        });
        answered
    }

    /// Do what was waiting, the unsaved edits having been answered for.
    pub(super) fn carry_out(
        &mut self,
        pending: Pending,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.conversion_active() {
            return;
        }
        // Not `editing_mut`: edits on a changed file are discarded this way too.
        if let Some(file) = self.file.as_mut() {
            if let Some(editing) = file.editing.as_mut() {
                editing.mark_saved();
            }
            file.unsaved = None;
        }
        match pending {
            Pending::Open(origin) => self.open(origin, window, cx),
            Pending::Close => window.remove_window(),
        }
    }

    /// The edit commands, reached from the plot and the menus.
    pub(super) fn edit_commands(
        &self,
        content: gpui_kit::Stateful<gpui_kit::Div>,
        cx: &mut Context<Self>,
    ) -> gpui_kit::Stateful<gpui_kit::Div> {
        content
            .on_action(cx.listener(|shell, _: &chrome::CloseWindow, window, cx| {
                shell.request_close(window, cx);
            }))
            .on_action(cx.listener(|shell, _: &Undo, window, cx| shell.undo(window, cx)))
            .on_action(cx.listener(|shell, _: &Redo, window, cx| shell.redo(window, cx)))
            .on_action(cx.listener(|shell, _: &CutSelection, window, cx| {
                shell.cut_selection(window, cx);
            }))
            .on_action(cx.listener(|shell, _: &CopySelection, _, cx| shell.copy_selection(cx)))
            .on_action(cx.listener(|shell, _: &PasteClipboard, window, cx| {
                if let Some(span) = shell.time_selection() {
                    shell.paste(Placement::Replace(span), window, cx);
                }
            }))
            .on_action(cx.listener(|shell, _: &PasteHere, window, cx| shell.paste_here(window, cx)))
            .on_action(cx.listener(|shell, _: &DeleteSelection, window, cx| {
                shell.delete_selection(window, cx);
            }))
            .on_action(cx.listener(|shell, _: &SuppressBand, window, cx| {
                shell.suppress_band(window, cx);
            }))
            .on_action(cx.listener(|shell, _: &InvertSpectrum, window, cx| {
                shell.invert_spectrum(window, cx);
            }))
            .on_action(cx.listener(|shell, _: &NormalizeLevel, window, cx| {
                shell.normalize(window, cx);
            }))
            .on_action(cx.listener(|shell, _: &EditSelection, window, cx| {
                shell.toggle_selection_panel(window, cx);
            }))
    }
}

#[cfg(test)]
#[path = "editing_ui_test.rs"]
mod tests;
