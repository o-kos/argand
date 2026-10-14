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
    /// The time span the rectangle had, selected again once the edit is made.
    selection: Option<argand_core::SampleSpan>,
    progress: Option<(u64, u64)>,
}

impl Shell {
    /// Whether the selected band can be suppressed now.
    pub(super) fn can_suppress(&self) -> bool {
        self.band.is_some()
            && self.saving.is_none()
            && self.suppressing.is_none()
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
            selection: self.selection,
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
        if editing.replace_computed(suppressed.span, computed, finished.selection) {
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
mod tests {
    use super::super::replace_ui::tests::{open_window, wait_until};
    use super::*;
    use crate::editing::Editing;
    use argand_core::{FrequencyBand, SampleSpan};
    use gpui_kit::TestAppContext;

    /// A 16-bit capture opened from a fresh folder and described, with a band and a span selected.
    fn opened(
        cx: &mut gpui_kit::VisualTestContext,
        shell: &Entity<Shell>,
        label: &str,
    ) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("argand-{label}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("capture.wav");
        let iq_i16: argand_core::SampleType = "iq_i16".parse().unwrap();
        // A tone inside the band and one outside it.
        let inside = argand_io::testutil::iq_tone(48_000, 24_000.0, 3_000.0, 0.25);
        let outside = argand_io::testutil::iq_tone(48_000, 24_000.0, -6_000.0, 0.25);
        let values: Vec<f32> = inside.iter().zip(&outside).map(|(a, b)| a + b).collect();
        argand_io::testutil::write_wav(&path, iq_i16, 24_000, &values, 1.0);
        shell.update_in(cx, |shell, window, cx| {
            shell.open(Origin::new(path.clone()), window, cx);
        });
        wait_until(cx, shell, "the file to describe itself", |shell| {
            shell
                .editing()
                .is_some_and(|editing| editing.is_described() && editing.can_compute())
        });
        shell.update(cx, |shell, _| {
            shell.band = FrequencyBand::between(2_000.0, 4_000.0);
            shell.selection = SampleSpan::between(10_000, 20_000);
        });
        path
    }

    fn read(path: &std::path::Path) -> Vec<f32> {
        let mut source = argand_io::open(path, &argand_io::OpenHints::default()).unwrap();
        let mut values = vec![0.0; source.meta().len_samples as usize * 2];
        let mut filled = 0;
        while filled < values.len() {
            filled += source.read(&mut values[filled..]).unwrap();
        }
        values
    }

    #[gpui_kit::test]
    fn a_rectangle_is_suppressed_as_one_edit_and_saved_in_the_captures_format(
        cx: &mut TestAppContext,
    ) {
        let (shell, cx) = open_window(cx);
        let path = opened(cx, &shell, "suppress-ui");
        assert!(shell.read_with(cx, |shell, _| shell.edit_commands_available().suppress));
        shell.update_in(cx, |shell, window, cx| shell.suppress_band(window, cx));
        assert!(shell.read_with(cx, |shell, _| {
            shell.suppressing.is_some()
                && !shell.can_save(false)
                && shell.edit_commands_available() == editing_ui::EditCommands::default()
        }));
        wait_until(cx, &shell, "the band to be suppressed", |shell| {
            shell.suppressing.is_none() && shell.editing().is_some_and(Editing::is_dirty)
        });
        shell.read_with(cx, |shell, _| {
            let editing = shell.editing().unwrap();
            assert_eq!(editing.len(), 48_000);
            assert_eq!(editing.capture().pieces().len(), 3);
            assert_eq!(shell.band, None);
            assert_eq!(shell.selection, SampleSpan::between(10_000, 20_000));
        });
        shell.update_in(cx, |_, window, cx| {
            window.dispatch_action(Box::new(editing_ui::Undo), cx);
        });
        cx.run_until_parked();
        assert!(shell.read_with(cx, |shell, _| shell.editing().unwrap().is_untouched()));
        shell.update_in(cx, |_, window, cx| {
            window.dispatch_action(Box::new(editing_ui::Redo), cx);
        });
        cx.run_until_parked();

        let target = path.with_file_name("saved.wav");
        shell.update_in(cx, |shell, window, cx| {
            let file = shell.file.as_ref().unwrap();
            let editing = file.editing.as_ref().unwrap();
            let request = editing.save_request(None, target.clone()).unwrap();
            let of = saving_ui::SaveOf {
                document: file.id,
                version: editing.version(),
                whole: false,
                rebind: false,
                then: None,
                over: false,
            };
            shell.start_save(request, of, window, cx);
        });
        wait_until(cx, &shell, "the save to end", |shell| {
            shell.saving.is_none()
        });
        let saved = argand_io::open(&target, &argand_io::OpenHints::default()).unwrap();
        assert_eq!(saved.meta().len_samples, 48_000);
        assert_eq!(
            saved.meta().sample_type.format,
            argand_core::SampleFormat::I16
        );
        let (before, after) = (read(&path), read(&target));
        let span = shell.read_with(cx, |shell, _| {
            let pieces = shell.editing().unwrap().capture().pieces().to_vec();
            pieces[0].len as usize..(pieces[0].len + pieces[1].len) as usize
        });
        assert!(span.start < 10_000 && span.end > 20_000);
        for n in (0..48_000).filter(|n| !span.contains(n)) {
            assert_eq!(
                after[2 * n..2 * n + 2],
                before[2 * n..2 * n + 2],
                "sample {n}"
            );
        }
        let level = |values: &[f32], frequency: f64| {
            let (mut re, mut im) = (0.0f64, 0.0f64);
            for n in 12_000..18_000 {
                let turn = -std::f64::consts::TAU * frequency * n as f64 / 24_000.0;
                let (sin, cos) = turn.sin_cos();
                let (i, q) = (f64::from(values[2 * n]), f64::from(values[2 * n + 1]));
                re += i * cos - q * sin;
                im += i * sin + q * cos;
            }
            re.hypot(im) / 6_000.0
        };
        assert!(level(&before, 3_000.0) > 0.2);
        assert!(level(&after, 3_000.0) < 1e-3, "{}", level(&after, 3_000.0));
        assert!((level(&after, -6_000.0) - 0.25).abs() < 1e-3);
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[gpui_kit::test]
    fn opening_another_file_drops_a_suppression_in_progress(cx: &mut TestAppContext) {
        let (shell, cx) = open_window(cx);
        let path = opened(cx, &shell, "suppress-ui-open");
        shell.update_in(cx, |shell, window, cx| {
            shell.suppress_band(window, cx);
            shell.open(Origin::new(path.clone()), window, cx);
        });
        assert!(shell.read_with(cx, |shell, _| shell.suppressing.is_none()));
        wait_until(cx, &shell, "the file to open again", |shell| {
            shell.editing().is_some_and(Editing::is_described)
        });
        assert!(shell.read_with(cx, |shell, _| shell.editing().unwrap().is_untouched()));
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }
}
