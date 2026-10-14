//! Files holding computed samples while a capture is edited, removed once nothing refers to them.
//!
//! Each running instance keeps its files in `<temp>/argand-scratch/<pid>/`
//! while it holds a lock on `<temp>/argand-scratch/<pid>.lock`. A lock that can
//! be taken belongs to an instance that has gone, so its folder is removed the
//! next time any instance starts, which is all a crash leaves behind.

use std::fs::{self, File, TryLockError};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};

/// The folder of this instance, with the lock that keeps other instances from removing it.
#[derive(Debug)]
pub struct Folder {
    path: PathBuf,
    next: AtomicU64,
    _lock: File,
}

impl Folder {
    /// Take this instance's folder under `root`, first removing those of instances that have gone.
    pub fn open(root: &Path, pid: u32) -> std::io::Result<Self> {
        fs::create_dir_all(root)?;
        sweep(root);
        let lock = File::options()
            .create(true)
            .write(true)
            .truncate(false)
            .open(root.join(format!("{pid}.lock")))?;
        match lock.try_lock() {
            Ok(()) => {}
            Err(TryLockError::WouldBlock) => {
                return Err(std::io::Error::other("the scratch folder is in use"));
            }
            Err(TryLockError::Error(error)) => return Err(error),
        }
        let path = root.join(pid.to_string());
        // Files a gone instance with this pid left are not ours to read.
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path)?;
        Ok(Self {
            path,
            next: AtomicU64::new(1),
            _lock: lock,
        })
    }

    /// A name no file of this instance has had.
    pub fn new_path(&self, extension: &str) -> PathBuf {
        let n = self.next.fetch_add(1, Ordering::Relaxed);
        self.path.join(format!("{n}.{extension}"))
    }
}

/// Remove the folders of instances whose lock nobody holds.
fn sweep(root: &Path) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_none_or(|extension| extension != "lock") {
            continue;
        }
        let Ok(lock) = File::options().write(true).open(&path) else {
            continue;
        };
        if lock.try_lock().is_err() {
            continue;
        }
        let _ = fs::remove_dir_all(path.with_extension(""));
        drop(lock);
        let _ = fs::remove_file(&path);
    }
}

/// This instance's folder, taken on first use, or none when the system gives no place for it.
pub fn folder() -> Option<&'static Folder> {
    static FOLDER: OnceLock<Option<Folder>> = OnceLock::new();
    FOLDER
        .get_or_init(|| {
            let root = std::env::temp_dir().join("argand-scratch");
            Folder::open(&root, std::process::id())
                .inspect_err(|error| {
                    tracing::warn!(%error, root = %root.display(), "no scratch folder for computed samples");
                })
                .ok()
        })
        .as_ref()
}

/// A scratch file, removed when the last reference to it goes.
#[derive(Debug)]
pub struct Scratch {
    path: PathBuf,
}

impl Scratch {
    /// Take ownership of the file at `path`.
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        if let Err(error) = fs::remove_file(&self.path)
            && error.kind() != std::io::ErrorKind::NotFound
        {
            tracing::debug!(%error, path = %self.path.display(), "scratch file not removed");
        }
    }
}

#[cfg(test)]
mod tests {
    include!("scratch_tests.rs");
}
