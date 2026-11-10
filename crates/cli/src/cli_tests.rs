use super::*;
use clap::CommandFactory;

#[test]
fn the_command_definition_is_valid() {
    Args::command().debug_assert();
}

#[test]
fn image_size_accepts_the_shapes_people_type() {
    for (text, want) in [
        ("2048x512", (2048, 512)),
        ("800X600", (800, 600)),
        (" 640 * 480 ", (640, 480)),
    ] {
        assert_eq!(image_size(text).unwrap(), want, "{text}");
    }
}

#[test]
fn image_size_rejects_what_it_cannot_use() {
    for bad in ["2048", "0x512", "512x0", "axb", "", "2048x"] {
        let err = image_size(bad).unwrap_err();
        assert!(err.contains("image"), "{bad}: {err}");
    }
}

#[test]
fn bad_values_are_rejected_with_a_useful_message() {
    let cases = [
        (vec!["x.wav", "-t", "iq_f64"], "iq_f16x8"),
        (vec!["x.wav", "-w", "kaiser"], "hann"),
        (vec!["x.wav", "-c", "plasma"], "viridis"),
        (vec!["x.wav", "--panels", "waterfall"], "unknown panel"),
        (vec!["x.wav", "--panels", "spectrogram"], "waveform, psd, db, none"),
        (vec!["x.wav", "--panels", "none,psd"], "cannot be combined"),
        (vec!["x.wav", "--orientation", "sideways"], "horizontal"),
        (vec!["x.wav", "--reduce", "median"], "mean"),
        (vec!["x.wav", "-d", "default"], "positive number or auto"),
        (vec!["x.wav", "-d", "0"], "positive number or auto"),
        (vec!["x.wav", "-d", "-40"], "positive number or auto"),
        (vec!["x.wav", "-r", "fast"], "frequency"),
        (vec!["x.wav", "--start", "soon"], "time"),
        (vec!["x.wav", "--raw", "iq_i16@fast"], "frequency"),
    ];
    for (args, expected) in cases {
        let err = Args::try_parse_from(std::iter::once("aspec").chain(args.iter().copied()))
            .unwrap_err()
            .to_string();
        assert!(err.contains(expected), "{args:?} said: {err}");
    }
}
