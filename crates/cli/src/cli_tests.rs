use super::*;
use clap::CommandFactory;

fn parse(args: &[&str]) -> Args {
    Args::try_parse_from(std::iter::once("aspec").chain(args.iter().copied()))
        .unwrap_or_else(|e| panic!("failed to parse {args:?}: {e}"))
}

#[test]
fn the_command_definition_is_valid() {
    Args::command().debug_assert();
}

#[test]
fn defaults_match_the_documented_ones() {
    let args = parse(&["capture.iqw"]);
    assert_eq!(args.image_size, (2048, 512));
    assert_eq!(args.fft_size, 2048);
    assert_eq!(args.hop, None);
    assert_eq!(args.window_type, Window::Hann);
    assert_eq!(args.color_scheme, Colormap::Oceanic);
    assert_eq!(args.dynamic_range, 110.0);
    assert_eq!(args.reduce, Reduce::Max);
    assert_eq!(args.reference, DbReference::FullScale);
    assert_eq!(args.panels.to_string(), "waveform");
    assert_eq!(args.orientation, Orientation::Horizontal);
    assert_eq!(args.center, 0.0);
    assert_eq!(args.gain, 0.0);
    assert_eq!(args.normalize, None);
    assert!(!args.json && !args.quiet);
}

#[test]
fn short_flags_match_the_sgvr_cli() {
    let args = parse(&[
        "x.wav", "-f", "4096", "-w", "hamming", "-c", "viridis", "-i", "800x600", "-d", "60",
    ]);
    assert_eq!(args.fft_size, 4096);
    assert_eq!(args.window_type, Window::Hamming);
    assert_eq!(args.color_scheme, Colormap::Viridis);
    assert_eq!(args.image_size, (800, 600));
    assert_eq!(args.dynamic_range, 60.0);
}

#[test]
fn frequency_and_time_literals_are_parsed_by_the_shared_grammar() {
    let args = parse(&[
        "x.wav", "--center", "12.579M", "-r", "24k", "--start", "1m30", "--duration", "250ms",
    ]);
    assert_eq!(args.center, 12_579_000.0);
    assert_eq!(args.rate, Some(24_000.0));
    assert_eq!(args.start, Some(90.0));
    assert_eq!(args.duration, Some(0.25));
}

#[test]
fn the_raw_spec_carries_type_and_rate_together() {
    let args = parse(&["dump.bin", "--raw", "iq_i16@24k", "--offset", "512"]);
    let raw = args.raw.unwrap();
    assert_eq!(raw.sample_type.to_string(), "iq_i16");
    assert_eq!(raw.sample_rate, Some(24_000.0));
    assert_eq!(args.offset, 512);
}

#[test]
fn level_controls_accept_their_documented_spellings() {
    assert_eq!(
        parse(&["x.wav", "-n", "auto"]).normalize,
        Some(Normalize::Auto)
    );
    assert_eq!(
        parse(&["x.wav", "--normalize", "2.5"]).normalize,
        Some(Normalize::Factor(2.5))
    );
    assert_eq!(parse(&["x.wav", "-g", "-6"]).gain, -6.0);
    // A negative value with a unit suffix must not look like a flag.
    assert_eq!(parse(&["x.wav", "--center", "-1M"]).center, -1_000_000.0);
    assert_eq!(parse(&["x.wav", "-g", "-6.5"]).gain, -6.5);
}

#[test]
fn output_defaults_to_the_input_with_a_png_suffix() {
    let input = Path::new("/data/capture.iqw");
    assert_eq!(
        parse(&["/data/capture.iqw"]).output_path(input),
        PathBuf::from("/data/capture.iqw.png")
    );
    assert_eq!(
        parse(&["x.wav", "-o", "out.png"]).output_path(input),
        PathBuf::from("out.png")
    );
}

#[test]
fn inputs_collect_every_positional_argument() {
    let args = parse(&["a.wav", "*.iqw", "-f", "256", "/data/b.wav"]);
    assert_eq!(
        args.inputs,
        [
            PathBuf::from("a.wav"),
            PathBuf::from("*.iqw"),
            PathBuf::from("/data/b.wav")
        ]
    );
    assert_eq!(args.fft_size, 256);
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
        (vec!["x.wav", "--ref", "loudest"], "peak"),
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

#[test]
fn quiet_and_verbose_are_mutually_exclusive() {
    assert!(Args::try_parse_from(["aspec", "x.wav", "-q", "-v"]).is_err());
    assert_eq!(parse(&["x.wav", "-vv"]).verbose, 2);
}

#[test]
fn an_input_file_is_required() {
    assert!(Args::try_parse_from(["aspec"]).is_err());
    assert_eq!(parse(&["x.wav"]).inputs, [PathBuf::from("x.wav")]);
}
