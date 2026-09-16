use super::*;

use std::time::Duration;

use argand_io::testutil::TempDir;

use crate::session::{Hints, Session, recent_labels};

fn entry(path: impl Into<PathBuf>) -> Recent {
    Recent {
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
    recent.refresh(&entries);
    receive_checks(&mut recent, entries.len());
    assert_eq!(recent.visible(), vec![entry(file)]);
    assert_eq!(recent.entries, entries);
}

#[test]
fn the_loaded_file_is_hidden_without_changing_history_or_shortcut_order() {
    let mut session = Session::default();
    let hints = argand_io::OpenHints {
        raw: Some("iq_i16@24k".parse().unwrap()),
        ..Default::default()
    };
    for i in (0..RECENT_LIMIT).rev() {
        session.remember(Path::new(&format!("capture-{i}.raw")), &hints);
    }
    let saved = session.clone();
    let mut recent = RecentFiles::new(&session.recent);
    recent.set_current(&session.recent[0].path);
    for entry in session.recent.iter().rev() {
        recent.apply(entry.path.clone(), true);
    }
    let visible = recent.visible();
    assert_eq!(visible, session.recent[1..]);
    let labels: Vec<_> = (1..RECENT_LIMIT)
        .map(|i| format!("capture-{i}.raw"))
        .collect();
    assert_eq!(recent_labels(&visible), labels);
    for (index, expected) in visible.iter().enumerate() {
        assert_eq!(recent.shortcut(index).as_ref(), Some(expected));
    }
    assert!(recent.shortcut(9).is_none());
    assert!(recent.shortcut(usize::MAX).is_none());
    assert_eq!(recent.entries, saved.recent);
    assert_eq!(session, saved);

    let mut restarted = RecentFiles::new(&session.recent);
    for entry in &session.recent {
        restarted.apply(entry.path.clone(), true);
    }
    assert_eq!(restarted.visible(), saved.recent);
}

#[test]
fn opening_another_file_restores_the_previous_file_in_recent_order() {
    let mut session = Session::default();
    for name in ["third.raw", "second.raw", "first.raw"] {
        session.remember(Path::new(name), &argand_io::OpenHints::default());
    }
    let mut recent = RecentFiles::new(&session.recent);
    recent.probe = Arc::new(|_| true);
    recent.set_current(Path::new("first.raw"));
    recent.refresh(&session.recent);
    receive_checks(&mut recent, 3);
    let saved = session.clone();
    assert_eq!(recent.visible(), saved.recent[1..]);
    assert_eq!(recent.entries, saved.recent);
    assert_eq!(session, saved);

    session.remember(Path::new("second.raw"), &argand_io::OpenHints::default());
    let saved = session.clone();
    recent.set_current(Path::new("second.raw"));
    recent.refresh(&session.recent);
    assert_eq!(recent.visible(), saved.recent[1..]);
    receive_checks(&mut recent, 3);
    assert_eq!(recent.visible(), saved.recent[1..]);
    assert_eq!(recent_labels(&recent.visible()), ["first.raw", "third.raw"]);
    assert_eq!(recent.shortcut(0), Some(saved.recent[1].clone()));
    assert_eq!(recent.shortcut(1), Some(saved.recent[2].clone()));
    assert!(recent.shortcut(2).is_none());
    assert_eq!(recent.entries, saved.recent);
    assert_eq!(session, saved);
}

#[test]
fn a_relative_current_path_matches_history_and_labels_use_the_filtered_list() {
    let mut session = Session::default();
    for name in ["three/b.wav", "two/a.raw", "./one/a.raw"] {
        session.remember(Path::new(name), &argand_io::OpenHints::default());
    }
    let saved = session.clone();
    assert!(saved.recent[0].path.is_absolute());
    let mut recent = RecentFiles::new(&session.recent);
    for entry in &session.recent {
        recent.apply(entry.path.clone(), true);
    }
    assert_eq!(
        recent_labels(&recent.visible()),
        [
            format!("a.raw - {}", saved.recent[0].path.parent().unwrap().display()),
            format!("a.raw - {}", saved.recent[1].path.parent().unwrap().display()),
            "b.wav".to_owned(),
        ]
    );
    recent.set_current(Path::new("one/./a.raw"));
    assert_eq!(recent.visible(), saved.recent[1..]);
    assert_eq!(recent_labels(&recent.visible()), ["a.raw", "b.wav"]);
    assert_eq!(recent.shortcut(0), Some(saved.recent[1].clone()));
    assert_eq!(recent.shortcut(1), Some(saved.recent[2].clone()));
    assert!(recent.shortcut(2).is_none());
    assert_eq!(recent.entries, saved.recent);
    assert_eq!(session, saved);
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
    recent.apply(entries[2].path.clone(), true);
    assert_eq!(recent.visible(), vec![entries[2].clone()]);
    recent.apply(entries[0].path.clone(), false);
    recent.apply(entries[1].path.clone(), true);
    assert_eq!(recent.visible(), entries[1..]);
    assert_eq!(recent.visible()[0].hints.to_open_hints().raw, hints.raw);
}

#[test]
fn a_blocked_probe_does_not_delay_another_file() {
    let (release, blocked) = std::sync::mpsc::channel();
    let blocked = std::sync::Mutex::new(blocked);
    let entries = vec![entry("network"), entry("local")];
    let mut recent = RecentFiles::new(&entries);
    recent.probe = Arc::new(move |path| {
        if path == Path::new("network") {
            blocked.lock().unwrap().recv().unwrap();
        }
        true
    });
    recent.refresh(&entries);
    let results = recent.updates();
    let (delivered, received) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        delivered.send(results.recv_blocking()).unwrap();
    });
    let first = received.recv_timeout(Duration::from_secs(2));
    release.send(()).unwrap();
    assert_eq!(first.unwrap().unwrap(), (PathBuf::from("local"), true));
}

