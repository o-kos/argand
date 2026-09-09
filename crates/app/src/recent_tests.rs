use super::*;

use std::time::Duration;

use argand_io::testutil::TempDir;

use crate::session::Hints;

fn entry(path: impl Into<PathBuf>) -> Recent {
    Recent {
        view: None,
        path: path.into(),
        hints: Hints::default(),
    }
}

#[test]
fn only_verified_regular_files_are_visible_and_history_is_unchanged() {
    let dir = TempDir::new("recent-files");
    let file = dir.join("capture.raw");
    std::fs::write(&file, []).unwrap();
    let entries = vec![entry(dir.join("gone")), entry(&file), entry(dir.path())];
    let mut recent = RecentFiles::new(&entries);
    assert!(recent.visible().is_empty());
    let results = recent.check();
    while let Ok((index, exists)) = results.recv_blocking() {
        recent.apply(index, exists);
    }
    assert_eq!(recent.visible(), vec![entry(file)]);
    assert_eq!(recent.entries, entries);
}

#[test]
fn completion_order_does_not_change_recent_order_or_saved_raw_hints() {
    let hints = argand_io::OpenHints {
        raw: Some("iq_i16@24k".parse().unwrap()),
        ..Default::default()
    };
    let mut entries = vec![entry("missing"), entry("new.raw"), entry("old.wav")];
    entries[1].hints = Hints::from(&hints);
    let mut recent = RecentFiles::new(&entries);
    recent.apply(2, true);
    assert_eq!(recent.visible(), vec![entries[2].clone()]);
    recent.apply(0, false);
    recent.apply(1, true);
    assert_eq!(recent.visible(), entries[1..]);
    assert_eq!(recent.visible()[0].hints.to_open_hints().raw, hints.raw);
}

#[test]
fn a_blocked_probe_does_not_delay_another_file() {
    let (release, blocked) = std::sync::mpsc::channel();
    let blocked = std::sync::Mutex::new(blocked);
    let results = check_paths(
        [PathBuf::from("network"), PathBuf::from("local")].into_iter(),
        Arc::new(move |path| {
            if path == Path::new("network") {
                blocked.lock().unwrap().recv().unwrap();
            }
            true
        }),
    );
    let (delivered, received) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        delivered.send(results.recv_blocking()).unwrap();
    });
    let first = received.recv_timeout(Duration::from_secs(2));
    release.send(()).unwrap();
    assert_eq!(first.unwrap().unwrap(), (1, true));
}

#[test]
fn an_oversized_history_is_bounded_to_the_session_limit() {
    let entries: Vec<_> = (0..100).map(|i| entry(i.to_string())).collect();
    let mut recent = RecentFiles::new(&entries);
    for i in 0..100 {
        recent.apply(i, true);
    }
    assert_eq!(recent.visible(), entries[..RECENT_LIMIT]);
}

#[test]
fn shortcuts_follow_the_filtered_list_and_stop_after_nine() {
    let entries: Vec<_> = (0..RECENT_LIMIT).map(|i| entry(i.to_string())).collect();
    let mut recent = RecentFiles::new(&entries);
    assert!(recent.shortcut(0).is_none());
    for i in (1..RECENT_LIMIT).rev() {
        recent.apply(i, true);
    }
    for i in 0..9 {
        assert_eq!(recent.shortcut(i), Some(entries[i + 1].clone()));
    }
    recent.apply(0, true);
    assert_eq!(recent.shortcut(8), Some(entries[8].clone()));
    assert!(recent.shortcut(9).is_none());
    assert!(recent.shortcut(usize::MAX).is_none());
    assert_eq!(recent.visible().len(), 10);
}
