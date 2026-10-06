//! Save as and Save selection as, from the dialog to the status bar.

use super::*;
use crate::saving::{self, Outcome, Update};
use gpui_kit::AnyElement;
use gpui_kit::component::IconName;

/// A save in progress.
pub(super) struct Saving {
    job: saving::Job,
    name: String,
    target: std::path::PathBuf,
    progress: Option<(u64, u64)>,
    /// Set when the file being written was asked to open, which waits until it is complete.
    open_refused: bool,
}

/// How the last save ended, shown until it is closed or another save starts.
pub(super) enum Notice {
    Saved(String),
    Failed(String),
}

impl Shell {
    /// Whether a save of the capture, or of its time selection, can start now.
    pub(super) fn can_save(&self, selection_only: bool) -> bool {
        self.saving.is_none()
            && self
                .file
                .as_ref()
                .is_some_and(|file| file.document.meta().is_some())
            && (!selection_only || self.selection.is_some())
    }

    /// Ask the desktop where to save, then start writing there.
    pub(super) fn save_as(
        &mut self,
        selection_only: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.can_save(selection_only) {
            return;
        }
        self.dismiss_application_menu(window, cx);
        self.close_analysis_hint(cx);
        // The release happens over the dialog, so nothing held now may stay pressed.
        self.interrupt_plot(cx);
        window.focus(&self.focus_target(cx), cx);
        cx.notify();
        let Some(file) = &self.file else { return };
        let Some(meta) = file.document.meta().cloned() else {
            return;
        };
        let hints = file.document.origin().hints.clone();
        let span = if selection_only { self.selection } else { None };
        let name = saving::suggested_name(&meta, hints.raw.is_some(), span);
        let chosen = cx.prompt_for_new_path(&saving::directory(&meta.source), Some(&name));
        cx.spawn_in(window, async move |shell, cx| {
            // A cancelled dialog, a platform without one, and a failed dialog all mean no target.
            let Ok(Ok(Some(target))) = chosen.await else {
                return;
            };
            let request = saving::request(&meta, &hints, span, target);
            let _ = shell.update_in(cx, |shell, window, cx| {
                shell.start_save(request, window, cx)
            });
        })
        .detach();
    }

    fn start_save(
        &mut self,
        request: argand_io::write::SaveRequest,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.saving.is_some() {
            return;
        }
        let name = file_name(&request.target);
        let target = request.target.clone();
        let (job, updates) = saving::start(request);
        self.save_updates = Some(cx.spawn_in(window, async move |shell, cx| {
            while let Ok(update) = updates.recv().await {
                if shell
                    .update_in(cx, |shell, _, cx| shell.receive_save(update, cx))
                    .is_err()
                {
                    break;
                }
            }
        }));
        self.save_notice = None;
        self.saving = Some(Saving {
            job,
            name,
            target,
            progress: None,
            open_refused: false,
        });
        cx.notify();
    }

    fn receive_save(&mut self, update: Update, cx: &mut Context<Self>) {
        match update {
            Update::Progress { done, total } => {
                if let Some(saving) = &mut self.saving {
                    saving.progress = Some((done, total));
                }
            }
            Update::Finished(outcome) => {
                self.saving = None;
                self.save_notice = match outcome {
                    Outcome::Saved(saved) => Some(Notice::Saved(file_name(&saved.path))),
                    Outcome::Cancelled => None,
                    Outcome::Failed(error) => Some(Notice::Failed(error)),
                };
            }
        }
        cx.notify();
    }

    /// Whether `path` is the file a save in progress is writing, which must not open half written.
    pub(super) fn refuse_save_target(
        &mut self,
        path: &std::path::Path,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(saving) = &mut self.saving else {
            return false;
        };
        if !saving::same_path(path, &saving.target) {
            return false;
        }
        saving.open_refused = true;
        cx.notify();
        true
    }

    fn cancel_save(&mut self, cx: &mut Context<Self>) {
        if let Some(saving) = &self.saving {
            saving.job.cancel();
        }
        cx.notify();
    }

    /// The status-bar item for a save in progress or the last one's outcome.
    pub(super) fn save_item(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let failure = match &self.save_notice {
            Some(Notice::Failed(error)) if self.saving.is_none() => Some(MetadataHint {
                title: "Save failed",
                value: error.clone(),
                explanation: String::new(),
                rows: Vec::new(),
            }),
            _ => None,
        };
        let (text, failed, cancels) = match (&self.saving, &self.save_notice) {
            (Some(saving), _) => {
                let percent = saving
                    .progress
                    .filter(|(_, total)| *total > 0)
                    .map_or(0, |(done, total)| {
                        (done as f64 * 100.0 / total as f64) as u32
                    });
                let text = if saving.open_refused {
                    format!("Saving {}… {percent}%, open it once saved", saving.name)
                } else {
                    format!("Saving {}… {percent}%", saving.name)
                };
                (text, false, true)
            }
            (None, Some(Notice::Saved(name))) => (format!("Saved {name}"), false, false),
            (None, Some(Notice::Failed(error))) => (format!("Save failed: {error}"), true, false),
            (None, None) => return None,
        };
        let close = Button::new("save-close")
            .ghost()
            .xsmall()
            .icon(IconName::Close)
            .tab_stop(false)
            .on_click(cx.listener(move |shell, _, _, cx| {
                cx.stop_propagation();
                if cancels {
                    shell.cancel_save(cx);
                } else {
                    shell.save_notice = None;
                    cx.notify();
                }
            }));
        Some(
            div()
                .id("save-status")
                .flex()
                .items_center()
                .gap_1()
                .min_w_0()
                .max_w(px(360.))
                .when(failed, |item| item.text_color(cx.theme().danger))
                .when_some(failure, |item, hint| {
                    item.tooltip(move |_, cx| metadata_tooltip(hint.clone(), cx))
                })
                .child(
                    div()
                        .min_w_0()
                        .overflow_hidden()
                        .text_ellipsis()
                        .whitespace_nowrap()
                        .child(text),
                )
                .child(close)
                .into_any_element(),
        )
    }
}

fn file_name(path: &std::path::Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    )
}
