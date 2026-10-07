use super::*;
use std::path::PathBuf;

use argand_core::{Domain, SampleFormat, SampleType};

fn meta(path: &str, len: u64, rate: f64) -> SignalMeta {
    SignalMeta {
        sample_rate: rate,
        center_freq: 0.0,
        sample_type: SampleType::new(Domain::Iq, SampleFormat::I16),
        len_samples: len,
        container: "wav",
        divisor: 32768.0,
        source: PathBuf::from(path),
    }
}

const LINEAR: Storage = Storage::Linear {
    format_tag: 1,
    bits: 16,
    channels: 2,
    block: 4,
    unscaled_float: false,
};

fn editing(path: &str, len: u64, rate: f64) -> Editing {
    let mut editing = Editing::new(meta(path, len, rate), OpenHints::default());
    editing.describe_file(None, Some(LINEAR));
    editing
}

fn span(a: u64, b: u64) -> SampleSpan {
    SampleSpan::between(a, b).unwrap()
}

#[test]
fn delete_and_undo_change_the_length_and_the_unsaved_state() {
    let mut document = editing("/a.wav", 100, 1000.0);
    assert!(document.is_untouched() && !document.is_dirty());
    document.delete(span(10, 30));
    assert_eq!(document.len(), 80);
    assert!(document.is_dirty() && document.selection().is_none());
    assert!(document.undo());
    assert_eq!(document.len(), 100);
    assert!(!document.is_dirty() && document.is_untouched());
    assert!(document.redo());
    document.mark_saved();
    assert!(!document.is_dirty());
}

#[test]
fn a_paste_into_the_same_capture_needs_no_check_and_selects_what_it_pasted() {
    let mut document = Editing::new(meta("/a.wav", 100, 1000.0), OpenHints::default());
    let clipboard = document.copy(span(0, 10));
    let pasted = document.paste(&clipboard, Placement::At(50)).unwrap();
    assert_eq!(pasted, span(50, 60));
    assert_eq!(document.len(), 110);
    assert_eq!(document.selection(), Some(pasted));
    let replaced = document.paste(&clipboard, Placement::Replace(span(0, 50))).unwrap();
    assert_eq!(replaced, span(0, 10));
    assert_eq!(document.len(), 70);
    assert_eq!(document.capture().sources(), [SourceId(0)]);
}

#[test]
fn a_paste_from_another_file_adds_it_once_when_it_matches() {
    let mut document = editing("/a.wav", 100, 1000.0);
    let other = editing("/b.wav", 50, 1000.0);
    let clipboard = other.copy(span(0, 20));
    document.paste(&clipboard, Placement::At(100)).unwrap();
    document.paste(&clipboard, Placement::At(0)).unwrap();
    assert_eq!(document.capture().sources(), [SourceId(0), SourceId(1)]);
    assert_eq!(document.len(), 140);
    let state = document.edit_state();
    assert!(state.sources[0].is_none());
    assert_eq!(state.sources[1].as_ref().map(|(meta, _)| meta.source.clone()), Some(PathBuf::from("/b.wav")));
    assert_eq!(document.missing_envelopes().len(), 1);
}

#[test]
fn a_paste_from_a_file_stored_differently_or_at_another_rate_is_refused() {
    let mut document = editing("/a.wav", 100, 1000.0);
    let mut float = Editing::new(meta("/f.wav", 50, 1000.0), OpenHints::default());
    float.describe_file(
        None,
        Some(Storage::Linear {
            format_tag: 3,
            bits: 32,
            channels: 2,
            block: 8,
            unscaled_float: false,
        }),
    );
    let faster = editing("/r.wav", 50, 2000.0);
    let unchecked = Editing::new(meta("/u.wav", 50, 1000.0), OpenHints::default());
    for (other, refused) in [
        (&float, PasteError::Storage { name: "f.wav".into() }),
        (&faster, PasteError::Rate { name: "r.wav".into() }),
        (&unchecked, PasteError::Unknown { name: "u.wav".into() }),
    ] {
        let clipboard = other.copy(span(0, 10));
        assert_eq!(document.paste(&clipboard, Placement::At(0)), Err(refused));
    }
    assert!(!document.is_dirty(), "a refused paste changes nothing");
}

