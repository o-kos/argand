use super::*;
use crate::SourceId;

fn capture(len: u64) -> Capture {
    Capture::whole(SourceId(0), len)
}

#[test]
fn undo_and_redo_walk_the_versions_with_their_state() {
    let mut history = History::new(capture(10), "opened");
    assert!(!history.can_undo() && !history.can_redo() && !history.is_dirty());
    history.apply(capture(8), "deleted");
    history.apply(capture(6), "deleted again");
    assert!(history.undo());
    assert_eq!((history.capture().len(), *history.state()), (8, "deleted"));
    assert!(history.redo());
    assert_eq!(history.capture().len(), 6);
    assert!(!history.redo());
}

#[test]
fn a_new_edit_drops_what_could_have_been_redone() {
    let mut history = History::new(capture(10), ());
    history.apply(capture(8), ());
    history.undo();
    history.apply(capture(9), ());
    assert!(!history.can_redo());
    assert_eq!(history.capture().len(), 9);
}

#[test]
fn versions_never_repeat_their_number() {
    let mut history = History::new(capture(10), ());
    history.apply(capture(8), ());
    let first = history.version();
    history.undo();
    history.apply(capture(9), ());
    assert_ne!(history.version(), first);
}

#[test]
fn the_saved_version_decides_the_unsaved_state() {
    let mut history = History::new(capture(10), ());
    history.apply(capture(8), ());
    assert!(history.is_dirty());
    history.undo();
    assert!(!history.is_dirty(), "back at the version that was opened");
    history.redo();
    history.mark_saved();
    assert!(!history.is_dirty());
    history.undo();
    history.apply(capture(7), ());
    assert!(history.is_dirty());
    assert!(!history.can_redo(), "the saved version is gone and nothing returns to it");
}
