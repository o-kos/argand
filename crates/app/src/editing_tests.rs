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
    assert!(document.is_dirty() && document.marked().time.is_none());
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
    assert_eq!(document.marked().time, Some(pasted));
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
    assert_eq!(state.sources[1].as_ref().map(|file| file.meta.source.clone()), Some(PathBuf::from("/b.wav")));
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

#[test]
fn a_clipboard_follows_its_samples_into_the_file_saved_over_them() {
    let mut document = editing("/a.wav", 100, 1000.0);
    let clipboard = document.copy(span(60, 70));
    let gone = document.copy(span(10, 20));
    document.delete(span(0, 30));
    let written = document.written_source();
    let moved = clipboard
        .moved_onto(document.file(), document.capture(), &written)
        .expect("the copied samples were kept");
    assert_eq!(
        moved.clip.pieces(),
        [argand_edit::Piece {
            source: SourceId(0),
            start: 30,
            len: 10,
        }]
    );
    assert_eq!(moved.sources[0].meta.len_samples, 70);
    let mut adopted = moved.clone();
    adopted.adopt(&Source {
        stamp: None,
        ..written
    });
    assert!(
        gone.moved_onto(document.file(), document.capture(), &document.written_source())
            .is_none(),
        "a copy of deleted samples cannot follow"
    );
}

#[test]
fn moved_positions_are_where_the_samples_sit_after_the_edits() {
    let mut document = editing("/a.wav", 100, 1000.0);
    document.delete(span(0, 30));
    assert_eq!(positions_in(document.capture(), 60, 10), Some(vec![(30, 10)]));
    assert_eq!(positions_in(document.capture(), 25, 10), None);
}

#[test]
fn an_envelope_left_unfinished_is_asked_for_again_after_a_restore() {
    let mut document = editing("/a.wav", 100, 1000.0);
    let other = editing("/b.wav", 50, 1000.0);
    document.paste(&other.copy(span(0, 20)), Placement::At(0)).unwrap();
    assert_eq!(document.missing_envelopes().len(), 1);
    assert!(document.missing_envelopes().is_empty());
    document.restart_envelopes();
    assert_eq!(document.missing_envelopes().len(), 1, "its builder went with the closed document");
}

/// Computed samples of `len`, at the capture's rate and kind, in a scratch file at `path`.
fn computed(path: &std::path::Path, len: u64, rate: f64) -> Source {
    std::fs::write(path, b"computed").unwrap();
    let mut meta = meta(&path.to_string_lossy(), len, rate);
    meta.sample_type = SampleType::new(Domain::Iq, SampleFormat::F32);
    meta.divisor = 1.0;
    Source::computed(
        Arc::new(Scratch::new(path.to_owned())),
        meta,
        argand_io::write::SourceStamp::of(path).ok(),
    )
}