#[test]
fn a_save_names_only_the_files_its_samples_come_from() {
    let mut document = editing("/a.wav", 100, 1000.0);
    let other = editing("/b.wav", 50, 1000.0);
    document.paste(&other.copy(span(0, 20)), Placement::At(100)).unwrap();
    let whole = document.save_request(None, PathBuf::from("/out.wav")).unwrap();
    assert_eq!(whole.sources.len(), 2);
    assert_eq!(whole.segments.len(), 2);
    assert_eq!((whole.segments[1].source, whole.segments[1].start, whole.segments[1].len), (1, 0, 20));
    let tail = document.save_request(Some(span(105, 110)), PathBuf::from("/out.wav")).unwrap();
    assert_eq!(tail.sources.len(), 1);
    assert_eq!(tail.sources[0].meta.source, PathBuf::from("/b.wav"));
    assert_eq!(tail.protected[0].path, PathBuf::from("/a.wav"), "the open file stays protected");
    assert_eq!(tail.meta.source, PathBuf::from("/a.wav"), "and describes the result");
    assert_eq!((tail.segments[0].source, tail.segments[0].start, tail.segments[0].len), (0, 5, 5));
}

#[test]
fn the_minimap_waits_for_the_file_envelope() {
    let mut document = editing("/a.wav", 10, 1000.0);
    assert!(document.minimap().is_none());
    let envelope = argand_core::WaveformEnvelope::new(10, 2);
    document.set_envelope(
        SourceId(0),
        Arc::new(Snapshot {
            envelope,
            full_scale: 1.0,
            complete: true,
            samples: 10,
        }),
    );
    document.delete(span(0, 5));
    assert_eq!(document.minimap().map(|snapshot| snapshot.samples), Some(5));
}

#[test]
fn an_empty_capture_has_nothing_to_save() {
    let mut document = editing("/a.wav", 10, 1000.0);
    document.delete(span(0, 10));
    assert!(document.save_request(None, PathBuf::from("/out.wav")).is_none());
}

#[test]
fn the_same_file_opened_another_way_is_another_source() {
    let mut document = editing("/a.raw", 100, 1000.0);
    let mut shifted = Editing::new(
        meta("/a.raw", 100, 1000.0),
        OpenHints {
            byte_offset: 4,
            ..Default::default()
        },
    );
    shifted.describe_file(None, Some(LINEAR));
    document.paste(&shifted.copy(span(0, 10)), Placement::At(0)).unwrap();
    assert_eq!(document.capture().sources(), [SourceId(0), SourceId(1)]);
}

#[test]
fn an_envelope_is_asked_for_once_and_a_save_waits_for_every_stamp() {
    let mut document = editing("/a.wav", 100, 1000.0);
    let other = editing("/b.wav", 50, 1000.0);
    assert!(!document.is_described());
    document.paste(&other.copy(span(0, 20)), Placement::At(0)).unwrap();
    assert_eq!(document.missing_envelopes().len(), 1);
    document.undo();
    document.redo();
    assert!(document.missing_envelopes().is_empty(), "the scan already started");
}

#[test]
fn a_clipboard_copied_before_its_file_was_checked_learns_its_storage() {
    let mut document = editing("/a.wav", 100, 1000.0);
    let early = Editing::new(meta("/b.wav", 50, 1000.0), OpenHints::default());
    let mut clipboard = early.copy(span(0, 10));
    let refused = document.paste(&clipboard, Placement::At(0));
    assert!(matches!(refused, Err(PasteError::Unknown { .. })));
    clipboard.describe(early.file(), Some(LINEAR));
    assert!(
        document.paste(&clipboard, Placement::At(0)).is_err(),
        "a file without a stamp from its opening is not trusted"
    );
}
