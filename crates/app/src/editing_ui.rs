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
        KeyBinding::new(&command("x"), CutSelection, Some("Plot")),
        KeyBinding::new(&command("c"), CopySelection, Some("Plot")),
        KeyBinding::new(&command("v"), PasteClipboard, Some("Plot")),
        KeyBinding::new("delete", DeleteSelection, Some("Plot")),
    ];
    if cfg!(target_os = "windows") {
        keys.push(KeyBinding::new("ctrl-y", Redo, Some("Plot")));
    }
    cx.bind_keys(keys);
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

    /// Start keeping edits for the file that has just described itself.
    pub(super) fn start_editing(&mut self, cx: &mut Context<Self>) {
        let Some(file) = &mut self.file else { return };
        let Some(meta) = file.state.document.meta().cloned() else {
            return;
        };
        let hints = file.state.document.origin().hints.clone();
        let source = argand_io::write::SourceFile {
            meta: meta.clone(),
            hints: hints.clone(),
            stamp: None,
        };
        file.editing = Some(Editing::new(meta, hints));
        // Metadata may live on a network mount, so it is read away from the window.
        let described = cx.background_spawn(async move {
            let stamp = argand_io::write::SourceStamp::of(&source.meta.source).ok();
            let storage = argand_io::write::storage(&source).ok();
            (stamp, storage)
        });
        let task = cx.spawn(async move |shell, cx| {
            let (stamp, storage) = described.await;
            let _ = shell.update(cx, |shell, _| {
                if let Some(editing) = shell.editing_mut() {
                    editing.describe_file(stamp, storage);
                }
            });
        });
        file._edit_tasks.push(task);
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
                    Some(saving_ui::Notice::Failed(format!("Cannot paste: {error}")));
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
        let Some(editing) = &file.editing else { return };
        let len = editing.len();
        let selection = editing.selection();
        file.state.document.set_len(len);
        file.state.analyst.set_edit(editing.edit_state());
        let missing = editing.missing_envelopes();
        self.selection = selection.and_then(|span| span.within(len));
        self.release_backdrop(window, cx);
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
        let updates = crate::minimap::start(origin, source.meta);
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
            &["Save as…", "Discard", "Cancel"],
            cx,
        );
        cx.spawn_in(window, async move |shell, cx| {
            let Ok(answer) = answer.await else { return };
            let _ = shell.update_in(cx, |shell, window, cx| match answer {
                0 => {
                    shell.after_save = Some(pending);
                    shell.save_as(false, window, cx);
                }
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
}
