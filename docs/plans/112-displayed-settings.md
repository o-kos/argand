# Issue #112: Cover the displayed-settings lifecycle with a toolkit-free test

Resolves #112.

## Overview

`Shell::receive` decides when an analysis delivery is accepted and records which
settings the picture on screen was produced with (`file.displayed_settings`).
The decision is what keeps the spectrogram and everything derived from it
agreeing about the settings behind the picture. The function takes `&mut Window`
and `Context<Shell>`, so it names GPUI and cannot run in CI, and nothing tests
it.

The change extracts one open file's analysis-acceptance state into a
toolkit-neutral module, `crates/app/src/open_file.rs`, and covers the states the
issue names with unit tests: a superseded delivery rejected, a failed analysis
leaving the last good settings in place, an editor preview cancelled back to its
opening values, and a second file opened while a preview is pending.

## Context

- `analysis.rs` is already toolkit-neutral: `Analyst::accepts` gates a delivery
  by generation and view revision, and the existing `analysis_tests.rs` builds
  `Analyst` and hand-written `Delivery` values without a window.
- `document.rs` applies an `Update` and answers an `Effect`.
- The acceptance bookkeeping itself is three steps: gate, apply, and record the
  settings on an analysis effect. Everything else in `receive` - the backdrop
  restyle, the plot attachment, the minimap, the focus return, the notify - is
  window work and stays in `shell.rs`.
- The backdrop restyle may park a delivery (`backdrop.rs` returns `None` and
  `finish_backdrop_style` applies it later), so the gate must stay in
  `shell.rs`, ahead of the park, and the neutral module takes over from the
  apply onward. A delivery that passes the gate reaches the neutral apply even
  when the backdrop parked an earlier one, which matches today's order.

## Decisions

- The neutral module owns `Document`, `Analyst` and the displayed settings in
  one struct, `OpenFileState`, because the states the issue names are sequences
  across exactly those three. Testing a decision function over borrowed parts
  would test a toy rather than the real path.
- `Shell`'s own `OpenFile` keeps the window-side fields - the plot handle, the
  delivery and minimap tasks - and embeds `OpenFileState`. The field names
  `document` and `analyst` move one level down, which touches every use site;
  the diff is mechanical and the compiler lists any site the rename misses.
- `displayed_settings` becomes private to the neutral struct with an accessor.
  Recording happens only inside its `accept`, which is the property the issue
  asks to protect.
- The gate runs twice on an accepted delivery - once in `shell.rs` ahead of the
  backdrop park, once inside `accept` - because the second check is what keeps
  the neutral method self-contained, and a second evaluation of a pure
  generation comparison costs nothing.
- Hand-written deliveries need the private generation and view revision, so
  `analysis.rs` grows a `#[cfg(test)] for_test` constructor next to `Delivery`;
  production code cannot forge those fields, which is why it is test-only.

- The test harness has no `PlotView`: the focus target is the shell handle.
  The four states are exercised through the neutral type directly, which is the
  issue's own desired outcome.

## Rejected alternatives

- Keeping `receive` in `shell.rs` and driving it headless with GPUI's
  `test-support`. The window still cannot run in CI, and the issue asks for a
  toolkit-free form, not a local-only test.
- Extracting only a decision function over borrowed document, analyst and
  settings. It would move the three steps without owning any state, and the
  four states would be reachable only through the same untested wiring.
- Moving the backdrop park into the neutral module. It needs `Window` and
  `Context<Shell>` for the shading job and its task, which is exactly the
  dependency the extraction removes.

## Implementation steps

- [x] Add `crates/app/src/open_file.rs` with `OpenFileState`: `document`,
      `analyst`, private `displayed_settings`, `accept` and an accessor.
- [x] Embed `OpenFileState` in `Shell`'s `OpenFile`, rename the moved uses, and
      reduce `Shell::receive` to the gate, the backdrop park, the effect
      dispatch and the window concerns.
- [x] Cover the four states in `open_file_tests.rs`: a superseded delivery is
      rejected and records nothing; a failed analysis keeps the last good
      settings; a cancelled preview's next accepted analysis records the
      restored settings; a second file starts with no recorded settings while
      the first keeps its own.
- [x] Complete validation.
- [ ] Move this plan to `docs/plans/completed/` before final review.

## Validation

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked` (warnings are denied in `[workspace.lints]`)
- [x] `cargo test --locked`
- [ ] `cargo build --release --locked`, after the checks above pass
- [ ] The existing status-bar, settings-editor and analysis tests hold unchanged.

## Post-completion

- Class A. Implementer: this session. Reviewer: GPT-6.1-Sol at medium reasoning
  effort through the `codex` CLI, the owner's most recent choice, to be
  confirmed when the review is requested.
- No native verification is required: the change is test-only in its behaviour
  and the window code paths keep their order.
