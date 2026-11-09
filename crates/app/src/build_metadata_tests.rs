#[path = "build_metadata.rs"]
mod metadata;

use std::path::{Path, PathBuf};
use std::process::Command;

struct Repository(PathBuf);

impl Repository {
    fn new() -> Self {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "argand-about-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).expect("temporary repository");
        Self(path)
    }

    fn git(&self, args: &[&str]) -> String {
        let result = Command::new("git")
            .current_dir(&self.0)
            .env("GIT_AUTHOR_NAME", "Test")
            .env("GIT_AUTHOR_EMAIL", "test@example.invalid")
            .env("GIT_COMMITTER_NAME", "Test")
            .env("GIT_COMMITTER_EMAIL", "test@example.invalid")
            .env("GIT_AUTHOR_DATE", "2026-11-09T12:34:56+03:00")
            .env("GIT_COMMITTER_DATE", "2026-11-09T12:34:56+03:00")
            .args(args)
            .output()
            .expect("Git executes");
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        String::from_utf8(result.stdout)
            .expect("Git text")
            .trim()
            .into()
    }
}

impl Drop for Repository {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn an_archive_never_borrows_an_enclosing_repository_identity() {
    let repo = Repository::new();
    repo.git(&["init", "-q"]);
    repo.git(&[
        "-c",
        "commit.gpgsign=false",
        "commit",
        "--allow-empty",
        "-qm",
        "Initial",
    ]);
    let archive = repo.0.join("archive");
    std::fs::create_dir(&archive).expect("archive");
    let got = metadata::read(&archive);
    assert_eq!(
        (got.hash.as_str(), got.date.as_str(), got.tag.as_str()),
        ("unavailable", "unavailable", "unavailable")
    );
}

#[test]
fn tags_are_exact_and_the_date_is_the_commit_date_including_offset() {
    let repo = Repository::new();
    repo.git(&["init", "-q"]);
    repo.git(&[
        "-c",
        "commit.gpgsign=false",
        "commit",
        "--allow-empty",
        "-qm",
        "Initial",
    ]);
    repo.git(&["tag", "v0.1.0"]);
    let tagged = metadata::read(&repo.0);
    assert_eq!(tagged.tag, "v0.1.0");
    assert_eq!(tagged.date, "2026-11-09T12:34:56+03:00");
    assert_eq!(tagged.hash, repo.git(&["rev-parse", "--short=12", "HEAD"]));
    repo.git(&[
        "-c",
        "commit.gpgsign=false",
        "commit",
        "--allow-empty",
        "-qm",
        "Next",
    ]);
    assert_eq!(metadata::read(&repo.0).tag, "untagged");
    repo.git(&["checkout", "--detach", "-q", "HEAD~1"]);
    assert_eq!(metadata::read(&repo.0).hash, tagged.hash);
    assert_eq!(metadata::read(&repo.0).tag, tagged.tag);
}

fn watched(paths: &[PathBuf], path: &Path) -> bool {
    let path = path.canonicalize().expect("existing input");
    paths
        .iter()
        .any(|input| input.canonicalize().ok().as_ref() == Some(&path))
}

#[test]
fn linked_worktrees_track_their_head_and_shared_refs_and_tags() {
    let repo = Repository::new();
    let worktree = Repository::new();
    repo.git(&["init", "-q"]);
    repo.git(&[
        "-c",
        "commit.gpgsign=false",
        "commit",
        "--allow-empty",
        "-qm",
        "Initial",
    ]);
    repo.git(&[
        "worktree",
        "add",
        "--detach",
        worktree.0.to_str().expect("temporary path"),
        "HEAD",
    ]);
    let inputs = metadata::inputs(&worktree.0);
    let head = Command::new("git")
        .current_dir(&worktree.0)
        .args(["rev-parse", "--git-path", "HEAD"])
        .output()
        .expect("Git path");
    let head = PathBuf::from(String::from_utf8(head.stdout).expect("path").trim());
    assert!(watched(&inputs, &worktree.0.join(".git")));
    assert!(watched(&inputs, &head));
    assert!(watched(&inputs, &repo.0.join(".git/refs")));
    assert!(
        !watched(&inputs, &repo.0.join(".git")),
        "never scan objects, logs or index"
    );
    assert_eq!(
        metadata::read(&worktree.0).hash,
        metadata::read(&repo.0).hash
    );
    repo.git(&["tag", "v0.2.0"]);
    assert_eq!(metadata::read(&worktree.0).tag, "v0.2.0");
    repo.git(&["pack-refs", "--all"]);
    repo.git(&["tag", "-d", "v0.2.0"]);
    assert_eq!(metadata::read(&worktree.0).tag, "untagged");
    assert!(watched(
        &metadata::inputs(&worktree.0),
        &repo.0.join(".git/packed-refs")
    ));
}

#[test]
fn the_tooltip_uses_the_built_identity() {
    assert_eq!(
        super::tooltip(),
        format!(
            "About Argand\n{} · {}\n{}",
            super::TAG,
            super::HASH,
            super::DATE
        )
    );
    assert_eq!(super::VERSION, env!("CARGO_PKG_VERSION"));
    assert_eq!(super::LICENSE, "MIT");
    assert_eq!(super::REPOSITORY, "https://github.com/o-kos/argand");
}
