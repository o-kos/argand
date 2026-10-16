//! Invert spectrum and Normalize level, from the commands to the edits, with the measurement in the status bar.

use super::*;
use crate::document::MetadataHint;
use crate::saving::{Ended, Update};
use gpui_kit::AnyElement;
use gpui_kit::Focusable as _;
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::component::{IconName, WindowExt as _, h_flex, v_flex};

/// A peak being measured for one version of one document, to normalize it to `target` dBFS.
pub(super) struct Levelling {
    job: crate::saving::Job,
    document: u64,
    version: u64,
    target: f64,
    /// What was selected, which stays selected after the edit and after undoing it.
    selection: argand_core::Selection,
    progress: Option<(u64, u64)>,
}

/// Gains closer than this to the one in force add no version, in dB.
const SAME_GAIN_DB: f64 = 0.001;

impl Shell {
    /// Whether a suppression or a measurement is running, which edits and saves wait for.
    pub(super) fn computing(&self) -> bool {
        self.suppressing.is_some() || self.levelling.is_some()
    }

    /// The time span and band selected now.
    fn marked_now(&self) -> argand_core::Selection {
        argand_core::Selection {
            time: self.selection,
            band: self.band,
        }
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
        self.can_adjust()
            && self
                .editing()
                .is_some_and(|editing| editing.full_scale().is_some())
    }

