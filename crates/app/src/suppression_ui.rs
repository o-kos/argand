//! Suppress band, from the command to the edit, with its progress in the status bar.

use super::*;
use crate::saving::{Ended, Update};
use crate::suppression::Suppressed;
use gpui_kit::AnyElement;
use gpui_kit::component::IconName;

/// A suppression being computed for one version of one document.
pub(super) struct Suppressing {
    job: crate::saving::Job,
    document: u64,
    version: u64,
    /// What was selected, which stays selected after the edit and after undoing it.
    selection: argand_core::Selection,
    progress: Option<(u64, u64)>,
}

impl Shell {
    /// Whether the selected band can be suppressed now.
    pub(super) fn can_suppress(&self) -> bool {
        self.band.is_some()
            && self.saving.is_none()
            && !self.computing()
            && !self.replacement_active
            && !self.capture_changed()
            && self.editing().is_some_and(|editing| {
                !editing.capture().is_empty() && editing.is_described() && editing.can_compute()
            })
    }

    /// Start computing the suppression of the selected band, over the selected span or the whole capture.
    pub(super) fn suppress_band(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.can_suppress() {
            return;
        }
        let (Some(band), Some(file)) = (self.band, &self.file) else {
            return;
        };
        let Some(editing) = file.editing.as_ref() else {
            return;
        };
        let mut meta = editing.file().meta.clone();
        meta.len_samples = editing.len();
        let Some(target) = crate::scratch::folder().map(|folder| folder.new_path("wav")) else {
            self.save_notice = Some(saving_ui::Notice::Failed(
                "Cannot suppress the band",
                "there is no folder for its samples".into(),
            ));
            cx.notify();
            return;
        };
        let request = crate::suppression::SuppressRequest {
            capture: editing.capture().clone(),
            sources: editing.source_files(),
            meta,
            band,
            time: self.selection,
            target,
        };
        if let Err(error) = request.check() {
            self.save_notice = Some(saving_ui::Notice::Failed("Cannot suppress the band", error));
            cx.notify();
            return;
        }
        tracing::info!(
            source = %request.meta.source.display(),
            band = ?request.band,
            time = ?request.time,
            "suppressing a band"
        );
        let (job, updates) = crate::suppression::start(request);
        self.suppress_updates = Some(cx.spawn_in(window, async move |shell, cx| {
            while let Ok(update) = updates.recv().await {
                let taken = shell.update_in(cx, |shell, window, cx| {
                    shell.receive_suppression(update, window, cx);
                });
                if taken.is_err() {
                    break;
                }
            }
        }));
        self.save_notice = None;
        self.suppressing = Some(Suppressing {
            job,
            document: file.id,
            version: editing.version(),
            selection: argand_core::Selection {
                time: self.selection,
                band: self.band,
            },
            progress: None,
        });
        cx.notify();
    }

    fn receive_suppression(
        &mut self,
        update: Update<Ended<Suppressed>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match update {
            Update::Progress { done, total } => {
                if let Some(suppressing) = &mut self.suppressing {
                    suppressing.progress = Some((done, total));
                }
            }
            Update::Finished(ended) => {
                let Some(finished) = self.suppressing.take() else {
                    return;
                };
                match ended {
                    Ended::Done(suppressed) => {
                        self.apply_suppression(&finished, suppressed, window, cx)
                    }
                    Ended::Cancelled => {}
                    Ended::Failed(error) => {
                        self.save_notice =
                            Some(saving_ui::Notice::Failed("Suppression failed", error));
                    }
                }
            }
        }
        cx.notify();
    }

    /// Put the computed samples in place, unless the document moved on while they were computed.
    fn apply_suppression(
        &mut self,
        finished: &Suppressing,
        suppressed: Suppressed,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let current = self
            .file
            .as_ref()
            .is_some_and(|file| file.id == finished.document);
        let Some(editing) = self.editing_mut().filter(|_| current) else {
            return;
        };
        if editing.version() != finished.version {
            return;
        }
        let computed =
            crate::editing::Source::computed(suppressed.scratch, suppressed.meta, suppressed.stamp);
        if editing.replace_computed(
            suppressed.span,
            computed,
            suppressed.offset,
            finished.selection,
        ) {
            self.edited(window, cx);
        }
    }

    pub(super) fn cancel_suppression(&mut self, cx: &mut Context<Self>) {
        if let Some(suppressing) = &self.suppressing {
            suppressing.job.cancel();
        }
        cx.notify();
    }

    /// The status-bar item for a suppression being computed.
    pub(super) fn suppress_item(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let suppressing = self.suppressing.as_ref()?;
        let percent = suppressing
            .progress
            .filter(|(_, total)| *total > 0)
            .map_or(0, |(done, total)| {
                (done as f64 * 100.0 / total as f64) as u32
            });
        let close = Button::new("suppress-cancel")
            .ghost()
            .xsmall()
            .icon(IconName::Close)
            .tab_stop(false)
            .on_click(cx.listener(|shell, _, _, cx| {
                cx.stop_propagation();
                shell.cancel_suppression(cx);
            }));
        Some(
            div()
                .id("suppress-status")
                .flex()
                .items_center()
                .gap_1()
                .child(
                    div()
                        .whitespace_nowrap()
                        .child(format!("Suppressing the band… {percent}%")),
                )
                .child(close)
                .into_any_element(),
        )
    }
}

#[cfg(test)]
#[path = "suppression_ui_test.rs"]
mod tests;
