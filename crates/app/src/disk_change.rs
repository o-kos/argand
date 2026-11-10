//! Telling that a file a capture reads is no longer the one that was opened.

use std::path::{Path, PathBuf};

use argand_core::SourceError;
use argand_io::IoError;
use argand_io::write::SourceStamp;

/// Whether `error` says a file changed on disk, and which one when it names it.
pub fn changed_file(error: &anyhow::Error) -> Option<Option<PathBuf>> {
    error.chain().find_map(|cause| {
        if let Some(io) = cause.downcast_ref::<IoError>()
            && io.is_changed()
        {
            let IoError::Source { path, .. } = io else {
                return Some(None);
            };
            return Some(Some(path.clone()));
        }
        cause
            .downcast_ref::<SourceError>()
            .is_some_and(|source| matches!(source, SourceError::Changed))
            .then_some(None)
    })
}

/// `error` with `path` named as the changed file, when it says a file changed without naming one.
pub fn naming(error: anyhow::Error, path: &Path) -> anyhow::Error {
    if changed_file(&error) != Some(None) {
        return error;
    }
    let context = format!("{error:#}");
    anyhow::Error::from(IoError::Source {
        path: path.to_owned(),
        source: SourceError::Changed,
    })
    .context(context)
}

/// The files a capture reads, each with the stamp it was opened with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Watched(pub Vec<(PathBuf, SourceStamp)>);

impl Watched {
    /// The first file whose stamp is no longer the one it was opened with, stamped by path.
    ///
    /// It can block on a slow mount, so it never runs on the window's thread.
    pub fn first_changed(&self) -> Option<PathBuf> {
        self.0
            .iter()
            .find(|(path, stamp)| !still(path, *stamp))
            .map(|(path, _)| path.clone())
    }
}

fn still(path: &Path, stamp: SourceStamp) -> bool {
    SourceStamp::of(path).ok() == Some(stamp)
}

#[cfg(test)]
#[path = "disk_change_test.rs"]
mod tests;
