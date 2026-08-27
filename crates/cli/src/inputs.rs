//! Turning the command line's inputs into the list of files to process.
//!
//! An argument is either an exact path or a mask over the filenames of one
//! directory. Exact paths are not checked here: an unusable one fails when it
//! is opened, like any other file, which is what lets a batch carry on past
//! it and keeps a single file's exit status what it always was.

use std::collections::HashSet;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use crate::mask::{Mask, MaskError, has_meta};

#[derive(Debug, thiserror::Error)]
pub enum ResolveError {
    #[error("{pattern}")]
    Mask {
        pattern: String,
        #[source]
        source: MaskError,
    },
    #[error("{pattern}: a mask matches filenames in one directory, not directory names")]
    MaskInDirectory { pattern: String },
    #[error("cannot list {dir}")]
    ListDir {
        dir: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("{pattern} matched no files")]
    NoMatch { pattern: String },
}

/// Expand every argument, in the order given, keeping each file once.
pub fn resolve(inputs: &[PathBuf]) -> Result<Vec<PathBuf>, ResolveError> {
    let mut files = Vec::with_capacity(inputs.len());
    let mut seen = HashSet::new();
    for input in inputs {
        for path in expand(input)? {
            // The same file reached by two spellings is still one file.
            let key = path.canonicalize().unwrap_or_else(|_| path.clone());
            if seen.insert(key) {
                files.push(path);
            }
        }
    }
    Ok(files)
}

fn expand(input: &Path) -> Result<Vec<PathBuf>, ResolveError> {
    let pattern = input.to_string_lossy().into_owned();

    if pattern.contains("**") {
        return Err(ResolveError::Mask {
            pattern,
            source: MaskError::Recursive,
        });
    }

    let Some(name) = input.file_name() else {
        // `.`, `..` and `/` have no filename to match against.
        return Ok(vec![input.to_owned()]);
    };
    let name = name.to_string_lossy().into_owned();

    let dir = input.parent().filter(|p| !p.as_os_str().is_empty());
    if dir.is_some_and(|d| has_meta(&d.to_string_lossy())) {
        return Err(ResolveError::MaskInDirectory { pattern });
    }
    if !has_meta(&name) {
        return Ok(vec![input.to_owned()]);
    }

    let mask = Mask::new(&name).map_err(|source| ResolveError::Mask {
        pattern: pattern.clone(),
        source,
    })?;
    let root = dir.unwrap_or_else(|| Path::new("."));
    let entries = std::fs::read_dir(root).map_err(|source| ResolveError::ListDir {
        dir: root.to_owned(),
        source,
    })?;

    let mut matched: Vec<(OsString, PathBuf)> = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|source| ResolveError::ListDir {
            dir: root.to_owned(),
            source,
        })?;
        let file_name = entry.file_name();
        if !mask.matches(&file_name.to_string_lossy()) || entry.path().is_dir() {
            continue;
        }
        // Keep the spelling the argument used: a bare mask yields bare names.
        let path = match dir {
            Some(dir) => dir.join(&file_name),
            None => PathBuf::from(&file_name),
        };
        matched.push((file_name, path));
    }

    if matched.is_empty() {
        return Err(ResolveError::NoMatch { pattern });
    }
    matched.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(matched.into_iter().map(|(_, path)| path).collect())
}

#[cfg(test)]
mod tests {
    include!("inputs_tests.rs");
}
