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
        version: VERSION,
        geometry: Some(Geometry::new(12.0, 34.0, 1280.0, 800.0)),
        window_state: WindowState::Maximized,
    };

    session.save(&path);
    assert_eq!(Session::load(&path), session);
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
        assert_eq!(
            Session::load(&path),
            Session::default(),
            "{name} did not fall back to defaults"
        );
    }
}

#[test]
fn a_save_interrupted_partway_leaves_the_previous_session_readable() {
    let dir = TempDir::new("atomic");
    let path = dir.join("session.toml");

    let first = Session {
        version: VERSION,
        geometry: Some(Geometry::new(0.0, 0.0, 800.0, 600.0)),
        window_state: WindowState::Normal,
    };
    first.save(&path);

    // What a kill mid-write leaves behind: the staging file exists with
    // whatever had been flushed, and the rename never happened.
    let staging = path.with_extension(format!("toml.{}.tmp", std::process::id()));
    std::fs::write(&staging, "version = 1\ngeometry = { x =").expect("write partial");

    assert_eq!(
        Session::load(&path),
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
    assert_eq!(Session::load(&path), session);
}

#[test]
fn a_save_that_cannot_happen_is_reported_rather_than_fatal() {
    let dir = TempDir::new("unwritable");
    // A path whose parent is a file, so neither the directory creation nor the
    // write can succeed.
    let blocker = dir.join("a-file");
    std::fs::write(&blocker, "not a directory").expect("write blocker");

    // Reaching the end of this test is the assertion: losing a window position
    // must not take the application down with it.
    Session::default().save(&blocker.join("session.toml"));
}

#[test]
fn a_drag_writes_a_few_times_rather_than_once_a_frame() {
    let dir = TempDir::new("debounce");
    let path = dir.join("session.toml");
    let mut writer = Writer::new(path.clone(), Session::default());

    let at = |ms: u64| Instant::now() + Duration::from_millis(ms);
    let moved = |x: f32| Session {
        version: VERSION,
        geometry: Some(Geometry::new(x, 0.0, 1280.0, 800.0)),
        window_state: WindowState::Normal,
    };

    // A frame every 8ms for a second, as a drag produces.
    let start = at(0);
    writer.offer(moved(0.0), start);
    for frame in 1..125u64 {
        writer.offer(moved(frame as f32), start + Duration::from_millis(frame * 8));
    }
    writer.flush(start + Duration::from_secs(1));

    // The file holds where the drag ended, not where it passed through.
    let restored = Session::load(&path);
    assert_eq!(restored.geometry.expect("a position").x, 124.0);
}

#[test]
fn a_position_that_has_not_changed_is_not_written_again() {
    let dir = TempDir::new("idle");
    let path = dir.join("session.toml");
    let held = Session {
        version: VERSION,
        geometry: Some(Geometry::new(10.0, 20.0, 1280.0, 800.0)),
        window_state: WindowState::Normal,
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
        version: VERSION,
        geometry: Some(Geometry::new(x, 0.0, 1280.0, 800.0)),
        window_state: WindowState::Normal,
    };

    writer.offer(moved(1.0), start);
    // Immediately after a write, so the schedule holds this one back.
    writer.offer(moved(2.0), start + Duration::from_millis(1));
    assert_eq!(Session::load(&path).geometry.expect("a position").x, 1.0);

    // Closing the window is what makes it land.
    writer.flush(start + Duration::from_millis(2));
    assert_eq!(Session::load(&path).geometry.expect("a position").x, 2.0);
}

#[test]
fn a_window_that_comes_back_to_where_it_started_writes_nothing() {
    let dir = TempDir::new("returned");
    let path = dir.join("session.toml");
    let at = |x: f32| Session {
        version: VERSION,
        geometry: Some(Geometry::new(x, 0.0, 1280.0, 800.0)),
        window_state: WindowState::Normal,
    };

    let mut writer = Writer::new(path.clone(), at(0.0));
    let start = Instant::now();
    // Settle a write first, so the schedule is holding the next one back.
    writer.offer(at(100.0), start);
    assert_eq!(Session::load(&path), at(100.0));

    // Dragged away and back again before the interval is up. The offer in
    // between is one the window has already left, so what finally lands has to
    // be where it actually is.
    writer.offer(at(400.0), start + Duration::from_millis(1));
    writer.offer(at(100.0), start + Duration::from_millis(2));
    writer.flush(start + Duration::from_secs(1));

    assert_eq!(
        Session::load(&path),
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
        version: VERSION,
        geometry: Some(Geometry::new(10.0, 20.0, 1280.0, 800.0)),
        window_state: WindowState::Normal,
    };
    let start = Instant::now();
    writer.offer(moved.clone(), start);
    assert!(!path.exists(), "the fixture did not actually block the write");

    // Clear the obstruction, as a full disk or a permission might clear, and
    // the value the window is still at has to reach the file.
    std::fs::remove_file(&blocked).expect("remove blocker");
    writer.flush(start + Duration::from_secs(1));
    assert_eq!(
        Session::load(&path),
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