fn scratch_dir(label: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("argand-editing-{label}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn computed_samples_replace_their_span_as_one_edit() {
    let dir = scratch_dir("replace");
    let mut document = editing("/a.wav", 100, 1000.0);
    document.describe_units(Some((32768.0, 0.0)));
    assert!(document.can_compute());
    assert!(!document.replace_computed(span(10, 30), computed(&dir.join("short.wav"), 19, 1000.0), Selection::default()));
    let rectangle = Selection {
        time: Some(span(15, 25)),
        band: argand_core::FrequencyBand::between(100.0, 200.0),
    };
    assert!(document.replace_computed(span(10, 30), computed(&dir.join("1.wav"), 20, 1000.0), rectangle));
    assert_eq!(document.len(), 100);
    assert_eq!(document.marked(), rectangle);
    let pieces: Vec<(u32, u64, u64)> = document
        .capture()
        .pieces()
        .iter()
        .map(|piece| (piece.source.0, piece.start, piece.len))
        .collect();
    assert_eq!(pieces, [(0, 0, 10), (1, 0, 20), (0, 30, 70)]);
    assert!(document.undo());
    assert!(document.is_untouched());
    assert_eq!(document.marked(), rectangle, "undoing it keeps the selection");
    assert!(document.redo());
    assert_eq!(document.marked(), rectangle);
    assert_eq!(document.missing_envelopes().len(), 1);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_scratch_file_goes_once_no_version_or_clipboard_reads_it() {
    let dir = scratch_dir("lifetime");
    let path = dir.join("1.wav");
    let mut document = editing("/a.wav", 100, 1000.0);
    document.replace_computed(span(0, 10), computed(&path, 10, 1000.0), Selection::default());
    let clipboard = document.copy(span(0, 5));
    drop(document);
    assert!(path.exists(), "the clipboard still reads it");
    drop(clipboard);
    assert!(!path.exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn computed_samples_paste_into_a_capture_stored_otherwise_of_the_same_rate_and_kind() {
    let dir = scratch_dir("paste");
    let mut source = editing("/a.wav", 100, 1000.0);
    source.replace_computed(span(0, 10), computed(&dir.join("1.wav"), 10, 1000.0), Selection::default());
    let clipboard = source.copy(span(0, 10));
    let mut other = Editing::new(meta("/b.wav", 50, 1000.0), OpenHints::default());
    other.describe_file(
        None,
        Some(Storage::Linear {
            format_tag: 1,
            bits: 32,
            channels: 2,
            block: 8,
            unscaled_float: false,
        }),
    );
    assert!(other.paste(&clipboard, Placement::At(0)).is_ok());
    let mut slower = editing("/c.wav", 50, 500.0);
    assert!(matches!(
        slower.paste(&clipboard, Placement::At(0)),
        Err(PasteError::Rate { .. })
    ));
    let mut real = Editing::new(
        SignalMeta {
            sample_type: SampleType::new(Domain::Real, SampleFormat::I16),
            ..meta("/d.wav", 50, 1000.0)
        },
        OpenHints::default(),
    );
    assert!(matches!(
        real.paste(&clipboard, Placement::At(0)),
        Err(PasteError::Kind { .. })
    ));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_picture_stays_over_samples_computed_in_place_and_brought_back() {
    let dir = scratch_dir("picture");
    let mut document = editing("/a.wav", 100, 1000.0);
    document.replace_computed(span(10, 30), computed(&dir.join("1.wav"), 20, 1000.0), Selection::default());
    let moved = document.repicture();
    assert_eq!(moved(5), Some(5));
    assert_eq!(moved(20), Some(20));
    assert!(document.undo());
    let moved = document.repicture();
    assert_eq!(moved(20), Some(20));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_save_of_computed_samples_reads_the_opened_file_first_and_encodes_them() {
    let dir = scratch_dir("save");
    let mut document = editing("/a.wav", 100, 1000.0);
    document.describe_units(Some((32768.0, 0.0)));
    document.replace_computed(span(0, 100), computed(&dir.join("1.wav"), 100, 1000.0), Selection::default());
    let request = document.save_request(None, PathBuf::from("/b.wav")).unwrap();
    assert_eq!(request.sources.len(), 2);
    assert_eq!(request.sources[0].meta.source, PathBuf::from("/a.wav"));
    assert_eq!(request.sources[0].encoding, None);
    assert_eq!(
        request.sources[1].encoding,
        Some(argand_io::write::Encoding {
            factor: 32768.0,
            offset: 0.0
        })
    );
    assert_eq!(request.segments.len(), 1);
    assert_eq!(request.segments[0].source, 1);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn an_adjustment_is_a_version_that_keeps_the_pieces_and_the_selection() {
    let mut document = editing("/a.wav", 100, 1000.0);
    let selected = Selection::time(span(10, 20));
    let inverted = Adjustment::IDENTITY.inverted();
    assert!(document.adjust(inverted, selected));
    assert!(!document.adjust(inverted, selected), "nothing new to add");
    assert_eq!(document.adjustment(), inverted);
    assert_eq!(document.marked(), selected);
    assert!(document.is_dirty() && !document.is_untouched());
    let request = document.save_request(Some(span(5, 50)), PathBuf::from("/b.wav")).unwrap();
    assert_eq!((request.adjustment, request.first), (inverted, 5));
    assert!(document.undo());
    assert_eq!(document.marked(), selected);
    assert!(document.adjustment().is_identity() && document.is_untouched());
}

#[test]
fn a_copy_pastes_only_under_the_adjustment_it_was_copied_under() {
    let mut document = editing("/a.wav", 100, 1000.0);
    let before = document.copy(span(0, 10));
    document.adjust(Adjustment::new(false, 2.0).unwrap(), Selection::default());
    assert_eq!(
        document.paste(&before, Placement::At(0)),
        Err(PasteError::Adjustment)
    );
    let after = document.copy(span(0, 10));
    assert!(document.paste(&after, Placement::At(0)).is_ok());
}

#[test]
fn copies_of_a_file_saved_adjusted_cannot_follow_it() {
    let mut document = editing("/a.wav", 100, 1000.0);
    let clipboard = document.copy(span(60, 70));
    document.adjust(Adjustment::IDENTITY.inverted(), Selection::default());
    let written = document.written_source();
    assert!(clipboard.moved_onto(document.file(), document.capture(), &written).is_none());
}

#[test]
fn full_scale_is_the_read_value_of_the_stored_full_scale() {
    let mut document = editing("/a.wav", 100, 1000.0);
    assert_eq!(document.full_scale(), None, "the reader's units are not known yet");
    document.describe_units(Some((32768.0 / 4.0, 0.0)));
    assert_eq!(document.full_scale(), Some(4.0));
}
