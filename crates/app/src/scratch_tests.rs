use super::*;

fn root(label: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("argand-scratch-test-{label}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    root
}

#[test]
fn the_folder_of_an_instance_that_has_gone_is_removed() {
    let root = root("gone");
    fs::create_dir_all(root.join("1")).unwrap();
    fs::write(root.join("1").join("1.wav"), b"left behind").unwrap();
    fs::write(root.join("1.lock"), b"").unwrap();
    let folder = Folder::open(&root, 2).unwrap();
    assert!(!root.join("1").exists());
    assert!(!root.join("1.lock").exists());
    assert!(root.join("2").is_dir());
    assert!(root.join("2.lock").exists() && root.join("sweep.lock").exists());
    drop(folder);
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn the_folder_of_a_running_instance_is_kept() {
    let root = root("running");
    let running = Folder::open(&root, 1).unwrap();
    let kept = running.new_path("wav");
    fs::write(&kept, b"in use").unwrap();
    let other = Folder::open(&root, 2).unwrap();
    assert!(kept.exists());
    assert_ne!(other.new_path("wav").parent(), kept.parent());
    drop((running, other));
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn names_are_never_reused_and_a_scratch_file_goes_with_its_last_reference() {
    let root = root("names");
    let folder = Folder::open(&root, 1).unwrap();
    let first = folder.new_path("wav");
    assert_ne!(first, folder.new_path("wav"));
    fs::write(&first, b"computed").unwrap();
    let scratch = std::sync::Arc::new(Scratch::new(first.clone()));
    let copy = scratch.clone();
    drop(scratch);
    assert!(first.exists());
    drop(copy);
    assert!(!first.exists());
    drop(folder);
    let _ = fs::remove_dir_all(&root);
}

#[cfg(unix)]
#[test]
fn the_folders_are_open_to_their_owner_alone() {
    use std::os::unix::fs::PermissionsExt;
    let root = root("private");
    let folder = Folder::open(&root, 1).unwrap();
    for path in [root.clone(), root.join("1")] {
        let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o700, "{}", path.display());
    }
    drop(folder);
    let _ = fs::remove_dir_all(&root);
}
