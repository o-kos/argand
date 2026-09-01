use super::*;

fn parse(args: &[&str]) -> Args {
    Args::try_parse_from(std::iter::once("aspec").chain(args.iter().copied()))
        .unwrap_or_else(|e| panic!("failed to parse {args:?}: {e}"))
}

#[test]
fn json_outranks_quiet_on_stdout() {
    // --json is an explicit request for machine output, so -q must not take
    // it away; -q is about what a person reads.
    for flags in [&["x.wav", "--json"][..], &["x.wav", "--json", "-q"][..]] {
        assert_eq!(stdout_line(&parse(flags), true), StdoutLine::Json, "{flags:?}");
    }
}

#[test]
fn quiet_silences_the_path_echo_but_json_still_prints() {
    let quiet = parse(&["x.wav", "-q"]);
    assert_eq!(stdout_line(&quiet, true), StdoutLine::Nothing);
    assert_eq!(stderr_block(&quiet, false), StderrBlock::Nothing);
    assert_eq!(stderr_block(&quiet, true), StderrBlock::Nothing);
}

#[test]
fn paths_reach_stdout_only_when_stdout_is_not_a_terminal() {
    // A terminal already names the render in the report above; piped, those
    // paths are what stdout is for. How many files there are does not enter
    // into it.
    //
    // The fact itself is passed in rather than read here: a pty test would
    // not run on the Windows job, and a test that let the harness answer
    // `is_terminal()` would pass under `cargo test` and fail for anyone
    // running the test binary from a shell. That one call is checked by hand
    // against the release binary instead.
    for (files, batch) in [(&["x.wav"][..], false), (&["a.wav", "b.wav"][..], true)] {
        let args = parse(files);
        let terminal = Reporting::for_stdout(&args, batch, files.len(), true);
        assert_eq!(terminal.stdout, StdoutLine::Nothing, "{files:?}");
        let piped = Reporting::for_stdout(&args, batch, files.len(), false);
        assert_eq!(piped.stdout, StdoutLine::Path, "{files:?}");
    }
}

#[test]
fn a_batch_shrinks_the_block_unless_verbose_asks_for_it() {
    let plain = parse(&["a.wav", "b.wav"]);
    assert_eq!(stderr_block(&plain, true), StderrBlock::Compact);
    // One file on its own always gets the sections.
    assert_eq!(
        stderr_block(&plain, false),
        StderrBlock::Block(Detail::Default)
    );

    // -v asks for every section in full, batch or not.
    let verbose = parse(&["a.wav", "b.wav", "-v"]);
    assert_eq!(
        stderr_block(&verbose, true),
        StderrBlock::Block(Detail::Verbose)
    );
    assert_eq!(
        stderr_block(&verbose, false),
        StderrBlock::Block(Detail::Verbose)
    );
}

#[test]
fn the_two_streams_are_decided_independently() {
    // --json changes stdout and leaves the human report on stderr alone.
    let json = parse(&["x.wav", "--json"]);
    assert_eq!(stdout_line(&json, true), StdoutLine::Json);
    assert_eq!(
        stderr_block(&json, false),
        StderrBlock::Block(Detail::Default)
    );
}

#[test]
fn the_colour_bar_gutter_bounds_each_dynamic_range_mode() {
    assert_eq!(
        colorbar_window(parse(&["x.wav"]).requested_dynamic_range()),
        (-110.0, 0.0)
    );
    assert_eq!(
        colorbar_window(parse(&["x.wav", "-d", "60"]).requested_dynamic_range()),
        (render::DB_FLOOR - 60.0, 0.0)
    );
    assert_eq!(
        colorbar_window(parse(&["x.wav", "-d", "auto"]).requested_dynamic_range()),
        (
            render::DB_FLOOR - f64::from(MAX_RECOMMENDED_RANGE_DB),
            0.0
        )
    );
}
