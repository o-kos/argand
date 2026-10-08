use super::*;

const A: SourceId = SourceId(0);
const B: SourceId = SourceId(1);

/// Each sample as the source and index it came from, the model every edit is checked against.
fn expand(capture: &Capture) -> Vec<(SourceId, u64)> {
    capture
        .pieces()
        .iter()
        .flat_map(|piece| (piece.start..piece.end()).map(move |at| (piece.source, at)))
        .collect()
}

fn span(a: u64, b: u64) -> SampleSpan {
    SampleSpan::between(a, b).unwrap()
}

/// A small deterministic generator, so the property test needs no dependency.
struct Lcg(u64);

impl Lcg {
    fn below(&mut self, bound: u64) -> u64 {
        self.0 = self.0.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 33) % bound.max(1)
    }
}

#[test]
fn a_whole_capture_is_one_piece() {
    let capture = Capture::whole(A, 10);
    assert_eq!(capture.len(), 10);
    assert_eq!(capture.pieces(), [Piece { source: A, start: 0, len: 10 }]);
    assert!(Capture::whole(A, 0).is_empty());
}

#[test]
fn delete_insert_and_replace_keep_the_rest_in_order() {
    let capture = Capture::whole(A, 10);
    let deleted = capture.delete(span(2, 5));
    assert_eq!(expand(&deleted), [0, 1, 5, 6, 7, 8, 9].map(|at| (A, at)));
    let clip = Capture::whole(B, 3).copy(span(0, 3));
    let inserted = deleted.insert(2, &clip);
    assert_eq!(inserted.len(), 10);
    assert_eq!(expand(&inserted)[2..5], [(B, 0), (B, 1), (B, 2)]);
    let replaced = capture.replace(span(8, 10), &clip);
    assert_eq!(expand(&replaced)[8..], [(B, 0), (B, 1), (B, 2)]);
}

#[test]
fn undoing_a_cut_by_pasting_it_back_joins_the_pieces_again() {
    let capture = Capture::whole(A, 100);
    let cut = capture.copy(span(30, 60));
    let restored = capture.delete(span(30, 60)).insert(30, &cut);
    assert_eq!(restored, capture);
}

#[test]
fn locate_answers_the_piece_and_the_offset() {
    let capture = Capture::whole(A, 10).insert(4, &Capture::whole(B, 2).copy(span(0, 2)));
    assert_eq!(capture.locate(0), (0, 0));
    assert_eq!(capture.locate(4), (1, 0));
    assert_eq!(capture.locate(5), (1, 1));
    assert_eq!(capture.locate(6), (2, 0));
    assert_eq!(capture.locate(12), (3, 0));
    assert_eq!(capture.piece_start(2), 6);
}

#[test]
fn edits_past_the_end_are_clamped() {
    let capture = Capture::whole(A, 5);
    let clip = Capture::whole(B, 1).copy(span(0, 1));
    assert_eq!(expand(&capture.insert(99, &clip)).last(), Some(&(B, 0)));
    assert_eq!(capture.delete(span(3, 99)).len(), 3);
    assert!(capture.segments(span(7, 9)).is_empty());
}

#[test]
fn a_clip_can_be_renamed_for_another_table() {
    let clip = Capture::whole(A, 4).copy(span(1, 3));
    let renamed = clip.remap(|_| B);
    assert_eq!(renamed.pieces(), [Piece { source: B, start: 1, len: 2 }]);
    assert_eq!(renamed.len(), 2);
}

type Model = Vec<(SourceId, u64)>;

/// One random edit applied to both the capture and the model.
fn edit_once(capture: &Capture, model: &mut Model, random: &mut Lcg) -> Capture {
    let other = Capture::whole(B, 40);
    let len = capture.len();
    let picked = SampleSpan::between(random.below(len + 1), random.below(len + 1));
    let from = random.below(39);
    match (random.below(4), picked) {
        (0, Some(s)) => {
            model.drain(s.start() as usize..s.end() as usize);
            capture.delete(s)
        }
        (1, _) => {
            let at = random.below(len + 1);
            let clip = other.copy(span(from, from + 1 + random.below(40 - from)));
            model.splice(at as usize..at as usize, expand(&Capture::from_pieces(clip.pieces().to_vec())));
            capture.insert(at, &clip)
        }
        (2, Some(s)) => {
            let at = random.below(len + 1);
            let copied = model[s.start() as usize..s.end() as usize].to_vec();
            model.splice(at as usize..at as usize, copied);
            capture.insert(at, &capture.copy(s))
        }
        (3, Some(s)) => {
            model.splice(s.start() as usize..s.end() as usize, [(B, from)]);
            capture.replace(s, &other.copy(span(from, from + 1)))
        }
        _ => capture.clone(),
    }
}

#[test]
fn random_edits_match_a_plain_vector() {
    let mut random = Lcg(7);
    for _ in 0..200 {
        let mut capture = Capture::whole(A, 1 + random.below(50));
        let mut model = expand(&capture);
        for _ in 0..20 {
            capture = edit_once(&capture, &mut model, &mut random);
            assert_eq!(expand(&capture), model);
            assert_eq!(capture.len(), model.len() as u64);
            let len = capture.len();
            let Some(s) = SampleSpan::between(random.below(len + 1), random.below(len + 1)) else {
                continue;
            };
            let segments = Capture::from_pieces(capture.segments(s));
            assert_eq!(expand(&segments), model[s.start() as usize..s.end() as usize]);
        }
    }
}

#[test]
fn a_sample_is_found_where_the_other_version_held_it() {
    let before = Capture::whole(A, 100);
    let after = before.delete(span(10, 20));
    assert_eq!(after.position_in(5, &before), Some(5));
    assert_eq!(after.position_in(10, &before), Some(20));
    let pasted = after.insert(0, &Clip::new(vec![Piece { source: B, start: 0, len: 5 }]));
    assert_eq!(pasted.position_in(2, &before), None, "the other file was never in it");
    assert_eq!(pasted.position_in(5, &after), Some(0));
}
