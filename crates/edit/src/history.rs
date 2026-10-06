//! Undo and redo as a list of capture versions.

use crate::Capture;

/// Every version of a capture with the state each left behind, and which one was saved.
///
/// `S` is whatever the caller restores along with a version, such as its selection.
#[derive(Debug, Clone)]
pub struct History<S> {
    versions: Vec<Version<S>>,
    current: usize,
    saved: Option<usize>,
    next_serial: u64,
}

#[derive(Debug, Clone)]
struct Version<S> {
    capture: Capture,
    state: S,
    serial: u64,
}

impl<S> History<S> {
    /// A history whose only version is the capture as it was opened, and saved.
    pub fn new(capture: Capture, state: S) -> Self {
        Self {
            versions: vec![Version {
                capture,
                state,
                serial: 0,
            }],
            current: 0,
            saved: Some(0),
            next_serial: 1,
        }
    }

    pub fn capture(&self) -> &Capture {
        &self.versions[self.current].capture
    }

    pub fn state(&self) -> &S {
        &self.versions[self.current].state
    }

    /// A number no other version of this history shares, for telling stale work apart.
    pub fn version(&self) -> u64 {
        self.versions[self.current].serial
    }

    /// Make `capture` the current version, dropping whatever could have been redone.
    pub fn apply(&mut self, capture: Capture, state: S) {
        self.versions.truncate(self.current + 1);
        if self.saved.is_some_and(|saved| saved > self.current) {
            self.saved = None;
        }
        self.versions.push(Version {
            capture,
            state,
            serial: self.next_serial,
        });
        self.next_serial += 1;
        self.current += 1;
    }

    pub fn can_undo(&self) -> bool {
        self.current > 0
    }

    pub fn can_redo(&self) -> bool {
        self.current + 1 < self.versions.len()
    }

    /// Step back one version, answering whether there was one.
    pub fn undo(&mut self) -> bool {
        let moved = self.can_undo();
        if moved {
            self.current -= 1;
        }
        moved
    }

    /// Step forward one version, answering whether there was one.
    pub fn redo(&mut self) -> bool {
        let moved = self.can_redo();
        if moved {
            self.current += 1;
        }
        moved
    }

    /// Whether the current version differs from the one last saved.
    pub fn is_dirty(&self) -> bool {
        self.saved != Some(self.current)
    }

    pub fn mark_saved(&mut self) {
        self.saved = Some(self.current);
    }
}

#[cfg(test)]
mod tests {
    include!("history_tests.rs");
}
