//! Edit commands, the unsaved state and what the window does about it.

use super::*;
use crate::editing::{Editing, Placement};
use argand_core::SampleSpan;
use argand_edit::SourceId;

actions!(
    edit,
    [
        Undo,
        Redo,
        CutSelection,
        CopySelection,
        PasteClipboard,
        PasteHere,
        DeleteSelection
    ]
);

pub(super) fn init(cx: &mut gpui_kit::App) {
    let command = |key: &str| {
        if cfg!(target_os = "macos") {
            format!("cmd-{key}")
        } else {
            format!("ctrl-{key}")
        }
    };
    let mut keys = vec![
        KeyBinding::new(&command("z"), Undo, Some("Plot")),
        KeyBinding::new(&command("shift-z"), Redo, Some("Plot")),
        // A capture edited down to nothing shows no plot, and its edits must still be undone.
        KeyBinding::new(&command("z"), Undo, Some("Shell")),
        KeyBinding::new(&command("shift-z"), Redo, Some("Shell")),
        KeyBinding::new(&command("x"), CutSelection, Some("Plot")),
        KeyBinding::new(&command("c"), CopySelection, Some("Plot")),
        KeyBinding::new(&command("v"), PasteClipboard, Some("Plot")),
        KeyBinding::new("delete", DeleteSelection, Some("Plot")),
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

/// A save whose target the window opens once it is written, keeping where it looked.
pub(super) struct Reopening {
    pub path: std::path::PathBuf,
    pub view: Option<crate::navigation::View>,
    pub selection: Option<SampleSpan>,
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
}

impl Shell {
    pub(super) fn editing(&self) -> Option<&Editing> {
        self.file.as_ref()?.editing.as_ref()
    }

    fn editing_mut(&mut self) -> Option<&mut Editing> {
        self.file.as_mut()?.editing.as_mut()
    }

    pub(super) fn edit_commands_available(&self) -> EditCommands {
        let Some(editing) = self.editing() else {
            return EditCommands::default();
        };
        let selected = self.selection.is_some();
        let clip = self.clipboard.is_some();
        EditCommands {
            undo: editing.can_undo(),
            redo: editing.can_redo(),
            cut: selected,
            paste: clip,
            replace: clip && selected,
        }
    }

    /// Start keeping edits for the file that has just described itself, answering whether earlier ones came back.
    ///
    /// Edits kept aside while their file failed to be replaced come back when it is still that file.
    pub(super) fn start_editing(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(file) = &mut self.file else {
            return false;
        };
        let Some(meta) = file.state.document.meta().cloned() else {
            return false;
        };
        let hints = file.state.document.origin().hints.clone();
        let stamp = file.state.document.stamp();
        let source = argand_io::write::SourceFile {
            meta: meta.clone(),
            hints: hints.clone(),
            stamp,
        };
        if let Some(clipboard) = &mut self.clipboard {
            clipboard.adopt(&editing_source(&source));
        }
        let restored = self.restoring.take().filter(|kept| {
            kept.file().meta.source == meta.source && stamp.is_some() && kept.file().stamp == stamp
        });
        if let Some(mut kept) = restored {
            kept.restart_envelopes();
            file.editing = Some(kept);
            return true;
        }
        let mut editing = Editing::new(meta, hints);
        editing.describe_file(stamp, None);
        file.editing = Some(editing);
        let document = file.id;
        let opened = editing_source(&source);
        // Reading the header may touch a network mount, so it happens away from the window.
        let storage = cx.background_spawn(async move { argand_io::write::storage(&source).ok() });
        cx.spawn(async move |shell, cx| {
            let storage = storage.await;
            let _ = shell.update(cx, |shell, cx| {
                shell.file_stored(document, &opened, storage, cx);
            });
        })
        .detach();
        false
    }

    /// Record how the file of `document` stores its samples, in its edits and in a clipboard copied from it.
    fn file_stored(
        &mut self,
        document: u64,
        opened: &crate::editing::Source,
        storage: Option<argand_io::write::Storage>,
        cx: &mut Context<Self>,
    ) {
        // A clipboard copied from the file outlives its document, so it learns either way.
        if let Some(clipboard) = &mut self.clipboard {
            clipboard.describe(opened, storage);
        }
        if let Some(editing) = self
            .file
            .as_mut()
            .filter(|file| file.id == document)
            .and_then(|file| file.editing.as_mut())
        {
            editing.describe_file(opened.stamp, storage);
        }
        cx.notify();
    }

    /// Bring back the view and selection of a capture saved and opened again.
    pub(super) fn restore_reopened(&mut self) {
        let Some(reopening) = self.reopening.take() else {
            return;
        };
        let Some(total) = self.sample_count() else {
            return;
        };
        if let Some(view) = reopening.view.filter(|view| view.start + view.len <= total) {
            self.view = Some(view);
        }
        self.selection = reopening.selection.and_then(|span| span.within(total));
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
        let Some(span) = self.selection else { return };
        let Some(editing) = self.editing_mut() else {
            return;
        };
        editing.delete(span);
        self.edited(window, cx);
    }

    fn copy_selection(&mut self, cx: &mut Context<Self>) {
        let Some(span) = self.selection else { return };
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
        if self.editing_mut().is_some_and(Editing::undo) {
            self.edited(window, cx);
        }
    }

    fn redo(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.editing_mut().is_some_and(Editing::redo) {
            self.edited(window, cx);
        }
    }

    /// Show the current version: its length, picture, minimap, selection and titles.
    pub(super) fn edited(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(file) = &mut self.file else { return };
        let Some(editing) = &mut file.editing else {
            return;
        };
        let len = editing.len();
        let selection = editing.selection();
        let missing = editing.missing_envelopes();
        let state = editing.edit_state();
        file.state.document.set_len(len);
        // The old picture puts other samples where it shows them, so nothing is shown until the new one.
        file.state.document.forget_picture();
        file.state.analyst.set_edit(state);
        self.selection = selection.and_then(|span| span.within(len));
        self.release_picture(window, cx);
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
    fn build_envelope(
        &mut self,
        id: SourceId,
        source: crate::editing::Source,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let origin = Origin {
            path: source.meta.source.clone(),
            hints: source.hints.clone(),
        };
        let Some(lease) = self.file.as_ref().map(|file| file.lease.clone()) else {
            return;
        };
        let updates = crate::minimap::start(origin, source.meta, lease);
        let task = cx.spawn_in(window, async move |shell, cx| {
            while let Ok(update) = updates.recv().await {
                let Ok(snapshot) = update else { break };
                let applied =
                    shell.update_in(cx, |shell, _, cx| shell.envelope_ready(id, snapshot, cx));
                if applied.is_err() {
                    break;
                }
            }
        });
        if let Some(file) = &mut self.file {
            file._edit_tasks.push(task);
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

    /// Whether `pending` can go ahead now; otherwise ask about the unsaved edits first.
    pub(super) fn settle_unsaved(
        &mut self,
        pending: Pending,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.refuse_while_replacing(cx) {
            return false;
        }
        if !self.editing().is_some_and(Editing::is_dirty) {
            return true;
        }
        let name = self
            .file
            .as_ref()
            .map(|file| file.state.document.origin().name())
            .unwrap_or_default();
        let answer = window.prompt(
            gpui_kit::PromptLevel::Warning,
            &format!("Save the edits to {name}?"),
            Some("They are lost otherwise."),
            &["Save", "Discard", "Cancel"],
            cx,
        );
        cx.spawn_in(window, async move |shell, cx| {
            let Ok(answer) = answer.await else { return };
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

    /// Do what was waiting, the unsaved edits having been answered for.
    pub(super) fn carry_out(
        &mut self,
        pending: Pending,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(editing) = self.editing_mut() {
            editing.mark_saved();
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
                if shell.settle_unsaved(Pending::Close, window, cx) {
                    window.remove_window();
                }
            }))
            .on_action(cx.listener(|shell, _: &Undo, window, cx| shell.undo(window, cx)))
            .on_action(cx.listener(|shell, _: &Redo, window, cx| shell.redo(window, cx)))
            .on_action(cx.listener(|shell, _: &CutSelection, window, cx| {
                shell.cut_selection(window, cx);
            }))
            .on_action(cx.listener(|shell, _: &CopySelection, _, cx| shell.copy_selection(cx)))
            .on_action(cx.listener(|shell, _: &PasteClipboard, window, cx| {
                if let Some(span) = shell.selection {
                    shell.paste(Placement::Replace(span), window, cx);
                }
            }))
            .on_action(cx.listener(|shell, _: &PasteHere, window, cx| shell.paste_here(window, cx)))
            .on_action(cx.listener(|shell, _: &DeleteSelection, window, cx| {
                shell.delete_selection(window, cx);
            }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::{FileInfo, Update};
    use argand_core::{Domain, SampleFormat, SampleType, SignalMeta};
    use gpui_kit::TestAppContext;
    use std::path::PathBuf;

    fn open_window(cx: &mut TestAppContext) -> (Entity<Shell>, &mut gpui_kit::VisualTestContext) {
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
        cx.run_until_parked();
        (shell, cx)
    }

    /// A capture that has described itself, with its edits started as an opened file's are.
    fn described(cx: &mut gpui_kit::VisualTestContext, shell: &Entity<Shell>, path: &str) {
        shell.update_in(cx, |shell, window, cx| {
            shell.open(Origin::new(PathBuf::from(path)), window, cx);
            let Some(file) = shell.file.as_mut() else {
                panic!("the capture never opened");
            };
            file.state.document.apply(Update::Opened(
                SignalMeta {
                    sample_rate: 24_000.0,
                    center_freq: 0.0,
                    sample_type: SampleType::new(Domain::Iq, SampleFormat::I16),
                    len_samples: 48_000,
                    container: "raw",
                    divisor: 32_768.0,
                    source: PathBuf::from(path),
                },
                FileInfo::default(),
            ));
            shell.start_editing(cx);
            shell.view = Some(crate::navigation::View::full(48_000));
        });
        cx.run_until_parked();
    }

    fn length(cx: &mut gpui_kit::VisualTestContext, shell: &Entity<Shell>) -> Option<u64> {
        shell.read_with(cx, |shell, _| shell.sample_count())
    }

    fn select(cx: &mut gpui_kit::VisualTestContext, shell: &Entity<Shell>, a: u64, b: u64) {
        shell.update_in(cx, |shell, _, _| {
            shell.selection = SampleSpan::between(a, b)
        });
    }

    #[gpui_kit::test]
    fn delete_undo_and_redo_follow_the_length_selection_and_title(cx: &mut TestAppContext) {
        let (shell, cx) = open_window(cx);
        described(cx, &shell, "/captures/a.iqw");
        select(cx, &shell, 1000, 3000);
        let can = shell.read_with(cx, |shell, _| shell.edit_commands_available());
        assert!(can.cut && !can.undo && !can.paste);
        shell.update_in(cx, |shell, window, cx| shell.delete_selection(window, cx));
        assert_eq!(length(cx, &shell), Some(46_000));
        assert!(shell.read_with(cx, |shell, _| shell.selection.is_none()));
        assert!(shell.read_with(cx, |shell, _| shell.file_name().starts_with('•')));
        shell.update_in(cx, |shell, window, cx| shell.undo(window, cx));
        assert_eq!(length(cx, &shell), Some(48_000));
        assert!(!shell.read_with(cx, |shell, _| shell.file_name().starts_with('•')));
        shell.update_in(cx, |shell, window, cx| shell.redo(window, cx));
        assert_eq!(length(cx, &shell), Some(46_000));
    }

    #[gpui_kit::test]
    fn copy_and_replace_selection_paste_and_select_the_pasted_samples(cx: &mut TestAppContext) {
        let (shell, cx) = open_window(cx);
        described(cx, &shell, "/captures/a.iqw");
        select(cx, &shell, 0, 100);
        shell.update_in(cx, |shell, _, cx| shell.copy_selection(cx));
        select(cx, &shell, 1000, 3000);
        assert!(shell.read_with(cx, |shell, _| shell.edit_commands_available().replace));
        shell.update_in(cx, |shell, window, cx| {
            shell.paste(
                Placement::Replace(SampleSpan::between(1000, 3000).unwrap()),
                window,
                cx,
            );
        });
        assert_eq!(length(cx, &shell), Some(46_100));
        assert_eq!(
            shell.read_with(cx, |shell, _| shell.selection),
            SampleSpan::between(1000, 1100)
        );
    }

    #[gpui_kit::test]
    fn opening_another_file_asks_about_unsaved_edits_first(cx: &mut TestAppContext) {
        let (shell, cx) = open_window(cx);
        described(cx, &shell, "/captures/a.iqw");
        select(cx, &shell, 0, 100);
        shell.update_in(cx, |shell, window, cx| shell.delete_selection(window, cx));
        let opened = |cx: &mut gpui_kit::VisualTestContext| {
            shell.read_with(cx, |shell, _| {
                shell
                    .file
                    .as_ref()
                    .map(|file| file.state.document.origin().path.clone())
            })
        };
        shell.update_in(cx, |shell, window, cx| {
            shell.open(Origin::new(PathBuf::from("/captures/b.iqw")), window, cx);
        });
        assert!(cx.has_pending_prompt());
        cx.simulate_prompt_answer("Cancel");
        cx.run_until_parked();
        assert_eq!(opened(cx), Some(PathBuf::from("/captures/a.iqw")));
        shell.update_in(cx, |shell, window, cx| {
            shell.open(Origin::new(PathBuf::from("/captures/b.iqw")), window, cx);
        });
        cx.simulate_prompt_answer("Discard");
        cx.run_until_parked();
        assert_eq!(opened(cx), Some(PathBuf::from("/captures/b.iqw")));
    }

    #[gpui_kit::test]
    fn the_close_button_asks_about_unsaved_edits(cx: &mut TestAppContext) {
        let (shell, cx) = open_window(cx);
        described(cx, &shell, "/captures/a.iqw");
        select(cx, &shell, 0, 100);
        shell.update_in(cx, |shell, window, cx| shell.delete_selection(window, cx));
        cx.update(|window, cx| window.dispatch_action(Box::new(chrome::CloseWindow), cx));
        cx.run_until_parked();
        assert!(cx.has_pending_prompt(), "the window asks before closing");
        cx.simulate_prompt_answer("Cancel");
    }

    #[gpui_kit::test]
    fn a_save_finished_after_more_edits_keeps_the_document_and_its_unsaved_state(
        cx: &mut TestAppContext,
    ) {
        let (shell, cx) = open_window(cx);
        described(cx, &shell, "/captures/a.iqw");
        select(cx, &shell, 0, 100);
        shell.update_in(cx, |shell, window, cx| shell.delete_selection(window, cx));
        let (document, version) = shell.read_with(cx, |shell, _| {
            let file = shell.file.as_ref().map_or(0, |file| file.id);
            (file, shell.editing().map_or(0, Editing::version))
        });
        select(cx, &shell, 0, 100);
        shell.update_in(cx, |shell, window, cx| shell.delete_selection(window, cx));
        shell.update_in(cx, |shell, window, cx| {
            shell.saved(
                PathBuf::from("/captures/saved.iqw"),
                saving_ui::SaveOf::whole_for_test(document, version),
                window,
                cx,
            );
        });
        cx.run_until_parked();
        let (path, dirty) = shell.read_with(cx, |shell, _| {
            (
                shell
                    .file
                    .as_ref()
                    .map(|file| file.state.document.origin().path.clone()),
                shell.editing().is_some_and(Editing::is_dirty),
            )
        });
        assert_eq!(
            path,
            Some(PathBuf::from("/captures/a.iqw")),
            "the window stays on its capture"
        );
        assert!(dirty, "the later edit is still unsaved");
    }

    #[gpui_kit::test]
    fn undo_reaches_a_capture_edited_down_to_nothing(cx: &mut TestAppContext) {
        let (shell, cx) = open_window(cx);
        described(cx, &shell, "/captures/a.iqw");
        select(cx, &shell, 0, 48_000);
        shell.update_in(cx, |shell, window, cx| {
            shell.delete_selection(window, cx);
            window.focus(&shell.focus, cx);
        });
        assert_eq!(length(cx, &shell), Some(0));
        cx.simulate_keystrokes(if cfg!(target_os = "macos") {
            "cmd-z"
        } else {
            "ctrl-z"
        });
        cx.run_until_parked();
        assert_eq!(length(cx, &shell), Some(48_000));
    }
}
