//! Build-time Git identity; source archives never inherit an enclosing repository.

use std::path::{Path, PathBuf};
use std::process::Command;

#[non_exhaustive]
pub struct Metadata {
    pub hash: String,
    pub date: String,
    pub tag: String,
}

fn git(root: &Path, args: &[&str]) -> Option<String> {
    if !root.join(".git").exists() {
        return None;
    }
    let output = Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .ok()?;
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
    let marker = root.join(".git");
    let mut paths = Vec::new();
    if marker.is_file() {
        paths.push(marker);
    }
    if let Some(common) = git(root, &["rev-parse", "--git-common-dir"]) {
        let common = root.join(common);
        for name in ["refs", "packed-refs"] {
            let path = common.join(name);
            if path.exists() {
                paths.push(path);
            }
        }
        // Shared refs cover first tags and loose-ref removal when packed-refs is created.
        if let Some(branch) = git(root, &["symbolic-ref", "-q", "HEAD"]) {
            let path = common.join(branch);
            if path.exists() {
                paths.push(path);
            }
        }
    }
    if let Some(head) = git(root, &["rev-parse", "--git-path", "HEAD"]) {
        paths.push(root.join(head));
    }
    paths
}
