# Conversion cost measurement outside the test suite

Refs #222 as a narrow prerequisite; broader optimization is outside this change.

## Overview

Move the existing manual million-frame conversion measurement from an ignored unit
test to an explicit Cargo benchmark. Preserve its scenarios, frame-count assertion,
timing output and optional Linux process-memory output. Do not change DSP behavior,
public APIs, dependencies, features or compilation profiles.

## Roles and risk

- Class B: local test/tooling relocation, without DSP algorithm changes.
- Coordinator: `gpt-6-astra` medium.
- Executor: `gpt-6.1-sol` medium.
- Reviewer: `gpt-6.1-sol` high, independent read-only context.
- Initial owner-directed local preparation was later included in the authorized
  test-only phase of PR #237 (Refs #222), retaining independent review and the
  complete local gate. The broader production-module task remains open.

## Decisions

- Use `[[bench]]` named `conversion_cost`, `harness = false`, `test = false`.
- Use only existing public conversion APIs and a benchmark-local metadata constructor.
- Keep the measurement out of ordinary `cargo test`; do not add performance thresholds.
- The intended explicit command is `cargo bench --workspace --bench conversion_cost`.
  Its dependency feature/artifact graph was checked through guarded workspace
  compilation with `--no-run`; only own crates compiled. The measurement has not
  been executed. Any dependency rebuild beyond the approved baseline still requires
  explicit owner approval.
- Preserve existing unrelated policy edits.

## Rejected alternatives

- Keeping a manual measurement as an ignored correctness test obscures test results.
- Adding a benchmark dependency or changing production visibility is unnecessary.
- Selecting only one package can change dependency feature unification and artifacts.

## Acceptance criteria and evidence

| Criterion | Verification | Result | Limits |
| --- | --- | --- | --- |
| No manual conversion measurement in unit tests | Source diff and workspace tests | Observed in source and passing workspace tests | Benchmark not run |
| Three scenarios and output retained | Exact relocated-body comparison | Exact body match after function rename | Benchmark not executed |
| Public API and dependency graph unchanged | Source and manifest review | No production API/dependency changes | Guarded workspace bench compilation passed; measurement not run |
| Explicit benchmark target excluded from ordinary tests | Cargo manifest and workspace tests | Tests report no ignored cases | Final workspace tests pass 1162/0/0 and release passes; independent round two clean; coordinator accepted |

## Implementation steps

- [x] Relocate the measurement and declare its explicit Cargo target.
- [x] Format, inspect the diff and rerun the existing workspace test command.
- [x] Prepare independent review snapshot with validation limits; round one clean.
- [x] Assess inputs with external compiler guard; workspace benchmark compiled without external rebuilds.
- [x] Complete local validation and move the scoped plan to completed for final review.
- [x] Complete current-revision independent review and coordinator assessment.

## Validation

Use the sole installed Rust 1.98.1 and the configured shared intermediate build-dir and workspace-local final outputs. Set RUSTC
and RUSTDOC to its direct executables to avoid dependency-local rustup instructions.

- [x] `cargo fmt --all -- --check`
- [x] `cargo test --locked` (existing workspace debug/test baseline)
- [x] `cargo clippy --all-targets --locked` (owner-approved check/Clippy baseline)
- [x] `cargo build --release --locked` after required checks (final phase release 2m46s)
- [x] `cargo bench --workspace --bench conversion_cost --no-run` with external compilation guard; measurement execution intentionally omitted
- [x] Independent read-only review (rounds one and two clean)

Validation recorded: formatting and workspace tests passed after the relocation;
1162 passed, 0 failed, 0 ignored. The initial relocation recompiled only argand-dsp project code (4.02s);
no dependencies rebuilt. The historical initial release preceded this relocation (7m42s) and is not
benchmark compilation evidence; the later guarded bench compilation is recorded below. Source comparison confirms
the measurement body is unchanged except its function name. A repeated rust-src
message comes from cached thiserror build-script diagnostics; the component remains
absent and no installation was active. No cache data was removed.

Final phase evidence: strict Clippy and workspace tests pass after all test-only
changes. Guarded workspace benchmark compilation passed in 18.84s, recompiling
only own crates; the manual measurement was not executed. Final release passed after the other checks (2m46s, own argand compilation only);
current-revision independent round-two review is clean and coordinator assessment accepted.

The local benchmark-extraction stage is complete. Current-head cross-platform CI
and protected publication/merge remain tracked by the containing test-only phase;
this does not close the broader Issue #222.
