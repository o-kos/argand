//! Build-time Git identity; source archives never inherit an enclosing repository.

use std::path::{Path, PathBuf};
use std::process::Command;

#[non_exhaustive]
pub struct Metadata {
    pub hash: String,
    pub date: String,
    pub tag: String,
}

// Repository-local variables reported by `git rev-parse --local-env-vars`, plus namespace.
const REPOSITORY_ENV: &[&str] = &[
    "GIT_ALTERNATE_OBJECT_DIRECTORIES",
    "GIT_CONFIG",
    "GIT_CONFIG_PARAMETERS",
    "GIT_CONFIG_COUNT",
    "GIT_OBJECT_DIRECTORY",
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_IMPLICIT_WORK_TREE",
    "GIT_GRAFT_FILE",
    "GIT_INDEX_FILE",
    "GIT_NO_REPLACE_OBJECTS",
    "GIT_REPLACE_REF_BASE",
    "GIT_PREFIX",
    "GIT_SHALLOW_FILE",
    "GIT_COMMON_DIR",
    "GIT_NAMESPACE",
];

pub(super) fn command(root: &Path) -> Command {
    let mut command = Command::new("git");
    command.current_dir(root);
    for name in REPOSITORY_ENV {
        command.env_remove(name);
    }
    command
}

fn git(root: &Path, args: &[&str]) -> Option<String> {
    if !root.join(".git").exists() {
        return None;
    }
    let output = command(root).args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8(output.stdout).ok()?.trim().to_owned();
    (!value.is_empty() && !value.contains(['\n', '\r'])).then_some(value)
}

pub fn read(root: &Path) -> Metadata {
    let hash = git(root, &["rev-parse", "--short=12", "HEAD"]);
    let date = git(root, &["show", "-s", "--format=%cI", "HEAD"]);
    let tag = hash.as_ref().map(|_| {
        git(root, &["describe", "--tags", "--exact-match", "HEAD"])
            .unwrap_or_else(|| "untagged".into())
    });
    Metadata {
        hash: hash.unwrap_or_else(|| "unavailable".into()),
        date: date.unwrap_or_else(|| "unavailable".into()),
        tag: tag.unwrap_or_else(|| "unavailable".into()),
    }
}

pub fn inputs(root: &Path) -> Vec<PathBuf> {
    [
        "Cargo.toml",
        "Cargo.lock",
        "crates/app/build.rs",
        "crates/app/Cargo.toml",
        "crates/app/src",
        "crates/app/assets",
        "crates/core/Cargo.toml",
        "crates/core/src",
        "crates/dsp/Cargo.toml",
        "crates/dsp/src",
        "crates/edit/Cargo.toml",
        "crates/edit/src",
        "crates/io/Cargo.toml",
        "crates/io/src",
    ]
    .map(|path| root.join(path))
    .into()
}
