#[path = "build_metadata.rs"]
mod metadata;

use std::path::PathBuf;
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

    fn initialized() -> Self {
        let repo = Self::new();
        repo.git(&["init", "-q"]);
        repo.git(&[
            "-c",
            "commit.gpgsign=false",
            "commit",
            "--allow-empty",
            "-qm",
            "Initial",
        ]);
        repo
    }

    fn git(&self, args: &[&str]) -> String {
        let result = metadata::command(&self.0)
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
    let repo = Repository::initialized();
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
    let repo = Repository::initialized();
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

#[test]
fn rebuild_inputs_are_sources_and_manifests_not_git_state() {
    let repo = Repository::initialized();
    let inputs = metadata::inputs(&repo.0);
    assert!(
        inputs
            .iter()
            .all(|path| !path.starts_with(repo.0.join(".git")))
    );
    for name in ["app", "core", "dsp", "edit", "io"] {
        let package = repo.0.join("crates").join(name);
        assert!(inputs.contains(&package.join("src")));
        assert!(inputs.contains(&package.join("Cargo.toml")));
    }
    assert!(inputs.contains(&repo.0.join("Cargo.toml")));
    assert!(inputs.contains(&repo.0.join("Cargo.lock")));
    assert!(inputs.contains(&repo.0.join("crates/app/build.rs")));
    assert!(inputs.contains(&repo.0.join("crates/app/assets")));
    repo.git(&["tag", "new-tag"]);
    repo.git(&["pack-refs", "--all"]);
    assert_eq!(metadata::inputs(&repo.0), inputs);
}

#[test]
fn linked_worktrees_read_their_head_and_shared_tags() {
    let repo = Repository::initialized();
    let worktree = Repository::new();
    repo.git(&[
        "worktree",
        "add",
        "--detach",
        worktree.0.to_str().expect("temporary path"),
        "HEAD",
    ]);
    assert_eq!(
        metadata::read(&worktree.0).hash,
        metadata::read(&repo.0).hash
    );
    repo.git(&["tag", "v0.2.0"]);
    assert_eq!(metadata::read(&worktree.0).tag, "v0.2.0");
    repo.git(&["pack-refs", "--all"]);
    repo.git(&["tag", "-d", "v0.2.0"]);
    assert_eq!(metadata::read(&worktree.0).tag, "untagged");
}

#[test]
fn package_metadata_identifies_the_application() {
    assert_eq!(super::NAME, "Argand");
    assert_eq!(super::VERSION, env!("CARGO_PKG_VERSION"));
    assert_eq!(super::LICENSE, "MIT");
    assert_eq!(super::REPOSITORY, "https://github.com/o-kos/argand");
    assert_eq!(
        super::DESCRIPTION,
        "Signal editor and analyzer for real and complex signals"
    );
}

#[test]
fn inherited_repository_environment_cannot_redirect_git() {
    const CHILD: &str = "ARGAND_METADATA_ISOLATION_CHILD";
    if std::env::var_os(CHILD).is_some() {
        let repo = Repository::initialized();
        let expected = repo.git(&["rev-parse", "--short=12", "HEAD"]);
        assert_eq!(metadata::read(&repo.0).hash, expected);
        assert_eq!(metadata::read(&repo.0).tag, "untagged");
        assert_eq!(
            PathBuf::from(repo.git(&["rev-parse", "--show-toplevel"]))
                .canonicalize()
                .expect("Git root"),
            repo.0.canonicalize().expect("fixture root")
        );
        assert_eq!(repo.git(&["config", "--local", "core.bare"]), "false");
        return;
    }
    let caller = Repository::initialized();
    caller.git(&[
        "-c",
        "commit.gpgsign=false",
        "commit",
        "--allow-empty",
        "-qm",
        "Caller",
    ]);
    caller.git(&["tag", "caller-only"]);
    let state = || {
        [
            caller.git(&["rev-parse", "HEAD"]),
            caller.git(&["show-ref"]),
            caller.git(&["config", "--local", "--list"]),
            caller.git(&["worktree", "list", "--porcelain"]),
        ]
    };
    let before = state();
    let git_dir = caller.0.join(".git");
    let index_before = std::fs::read(git_dir.join("index")).ok();
    let test_name = std::thread::current().name().expect("test name").to_owned();
    let result = Command::new(std::env::current_exe().expect("test executable"))
        .args(["--exact", &test_name, "--nocapture"])
        .env(CHILD, "1")
        .env("GIT_DIR", &git_dir)
        .env("GIT_COMMON_DIR", &git_dir)
        .env("GIT_WORK_TREE", &caller.0)
        .env("GIT_INDEX_FILE", git_dir.join("index"))
        .env("GIT_OBJECT_DIRECTORY", git_dir.join("objects"))
        .env("GIT_ALTERNATE_OBJECT_DIRECTORIES", git_dir.join("objects"))
        .env("GIT_NAMESPACE", "foreign")
        .env("GIT_CONFIG", git_dir.join("config"))
        .env("GIT_CONFIG_PARAMETERS", "'core.bare'='true'")
        .env("GIT_CONFIG_COUNT", "1")
        .env("GIT_CONFIG_KEY_0", "core.bare")
        .env("GIT_CONFIG_VALUE_0", "true")
        .output()
        .expect("isolated child test");
    assert_eq!(
        state(),
        before,
        "caller refs/config/worktrees are unchanged"
    );
    assert_eq!(std::fs::read(git_dir.join("index")).ok(), index_before);
    assert!(
        result.status.success()
            && String::from_utf8_lossy(&result.stdout).contains("1 passed; 0 failed;"),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
}

#[test]
fn commit_date_is_readable_without_losing_its_timezone_or_fallback() {
    assert_eq!(
        super::display_date("2026-11-09T12:34:56+03:00"),
        "2026-11-09 12:34:56+03:00"
    );
    assert_eq!(
        super::display_date("2026-11-09T09:34:56Z"),
        "2026-11-09 09:34:56Z"
    );
    assert_eq!(super::display_date("unavailable"), "unavailable");
}
