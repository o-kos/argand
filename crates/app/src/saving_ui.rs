//! Save as and Save selection as, from the dialog to the status bar.

use super::*;
use crate::saving::{self, Outcome, Update};
use gpui_kit::AnyElement;
use gpui_kit::component::IconName;

/// A save in progress.
pub(super) struct Saving {
    job: saving::Job,
    pub(super) name: String,
    target: std::path::PathBuf,
    pub(super) progress: Option<(u64, u64)>,
    /// Set when the file being written was asked to open, which waits until it is complete.
    open_refused: bool,
    /// Why a FLAC source is written as WAVE, said once it is saved.
    as_wave: Option<String>,
    pub(super) of: SaveOf,
}

/// What a finished save means for the document it was started from.
pub(super) struct SaveOf {
    /// The document, told apart from any opened since.
    pub(super) document: u64,
    /// The edit version written.
    pub(super) version: u64,
    /// Whether the whole capture was written, which makes that version saved.
    pub(super) whole: bool,
    /// Whether the saved file is read in place of the edited capture once it is written.
    pub(super) rebind: bool,
    /// What waited for this save, an open or a close.
    pub(super) then: Option<editing_ui::Pending>,
    /// Whether this save replaces the open file, which needs it let go of first.
    pub(super) over: bool,
    /// The scratch files it reads, kept until it ends even when the document goes first.
    pub(super) _scratch: Vec<Arc<crate::scratch::Scratch>>,
}

#[cfg(test)]
impl SaveOf {
    /// A whole save of `version` of `document` that goes on from the saved file.
    pub(super) fn whole_for_test(document: u64, version: u64) -> Self {
        Self {
            document,
            version,
            whole: true,
            rebind: true,
            then: None,
            over: false,
            _scratch: Vec::new(),
        }
    }
}

/// How the last save ended, or why an edit was refused, shown until it is closed.
pub(super) enum Notice {
    Saved(String),
    /// What failed, as a short title, and why.
    Failed(&'static str, String),
}

impl Shell {
    /// User-facing naming is independent of temporary sample backing for every export.
    pub(super) fn save_presentation(
        &self,
        meta: &argand_core::SignalMeta,
    ) -> (argand_core::SignalMeta, std::path::PathBuf) {
        let mut display = meta.clone();
        let directory = match self.file.as_ref().and_then(|file| file.unsaved.as_ref()) {
            Some(unsaved) => {
                display.source = unsaved.name.clone().into();
                display.container = unsaved.container.extension();
                std::path::PathBuf::from(".")
            }
            None => saving::directory(&meta.source),
        };
        (display, directory)
    }

    fn save_matches(&self, of: &SaveOf) -> bool {
        self.saving.is_none()
            && self.file.as_ref().is_some_and(|file| {
                file.id == of.document
                    && file
                        .editing
                        .as_ref()
                        .is_some_and(|editing| editing.version() == of.version)
            })
    }

    /// Whether a save of the capture, or of its time selection, can start now.
    pub(super) fn can_save(&self, selection_only: bool) -> bool {
        self.saving.is_none()
            && !self.computing()
            && self
                .editing()
                .is_some_and(|editing| !editing.capture().is_empty() && editing.is_described())
            && (!selection_only || self.selection.is_some() || self.band.is_some())
    }

    /// Ask the desktop where to save, then start writing there.
    pub(super) fn save_as(
        &mut self,
        selection_only: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.save_as_then(selection_only, None, window, cx);
    }

