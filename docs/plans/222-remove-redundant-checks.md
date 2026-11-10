# Issue #222: Consolidate four proven duplicate test cases

Refs [#222](https://github.com/o-kos/argand/issues/222). This owner-authorized second test-only phase follows PR #237. It leaves the broader Issue open; production module organization remains a separate discussion.

## Overview and roles

Consolidate four source-proven duplicate fixture/scenario checks while retaining every unique assertion and risk boundary. The expected 1162 to 1158 test count follows these changes; it is not an optimization target. No production module, API, behavior, dependency, feature, profile or fixture changes are authorized.

- Class A: DSP correctness and cross-subsystem coverage require independent scrutiny despite four small test-file edits.
- Coordinator: `gpt-6-astra` medium.
- Executor: `gpt-6.1-sol` medium.
- Reviewer: `gpt-6.1-sol` high, fresh independent read-only context for this PR, at most three rounds.

## Evidence and decisions

Reuse the prior coverage/cost audit and the focused read-only redundancy investigation; do not repeat a broad audit or infer usefulness from test count.

1. In `crates/app/src/spectrogram_tests.rs`, remove `every_pixel_keeps_the_alpha_the_shading_step_gave_it`. The preceding `the_upload_reads_as_bgra_because_that_is_what_the_shader_reads` uses the identical `two_pixels()` input and Horizontal mode and asserts every output byte, including both alpha bytes of 255. Keep that exact byte assertion, channel-order distinction, real-transform path and vertical/padding/boundary tests. A short comment can make the preserved opacity assertion explicit.
2. In `crates/dsp/src/window_tests.rs`, combine the two 4096-point textbook tests into `coherent_gain_and_enbw_match_the_textbook_values`, constructing each of four WindowTables once (8 to 4 constructions). Preserve Hann gain0.5/ENBW1.5, Hamming0.54/1.3628, BlackmanHarris0.35875/2.0044, Rect1.0/1.0; gain tolerance1e-3 and ENBW tolerance1e-2 remain unchanged. Preserve periodicity, zero-length, rectangular and physical-hertz scaling tests separately.
3. In `crates/core/src/colormap_tests.rs`, combine gradient stops and end-to-end brightness into `gradient_ends_land_on_the_declared_stops_and_get_brighter`. Construct all six map gradients once (12 to 6), retaining every start/end channel comparison (distance at most1) and strict RGB-sum increase. Brightness is not inferred from endpoint matching; retain its explicit assertion. Other color/ink/clamp contracts stay separate.
4. In `crates/cli/src/main_tests.rs`, combine `the_two_streams_are_decided_independently` into the existing two-case JSON/quiet matrix, named `json_outranks_quiet_on_stdout_and_respects_it_on_stderr`. Parse each original case once. Preserve explicit JSON stdout for both quiet states and the nonquiet default stderr block; assert quiet stderr Nothing in the same matrix, with no redundant parsing. Retain separate path/terminal/batch/verbose tests.

No recent/session, numerical endpoint, concurrency, lifetime, persistence, I/O, real-capture or GUI dispatch tests are consolidated by these decisions. No tolerance, seed, numerical sample input, platform gate, ignore or subprocess behavior changes.

## Exact discovery delta

Remove these seven old names and add only the three combined names above:

- `spectrogram::tests::every_pixel_keeps_the_alpha_the_shading_step_gave_it`
- `window::tests::coherent_gain_matches_the_textbook_values`
- `window::tests::enbw_matches_the_textbook_values`
- `colormap::tests::gradient_ends_land_on_the_declared_stops`
- `colormap::tests::gradient_gets_brighter_from_start_to_end`
- `tests::json_outranks_quiet_on_stdout`
- `tests::the_two_streams_are_decided_independently`

Check actual executable-qualified package/target names against the recorded 1162-name baseline; the root module prefix must be verified rather than assumed from this readable list. All other discovered names and zero ignored cases must match exactly. Expected net change is four fewer tests, not seven lost contracts.

## Rejected alternatives

An arbitrary test-count reduction; removing unique assertions because another test sounds similar; inferring independent brightness from matched endpoints; modifying production code or module layout; consolidating distinct recent/session paths; lowering numerical tolerances, shortening concurrency/data-safety scenarios or installing new test tools. No percentage speedup is claimed; the reduced constructions and duplicate parse are directly countable.

## Acceptance and evidence

| Criterion | Verification | Result / limit |
| --- | --- | --- |
| Every original unique assertion/input survives | Before/after source mapping and independent numerical/coverage review | Pending |
| Exactly four test-file changes, no production/manifest/lock changes | Exact diff and source review | Pending |
| Only the seven-to-three discovery mapping changes; zero ignored | Compare actual executable discovery against baseline | Pending; count alone is insufficient |
| Correctness and required gate remain green | fmt, strict Clippy, workspace tests, final release | Pending; optional external capture presence is not assumed |
| Independent review accepts exact final snapshot | Fresh read-only reviewer and coordinator assessment | Pending |

## Implementation steps

- [x] Inspect the four exact source seams and record approved coverage reasoning.
- [ ] Commit this plan and open the early Draft PR before code edits.
- [ ] Apply only the four accepted consolidations with retained assertions and inputs.
- [ ] Compare exact discovery and run the complete required local gate.
- [ ] Update evidence and move this plan to completed before independent review.
- [ ] Resolve review findings within three rounds; coordinator assesses preserved invariants.

## Validation

Retain the sole installed Rust1.98.1, agreed toolkit/locked graph, shared intermediate build-dir and local final outputs. Use the established direct compiler/Cargo selection and external compilation guard. Ordinary changed own-crate compilation is allowed within the approved debug/test, release and check/Clippy baseline. Stop on any external rebuild beyond that baseline; do not change flags/features/profiles to make a check pass. No Clippy launcher or shared-policy work is part of this phase.

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --locked`
- [ ] `cargo test --locked` (expected1158 passed, zero failed/ignored)
- [ ] Compare actual per-executable names to the preserved1162-name baseline with the exact allowlist above.
- [ ] `cargo build --release --locked` last after the other checks pass.
- [ ] Independent current-snapshot review and coordinator assessment.

A plan-only initial push may use the coordinator-approved scoped non-code hook exception: no code is included and the unconditional hook would repeat the already-established gate. Code publication must retain the full gate and unchanged hook. Ready requests current-head Linux/Windows/macOS and ci/full; no local result substitutes for these checks. No native UI acceptance or benchmark execution is required for unchanged production behavior.

## Post-completion

After accepted current-head full CI and protected merge, leave #222 open for further owner-directed coverage audit and production-layout discussion. Preserve existing stashes, external task notes and unrelated worktrees. Accepted PR237 branch refs remain retained because automatic approval review rejected remote deletion; do not retry or bypass that rejection without explicit authorization.
