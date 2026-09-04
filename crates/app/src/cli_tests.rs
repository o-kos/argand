use super::*;

use argand_core::{Domain, SampleFormat};
use clap::CommandFactory;

fn parse(args: &[&str]) -> Args {
    Args::try_parse_from(std::iter::once("argand").chain(args.iter().copied()))
        .expect("these arguments should parse")
}

#[test]
fn the_command_line_is_checked_the_way_clap_checks_its_own() {
    Args::command().debug_assert();
}

#[test]
fn a_window_can_be_opened_with_no_file_at_all() {
    assert!(parse(&[]).origin().is_none());
}

#[test]
fn a_headerless_capture_carries_its_layout_into_the_hints() {
    let args = parse(&[
        "dump.bin",
        "--raw",
        "iq_i16@2.4M",
        "--center",
        "12.579M",
        "--offset",
        "44",
    ]);
    let origin = args.origin().expect("a file was named");

    assert_eq!(origin.path, PathBuf::from("dump.bin"));
    let raw = origin.hints.raw.expect("a raw layout");
    assert_eq!(
        raw.sample_type,
        SampleType::new(Domain::Iq, SampleFormat::I16)
    );
    assert_eq!(raw.sample_rate, Some(2_400_000.0));
    assert_eq!(origin.hints.center_freq, 12_579_000.0);
    assert_eq!(origin.hints.byte_offset, 44);
}

#[test]
fn a_negative_centre_frequency_is_a_value_rather_than_a_flag() {
    let args = parse(&["capture.wav", "--center", "-1M", "-g", "-6"]);
    let hints = args.origin().expect("a file was named").hints;

    assert_eq!(hints.center_freq, -1_000_000.0);
    assert_eq!(hints.gain_db, -6.0);
}

#[test]
fn level_scaling_is_left_to_the_format_unless_it_is_named() {
    assert!(
        parse(&["capture.wav"])
            .origin()
            .expect("a file was named")
            .hints
            .normalize
            .is_none(),
        "the reader decides from the sample format when nobody has said"
    );
    assert_eq!(
        parse(&["capture.wav", "-n", "auto"])
            .origin()
            .expect("a file was named")
            .hints
            .normalize,
        Some(Normalize::Auto)
    );
}

#[test]
fn the_options_are_spelled_the_way_aspec_spells_them() {
    // Same seven fields, same short flags: a capture opened by one tool is
    // opened by the other with the arguments copied across unchanged.
    let args = parse(&[
        "capture.wav",
        "-t",
        "iq_f32",
        "-r",
        "24k",
        "-n",
        "none",
        "-g",
        "3",
    ]);
    let hints = args.origin().expect("a file was named").hints;

    assert_eq!(
        hints.sample_type,
        Some(SampleType::new(Domain::Iq, SampleFormat::F32))
    );
    assert_eq!(hints.sample_rate, Some(24_000.0));
    assert_eq!(hints.normalize, Some(Normalize::None));
    assert_eq!(hints.gain_db, 3.0);
}
