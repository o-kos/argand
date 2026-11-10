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
