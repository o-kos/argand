//! Availability of start-page captures, checked without blocking the window.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::session::{RECENT_LIMIT, Recent};

pub struct RecentFiles {
    entries: Vec<Recent>,
    available: Vec<bool>,
}

impl RecentFiles {
    pub fn new(entries: &[Recent]) -> Self {
        let entries: Vec<_> = entries.iter().take(RECENT_LIMIT).cloned().collect();
        Self {
            available: vec![false; entries.len()],
            entries,
        }
    }

    pub fn check(&self) -> async_channel::Receiver<(usize, bool)> {
        check_paths(
            self.entries.iter().map(|entry| entry.path.clone()),
            Arc::new(|path| path.is_file()),
        )
    }

    pub fn apply(&mut self, index: usize, exists: bool) {
        if let Some(available) = self.available.get_mut(index) {
            *available = exists;
        }
    }

    pub fn shortcut(&self, index: usize) -> Option<Recent> {
        if index >= 9 {
            return None;
        }
        self.visible().get(index).cloned()
    }

    pub fn visible(&self) -> Vec<Recent> {
        self.entries
            .iter()
            .zip(&self.available)
            .filter(|(_, available)| **available)
            .map(|(entry, _)| entry.clone())
            .collect()
    }
}

fn check_paths(
    paths: impl Iterator<Item = PathBuf>,
    probe: Arc<dyn Fn(&Path) -> bool + Send + Sync>,
) -> async_channel::Receiver<(usize, bool)> {
    let (sender, receiver) = async_channel::unbounded();
    for (index, path) in paths.enumerate() {
        let sender = sender.clone();
        let probe = probe.clone();
        // A network stat may never return. Independent, detached workers keep
        // other results and process shutdown free of that filesystem's timeout.
        if let Err(error) = std::thread::Builder::new()
            .name("argand-recent".to_owned())
            .spawn(move || {
                let _ = sender.send_blocking((index, probe(&path)));
            })
        {
            tracing::warn!(%error, "cannot check a recent file");
        }
    }
    receiver
}

#[cfg(test)]
mod tests {
    include!("recent_tests.rs");
}
