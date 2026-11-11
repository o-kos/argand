# Issues #241 and #240: Validation artifact identity and explicit probes

Resolves #241. Resolves #240.

## Overview

Preserve the current checkout's executable and fixture identity when Cargo reuses
the common intermediate build directory. Move performance probes out of ordinary
test compilation without changing their measurements. The owner explicitly
approved combining these related validation-workflow tasks in one Pull Request;
their root causes are not assumed to be shared.

## Roles and risk

- Class: A, because cache reuse can make validation exercise an unintended binary.
- Coordinator: `gpt-6-astra` medium.
- Executor: `gpt-6.1-sol` medium.
- Reviewer: `gpt-6.1-sol` high, read-only in a separate context.

## Context and boundaries

CLI integration tests embed `CARGO_BIN_EXE_aspec`; CLI/I/O capture tests embed
`CARGO_MANIFEST_DIR`. Previously reused test executables retained paths from a
removed checkout despite Cargo reporting them fresh. The exact fingerprint cause
requires a bounded reproduction before choosing a fix.

The FFT scheduling and snapshot-cost executables are performance probes rather
than API examples. Ordinary tests currently build them automatically. Explicit
target selection must retain deliberate Clippy/CI compile coverage.

No About edits or dedicated About verification, product UI/DSP changes, toolkit
or dependency upgrades, cache clearing or new linked worktrees are in scope.
The owner is arranging independent #225 UI work in another session.

## Resources and decisions

- Use the installed `1.98.1-x86_64-unknown-linux-gnu` toolchain directly; keep
  pins, defaults and PATH unchanged. Rustup auto-install is disabled.
- Preserve the existing common `/home/xokc/.cache/cargo-target` build-dir and
  workspace-local final `target/` outputs, including reproduction final outputs.
- Preserve the approved debug/test, release and check/Clippy dependency baseline.
  Use the existing external-compilation guard without its optional allowance;
  stop before any dependency rebuild requiring additional owner approval.
- No active Cargo compilation was observed during initial inspection. Coordinate
  common-cache builds with the independent session; do not clear shared artifacts.
- The plan-only commit has no executable changes. Validate its diff/links and
  omit Rust checks for that publication; the full gate applies to code pushes.

## Rejected alternatives

Per-checkout intermediate caches, shared final output directories, deleting the
common cache and repeatedly touching source files do not establish correct reuse.
Do not silently replace optional real-fixture checks with a passing test count.

## Acceptance criteria and evidence

| Criterion | Verification / scenario | Result | Limits |
| --- | --- | --- | --- |
| Bounded cause evidence | Record exact toolchain/configuration/source and artifact identities across checkout/output transitions | Pending | No causal attribution yet |
| Current executable/fixture identity | Inspect embedded paths and exercise intended binary/fixtures with old source present and absent | Pending | Report optional branches explicitly |
| Dependency reuse preserved | Guarded commands using the unchanged common build-dir | Pending | Configuration alone does not prove reuse |
| Probes require intentional selection | Observe ordinary test and explicit bench/Clippy target selection | Pending | Measurements must remain unchanged |
| CI compiles probes deliberately | Review workflow and exercise matching local command | Pending | Remote results reported separately |

## Implementation steps

- [ ] Build a dependency-free minimal reproduction and isolate Cargo freshness inputs.
- [ ] Select and implement a bounded identity correction based on evidence.
- [ ] Exercise current CLI executable and CLI/I/O fixture paths across transitions.
- [ ] Move probes into explicit opt-in targets, preserving implementation and usage.
- [ ] Update intentional CI coverage and relevant development/performance documentation.
- [ ] Complete current-revision validation and independent review.
- [ ] Move this plan to `completed/` before final review.

## Validation

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --locked`
- [ ] `cargo test --locked`
- [ ] `cargo build --release --locked`, last after other checks pass
- [ ] Identity reproduction/regression and probe-target selection checks
- [ ] Independent review and coordinator assessment

## Post-completion

Merge and release actions remain subject to owner authority and current full CI.
