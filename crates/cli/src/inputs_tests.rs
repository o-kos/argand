use super::*;

use argand_io::testutil::TempDir;

fn fixture(label: &str, names: &[&str]) -> TempDir {
    let dir = TempDir::new(label);
    for name in names {
        std::fs::write(dir.join(name), b"x").expect("write fixture");
    }
    dir
}

fn resolved(dir: &TempDir, args: &[&str]) -> Vec<String> {
    let inputs: Vec<PathBuf> = args.iter().map(|a| dir.join(a)).collect();
    resolve(&inputs)
        .unwrap_or_else(|e| panic!("{args:?}: {e}"))
        .iter()
        .map(|p| {
            p.file_name()
                .expect("a resolved file has a name")
                .to_string_lossy()
                .into_owned()
        })
        .collect()
}

fn error(dir: &TempDir, arg: &str) -> ResolveError {
    resolve(&[dir.join(arg)]).expect_err("should not resolve")
}

#[test]
fn an_exact_path_passes_through_without_touching_the_disk() {
    let dir = TempDir::new("resolve-exact");
    let missing = dir.join("nope.wav");
    let files = resolve(std::slice::from_ref(&missing)).expect("an exact path always resolves");
    assert_eq!(files, vec![missing]);
}

#[test]
fn several_exact_paths_keep_the_order_they_were_given() {
    let dir = fixture("resolve-order", &["b.wav", "a.wav"]);
    assert_eq!(resolved(&dir, &["b.wav", "a.wav"]), ["b.wav", "a.wav"]);
}

#[test]
fn a_mask_lists_one_directory_and_sorts_by_filename() {
    let dir = fixture(
        "resolve-mask",
        &["c.iqw", "a.iqw", "b.iqw", "other.wav", ".hidden.iqw"],
    );
    assert_eq!(resolved(&dir, &["*.iqw"]), ["a.iqw", "b.iqw", "c.iqw"]);
    assert_eq!(resolved(&dir, &["[ab].iqw"]), ["a.iqw", "b.iqw"]);
    assert_eq!(resolved(&dir, &["*"]), ["a.iqw", "b.iqw", "c.iqw", "other.wav"]);
}

#[test]
fn a_mask_keeps_the_directory_the_argument_named() {
    let dir = fixture("resolve-dir", &["a.iqw"]);
    let files = resolve(&[dir.join("*.iqw")]).expect("resolves");
    assert_eq!(files, vec![dir.join("a.iqw")]);
}

#[test]
fn a_bare_mask_resolves_against_the_working_directory() {
    // Relative to wherever the test runs; `Cargo.toml` is beside this crate.
    let files = resolve(&[PathBuf::from("Cargo.*")]).expect("resolves");
    assert!(
        files.contains(&PathBuf::from("Cargo.toml")),
        "bare masks stay bare: {files:?}"
    );
}

#[test]
fn a_mask_skips_directories() {
    let dir = fixture("resolve-subdir", &["a.iqw"]);
    std::fs::create_dir(dir.join("sub.iqw")).expect("create subdirectory");
    assert_eq!(resolved(&dir, &["*.iqw"]), ["a.iqw"]);
}

#[test]
fn the_same_file_reached_twice_is_processed_once() {
    let dir = fixture("resolve-dedup", &["a.iqw", "b.iqw"]);
    assert_eq!(
        resolved(&dir, &["a.iqw", "*.iqw", "./a.iqw"]),
        ["a.iqw", "b.iqw"],
        "the first spelling wins and the rest are dropped"
    );
}

#[test]
fn a_mask_that_matches_nothing_is_an_error() {
    let dir = fixture("resolve-empty", &["a.iqw"]);
    let err = error(&dir, "*.wav");
    assert!(matches!(err, ResolveError::NoMatch { .. }), "{err}");
    assert!(err.to_string().contains("*.wav"), "{err}");
}

#[test]
fn a_mask_in_a_directory_component_is_refused() {
    let dir = fixture("resolve-dirmask", &["a.iqw"]);
    let err = error(&dir, "*/a.iqw");
    assert!(matches!(err, ResolveError::MaskInDirectory { .. }), "{err}");
    assert!(err.to_string().contains("directory names"), "{err}");
}

#[test]
fn a_recursive_mask_is_refused_wherever_it_appears() {
    let dir = fixture("resolve-recursive", &["a.iqw"]);
    for arg in ["**/a.iqw", "a**.iqw"] {
        let err = error(&dir, arg);
        assert!(
            matches!(
                err,
                ResolveError::Mask {
                    source: MaskError::Recursive,
                    ..
                }
            ),
            "{arg}: {err}"
        );
    }
}

#[test]
fn a_malformed_mask_names_itself_and_the_reason() {
    let dir = fixture("resolve-malformed", &["a.iqw"]);
    let err = error(&dir, "a[0-9.iqw");
    assert!(
        matches!(
            err,
            ResolveError::Mask {
                source: MaskError::UnclosedClass,
                ..
            }
        ),
        "{err}"
    );
    assert!(err.to_string().contains("a[0-9.iqw"), "{err}");
}

#[test]
fn a_mask_over_a_missing_directory_names_the_directory() {
    let dir = TempDir::new("resolve-nodir");
    let err = error(&dir, "gone/*.iqw");
    assert!(matches!(err, ResolveError::ListDir { .. }), "{err}");
    assert!(err.to_string().contains("gone"), "{err}");
}