    /// Whether Invert spectrum can act now, which a selection forbids, as it would suggest inverting only what is selected.
    pub(super) fn can_invert(&self) -> bool {
        self.can_adjust() && self.selection.is_none() && self.band.is_none()
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

    /// Ask for the target level, then normalize to it.
    pub(super) fn ask_normalize(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.can_normalize() {
            return;
        }
        // A window without the toolkit's root, as in a headless test, has no dialog layer.
        if window
            .root::<gpui_kit::component::Root>()
            .flatten()
            .is_none()
        {
            return;
        }
        let shell = cx.entity().downgrade();
        let proposed = crate::numbers::input(self.level_target);
        let input = cx.new(|cx| InputState::new(window, cx).default_value(proposed));
        let error: Entity<Option<String>> = cx.new(|_| None);
        let focus = input.read(cx).focus_handle(cx);
        window.open_dialog(cx, move |dialog, _, cx| {
            let accept = {
                let (input, error, shell) = (input.clone(), error.clone(), shell.clone());
                std::rc::Rc::new(move |window: &mut Window, cx: &mut gpui_kit::App| {
                    accept_level(&input, &error, &shell, window, cx)
                })
            };
            let message = error.read(cx).clone();
            let keyed = accept.clone();
            let clicked = accept.clone();
            dialog
                .title("Normalize level")
                .w(px(360.))
                .close_button(false)
                .overlay_closable(false)
                .child(
                    v_flex()
                        .gap_2()
                        .child("Scale the whole capture so that its peak lands on")
                        .child(
                            h_flex()
                                .gap_2()
                                .items_center()
                                .child(Input::new(&input).w(px(120.)))
                                .child("dBFS"),
                        )
                        .children(
                            message
                                .map(|message| div().text_color(cx.theme().danger).child(message)),
                        ),
                )
                .footer(
                    h_flex()
                        .w_full()
                        .justify_end()
                        .gap_2()
                        .child(
                            Button::new("cancel")
                                .label("Cancel")
                                .on_click(|_, window, cx| window.close_dialog(cx)),
                        )
                        .child(
                            Button::new("normalize")
                                .primary()
                                .label("Normalize")
                                .on_click(move |_, window, cx| {
                                    if clicked(window, cx) {
                                        window.close_dialog(cx);
                                    }
                                }),
                        ),
                )
                .on_ok(move |_, window, cx| keyed(window, cx))
        });
        window.focus(&focus, cx);
    }

    /// Measure the peak, then normalize the version it was measured on to `target` dBFS.
    pub(super) fn normalize_to(
        &mut self,
        target: f64,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.can_normalize() {
            return;
        }
        let Some(file) = &self.file else { return };
        let Some(editing) = file.editing.as_ref() else {
            return;
        };
        let mut meta = editing.file().meta.clone();
        meta.len_samples = editing.len();
        let request = crate::levelling::LevelRequest {
            capture: editing.capture().clone(),
            sources: editing.source_files(),
            meta,
        };
        tracing::info!(source = %request.meta.source.display(), target, "normalizing the level");
        let (job, updates) = crate::levelling::start(request);
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
        self.level_target = target;
        self.save_notice = None;
        self.levelling = Some(Levelling {
            job,
            document: file.id,
            version: editing.version(),
            target,
            selection: self.marked_now(),
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
            .and_then(|full_scale| crate::levelling::gain(peak, full_scale, finished.target));
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
            .is_some_and(|adjusted| editing.adjust(adjusted, finished.selection))
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

/// Normalize to the level typed into `input`, answering whether the dialog can close.
fn accept_level(
    input: &Entity<InputState>,
    error: &Entity<Option<String>>,
    shell: &gpui_kit::WeakEntity<Shell>,
    window: &mut Window,
    cx: &mut gpui_kit::App,
) -> bool {
    let Some(target) = target_level(input, error, cx) else {
        return false;
    };
    let _ = shell.update(cx, |shell, cx| shell.normalize_to(target, window, cx));
    true
}

/// The level typed into `input`, or none with the reason put in `error`.
fn target_level(
    input: &Entity<InputState>,
    error: &Entity<Option<String>>,
    cx: &mut gpui_kit::App,
) -> Option<f64> {
    let text = input.read(cx).value().to_string();
    let lowest = crate::levelling::LOWEST_TARGET;
    let target = crate::numbers::parse::<f64>(text.trim())
        .ok()
        .filter(|target| (lowest..=0.0).contains(target));
    if target.is_none() {
        error.update(cx, |error, cx| {
            let lowest = crate::numbers::number(lowest);
            *error = Some(format!("Enter a level from {lowest} to 0 dBFS"));
            cx.notify();
        });
    }
    target
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
mod tests {
    use super::super::replace_ui::tests::{open_window, wait_until};
    use super::*;
    use crate::editing::{Editing, Placement};
    use argand_core::{Adjustment, SampleSpan};
    use gpui_kit::TestAppContext;

    /// A 16-bit I/Q capture of a 3 kHz tone at a quarter of full scale, opened from a fresh folder and described.
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
        let values = argand_io::testutil::iq_tone(48_000, 24_000.0, 3_000.0, 0.25);
        argand_io::testutil::write_wav(&path, iq_i16, 24_000, &values, 1.0);
        shell.update_in(cx, |shell, window, cx| {
            shell.open(Origin::new(path.clone()), window, cx);
        });
        wait_until(cx, shell, "the file to describe itself", |shell| {
            shell
                .editing()
                .is_some_and(|editing| editing.is_described() && editing.full_scale().is_some())
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

    fn adjustment(cx: &mut gpui_kit::VisualTestContext, shell: &Entity<Shell>) -> Adjustment {
        shell.read_with(cx, |shell, _| shell.editing().unwrap().adjustment())
    }

    #[gpui_kit::test]
    fn inverting_is_refused_with_a_selection_and_saved_conjugated_without_one(
        cx: &mut TestAppContext,
    ) {
        let (shell, cx) = open_window(cx);
        let path = opened(cx, &shell, "invert-ui");
        let selections = [
            (SampleSpan::between(100, 200), None),
            (None, argand_core::FrequencyBand::between(1_000.0, 2_000.0)),
            (
                SampleSpan::between(100, 200),
                argand_core::FrequencyBand::between(1_000.0, 2_000.0),
            ),
        ];
        for (time, band) in selections {
            shell.update(cx, |shell, _| {
                shell.selection = time;
                shell.band = band;
            });
            assert!(!shell.read_with(cx, |shell, _| shell.edit_commands_available().invert));
            shell.update_in(cx, |_, window, cx| {
                window.dispatch_action(Box::new(editing_ui::InvertSpectrum), cx);
            });
            cx.run_until_parked();
            assert!(adjustment(cx, &shell).is_identity(), "{time:?} {band:?}");
        }
        shell.update(cx, |shell, _| {
            shell.selection = None;
            shell.band = None;
        });
        let copied = shell.update(cx, |shell, _| {
            shell
                .editing()
                .unwrap()
                .copy(SampleSpan::between(0, 10).unwrap())
        });
        assert!(shell.read_with(cx, |shell, _| shell.edit_commands_available().invert));
        shell.update_in(cx, |_, window, cx| {
            window.dispatch_action(Box::new(editing_ui::InvertSpectrum), cx);
        });
        cx.run_until_parked();
        assert!(adjustment(cx, &shell).invert());
        shell.read_with(cx, |shell, _| {
            let editing = shell.editing().unwrap();
            assert!(editing.is_dirty() && !editing.is_untouched());
            assert_eq!(editing.capture().pieces().len(), 1);
            assert!(shell.edit_commands_available().inverted);
        });
        let refused = shell.update(cx, |shell, _| {
            let span = SampleSpan::between(0, 10).unwrap();
            shell
                .file
                .as_mut()
                .unwrap()
                .editing
                .as_mut()
                .unwrap()
                .paste(&copied, Placement::Replace(span))
        });
        assert_eq!(refused, Err(crate::editing::PasteError::Adjustment));
        shell.update_in(cx, |_, window, cx| {
            window.dispatch_action(Box::new(editing_ui::Undo), cx);
        });
        cx.run_until_parked();
        assert!(adjustment(cx, &shell).is_identity());
        shell.update_in(cx, |_, window, cx| {
            window.dispatch_action(Box::new(editing_ui::Redo), cx);
        });
        cx.run_until_parked();
        assert!(adjustment(cx, &shell).invert());

        let before = read(&path);
        shell.update_in(cx, |shell, window, cx| shell.save_over(window, cx));
        wait_until(cx, &shell, "the saved file to open again", |shell| {
            shell.saving.is_none()
                && shell.editing().is_some_and(|editing| {
                    !editing.is_dirty() && editing.adjustment().is_identity()
                })
        });
        let after = read(&path);
        for (n, (before, after)) in before
            .chunks_exact(2)
            .zip(after.chunks_exact(2))
            .enumerate()
        {
            assert_eq!(after[0], before[0], "I of sample {n}");
            let expected = (-before[1]).min(32_767.0 / 32_768.0);
            assert_eq!(after[1], expected, "Q of sample {n}");
        }
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[gpui_kit::test]
    fn normalizing_measures_the_peak_and_saves_it_at_the_target(cx: &mut TestAppContext) {
        let (shell, cx) = open_window(cx);
        let path = opened(cx, &shell, "normalize-ui");
        shell.update_in(cx, |shell, window, cx| {
            shell.normalize_to(-6.020_599_913_279_624, window, cx);
        });
        assert!(shell.read_with(cx, |shell, _| {
            shell.levelling.is_some()
                && !shell.can_save(false)
                && shell.can_save_over().is_err()
                && shell.edit_commands_available() == editing_ui::EditCommands::default()
        }));
        wait_until(cx, &shell, "the level to be measured", |shell| {
            shell.levelling.is_none() && shell.editing().is_some_and(Editing::is_dirty)
        });
        let gain = adjustment(cx, &shell).gain();
        assert!((gain - 2.0).abs() < 1e-3, "{gain}");
        assert_eq!(
            adjustment_text(adjustment(cx, &shell)).as_deref(),
            Some("+6.0 dB")
        );
        let versions = shell.read_with(cx, |shell, _| shell.editing().unwrap().version());
        shell.update_in(cx, |shell, window, cx| {
            shell.normalize_to(-6.020_599_913_279_624, window, cx);
        });
        wait_until(cx, &shell, "the level to be measured again", |shell| {
            shell.levelling.is_none()
        });
        assert_eq!(
            shell.read_with(cx, |shell, _| shell.editing().unwrap().version()),
            versions,
            "the same level adds no version"
        );

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
                _scratch: editing.scratches(),
            };
            shell.start_save(request, of, window, cx);
        });
        wait_until(cx, &shell, "the save to end", |shell| {
            shell.saving.is_none()
        });
        let peak = read(&target)
            .chunks_exact(2)
            .map(|sample| f64::from(sample[0]).hypot(f64::from(sample[1])))
            .fold(0.0, f64::max);
        assert!((peak - 0.5).abs() < 1e-3, "{peak}");
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[gpui_kit::test]
    fn opening_another_file_drops_a_measurement_in_progress(cx: &mut TestAppContext) {
        let (shell, cx) = open_window(cx);
        let path = opened(cx, &shell, "normalize-ui-open");
        shell.update_in(cx, |shell, window, cx| {
            shell.normalize_to(-1.0, window, cx);
            shell.open(Origin::new(path.clone()), window, cx);
        });
        assert!(shell.read_with(cx, |shell, _| shell.levelling.is_none()));
        wait_until(cx, &shell, "the file to open again", |shell| {
            shell.editing().is_some_and(Editing::is_described)
        });
        assert!(shell.read_with(cx, |shell, _| shell.editing().unwrap().is_untouched()));
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }
}