    /// Save as, then do `then` once the whole capture is written and nothing changed meanwhile.
    pub(super) fn save_as_then(
        &mut self,
        selection_only: bool,
        then: Option<editing_ui::Pending>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.can_save(selection_only) {
            if then.is_some() {
                self.save_notice = Some(Notice::Failed(
                    "Cannot save now",
                    "another save is running or the capture is still being checked".into(),
                ));
                cx.notify();
            }
            return;
        }
        self.dismiss_application_menu(window, cx);
        self.close_hints(cx);
        // The release happens over the dialog, so nothing held now may stay pressed.
        self.interrupt_plot(cx);
        window.focus(&self.focus_target(cx), cx);
        cx.notify();
        let Some(file) = &self.file else { return };
        let Some(editing) = file.editing.as_ref() else {
            return;
        };
        if let Some(band) = self.band.filter(|_| selection_only) {
            self.save_band_as(band, window, cx);
            return;
        }
        let span = if selection_only { self.selection } else { None };
        let output = editing.file();
        let as_wave = argand_io::write::writes_as_wave(&output.meta, &output.hints);
        let (display, directory) = self.save_presentation(&output.meta);
        let name = saving::suggested_name(&display, as_wave, span);
        let Some(mut request) = editing.save_request(span, std::path::PathBuf::new()) else {
            return;
        };
        if let Some(unsaved) = &file.unsaved {
            request.output = argand_io::write::Output::Container(unsaved.container);
        }
        let whole = !selection_only;
        let edited = editing.is_dirty() || !editing.is_untouched();
        let of = SaveOf {
            document: file.id,
            version: editing.version(),
            whole,
            // Saving the whole edited capture turns the window to the saved file.
            rebind: whole && edited,
            then: then.filter(|_| whole),
            over: false,
            _scratch: editing.scratches(),
        };
        cx.spawn_in(window, async move |shell, cx| {
            let directory = cx
                .background_executor()
                .spawn(async move { saving::existing_directory(&directory) })
                .await;
            let Ok(Some(chosen)) = shell.update_in(cx, |shell, _, cx| {
                shell
                    .save_matches(&of)
                    .then(|| cx.prompt_for_new_path(&directory, Some(&name)))
            }) else {
                return;
            };
            // A cancelled dialog, a platform without one, and a failed dialog all mean no target.
            let Ok(Ok(Some(target))) = chosen.await else {
                return;
            };
            if let argand_io::write::Output::Container(container) = request.output {
                request.output = argand_io::write::Output::Container(saving::target_container(
                    container, &target,
                ));
            }
            request.target = target;
            let _ = shell.update_in(cx, |shell, window, cx| {
                shell.start_save(request, of, window, cx)
            });
        })
        .detach();
    }

    pub(super) fn start_save(
        &mut self,
        request: argand_io::write::SaveRequest,
        of: SaveOf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.saving.is_some() {
            return;
        }
        tracing::info!(
            source = %request.meta.source.display(),
            target = %request.target.display(),
            segments = ?request.segments,
            "saving"
        );
        let name = file_name(&request.target);
        let target = request.target.clone();
        let output = &request.meta;
        let as_wave = (output.container == "flac"
            && argand_io::write::writes_as_wave(output, &argand_io::OpenHints::default()))
        .then(|| {
            format!(
                "as WAV, the FLAC encoder cannot write {} Hz",
                crate::numbers::number(output.sample_rate)
            )
        });
        let (job, updates) = saving::start(request, of.over);
        self.watch_save(job, updates, (name, target, as_wave), of, window, cx);
    }

    /// Ask where to save the selected band, over the selected span or the whole capture, then compute it there.
    fn save_band_as(
        &mut self,
        band: argand_core::FrequencyBand,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(file) = &self.file else { return };
        let Some(editing) = file.editing.as_ref() else {
            return;
        };
        let mut meta = editing.file().meta.clone();
        meta.len_samples = editing.len();
        let Some(span) = self
            .selection
            .or_else(|| argand_core::SampleSpan::between(0, editing.len()))
        else {
            return;
        };
        let (display, directory) = self.save_presentation(&meta);
        let container = file
            .unsaved
            .as_ref()
            .map(|unsaved| unsaved.container)
            .or_else(|| (meta.container == "flac").then_some(argand_io::write::Container::Flac));
        let mut name = saving::suggested_band_name(&display, self.selection, band);
        if container == Some(argand_io::write::Container::Flac) {
            name = std::path::Path::new(&name)
                .with_extension("flac")
                .to_string_lossy()
                .into_owned();
        }
        let mut request = crate::extraction::ExtractRequest {
            capture: editing.capture().clone(),
            sources: editing.source_files(),
            meta,
            span,
            band,
            target: std::path::PathBuf::new(),
            protected: editing.protected(),
        };
        // A band that cannot be saved says why before any dialog asks where.
        if let Err(error) = request.check() {
            self.save_notice = Some(Notice::Failed(
                "Cannot save the band",
                saving::message(&error),
            ));
            cx.notify();
            return;
        }
        let of = SaveOf {
            document: file.id,
            version: editing.version(),
            whole: false,
            rebind: false,
            then: None,
            over: false,
            _scratch: editing.scratches(),
        };
        cx.spawn_in(window, async move |shell, cx| {
            let directory = cx
                .background_executor()
                .spawn(async move { saving::existing_directory(&directory) })
                .await;
            let Ok(Some(chosen)) = shell.update_in(cx, |shell, _, cx| {
                shell
                    .save_matches(&of)
                    .then(|| cx.prompt_for_new_path(&directory, Some(&name)))
            }) else {
                return;
            };
            let Ok(Ok(Some(target))) = chosen.await else {
                return;
            };
            let container = container.map(|container| saving::target_container(container, &target));
            request.target = target;
            let _ = shell.update_in(cx, |shell, window, cx| {
                if shell.saving.is_some() {
                    return;
                }
                tracing::info!(
                    source = %request.meta.source.display(),
                    target = %request.target.display(),
                    span = ?request.span,
                    band = ?request.band,
                    "saving a band"
                );
                let name = file_name(&request.target);
                let target = request.target.clone();
                let (job, updates) = saving::start_extraction(request, container);
                shell.watch_save(job, updates, (name, target, None), of, window, cx);
            });
        })
        .detach();
    }

