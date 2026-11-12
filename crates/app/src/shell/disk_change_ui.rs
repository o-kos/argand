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
        if file.state.document.is_changed() || file.disk_check.is_some() || file.rebinding.is_some()
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
            file.state.document.mark_changed(Some(path));
            cx.notify();
        }
    }

    /// Whether a file the shown capture reads changed on disk.
    pub(super) fn capture_changed(&self) -> bool {
        self.file
            .as_ref()
            .is_some_and(|file| file.state.document.is_changed())
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
        let document = &self.file.as_ref()?.state.document;
        if !document.is_changed() {
            return None;
        }
        let path = document.changed_file();
        let text = match path.and_then(|path| path.file_name()) {
            Some(name) => format!("{} changed on disk", name.to_string_lossy()),
            None => "The capture changed on disk".to_owned(),
        };
        let hint = MetadataHint {
            title: "Changed on disk",
            value: path.map_or_else(
                || "A file the capture reads".to_owned(),
                |path| path.display().to_string(),
            ),
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
                .when(self.external_tooltips_allowed(cx), |element| {
                    element.tooltip(move |_, cx| metadata_tooltip(hint.clone(), cx))
                })
                .child(
                    div()
                        .min_w_0()
                        .overflow_hidden()
                        .text_ellipsis()
                        .whitespace_nowrap()
                        .child(text),
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
#[path = "disk_change_ui_test.rs"]
mod tests;
