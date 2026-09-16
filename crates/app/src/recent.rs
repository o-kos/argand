//! Shared recent-file availability, checked without blocking the window.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::session::{RECENT_LIMIT, Recent, normalize_recent_path};

type Availability = (PathBuf, bool);
type Probe = Arc<dyn Fn(&Path) -> bool + Send + Sync>;

pub struct RecentFiles {
    entries: Vec<Recent>,
    current: Option<PathBuf>,
    available: HashSet<PathBuf>,
    pending: HashSet<PathBuf>,
    sender: async_channel::Sender<Availability>,
    receiver: async_channel::Receiver<Availability>,
    probe: Probe,
}

impl RecentFiles {
    pub fn new(entries: &[Recent]) -> Self {
        let (sender, receiver) = async_channel::unbounded();
        Self {
            entries: entries.iter().take(RECENT_LIMIT).cloned().collect(),
            current: None,
            available: HashSet::new(),
            pending: HashSet::new(),
            sender,
            receiver,
            probe: Arc::new(|path| path.is_file()),
        }
    }

    pub fn updates(&self) -> async_channel::Receiver<Availability> {
        self.receiver.clone()
    }

    pub fn set_current(&mut self, path: &Path) {
        self.current = Some(normalize_recent_path(path));
    }

    pub fn clear_current(&mut self) {
        self.current = None;
    }

    pub fn refresh(&mut self, entries: &[Recent]) {
        self.entries = entries.iter().take(RECENT_LIMIT).cloned().collect();
        self.available
            .retain(|path| self.entries.iter().any(|entry| entry.path == *path));
        for path in self
            .entries
            .iter()
            .map(|entry| entry.path.clone())
            .collect::<Vec<_>>()
        {
            self.start_check(path);
        }
    }

    pub fn apply(&mut self, path: PathBuf, exists: bool) {
        self.pending.remove(&path);
        if exists && self.entries.iter().any(|entry| entry.path == path) {
            self.available.insert(path);
        } else {
            self.available.remove(&path);
        }
    }

    fn start_check(&mut self, path: PathBuf) {
        if !self.pending.insert(path.clone()) {
            return;
        }
        let sender = self.sender.clone();
        let probe = self.probe.clone();
        let checked = path.clone();
        // One outstanding probe per path, even if history changes. A stuck old
        // path must not prevent a newly opened local file from being checked.
        if let Err(error) = std::thread::Builder::new()
            .name("argand-recent".to_owned())
            .spawn(move || {
                let exists = probe(&checked);
                let _ = sender.send_blocking((checked, exists));
            })
        {
            self.pending.remove(&path);
            self.available.remove(&path);
            tracing::warn!(%error, "cannot check a recent file");
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
            .filter(|entry| self.available.contains(&entry.path))
            .filter(|entry| self.current.as_ref() != Some(&entry.path))
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    include!("recent_tests.rs");
}