    /// Follow a save job until it finishes, showing it in the status bar.
    fn watch_save(
        &mut self,
        job: saving::Job,
        updates: async_channel::Receiver<Update>,
        (name, target, as_wave): (String, std::path::PathBuf, Option<String>),
        of: SaveOf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.save_updates = Some(cx.spawn_in(window, async move |shell, cx| {
            while let Ok(update) = updates.recv().await {
                if shell
                    .update_in(cx, |shell, window, cx| {
                        shell.receive_save(update, window, cx)
                    })
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
            as_wave,
            of,
        });
        cx.notify();
    }

    fn receive_save(&mut self, update: Update, window: &mut Window, cx: &mut Context<Self>) {
        match update {
            Update::Progress { done, total } => {
                if let Some(saving) = &mut self.saving {
                    saving.progress = Some((done, total));
                }
            }
            Update::Finished(outcome) => {
                let Some(finished) = self.saving.take() else {
                    return;
                };
                match outcome {
                    Outcome::Saved(saved) => {
                        self.save_notice = Some(Notice::Saved(match &finished.as_wave {
                            Some(reason) => format!("{} {reason}", file_name(&saved.path)),
                            None => file_name(&saved.path),
                        }));
                        self.saved(saved.path, finished.of, window, cx);
                    }
                    Outcome::Encoded { saved, clipped } => {
                        let name = file_name(&saved.path);
                        self.save_notice = Some(Notice::Saved(if clipped == 0 {
                            name
                        } else {
                            format!("{name} — {clipped} clipped scalar values")
                        }));
                        self.saved(saved.path, finished.of, window, cx);
                    }
                    Outcome::Staged(staged) => self.staged(*staged, finished, window, cx),
                    Outcome::Cancelled => self.save_notice = None,
                    Outcome::Failed(error) => {
                        self.save_notice = Some(Notice::Failed("Save failed", error));
                    }
                }
            }
        }
        cx.notify();
    }

    /// Mark the version written as saved, then go on from the saved file or do what waited, unless the document moved on.
    pub(super) fn saved(
        &mut self,
        path: std::path::PathBuf,
        of: SaveOf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(file) = self.file.as_mut().filter(|file| file.id == of.document) else {
            return;
        };
        let Some(editing) = file.editing.as_mut() else {
            return;
        };
        if of.whole {
            editing.mark_saved_version(of.version);
        }
        let unchanged = editing.version() == of.version;
        self.refresh_titles(window);
        if !unchanged {
            return;
        }
        if let Some(then) = of.then {
            self.carry_out(then, window, cx);
            return;
        }
        if !of.rebind {
            return;
        }
        self.rebind_saved_as(path, window, cx);
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
            Some(Notice::Failed(title, error)) if self.saving.is_none() => Some(MetadataHint {
                title,
                value: error.clone(),
                explanation: String::new(),
                rows: Vec::new(),
            }),
            _ => None,
        };
        let (text, failed, closes) = match (&self.saving, &self.save_notice) {
            (Some(saving), _) => {
                let percent = saving
                    .progress
                    .filter(|(_, total)| *total > 0)
                    .map_or(0, |(done, total)| {
                        (done as f64 * 100.0 / total as f64) as u32
                    });
                let text = if self.replacement_active {
                    format!("Replacing {}…", saving.name)
                } else if saving.open_refused {
                    format!("Saving {}… {percent}%, open it once saved", saving.name)
                } else {
                    format!("Saving {}… {percent}%", saving.name)
                };
                // Once the file is being replaced there is nothing left to cancel.
                (text, false, !self.replacement_active)
            }
            (None, Some(Notice::Failed(title, error))) => (format!("{title}: {error}"), true, true),
            (None, Some(Notice::Saved(_)) | None) => return None,
        };
        let close = Button::new("save-close")
            .ghost()
            .xsmall()
            .icon(IconName::Close)
            .tab_stop(false)
            .on_click(cx.listener(move |shell, _, _, cx| {
                cx.stop_propagation();
                if shell.saving.is_some() {
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
                .when(closes, |item| item.child(close))
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
