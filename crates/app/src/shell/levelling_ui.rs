//! Invert spectrum and Normalize level, from the commands to the edits, with the measurement in the status bar.

use super::*;
use crate::document::MetadataHint;
use crate::processing::saving::{Ended, Update};
use gpui_kit::AnyElement;
use gpui_kit::component::IconName;

/// A peak being measured for one version of one document, to normalize it.
pub(super) struct Levelling {
    job: crate::processing::saving::Job,
    document: u64,
    version: u64,
    progress: Option<(u64, u64)>,
}

/// Gains closer than this to the one in force add no version, in dB.
const SAME_GAIN_DB: f64 = 0.001;

impl Shell {
    /// Whether a suppression or a measurement is running, which edits and saves wait for.
    pub(super) fn computing(&self) -> bool {
        self.suppressing.is_some() || self.levelling.is_some() || self.conversion_active()
    }

    /// Whether the whole capture can be inverted or levelled now.
    pub(super) fn can_adjust(&self) -> bool {
        self.saving.is_none()
            && !self.computing()
            && !self.replacement_active
            && !self.capture_changed()
            && self
                .editing()
                .is_some_and(|editing| !editing.capture().is_empty() && editing.is_described())
    }

    /// Whether Normalize level can start now, which needs the stored full scale.
    pub(super) fn can_normalize(&self) -> bool {
        self.nothing_selected()
            && self.can_adjust()
            && self
                .editing()
                .is_some_and(|editing| editing.full_scale().is_some())
    }

    /// Whether Invert spectrum can act now.
    pub(super) fn can_invert(&self) -> bool {
        self.nothing_selected() && self.can_adjust()
    }

    /// Whether nothing is selected, which both commands need, as a selection would suggest acting on it alone.
    fn nothing_selected(&self) -> bool {
        self.selection.is_none() && self.band.is_none()
    }

    pub(super) fn invert_spectrum(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.can_invert() {
            return;
        }
        let Some(editing) = self.editing_mut() else {
            return;
        };
        let inverted = editing.adjustment().inverted();
        if editing.adjust(inverted, argand_core::Selection::default()) {
            self.edited(window, cx);
        }
    }

    /// Measure the peak, then normalize the version it was measured on.
    pub(super) fn normalize(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.can_normalize() {
            return;
        }
        let Some(file) = &self.file else { return };
        let Some(editing) = file.editing.as_ref() else {
            return;
        };
        let mut meta = editing.file().meta.clone();
        meta.len_samples = editing.len();
        let request = crate::processing::levelling::LevelRequest {
            capture: editing.capture().clone(),
            sources: editing.source_files(),
            meta,
        };
        tracing::info!(source = %request.meta.source.display(), "normalizing the level");
        let (job, updates) = crate::processing::levelling::start(request);
        self.level_updates = Some(cx.spawn_in(window, async move |shell, cx| {
            while let Ok(update) = updates.recv().await {
                let taken = shell.update_in(cx, |shell, window, cx| {
                    shell.receive_level(update, window, cx);
                });
                if taken.is_err() {
                    break;
                }
            }
        }));
        self.save_notice = None;
        self.levelling = Some(Levelling {
            job,
            document: file.id,
            version: editing.version(),
            progress: None,
        });
        cx.notify();
    }

    fn receive_level(
        &mut self,
        update: Update<Ended<f64>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match update {
            Update::Progress { done, total } => {
                if let Some(levelling) = &mut self.levelling {
                    levelling.progress = Some((done, total));
                }
            }
            Update::Finished(ended) => {
                let Some(finished) = self.levelling.take() else {
                    return;
                };
                match ended {
                    Ended::Done(peak) => self.apply_level(&finished, peak, window, cx),
                    Ended::Cancelled => {}
                    Ended::Failed(error) => {
                        self.save_notice =
                            Some(saving_ui::Notice::Failed("Normalization failed", error));
                    }
                }
            }
        }
        cx.notify();
    }

    /// Set the gain that puts `peak` at the target, unless the document moved on while it was measured.
    fn apply_level(
        &mut self,
        finished: &Levelling,
        peak: f64,
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
        let gain = editing
            .full_scale()
            .and_then(|full_scale| crate::processing::levelling::gain(peak, full_scale));
        let Some(gain) = gain else {
            self.save_notice = Some(saving_ui::Notice::Failed(
                "Cannot normalize",
                "the capture is silent".into(),
            ));
            return;
        };
        let adjustment = editing.adjustment();
        if (20.0 * (gain / adjustment.gain()).log10()).abs() < SAME_GAIN_DB {
            return;
        }
        if adjustment
            .with_gain(gain)
            .is_some_and(|adjusted| editing.adjust(adjusted, argand_core::Selection::default()))
        {
            self.edited(window, cx);
        }
    }

    pub(super) fn cancel_level(&mut self, cx: &mut Context<Self>) {
        if let Some(levelling) = &self.levelling {
            levelling.job.cancel();
        }
        cx.notify();
    }

    /// The status-bar item for a peak being measured.
    pub(super) fn level_item(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let levelling = self.levelling.as_ref()?;
        let percent = levelling
            .progress
            .filter(|(_, total)| *total > 0)
            .map_or(0, |(done, total)| {
                (done as f64 * 100.0 / total as f64) as u32
            });
        let close = Button::new("level-cancel")
            .ghost()
            .xsmall()
            .icon(IconName::Close)
            .tab_stop(false)
            .on_click(cx.listener(|shell, _, _, cx| {
                cx.stop_propagation();
                shell.cancel_level(cx);
            }));
        Some(
            div()
                .id("level-status")
                .flex()
                .items_center()
                .gap_1()
                .child(
                    div()
                        .whitespace_nowrap()
                        .child(format!("Measuring the level… {percent}%")),
                )
                .child(close)
                .into_any_element(),
        )
    }

    /// The status-bar item saying how the capture is read, while it is inverted or levelled.
    pub(super) fn adjustment_item(&self, cx: &Context<Self>) -> Option<AnyElement> {
        let text = adjustment_text(self.editing()?.adjustment())?;
        let hint = MetadataHint {
            title: "Adjustment",
            value: text.clone(),
            explanation:
                "Applied while the capture is read, and written into the file when it is saved"
                    .into(),
            rows: Vec::new(),
        };
        Some(
            div()
                .id("adjustment-status")
                .px_2()
                .border_l_1()
                .border_color(cx.theme().border)
                .flex_shrink_0()
                .whitespace_nowrap()
                .tooltip(move |_, cx| metadata_tooltip(hint.clone(), cx))
                .child(text)
                .into_any_element(),
        )
    }
}

/// `Inverted`, the gain in dB, or both, none for the identity.
pub(super) fn adjustment_text(adjustment: argand_core::Adjustment) -> Option<String> {
    let mut parts = Vec::new();
    if adjustment.invert() {
        parts.push("Inverted".to_string());
    }
    if adjustment.gain() != 1.0 {
        let db = 20.0 * adjustment.gain().log10();
        let sign = if db >= 0.0 { "+" } else { "−" };
        parts.push(format!(
            "{sign}{} dB",
            crate::numbers::number(format!("{:.1}", db.abs()))
        ));
    }
    (!parts.is_empty()).then(|| parts.join(", "))
}

#[cfg(test)]
#[path = "levelling_ui_test.rs"]
mod tests;