#[test]
fn an_oversized_history_is_bounded_to_the_session_limit() {
    let entries: Vec<_> = (0..100).map(|i| entry(i.to_string())).collect();
    let mut recent = RecentFiles::new(&entries);
    for entry in &entries {
        recent.apply(entry.path.clone(), true);
    }
    assert_eq!(recent.visible(), entries[..RECENT_LIMIT]);
}

#[test]
fn shortcuts_follow_the_filtered_list_and_stop_after_nine() {
    let entries: Vec<_> = (0..RECENT_LIMIT).map(|i| entry(i.to_string())).collect();
    let mut recent = RecentFiles::new(&entries);
    assert!(recent.shortcut(0).is_none());
    for i in (1..RECENT_LIMIT).rev() {
        recent.apply(entries[i].path.clone(), true);
    }
    for i in 0..9 {
        assert_eq!(recent.shortcut(i), Some(entries[i + 1].clone()));
    }
    recent.apply(entries[0].path.clone(), true);
    assert_eq!(recent.shortcut(8), Some(entries[8].clone()));
    assert!(recent.shortcut(9).is_none());
    assert!(recent.shortcut(usize::MAX).is_none());
    assert_eq!(recent.visible().len(), 10);
}

fn receive_checks(recent: &mut RecentFiles, count: usize) {
    let results = recent.updates();
    for _ in 0..count {
        let (path, exists) = results.recv_blocking().unwrap();
        recent.apply(path, exists);
    }
}

#[test]
fn refresh_hides_removed_files_and_restores_reappearing_files_without_losing_history() {
    let dir = TempDir::new("recent-refresh");
    let path = dir.join("capture.raw");
    let entries = vec![entry(&path), entry(dir.path())];
    let mut recent = RecentFiles::new(&entries);
    recent.refresh(&entries);
    receive_checks(&mut recent, 2);
    assert!(recent.visible().is_empty());

    std::fs::write(&path, []).unwrap();
    recent.refresh(&entries);
    receive_checks(&mut recent, 2);
    assert_eq!(recent.visible(), entries[..1]);

    std::fs::remove_file(&path).unwrap();
    recent.refresh(&entries);
    receive_checks(&mut recent, 2);
    assert!(recent.visible().is_empty());
    assert_eq!(recent.entries, entries);
}

#[test]
fn reordered_history_receives_checks_by_path_and_keeps_new_hints() {
    let entries = vec![entry("old.raw"), entry("new.wav")];
    let mut recent = RecentFiles::new(&entries);
    recent.probe = Arc::new(|_| true);
    recent.refresh(&entries);
    let mut reordered = vec![entries[1].clone(), entries[0].clone()];
    let hints = argand_io::OpenHints {
        raw: Some("iq_i16@24k".parse().unwrap()),
        ..Default::default()
    };
    reordered[1].hints = Hints::from(&hints);
    recent.refresh(&reordered);
    receive_checks(&mut recent, 2);
    assert_eq!(recent.visible(), reordered);
    assert_eq!(recent.shortcut(1).unwrap().hints.to_open_hints().raw, hints.raw);
}

