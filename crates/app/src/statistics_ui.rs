//! Scheduling current-capture metrics independently of view rendering.

use super::*;

#[derive(Clone, PartialEq)]
struct Key {
    document: u64,
    version: u64,
    source_revision: u64,
    adjustment: argand_core::Adjustment,
    changed: bool,
}

#[derive(Default)]
pub(super) struct State {
    key: Option<Key>,
    task: Option<Task<()>>,
    finished: bool,
    cache: Arc<crate::sample_statistics::Cache>,
}

impl Shell {
    fn statistics_key(&self) -> Option<Key> {
        let file = self.file.as_ref()?;
        let editing = file.editing.as_ref()?;
        Some(Key {
            document: file.id,
            version: editing.version(),
            source_revision: editing.statistics_revision(),
            adjustment: editing.adjustment(),
            changed: file.state.document.is_changed(),
        })
    }

    pub(super) fn prepare_statistics(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let key = self.statistics_key();
        if self.statistics.key != key {
            self.statistics.task = None;
            self.statistics.finished = false;
            self.statistics.key = key.clone();
            if let Some(file) = &mut self.file {
                file.state.document.set_statistics(None);
            }
        }
        if self.statistics.task.is_some() || self.statistics.finished {
            return;
        }
        let Some(key) = key else {
            return;
        };
        let Some(file) = &self.file else {
            return;
        };
        if file.state.document.is_changed() {
            self.statistics.finished = true;
            if let Some(file) = &mut self.file {
                file.state
                    .document
                    .set_statistics(Some(Err("Source changed".into())));
            }
            return;
        }
        let Some(editing) = &file.editing else {
            return;
        };
        let Some(request) = editing.statistics_request() else {
            return;
        };
        let sources = editing.retained_sources();
        let lease = file.lease.clone();
        let cache = self.statistics.cache.clone();
        let (sender, receiver) = async_channel::bounded(1);
        let fallback = sender.clone();
        let spawned = std::thread::Builder::new()
            .name("argand-statistics".into())
            .spawn(move || {
                let _held = (sources, lease);
                let result =
                    crate::sample_statistics::measure(&request, &cache, &|| !sender.is_closed());
                let _ = sender.try_send(result);
            });
        if let Err(error) = spawned {
            let _ = fallback.try_send(Err(error.into()));
        }
        self.statistics.task = Some(cx.spawn_in(window, async move |shell, cx| {
            let Ok(result) = receiver.recv().await else {
                return;
            };
            let _ = shell.update(cx, |shell, cx| shell.receive_statistics(key, result, cx));
        }));
    }
    fn receive_statistics(
        &mut self,
        key: Key,
        result: anyhow::Result<crate::sample_statistics::Metrics>,
        cx: &mut Context<Self>,
    ) {
        if self.statistics_key().as_ref() != Some(&key) {
            if self.statistics.key.as_ref() == Some(&key) {
                self.statistics.task = None;
            }
            cx.notify();
            return;
        }
        self.statistics.finished = true;
        self.statistics.task = None;
        if let Some(file) = &mut self.file {
            if let Err(error) = &result {
                file.state.document.note_failure(error);
            }
            file.state
                .document
                .set_statistics(Some(result.map_err(|_| "Unavailable".into())));
        }
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::TestAppContext;
    #[gpui_kit::test]
    fn stale_capture_metrics_are_rejected_and_view_changes_keep_the_same_query(
        cx: &mut TestAppContext,
    ) {
        use super::super::conversion_ui::tests::panel_window;
        use super::super::replace_ui::tests::{wait_until, write_wave};
        let (shell, cx) = panel_window(cx);
        let dir = argand_io::testutil::TempDir::new("statistics-route");
        let path = dir.join("source.wav");
        write_wave(&path, 2400);
        shell.update_in(cx, |shell, window, cx| {
            shell.open(Origin::new(path), window, cx)
        });
        wait_until(cx, &shell, "statistics source", |shell| {
            shell
                .editing()
                .is_some_and(|editing| editing.is_described())
        });
        shell.update_in(cx, |shell, _, cx| {
            let before = shell.statistics_key().unwrap();
            shell.settings.overlap = 25;
            assert!(
                shell.statistics_key().as_ref() == Some(&before),
                "FFT controls are not amplitude-query generations"
            );
            shell
                .editing_mut()
                .unwrap()
                .delete(argand_core::SampleSpan::between(100, 200).unwrap());
            shell
                .file
                .as_mut()
                .unwrap()
                .state
                .document
                .set_statistics(None);
            shell.receive_statistics(
                before,
                Ok(crate::sample_statistics::Metrics {
                    peak: 999.,
                    rms: 999.,
                }),
                cx,
            );
            assert!(
                shell
                    .file
                    .as_ref()
                    .unwrap()
                    .state
                    .document
                    .file_summary()
                    .unwrap()
                    .hint
                    .value
                    .contains("Peak: Calculating…")
            );
            let current = shell.statistics_key().unwrap();
            shell.receive_statistics(
                current,
                Ok(crate::sample_statistics::Metrics { peak: 5., rms: 3. }),
                cx,
            );
            assert!(
                shell
                    .file
                    .as_ref()
                    .unwrap()
                    .state
                    .document
                    .file_summary()
                    .unwrap()
                    .hint
                    .value
                    .contains("RMS: 3")
            );
        });
    }
    #[gpui_kit::test]
    fn same_scale_save_rebuilds_statistics_behind_a_reused_complete_envelope(
        cx: &mut TestAppContext,
    ) {
        use super::super::conversion_ui::tests::panel_window;
        use super::super::replace_ui::tests::{wait_for_save_chooser, wait_until, write_wave};
        let (shell, cx) = panel_window(cx);
        let dir = argand_io::testutil::TempDir::new("statistics-rebind");
        let source = dir.join("source.wav");
        let target = dir.join("saved.wav");
        write_wave(&source, 2400);
        shell.update_in(cx, |shell, window, cx| {
            shell.open(Origin::new(source), window, cx)
        });
        wait_until(cx, &shell, "source statistics", |shell| {
            shell.statistics.finished
                && shell
                    .editing()
                    .is_some_and(|editing| editing.statistics_request().is_some())
        });
        shell.update_in(cx, |shell, window, cx| {
            shell
                .editing_mut()
                .unwrap()
                .delete(argand_core::SampleSpan::between(100, 200).unwrap());
            shell.edited(window, cx);
            assert!(shell.editing().unwrap().minimap().unwrap().complete);
            shell.save_as(false, window, cx);
        });
        wait_for_save_chooser(cx);
        let chosen = target.clone();
        cx.simulate_new_path_selection(move |_| Some(chosen));
        wait_until(cx, &shell, "rebound source statistics", |shell| {
            shell.editing().is_some_and(|editing| {
                editing.file().meta.source == target
                    && editing.is_described()
                    && editing.statistics_request().is_some()
            }) && shell.statistics.finished
        });
        shell.read_with(cx, |shell, _| {
            assert_eq!(shell.editing().unwrap().len(), 2300);
            let summary = shell
                .file
                .as_ref()
                .unwrap()
                .state
                .document
                .file_summary()
                .unwrap();
            let peak: f64 = crate::numbers::parse(
                &summary
                    .hint
                    .rows
                    .iter()
                    .find(|(label, _)| label == "Peak")
                    .unwrap()
                    .1,
            )
            .unwrap();
            assert!((peak - 2399. * 2f64.sqrt()).abs() < 0.001);
        });
    }
}
