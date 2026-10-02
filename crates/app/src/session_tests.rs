use super::*;

/// A scratch directory removed when it goes out of scope.
struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new(label: &str) -> Self {
        use std::sync::atomic::{AtomicU32, Ordering};
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "argand-session-{label}-{}-{unique}",
            std::process::id()
        ));
        std::fs::create_dir_all(&path).expect("create scratch dir");
        Self { path }
    }

    fn join(&self, name: &str) -> PathBuf {
        self.path.join(name)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// A single 1920x1080 display with its origin at zero.
const PRIMARY: Geometry = Geometry::new(0.0, 0.0, 1920.0, 1080.0);
/// A second display to the right of it, as a dual-head desktop reports one.
const SECONDARY: Geometry = Geometry::new(1920.0, 0.0, 1920.0, 1080.0);

#[test]
fn a_window_that_still_fits_is_restored_untouched() {
    let saved = Geometry::new(100.0, 80.0, 1280.0, 800.0);
    assert_eq!(place(Some(saved), &[PRIMARY]), Some(saved));
}

#[test]
fn a_window_left_on_a_display_that_is_gone_comes_back_where_it_can_be_seen() {
    // Saved on the second screen, which is no longer attached.
    let saved = Geometry::new(2200.0, 300.0, 1280.0, 800.0);
    let placed = place(Some(saved), &[PRIMARY]).expect("somewhere to open");

    assert!(
        placed.x >= PRIMARY.x && placed.right() <= PRIMARY.right(),
        "{placed:?} is outside {PRIMARY:?} horizontally"
    );
    assert!(
        placed.y >= PRIMARY.y && placed.bottom() <= PRIMARY.bottom(),
        "{placed:?} is outside {PRIMARY:?} vertically"
    );
    // The size it was left at is worth keeping; only the position had to move.
    assert_eq!((placed.width, placed.height), (saved.width, saved.height));
}

#[test]
fn a_window_still_on_its_own_display_stays_there() {
    // Both screens present, and the rectangle lies wholly within the second
    // one, so nothing about it needs adjusting.
    let saved = Geometry::new(2200.0, 200.0, 1280.0, 800.0);
    assert_eq!(place(Some(saved), &[PRIMARY, SECONDARY]), Some(saved));
}

#[test]
fn a_window_wider_than_the_display_is_shrunk_to_it() {
    // The display it was saved on had a higher resolution than this one.
    let saved = Geometry::new(0.0, 0.0, 3840.0, 2160.0);
    let small = Geometry::new(0.0, 0.0, 1280.0, 720.0);
    let placed = place(Some(saved), &[small]).expect("somewhere to open");
    assert_eq!(placed, small, "a window larger than the screen was not fitted");
}

#[test]
fn a_window_hanging_off_an_edge_is_pulled_back_onto_the_screen() {
    // Dragged mostly off the right of the only display.
    let saved = Geometry::new(1800.0, 900.0, 1280.0, 800.0);
    let placed = place(Some(saved), &[PRIMARY]).expect("somewhere to open");
    assert_eq!(placed.right(), PRIMARY.right());
    assert_eq!(placed.bottom(), PRIMARY.bottom());
    assert_eq!((placed.width, placed.height), (saved.width, saved.height));
}

#[test]
fn a_rectangle_worth_nothing_hands_the_placement_back_to_the_platform() {
    for saved in [
        Geometry::new(0.0, 0.0, 0.0, 800.0),
        Geometry::new(0.0, 0.0, 1280.0, 0.0),
        Geometry::new(f32::NAN, 0.0, 1280.0, 800.0),
        Geometry::new(0.0, f32::INFINITY, 1280.0, 800.0),
        Geometry::new(0.0, 0.0, f32::NAN, 800.0),
    ] {
        assert_eq!(place(Some(saved), &[PRIMARY]), None, "{saved:?} was restored");
    }
    // A first run has nothing saved, whatever the displays say.
    assert_eq!(place(None, &[PRIMARY]), None);
}

#[test]
fn a_platform_that_names_no_displays_still_restores_the_size() {
    // The first window on Wayland sees this: gpui has not processed the display
    // globals by the time the placement is decided. Throwing the saved
    // rectangle away over that would lose the size as well as the position, and
    // the size is the half that can still be honoured.
    let saved = Geometry::new(0.0, 0.0, 1000.0, 700.0);
    assert_eq!(place(Some(saved), &[]), Some(saved));
    // A rectangle that is not worth restoring is still refused.
    assert_eq!(place(Some(Geometry::new(0.0, 0.0, 0.0, 700.0)), &[]), None);
}

#[test]
fn a_session_survives_the_round_trip() {
    let dir = TempDir::new("roundtrip");
    let path = dir.join("session.toml");
    let session = Session {
        show_grid: true,
        show_scale_ui: true,
        theme: None,
        orientation: crate::orientation::Mode::default(),
        analysis_settings: None,        version: VERSION,
        time_ruler: crate::time_ruler::Mode::Clock,
        geometry: Some(Geometry::new(12.0, 34.0, 1280.0, 800.0)),
        window_state: WindowState::Maximized,
        recent: Vec::new(),
    };

    session.save(&path);
    assert_eq!(Session::load(&path).session, session);
}

#[test]
fn nothing_in_the_file_can_stop_the_application_starting() {
    let dir = TempDir::new("broken");
    let cases = [
        ("missing.toml", None),
        ("empty.toml", Some("")),
        ("truncated.toml", Some("version = 1\ngeometry = { x = 1.0")),
        ("garbage.toml", Some("\u{0}\u{1}not a session at all")),
        ("wrong-type.toml", Some("version = \"one\"\n")),
        // A file this version does not know how to read is left alone rather
        // than guessed at.
        ("from-the-future.toml", Some("version = 9999\n")),
    ];

    for (name, text) in cases {
        let path = dir.join(name);
        if let Some(text) = text {
            std::fs::write(&path, text).expect("write fixture");
        }
        let restored = Session::load(&path);
        assert_eq!(
            restored.session,
            Session::default(),
            "{name} did not fall back to defaults"
        );
        // Everything here but a newer version is a file with nothing worth
        // keeping, so the next run is free to write over it.
        assert_eq!(
            restored.writable,
            name != "from-the-future.toml",
            "{name} was given the wrong permission to overwrite"
        );
    }
}

#[test]
fn a_save_interrupted_partway_leaves_the_previous_session_readable() {
    let dir = TempDir::new("atomic");
    let path = dir.join("session.toml");

    let first = Session {
        show_grid: true,
        show_scale_ui: true,
        theme: None,
        orientation: crate::orientation::Mode::default(),
        analysis_settings: None,        version: VERSION,
        time_ruler: crate::time_ruler::Mode::Clock,
        geometry: Some(Geometry::new(0.0, 0.0, 800.0, 600.0)),
        window_state: WindowState::Normal,
        recent: Vec::new(),
    };
    first.save(&path);

    // What a kill mid-write leaves behind: the staging file exists with
    // whatever had been flushed, and the rename never happened.
    let staging = path.with_extension(format!("toml.{}.tmp", std::process::id()));
    std::fs::write(&staging, "version = 1\ngeometry = { x =").expect("write partial");

    assert_eq!(
        Session::load(&path).session,
        first,
        "the previous session was not intact after an interrupted write"
    );
    let _ = std::fs::remove_file(&staging);
}

#[test]
fn a_directory_that_does_not_exist_yet_is_created_to_save_into() {
    let dir = TempDir::new("mkdir");
    let path = dir.join("nested").join("deeper").join("session.toml");
    let session = Session::default();

    session.save(&path);
    assert!(path.exists(), "the state directory was not created");
    assert_eq!(Session::load(&path).session, session);
}

#[test]
fn a_save_that_cannot_happen_is_reported_rather_than_fatal() {
    let dir = TempDir::new("unwritable");
    // A path whose parent is a file, so neither the directory creation nor the
    // write can succeed.
    let blocker = dir.join("a-file");
    std::fs::write(&blocker, "not a directory").expect("write blocker");

    // Reaching the end of this test is half the assertion: losing a window
    // position must not take the application down with it. The other half is
    // that the caller is told, so that it can keep the value and try again.
    assert!(
        !Session::default().save(&blocker.join("session.toml")),
        "a write that could not happen was reported as done"
    );
}

#[test]
fn a_drag_writes_a_few_times_rather_than_once_a_frame() {
    let dir = TempDir::new("debounce");
    let path = dir.join("session.toml");
    let mut writer = Writer::new(path.clone(), Session::default());

    let at = |ms: u64| Instant::now() + Duration::from_millis(ms);
    let moved = |x: f32| Session {
        show_grid: true,
        show_scale_ui: true,
        theme: None,
        orientation: crate::orientation::Mode::default(),
        analysis_settings: None,        version: VERSION,
        time_ruler: crate::time_ruler::Mode::Clock,
        geometry: Some(Geometry::new(x, 0.0, 1280.0, 800.0)),
        window_state: WindowState::Normal,
        recent: Vec::new(),
    };

    // A frame every 8ms, as a drag produces. The first offer writes, because
    // nothing has been written yet.
    let start = at(0);
    writer.offer(moved(0.0), start);
    assert_eq!(Session::load(&path).session.geometry.expect("a position").x, 0.0);

    // Every frame inside the interval must leave the file alone. This is what
    // separates a debounce from a write per frame: an implementation that
    // wrote each offer would advance the file here, and the assertion below
    // would see the frame it had reached rather than the one it started at.
    for frame in 1..60u64 {
        writer.offer(moved(frame as f32), start + Duration::from_millis(frame * 8));
        assert_eq!(
            Session::load(&path).session.geometry.expect("a position").x,
            0.0,
            "frame {frame} was written before the interval had passed"
        );
    }

    // Once it has passed, the next offer lands, and it is the current one
    // rather than any of the frames it skipped.
    writer.offer(moved(60.0), start + Writer::INTERVAL);
    assert_eq!(Session::load(&path).session.geometry.expect("a position").x, 60.0);

    // And the last position wins whatever the schedule was about to allow.
    writer.offer(moved(124.0), start + Writer::INTERVAL + Duration::from_millis(8));
    writer.flush(start + Duration::from_secs(2));
    assert_eq!(Session::load(&path).session.geometry.expect("a position").x, 124.0);
}

#[test]
fn a_position_that_has_not_changed_is_not_written_again() {
    let dir = TempDir::new("idle");
    let path = dir.join("session.toml");
    let held = Session {
        show_grid: true,
        show_scale_ui: true,
        theme: None,
        orientation: crate::orientation::Mode::default(),
        analysis_settings: None,        version: VERSION,
        time_ruler: crate::time_ruler::Mode::Clock,
        geometry: Some(Geometry::new(10.0, 20.0, 1280.0, 800.0)),
        window_state: WindowState::Normal,
        recent: Vec::new(),
    };

    let mut writer = Writer::new(path.clone(), held.clone());
    let start = Instant::now();
    // Idle frames, far enough apart that the schedule would allow a write.
    for frame in 0..10u64 {
        writer.offer(held.clone(), start + Duration::from_secs(frame));
    }
    writer.flush(start + Duration::from_secs(10));

    assert!(
        !path.exists(),
        "a window that never moved still rewrote its session"
    );
}

#[test]
fn the_last_position_survives_even_if_the_schedule_would_have_skipped_it() {
    let dir = TempDir::new("flush");
    let path = dir.join("session.toml");
    let mut writer = Writer::new(path.clone(), Session::default());

    let start = Instant::now();
    let moved = |x: f32| Session {
        show_grid: true,
        show_scale_ui: true,
        theme: None,
        orientation: crate::orientation::Mode::default(),
        analysis_settings: None,        version: VERSION,
        time_ruler: crate::time_ruler::Mode::Clock,
        geometry: Some(Geometry::new(x, 0.0, 1280.0, 800.0)),
        window_state: WindowState::Normal,
        recent: Vec::new(),
    };

    writer.offer(moved(1.0), start);
    // Immediately after a write, so the schedule holds this one back.
    writer.offer(moved(2.0), start + Duration::from_millis(1));
    assert_eq!(Session::load(&path).session.geometry.expect("a position").x, 1.0);

    // Closing the window is what makes it land.
    writer.flush(start + Duration::from_millis(2));
    assert_eq!(Session::load(&path).session.geometry.expect("a position").x, 2.0);
}

#[test]
fn a_position_the_window_has_already_left_is_not_the_one_written() {
    let dir = TempDir::new("returned");
    let path = dir.join("session.toml");
    let at = |x: f32| Session {
        show_grid: true,
        show_scale_ui: true,
        theme: None,
        orientation: crate::orientation::Mode::default(),
        analysis_settings: None,        version: VERSION,
        time_ruler: crate::time_ruler::Mode::Clock,
        geometry: Some(Geometry::new(x, 0.0, 1280.0, 800.0)),
        window_state: WindowState::Normal,
        recent: Vec::new(),
    };

    let mut writer = Writer::new(path.clone(), at(0.0));
    let start = Instant::now();
    // Settle a write first, so the schedule is holding the next one back.
    writer.offer(at(100.0), start);
    assert_eq!(Session::load(&path).session, at(100.0));

    // Dragged away and back again before the interval is up. The offer in
    // between is one the window has already left, so what finally lands has to
    // be where it actually is.
    writer.offer(at(400.0), start + Duration::from_millis(1));
    writer.offer(at(100.0), start + Duration::from_millis(2));
    writer.flush(start + Duration::from_secs(1));

    assert_eq!(
        Session::load(&path).session,
        at(100.0),
        "a position the window had already left was written over the real one"
    );
}

#[test]
fn a_write_that_failed_is_tried_again_rather_than_forgotten() {
    let dir = TempDir::new("retry");
    // The parent is a file, so every write fails until it is not.
    let blocked = dir.join("blocked");
    std::fs::write(&blocked, "not a directory").expect("write blocker");
    let path = blocked.join("session.toml");

    let mut writer = Writer::new(path.clone(), Session::default());
    let moved = Session {
        show_grid: true,
        show_scale_ui: true,
        theme: None,
        orientation: crate::orientation::Mode::default(),
        analysis_settings: None,        version: VERSION,
        time_ruler: crate::time_ruler::Mode::Clock,
        geometry: Some(Geometry::new(10.0, 20.0, 1280.0, 800.0)),
        window_state: WindowState::Normal,
        recent: Vec::new(),
    };
    let start = Instant::now();
    writer.offer(moved.clone(), start);
    assert!(!path.exists(), "the fixture did not actually block the write");

    // Clear the obstruction, as a full disk or a permission might clear, and
    // the value the window is still at has to reach the file.
    std::fs::remove_file(&blocked).expect("remove blocker");
    writer.flush(start + Duration::from_secs(1));
    assert_eq!(
        Session::load(&path).session,
        moved,
        "the position was lost to a failure that had passed"
    );
}

#[test]
fn a_size_no_surface_could_hold_is_not_restored() {
    // A corrupt or hand-edited file can hold a finite, positive, absurd size.
    // With no display to clamp against it would otherwise reach the toolkit.
    for saved in [
        Geometry::new(0.0, 0.0, 1e30, 800.0),
        Geometry::new(0.0, 0.0, 1280.0, 1e30),
        Geometry::new(0.0, 0.0, 0.4, 800.0),
    ] {
        assert_eq!(place(Some(saved), &[]), None, "{saved:?} was restored");
        assert_eq!(place(Some(saved), &[PRIMARY]), None, "{saved:?} was restored");
    }
}

#[test]
fn a_failure_that_persists_does_not_retry_on_every_frame() {
    let dir = TempDir::new("storm");
    // The parent is a file, so the write fails for as long as it is there.
    let blocked = dir.join("blocked");
    std::fs::write(&blocked, "not a directory").expect("write blocker");
    let path = blocked.join("session.toml");

    let mut writer = Writer::new(path.clone(), Session::default());
    let start = Instant::now();
    let at = |x: f32| Session {
        show_grid: true,
        show_scale_ui: true,
        theme: None,
        orientation: crate::orientation::Mode::default(),
        analysis_settings: None,        version: VERSION,
        time_ruler: crate::time_ruler::Mode::Clock,
        geometry: Some(Geometry::new(x, 0.0, 1280.0, 800.0)),
        window_state: WindowState::Normal,
        recent: Vec::new(),
    };

    // One attempt, which fails.
    writer.offer(at(1.0), start);
    assert!(!path.exists(), "the fixture did not actually block the write");

    // The obstruction is gone, so the next attempt would succeed -- but the
    // schedule has to hold it back exactly as it holds a success back. If the
    // failure had not counted as an attempt, this frame would write, and a
    // permission that never comes back would be retried on every one of them.
    std::fs::remove_file(&blocked).expect("remove blocker");
    writer.offer(at(2.0), start + Duration::from_millis(8));
    assert!(
        !path.exists(),
        "a failed write left the schedule open, so the next frame wrote"
    );

    // And once the interval has passed, it does write.
    writer.offer(at(3.0), start + Writer::INTERVAL);
    assert_eq!(Session::load(&path).session, at(3.0));
}

#[test]
fn a_rectangle_no_window_ever_had_is_not_restored() {
    // Past this, a file has been corrupted or hand-edited into something that
    // was never a window, and there is nothing in it to restore. A size merely
    // larger than the guard can vouch for is capped instead; that is the test
    // above.
    for saved in [
        Geometry::new(0.0, 0.0, ABSURD * 2.0, 800.0),
        Geometry::new(0.0, 0.0, 1280.0, ABSURD * 2.0),
        Geometry::new(1.0e6, 0.0, 1280.0, 800.0),
        Geometry::new(0.0, -1.0e6, 1280.0, 800.0),
    ] {
        assert_eq!(place(Some(saved), &[]), None, "{saved:?} was restored");
    }
    // The bound is in logical pixels and the driver compares device pixels, so
    // it has to leave room for the scale factor: what is allowed here must
    // still be allowed after a display at scale 3 multiplies it.
    let widest = Geometry::new(0.0, 0.0, UNVERIFIED_MAX_DIMENSION, UNVERIFIED_MAX_DIMENSION);
    assert_eq!(place(Some(widest), &[]), Some(widest));
}

#[test]
fn a_size_no_display_vouches_for_is_cut_down_rather_than_thrown_away() {
    // With nothing to clamp against, the size is capped instead of refused: a
    // window that was genuinely this large keeps everything up to the cap,
    // rather than losing its size entirely to a guard.
    let large = Geometry::new(0.0, 0.0, 3840.0, 2160.0);
    let placed = place(Some(large), &[]).expect("a size worth restoring");
    assert_eq!(placed.width, UNVERIFIED_MAX_DIMENSION);
    assert_eq!(placed.height, UNVERIFIED_MAX_DIMENSION);

    // Under the cap, nothing is touched.
    let modest = Geometry::new(0.0, 0.0, 1280.0, 800.0);
    assert_eq!(place(Some(modest), &[]), Some(modest));

    // A real display vouches for its own size, so nothing is capped there.
    let wide = Geometry::new(0.0, 0.0, 3840.0, 2160.0);
    assert_eq!(place(Some(wide), &[wide]), Some(wide));
}

#[test]
fn a_session_from_a_newer_version_is_read_as_defaults_and_left_alone() {
    let dir = TempDir::new("future");
    let path = dir.join("session.toml");
    let text = "version = 9999
something_this_version_never_heard_of = true
";
    std::fs::write(&path, text).expect("write fixture");

    let restored = Session::load(&path);
    assert_eq!(restored.session, Session::default());
    assert!(
        !restored.writable,
        "a file from a newer version was cleared for overwriting"
    );
    assert_eq!(
        std::fs::read_to_string(&path).expect("still there"),
        text,
        "reading a newer session changed it"
    );
}

#[test]
fn only_an_ordinary_window_says_what_size_to_come_back_to() {
    let ordinary = Geometry::new(100.0, 80.0, 1000.0, 700.0);
    assert_eq!(
        restore_rectangle(ordinary, WindowState::Normal),
        Some(ordinary)
    );

    // A maximized or fullscreen window reports the screen it covers, which is
    // not the size it would return to, so it says nothing here.
    for state in [WindowState::Maximized, WindowState::Fullscreen] {
        assert_eq!(restore_rectangle(ordinary, state), None);
    }

    // A window that is genuinely the size of a display is still an ordinary
    // window. Refusing it on its size alone was tried and withdrawn: a display
    // it does not sit on is no evidence about it, and the resize would be lost.
    let large = Geometry::new(0.0, 0.0, PRIMARY.width, PRIMARY.height);
    assert_eq!(
        restore_rectangle(large, WindowState::Normal),
        Some(large)
    );
}

/// The hints a headerless capture needs, as the command line would give them.
fn raw_hints() -> OpenHints {
    OpenHints {
            level_scan_bytes: None,
        raw: Some("iq_i16@2M".parse().expect("a raw spec")),
        sample_type: None,
        sample_rate: None,
        center_freq: 12_579_000.0,
        byte_offset: 44,
        normalize: Some(argand_io::Normalize::Auto),
        gain_db: -6.0,
    }
}

#[test]
fn a_headerless_capture_comes_back_with_the_layout_it_was_opened_with() {
    let mut session = Session::default();
    session.remember(Path::new("/captures/dump.bin"), &raw_hints());

    let stored = session.recent.first().expect("one entry");
    let hints = stored.hints.to_open_hints();
    let raw = hints.raw.expect("the layout that made it readable");
    assert_eq!(raw.to_string(), "iq_i16@2000000");
    assert_eq!(hints.center_freq, 12_579_000.0);
    assert_eq!(hints.byte_offset, 44);
    assert_eq!(hints.normalize, Some(argand_io::Normalize::Auto));
    assert_eq!(hints.gain_db, -6.0);
}

#[test]
fn the_recent_list_survives_the_round_trip_through_the_file() {
    let dir = TempDir::new("recent");
    let path = dir.join(FILE_NAME);
    let mut session = Session::default();
    session.remember(Path::new("/captures/dump.bin"), &raw_hints());

    assert!(session.save(&path));
    let restored = Session::load(&path);
    assert!(restored.writable);
    assert_eq!(restored.session.recent, session.recent);
}

#[test]
fn a_file_opened_again_moves_to_the_head_rather_than_appearing_twice() {
    let dir = TempDir::new("head");
    let mut session = Session::default();
    for name in ["a.wav", "b.wav", "c.wav"] {
        session.remember(&dir.join(name), &OpenHints::default());
    }
    // And with different hints the second time, which are the ones worth
    // keeping: they are the ones that worked.
    session.remember(&dir.join("a.wav"), &raw_hints());

    let names: Vec<_> = session.recent.iter().map(|entry| &entry.path).collect();
    assert_eq!(
        names,
        [&dir.join("a.wav"), &dir.join("c.wav"), &dir.join("b.wav")]
    );
    assert_eq!(
        session.recent[0].hints.to_open_hints().byte_offset,
        44,
        "the newer hints should replace the older ones"
    );
}

#[test]
fn a_file_named_from_the_current_directory_is_remembered_from_the_root() {
    // The list outlives the directory the application was started in. A bare
    // `dump.bin` kept as written would, from somewhere else, either fail to
    // open or open a different file with this one's layout hints.
    let mut session = Session::default();
    session.remember(Path::new("dump.bin"), &raw_hints());

    let stored = &session.recent[0].path;
    assert!(stored.is_absolute(), "{} is not absolute", stored.display());
    assert!(stored.ends_with("dump.bin"), "{}", stored.display());

    // And the same file named the same way twice is still one entry.
    session.remember(Path::new("dump.bin"), &raw_hints());
    assert_eq!(session.recent.len(), 1);
}

#[test]
fn the_recent_list_does_not_grow_without_end() {
    let dir = TempDir::new("limit");
    let mut session = Session::default();
    for i in 0..RECENT_LIMIT * 2 {
        session.remember(&dir.join(&format!("{i}.wav")), &OpenHints::default());
    }
    assert_eq!(session.recent.len(), RECENT_LIMIT);
    // The newest is at the head and the oldest have gone.
    assert_eq!(session.recent[0].path, dir.join("19.wav"));
}

#[test]
fn a_hint_this_version_cannot_read_costs_the_flag_and_not_the_entry() {
    // A session written by a version that knew a sample type this one does
    // not. The entry is still worth offering: the path is the half that makes
    // it an entry at all.
    let stored = Hints {
        raw: Some("iq_q7@2M".to_owned()),
        sample_type: Some("not-a-type".to_owned()),
        center_freq: 12_579_000.0,
        ..Hints::default()
    };
    let hints = stored.to_open_hints();

    assert!(hints.raw.is_none());
    assert!(hints.sample_type.is_none());
    assert_eq!(hints.center_freq, 12_579_000.0);
}

#[test]
fn a_session_from_before_the_recent_list_is_read_and_brought_forward() {
    let dir = TempDir::new("recent-absent");
    let path = dir.join(FILE_NAME);
    std::fs::write(
        &path,
        "version = 1\nwindow_state = \"normal\"\n\n[geometry]\nx = 10.0\ny = 20.0\nwidth = 800.0\nheight = 600.0\n",
    )
    .expect("write session");

    let restored = Session::load(&path);
    assert!(restored.writable);
    assert_eq!(
        restored.session.geometry,
        Some(Geometry::new(10.0, 20.0, 800.0, 600.0)),
        "a window left by the previous layout should still come back"
    );
    assert!(restored.session.recent.is_empty());
    assert_eq!(
        restored.session.version, VERSION,
        "it is written back at the version this binary writes"
    );
}

#[test]
fn the_version_goes_up_when_the_layout_gains_something() {
    // The point of the number: an older binary must see one it does not know
    // and leave the file, rather than reading what it understands and
    // rewriting without the rest. The recent list is what it would have lost.
    let dir = TempDir::new("downgrade");
    let path = dir.join(FILE_NAME);
    let mut session = Session::default();
    session.remember(Path::new("/captures/dump.bin"), &raw_hints());
    assert!(session.save(&path));

    let text = std::fs::read_to_string(&path).expect("read back");
    assert!(
        text.contains("version = 11"),
        "the current session layout is version 11: {text}"
    );
}

#[test]
fn a_chosen_theme_round_trips_and_an_older_session_chooses_none() {
    use crate::config::Theme;
    let dir = TempDir::new("theme");
    let path = dir.join(FILE_NAME);
    std::fs::write(&path, "version = 10
window_state = \"normal\"\n")
        .expect("write session");
    let restored = Session::load(&path);
    assert!(restored.writable);
    assert_eq!(restored.session.theme, None, "an older session follows the configuration");

    for theme in [Theme::System, Theme::Light, Theme::Dark] {
        let session = Session {
            theme: Some(theme),
            ..Session::default()
        };
        assert!(session.save(&path));
        assert_eq!(Session::load(&path).session.theme, Some(theme));
    }

    assert!(Session::default().save(&path));
    let text = std::fs::read_to_string(&path).expect("read back");
    assert!(!text.contains("theme"), "no choice writes no theme: {text}");
}

#[test]
fn two_captures_with_the_same_name_share_the_bare_name() {
    // One capture name in two real directories, which the list shows alike.
    let root = TempDir::new("labels");
    let (a, b) = (root.join("a"), root.join("b"));
    let mut session = Session::default();
    for path in [b.join("12.579.iqw"), a.join("12.579.iqw"), a.join("other.iqw")] {
        session.remember(&path, &OpenHints::default());
    }

    assert_eq!(
        recent_labels(&session.recent),
        ["other.iqw", "12.579.iqw", "12.579.iqw"],
        "every entry is named by its file alone"
    );
}

#[test]
fn a_saved_panel_split_is_read_and_forgotten() {
    let dir = TempDir::new("panel-split");
    let path = dir.join(FILE_NAME);
    let geometry = "[geometry]\nx = 1.0\ny = 2.0\nwidth = 900.0\nheight = 600.0\n";
    for version in 1..=VERSION {
        for split in ["", "waveform_fraction = 0.3\n", "waveform_fraction = nan\n"] {
            let text = format!("version = {version}\nwindow_state = \"normal\"\n{split}{geometry}");
            std::fs::write(&path, &text).expect("write session");
            let restored = Session::load(&path);
            assert!(restored.writable, "{text}");
            assert_eq!(restored.session.version, VERSION, "{text}");
            let kept = restored.session.geometry;
            assert!(kept.is_some(), "{text}");
            assert!(restored.session.save(&path));
            let written = std::fs::read_to_string(&path).expect("read session");
            assert!(!written.contains("waveform_fraction"), "{written}");
            assert_eq!(Session::load(&path).session.geometry, kept, "{written}");
        }
    }
}

#[test]
fn analysis_settings_survive_restart_without_changing_configuration_or_older_geometry() {
    let dir = TempDir::new("settings-session");
    let path = dir.join(FILE_NAME);
    let config_path = dir.join("argand.toml");
    let config_text = "# My defaults\ndynamic_range = 'auto'\n";
    std::fs::write(&config_path, config_text).unwrap();
    std::fs::write(&path, "version=3\nwindow_state='normal'\nwaveform_fraction=0.3\n[geometry]\nx=1.0\ny=2.0\nwidth=900.0\nheight=600.0\n").unwrap();
    let mut session = Session::load(&path).session;
    assert!(session.analysis_settings.is_none());
    let settings = crate::settings::Settings {
        fft_size: 4096, overlap: 50, window: argand_dsp::Window::Hamming,
        aggregation: crate::config::Aggregation::MeanPower,
        colormap: argand_core::Colormap::Viridis,
        dynamic_range: argand_dsp::DynamicRange::Fixed(43.0),
    };
    session.analysis_settings = Some(settings);
    assert!(session.save(&path));
    let restored = Session::load(&path).session;
    assert_eq!(restored.analysis_settings, Some(crate::settings::Settings { dynamic_range: argand_dsp::DynamicRange::Default, ..settings }));
    assert!(!std::fs::read_to_string(&path).unwrap().contains("dynamic_range"));
    assert_eq!(restored.geometry, session.geometry);
    assert_eq!(std::fs::read_to_string(config_path).unwrap(), config_text);
}


#[test]
fn legacy_saved_zoom_is_ignored() {
    let dir = TempDir::new("legacy-view");
    let path = dir.join("session.toml");
    let mut session = Session::default();
    session.remember(Path::new("/captures/one.wav"), &OpenHints::default());
    assert!(session.save(&path));
    let mut text = std::fs::read_to_string(&path).unwrap();
    text.push_str("\n[recent.view]\nstart = 100\nlen = 2048\n");
    std::fs::write(&path, text).unwrap();
    let restored = Session::load(&path).session;
    assert_eq!(restored.recent.len(), 1);
    assert_eq!(restored.recent, session.recent);
}

#[test]
fn ruler_mode_round_trips_and_all_older_sessions_default_to_clock() {
    use crate::time_ruler::Mode;
    let dir = TempDir::new("ruler-mode");
    let path = dir.join(FILE_NAME);
    for mode in [Mode::Clock, Mode::Seconds, Mode::Samples] {
        let session = Session { time_ruler: mode, ..Session::default() };
        assert!(session.save(&path));
        assert_eq!(Session::load(&path).session.time_ruler, mode);
    }
    for version in 1..=6 {
        std::fs::write(&path, format!("version = {version}\nwindow_state = \"normal\"\n")).unwrap();
        let restored = Session::load(&path);
        assert!(restored.writable);
        assert_eq!(restored.session.time_ruler, Mode::Clock);
        assert_eq!(restored.session.version, VERSION);
    }
}

#[test]
fn grid_visibility_round_trips_both_values_and_defaults_for_older_sessions() {
    let dir = TempDir::new("grid-visibility");
    let path = dir.join(FILE_NAME);
    for show_grid in [false, true] {
        let session = Session { show_grid, ..Session::default() };
        assert!(session.save(&path));
        assert_eq!(Session::load(&path).session.show_grid, show_grid);
    }
    for version in 1..VERSION {
        std::fs::write(&path, format!("version = {version}\nwindow_state = \"maximized\"\n")).unwrap();
        let restored = Session::load(&path);
        assert!(restored.writable);
        assert!(restored.session.show_grid);
        assert_eq!(restored.session.window_state, WindowState::Maximized);
    }
}

#[test]
fn orientation_round_trips_and_older_sessions_keep_horizontal_default() {
    use crate::orientation::Mode;
    let dir = TempDir::new("orientation");
    let path = dir.join(FILE_NAME);
    for orientation in [Mode::Horizontal, Mode::Vertical] {
        let session = Session { orientation, ..Session::default() };
        assert!(session.save(&path));
        assert_eq!(Session::load(&path).session.orientation, orientation);
    }
    for version in 1..VERSION {
        std::fs::write(&path, format!("version = {version}\nwindow_state = \"maximized\"\n")).unwrap();
        let restored = Session::load(&path);
        assert!(restored.writable);
        assert_eq!(restored.session.orientation, Mode::Horizontal);
        assert_eq!(restored.session.window_state, WindowState::Maximized);
    }
}

/// Two instances that started from one file, as `main` seeds each `Writer`.
fn two_instances(path: &Path, start: &Session) -> (Writer, Writer, Session, Session) {
    (
        Writer::new(path.to_owned(), start.clone()),
        Writer::new(path.to_owned(), start.clone()),
        start.clone(),
        start.clone(),
    )
}

fn recent_paths(session: &Session) -> Vec<PathBuf> {
    session.recent.iter().map(|entry| entry.path.clone()).collect()
}

#[test]
fn two_instances_that_both_exit_keep_both_recent_files_and_their_hints() {
    let dir = TempDir::new("two-recent");
    let path = dir.join(FILE_NAME);
    let mut start = Session::default();
    start.remember(&dir.join("old.wav"), &OpenHints::default());
    assert!(start.save(&path));

    let (mut first, mut second, mut a, mut b) = two_instances(&path, &start);
    let now = Instant::now();
    a.remember(&dir.join("first.bin"), &raw_hints());
    first.offer(a.clone(), now);
    b.remember(&dir.join("second.wav"), &OpenHints::default());
    second.offer(b.clone(), now);
    first.flush(now);
    second.flush(now);

    let stored = Session::load(&path).session;
    assert_eq!(
        recent_paths(&stored),
        [dir.join("second.wav"), dir.join("first.bin"), dir.join("old.wav")]
    );
    assert_eq!(stored.recent[1].hints.to_open_hints().byte_offset, 44);
}

#[test]
fn a_setting_one_instance_changed_survives_the_other_writing_its_own() {
    let dir = TempDir::new("two-settings");
    let path = dir.join(FILE_NAME);
    let (mut first, mut second, mut a, mut b) = two_instances(&path, &Session::default());
    let now = Instant::now();

    a.show_grid = false;
    a.theme = Some(crate::config::Theme::Dark);
    first.offer(a.clone(), now);
    b.geometry = Some(Geometry::new(40.0, 30.0, 900.0, 700.0));
    b.window_state = WindowState::Maximized;
    second.offer(b.clone(), now);

    let stored = Session::load(&path).session;
    assert!(!stored.show_grid);
    assert_eq!(stored.theme, Some(crate::config::Theme::Dark));
    assert_eq!(stored.geometry, b.geometry);
    assert_eq!(stored.window_state, WindowState::Maximized);

    // The first instance writing again must not take back what the second changed.
    a.show_scale_ui = false;
    first.offer(a.clone(), now + Writer::INTERVAL);
    let stored = Session::load(&path).session;
    assert_eq!(stored.geometry, b.geometry);
    assert!(!stored.show_scale_ui && !stored.show_grid);
}

#[test]
fn the_same_setting_changed_by_both_is_the_last_writers() {
    let dir = TempDir::new("two-same");
    let path = dir.join(FILE_NAME);
    let (mut first, mut second, mut a, mut b) = two_instances(&path, &Session::default());
    let now = Instant::now();

    a.time_ruler = crate::time_ruler::Mode::Seconds;
    first.offer(a, now);
    b.time_ruler = crate::time_ruler::Mode::Samples;
    second.offer(b, now);

    assert_eq!(
        Session::load(&path).session.time_ruler,
        crate::time_ruler::Mode::Samples
    );
}

#[test]
fn a_file_reopened_with_corrected_hints_carries_them_into_the_merge() {
    let dir = TempDir::new("reopen");
    let mut base = Session::default();
    for name in ["a.bin", "b.wav", "c.wav"] {
        base.remember(&dir.join(name), &OpenHints::default());
    }
    let mut mine = base.clone();
    mine.remember(&dir.join("a.bin"), &raw_hints());
    let mut theirs = base.clone();
    theirs.remember(&dir.join("d.wav"), &OpenHints::default());

    let merged = merge(&base, &mine, &theirs);
    assert_eq!(
        recent_paths(&merged),
        [
            dir.join("a.bin"),
            dir.join("d.wav"),
            dir.join("c.wav"),
            dir.join("b.wav")
        ]
    );
    assert_eq!(merged.recent[0].hints.to_open_hints().byte_offset, 44);
}

#[test]
fn the_merged_recent_list_is_still_bounded() {
    let dir = TempDir::new("merge-limit");
    let base = Session::default();
    let mut mine = base.clone();
    let mut theirs = base.clone();
    for i in 0..RECENT_LIMIT {
        mine.remember(&dir.join(&format!("mine-{i}.wav")), &OpenHints::default());
        theirs.remember(&dir.join(&format!("theirs-{i}.wav")), &OpenHints::default());
    }

    let merged = merge(&base, &mine, &theirs);
    assert_eq!(merged.recent, mine.recent);
}

#[test]
fn a_recent_list_no_opening_explains_is_merged_as_a_union() {
    let dir = TempDir::new("merge-union");
    let mut base = Session::default();
    base.remember(&dir.join("kept.wav"), &OpenHints::default());
    let mine = Session {
        recent: vec![Recent {
            path: dir.join("odd.wav"),
            hints: Hints::default(),
        }],
        ..Session::default()
    };
    let mut theirs = base.clone();
    theirs.remember(&dir.join("theirs.wav"), &OpenHints::default());

    assert_eq!(
        recent_paths(&merge(&base, &mine, &theirs)),
        [dir.join("odd.wav"), dir.join("theirs.wav"), dir.join("kept.wav")]
    );
}

#[test]
fn an_unchanged_session_takes_everything_from_the_file() {
    let dir = TempDir::new("merge-unchanged");
    let base = Session::default();
    let mut theirs = Session {
        show_grid: false,
        geometry: Some(Geometry::new(1.0, 2.0, 300.0, 200.0)),
        ..Session::default()
    };
    theirs.remember(&dir.join("theirs.wav"), &OpenHints::default());

    assert_eq!(merge(&base, &base, &theirs), theirs);
}

/// The lock another instance would hold while it writes.
fn hold_lock(path: &Path) -> std::fs::File {
    let file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(false)
        .open(path.with_extension("toml.lock"))
        .expect("open lock");
    file.lock().expect("take lock");
    file
}

#[test]
fn a_held_lock_defers_the_write_to_a_later_interval() {
    let dir = TempDir::new("busy");
    let path = dir.join(FILE_NAME);
    let mut writer = Writer::new(path.clone(), Session::default());
    let moved = Session {
        geometry: Some(Geometry::new(5.0, 6.0, 800.0, 600.0)),
        ..Session::default()
    };
    let start = Instant::now();

    let held = hold_lock(&path);
    writer.offer(moved.clone(), start);
    assert!(!path.exists(), "a write went ahead under another instance's lock");

    drop(held);
    writer.offer(moved.clone(), start + Writer::INTERVAL);
    assert_eq!(Session::load(&path).session, moved);
}

#[test]
fn a_lock_held_through_the_exit_costs_a_bounded_wait() {
    let dir = TempDir::new("busy-exit");
    let path = dir.join(FILE_NAME);
    let mut writer = Writer::new(path.clone(), Session::default());
    let start = Instant::now();
    let _held = hold_lock(&path);

    writer.offer(
        Session {
            show_grid: false,
            ..Session::default()
        },
        start,
    );
    let waited = Instant::now();
    writer.flush(start);

    assert!(waited.elapsed() < Duration::from_secs(2));
    assert!(!path.exists());
}

#[test]
fn a_newer_version_written_while_running_is_left_alone() {
    let dir = TempDir::new("future-running");
    let path = dir.join(FILE_NAME);
    let mut writer = Writer::new(path.clone(), Session::default());
    let text = "version = 9999\nsomething_new = true\n";
    std::fs::write(&path, text).expect("write fixture");
    let start = Instant::now();

    let moved = |x: f32| Session {
        geometry: Some(Geometry::new(x, 0.0, 800.0, 600.0)),
        ..Session::default()
    };
    writer.offer(moved(1.0), start);
    writer.offer(moved(2.0), start + Writer::INTERVAL);
    writer.flush(start + Writer::INTERVAL * 2);

    assert_eq!(std::fs::read_to_string(&path).expect("still there"), text);
}

#[test]
fn a_corrupt_file_found_at_write_time_is_replaced_by_this_instances_session() {
    let dir = TempDir::new("corrupt-running");
    let path = dir.join(FILE_NAME);
    let mut writer = Writer::new(path.clone(), Session::default());
    std::fs::write(&path, "not = [toml").expect("write fixture");
    let moved = Session {
        show_grid: false,
        ..Session::default()
    };

    writer.offer(moved.clone(), Instant::now());
    assert_eq!(Session::load(&path).session, moved);
}

#[test]
fn a_range_change_the_file_never_keeps_does_not_take_back_anothers_settings() {
    let saved = crate::settings::Settings::from_config(&crate::config::Config::default());
    let base = Session {
        analysis_settings: Some(saved),
        ..Session::default()
    };
    let mine = Session {
        analysis_settings: Some(crate::settings::Settings {
            dynamic_range: argand_dsp::DynamicRange::Fixed(60.0),
            ..saved
        }),
        ..base.clone()
    };
    let theirs = Session {
        analysis_settings: Some(crate::settings::Settings {
            fft_size: saved.fft_size * 4,
            ..saved
        }),
        ..base.clone()
    };

    let merged = merge(&base, &mine, &theirs);
    assert_eq!(
        merged.analysis_settings.map(|settings| settings.fft_size),
        Some(saved.fft_size * 4)
    );
}

#[test]
fn orientation_and_analysis_settings_merge_as_their_own_units() {
    let saved = crate::settings::Settings::from_config(&crate::config::Config::default());
    let base = Session::default();
    let mine = Session {
        orientation: crate::orientation::Mode::Vertical,
        ..Session::default()
    };
    let theirs = Session {
        analysis_settings: Some(saved),
        ..Session::default()
    };

    let merged = merge(&base, &mine, &theirs);
    assert_eq!(merged.orientation, crate::orientation::Mode::Vertical);
    assert_eq!(merged.analysis_settings, Some(saved));
}

#[test]
fn a_lock_that_cannot_be_taken_still_lets_the_session_be_saved() {
    let dir = TempDir::new("no-lock");
    let path = dir.join(FILE_NAME);
    // A directory where the lock file belongs cannot be opened as one.
    std::fs::create_dir_all(path.with_extension("toml.lock")).expect("block the lock");
    let mut writer = Writer::new(path.clone(), Session::default());
    let moved = Session {
        show_grid: false,
        ..Session::default()
    };

    writer.offer(moved.clone(), Instant::now());
    assert_eq!(Session::load(&path).session, moved);
}

#[test]
fn the_lock_is_released_once_a_write_is_done() {
    let dir = TempDir::new("released");
    let path = dir.join(FILE_NAME);
    let mut writer = Writer::new(path.clone(), Session::default());
    writer.offer(
        Session {
            show_grid: false,
            ..Session::default()
        },
        Instant::now(),
    );

    let lock = std::fs::File::open(path.with_extension("toml.lock")).expect("lock file");
    assert!(lock.try_lock().is_ok(), "the writer kept the lock after writing");
}
