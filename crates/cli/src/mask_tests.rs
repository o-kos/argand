use super::*;

fn mask(pattern: &str) -> Mask {
    Mask::new(pattern).unwrap_or_else(|e| panic!("{pattern}: {e}"))
}

fn assert_matches(pattern: &str, yes: &[&str], no: &[&str]) {
    let m = mask(pattern);
    for name in yes {
        assert!(m.matches(name), "{pattern} should match {name}");
    }
    for name in no {
        assert!(!m.matches(name), "{pattern} should not match {name}");
    }
}

#[test]
fn a_pattern_without_metacharacters_matches_only_itself() {
    assert!(!has_meta("capture.iqw"));
    assert_matches("capture.iqw", &["capture.iqw"], &["capture.wav", "capture"]);
}

#[test]
fn star_matches_any_run_including_an_empty_one() {
    assert!(has_meta("*.iqw"));
    assert_matches(
        "*.iqw",
        &["a.iqw", "long.name.iqw"],
        &["a.wav", "iqw", "a.iqw.png", ".iqw"],
    );
    assert_matches("*", &["anything", "a"], &[]);
    assert_matches("a*b*c", &["abc", "axxbyyc"], &["abcx", "acb"]);
}

#[test]
fn star_backtracks_when_the_tail_does_not_fit_the_first_try() {
    assert_matches("*.wav", &["a.wav.wav", "x.wav"], &["a.wav.png"]);
    assert_matches("*bb", &["abb", "abbbb"], &["ab", "abbc"]);
}

#[test]
fn question_mark_matches_exactly_one_character() {
    assert!(has_meta("iq_?.wav"));
    assert_matches("iq_?.wav", &["iq_a.wav"], &["iq_.wav", "iq_ab.wav"]);
    assert_matches("???", &["abc"], &["ab", "abcd"]);
}

#[test]
fn a_class_matches_the_characters_and_ranges_it_lists() {
    assert!(has_meta("[0-9].wav"));
    assert_matches(
        "capture[0-9].wav",
        &["capture0.wav", "capture7.wav"],
        &["capturex.wav", "capture10.wav"],
    );
    assert_matches("[abc].bin", &["a.bin", "c.bin"], &["d.bin"]);
    assert_matches("[a-cx-z]", &["b", "y"], &["m"]);
}

#[test]
fn a_negated_class_matches_everything_it_does_not_list() {
    for pattern in ["[!0-9].wav", "[^0-9].wav"] {
        assert_matches(pattern, &["a.wav", "_.wav"], &["1.wav"]);
    }
    assert_matches("[!a]*", &["boat"], &["about"]);
}

#[test]
fn a_bracket_or_dash_can_be_a_literal_member() {
    assert_matches("[]-]x", &["]x", "-x"], &["ax"]);
    assert_matches("[a-]x", &["ax", "-x"], &["bx"]);
}

#[test]
fn a_leading_dot_has_to_be_asked_for_by_name() {
    assert_matches("*", &["visible"], &[".hidden"]);
    assert_matches("?hidden", &["Xhidden"], &[".hidden"]);
    assert_matches("[.]hidden", &[], &[".hidden"]);
    assert_matches(".*", &[".hidden", ".config.toml"], &["visible"]);
}

#[test]
fn recursive_and_malformed_patterns_are_refused() {
    assert_eq!(Mask::new("**/*.wav"), Err(MaskError::Recursive));
    assert_eq!(Mask::new("a**b"), Err(MaskError::Recursive));
    assert_eq!(Mask::new("[0-9.wav"), Err(MaskError::UnclosedClass));
    assert_eq!(Mask::new("x[]"), Err(MaskError::UnclosedClass));
    assert_eq!(
        Mask::new("[9-0].wav"),
        Err(MaskError::ReversedRange { from: '9', to: '0' })
    );
}

#[test]
fn matching_is_by_character_not_by_byte() {
    assert_matches("?.wav", &["ы.wav"], &["ыы.wav"]);
    assert_matches("захват*", &["захват-1.iqw"], &["capture.iqw"]);
}