#[test]
fn removed_history_results_cannot_mark_a_replacement_available() {
    let mut recent = RecentFiles::new(&[entry("old")]);
    recent.probe = Arc::new(|_| false);
    recent.refresh(&[entry("new")]);
    recent.apply(PathBuf::from("old"), true);
    assert!(recent.visible().is_empty());
    receive_checks(&mut recent, 1);
    assert!(recent.visible().is_empty());
}

#[test]
fn repeated_refreshes_coalesce_pending_paths_even_when_history_changes() {
    let entries: Vec<_> = (0..RECENT_LIMIT).map(|i| entry(i.to_string())).collect();
    let mut recent = RecentFiles::new(&entries);
    recent.probe = Arc::new(|_| true);
    recent.refresh(&entries);
    for _ in 0..20 {
        recent.refresh(&entries);
        assert_eq!(recent.pending.len(), RECENT_LIMIT);
    }
    let replacement = vec![entry("replacement")];
    recent.refresh(&replacement);
    assert_eq!(recent.pending.len(), RECENT_LIMIT + 1);
    receive_checks(&mut recent, RECENT_LIMIT + 1);
    assert_eq!(recent.visible(), replacement);
    assert!(recent.pending.is_empty());
}

#[test]
fn labels_are_disambiguated_after_filtering_and_match_shortcut_targets() {
    let entries = vec![entry("one/a.raw"), entry("two/a.raw"), entry("three/b.wav")];
    let mut recent = RecentFiles::new(&entries);
    recent.apply(entries[0].path.clone(), true);
    recent.apply(entries[2].path.clone(), true);
    let visible = recent.visible();
    assert_eq!(crate::session::recent_labels(&visible), ["a.raw", "b.wav"]);
    for (index, expected) in visible.iter().enumerate() {
        assert_eq!(recent.shortcut(index).as_ref(), Some(expected));
    }
    recent.apply(entries[1].path.clone(), true);
    assert_eq!(
        crate::session::recent_labels(&recent.visible()),
        ["a.raw - one", "a.raw - two", "b.wav"]
    );
}

#[test]
fn refreshing_keeps_verified_entries_visible_in_the_new_history_order() {
    let entries = vec![entry("first"), entry("second")];
    let mut recent = RecentFiles::new(&entries);
    for entry in &entries {
        recent.apply(entry.path.clone(), true);
    }
    recent.probe = Arc::new(|_| true);
    let reordered = vec![entry("new"), entries[1].clone(), entries[0].clone()];
    recent.refresh(&reordered);
    assert_eq!(recent.visible(), reordered[1..]);
    receive_checks(&mut recent, 3);
    assert_eq!(recent.visible(), reordered);
}

#[test]
fn dropping_the_model_does_not_join_a_blocked_probe() {
    let (started, entered) = std::sync::mpsc::channel();
    let (release, blocked) = std::sync::mpsc::channel();
    let blocked = std::sync::Mutex::new(blocked);
    let entries = vec![entry("network")];
    let mut recent = RecentFiles::new(&entries);
    recent.probe = Arc::new(move |_| {
        started.send(()).unwrap();
        blocked.lock().unwrap().recv().unwrap();
        true
    });
    recent.refresh(&entries);
    entered.recv_timeout(Duration::from_secs(2)).unwrap();
    let (dropped, completed) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        drop(recent);
        dropped.send(()).unwrap();
    });
    let result = completed.recv_timeout(Duration::from_secs(2));
    release.send(()).unwrap();
    result.unwrap();
}

#[test]
fn a_full_history_of_blocked_probes_does_not_hide_a_new_local_file() {
    let entries: Vec<_> = (0..RECENT_LIMIT).map(|i| entry(i.to_string())).collect();
    let (release, blocked) = async_channel::bounded::<()>(RECENT_LIMIT);
    let mut recent = RecentFiles::new(&entries);
    recent.probe = Arc::new(move |path| {
        if path != Path::new("local") {
            blocked.recv_blocking().unwrap();
        }
        true
    });
    recent.refresh(&entries);
    let replacement = vec![entry("local")];
    recent.refresh(&replacement);
    let results = recent.updates();
    let (delivered, received) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        delivered.send(results.recv_blocking()).unwrap();
    });
    let first = received.recv_timeout(Duration::from_secs(2));
    for _ in 0..RECENT_LIMIT {
        release.send_blocking(()).unwrap();
    }
    let (path, exists) = first.unwrap().unwrap();
    assert_eq!(path, Path::new("local"));
    recent.apply(path, exists);
    assert_eq!(recent.visible(), replacement);
    receive_checks(&mut recent, RECENT_LIMIT);
    assert_eq!(recent.visible(), replacement);
}
