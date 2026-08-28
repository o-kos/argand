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
fn paths_reach_stdout_only_when_the_caller_wants_them() {
    let args = parse(&["x.wav"]);
    assert_eq!(stdout_line(&args, true), StdoutLine::Path);
    assert_eq!(stdout_line(&args, false), StdoutLine::Nothing);
}

#[test]
fn a_batch_shrinks_the_block_unless_verbose_asks_for_it() {
    let plain = parse(&["a.wav", "b.wav"]);
    assert_eq!(stderr_block(&plain, true), StderrBlock::Compact);
    // One file on its own always gets the full block.
    assert_eq!(stderr_block(&plain, false), StderrBlock::Human);

    let verbose = parse(&["a.wav", "b.wav", "-v"]);
    assert_eq!(stderr_block(&verbose, true), StderrBlock::Human);
}

#[test]
fn the_two_streams_are_decided_independently() {
    // --json changes stdout and leaves the human report on stderr alone.
    let json = parse(&["x.wav", "--json"]);
    assert_eq!(stdout_line(&json, true), StdoutLine::Json);
    assert_eq!(stderr_block(&json, false), StderrBlock::Human);
}
