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
mod tests {
    use super::*;
    use argand_io::testutil::TempDir;

    #[test]
    fn a_reopen_error_names_its_file() {
        let path = PathBuf::from("/captures/a.wav");
        let error = anyhow::Error::from(IoError::Source {
            path: path.clone(),
            source: SourceError::Changed,
        })
        .context("cannot draw it");
        assert_eq!(changed_file(&error), Some(Some(path)));
    }

    #[test]
    fn a_reader_error_names_no_file() {
        let error = anyhow::Error::from(SourceError::Changed).context("transform failed");
        assert_eq!(changed_file(&error), Some(None));
    }

    #[test]
    fn an_unnamed_change_takes_the_file_it_was_read_from() {
        let path = PathBuf::from("/captures/b.flac");
        let error = anyhow::Error::from(SourceError::Changed).context("minimap");
        assert_eq!(
            changed_file(&naming(error, &path)),
            Some(Some(path.clone()))
        );
        let other = anyhow::Error::from(IoError::Source {
            path: PathBuf::from("/captures/a.wav"),
            source: SourceError::Changed,
        });
        assert_eq!(
            changed_file(&naming(other, &path)),
            Some(Some(PathBuf::from("/captures/a.wav")))
        );
    }

    #[test]
    fn other_errors_are_not_changes() {
        let error = anyhow::Error::from(SourceError::Decode("bad frame".into()));
        assert_eq!(changed_file(&error), None);
    }

    #[test]
    fn a_rewritten_file_is_found() {
        let dir = TempDir::new("watched");
        let a = dir.join("a.raw");
        let b = dir.join("b.raw");
        std::fs::write(&a, [0u8; 8]).unwrap();
        std::fs::write(&b, [0u8; 8]).unwrap();
        let watched = Watched(vec![
            (a.clone(), SourceStamp::of(&a).unwrap()),
            (b.clone(), SourceStamp::of(&b).unwrap()),
        ]);
        assert_eq!(watched.first_changed(), None);
        let file = std::fs::OpenOptions::new().write(true).open(&b).unwrap();
        file.set_modified(std::time::SystemTime::now() + std::time::Duration::from_secs(5))
            .unwrap();
        drop(file);
        assert_eq!(watched.first_changed(), Some(b.clone()));
        std::fs::remove_file(&b).unwrap();
        assert_eq!(watched.first_changed(), Some(b));
    }
}
