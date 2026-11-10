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
        if editing.statistics_failed() {
            self.statistics.finished = true;
            if let Some(file) = &mut self.file {
                file.state
                    .document
                    .set_statistics(Some(Err("Unavailable".into())));
            }
            return;
        }
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
#[path = "statistics_ui_test.rs"]
mod tests;
