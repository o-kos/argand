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

## Investigation and selected correction

A dependency-free package copied with source bytes and mtimes preserved reproduces
the issue across two temporary source directories. Cargo reports the second fresh;
the reused harness's `env!` paths still identify the first directory and execute its
binary. Cargo's runtime environment correctly identifies the second directory.

The installed Cargo reports 1.98.1 (source revision `797e8a9bc`). Its
[dep-info translation](https://github.com/rust-lang/cargo/blob/797e8a9bc/src/cargo/core/compiler/fingerprint/dep_info.rs#L306-L342)
deliberately excludes environment dependencies synthesized for the rustc command,
except tracked package metadata, configured environment and Cargo itself. Relative
source paths allow reuse across locations. Thus rustc's textual dep-info records
the old manifest/executable values, but Cargo's encoded freshness input omits them.

Use Cargo's exact runtime `CARGO_BIN_EXE_aspec` and `CARGO_MANIFEST_DIR` values in
the affected CLI/I/O tests, through `var_os`. Missing Cargo context fails explicitly;
do not guess a target directory or borrow a source location. The owned source audit
found no other compile-time test uses of these checkout-dependent Cargo variables.
The build script already reads its manifest directory at runtime; About is excluded.

The regression checks a reused harness's SHA256, native paths containing spaces,
workspace/package/test target selection, distinct binary/fixture markers, a live
then removed old source directory, and a final-output-only directory transition.
The compile-time negative control reports failure on the affected Cargo version;
the runtime checks pass. Temporary packages have no dependencies and share the
existing common intermediate directory; no separate cache is created or cleared.

The two probes become explicit `harness = false`, `test = false` benches without
changes to their Rust source bytes. CI's existing all-target Clippy deliberately
checks them; ordinary tests do not select them. No feature graph changes are needed.

## Acceptance criteria and evidence

| Criterion | Verification / scenario | Result | Limits |
| --- | --- | --- | --- |
| Bounded cause evidence | Exact Cargo source, matching temporary sources, reused harness SHA256 and old/new runtime markers | Reproduced; runtime correction passes | Cargo-specific negative control reported separately |
| Current executable/fixture identity | Workspace/package/test selection; old checkout live/deleted; final-output transition | Passed dependency-free regression and full workspace tests | Native OS-string paths with spaces tested; non-Unicode paths not exercised |
| Dependency reuse preserved | Guarded full fmt/Clippy/test gate with unchanged common build-dir | Passed without external compilation | A narrower package selection requested missing feature variants and was refused before compilation |
| Probes require intentional selection | Ordinary test no-run JSON and guarded all-target Clippy records | No bench/example in ordinary tests; both benches checked | Probe source bytes preserved; timings not rerun |
| CI compiles probes deliberately | Existing all-target Clippy plus explicit regression/hook checks | Matching local checks passed | Remote CI pending |

## Implementation steps

- [x] Build a dependency-free minimal reproduction and isolate Cargo freshness inputs.
- [x] Select and implement a bounded identity correction based on evidence.
- [x] Exercise current CLI executable and CLI/I/O fixture paths across transitions.
- [x] Move probes into explicit opt-in targets, preserving implementation and usage.
- [x] Update intentional CI coverage and relevant development/performance documentation.
- [x] ➕ Preserve explicit direct Cargo/formatter/Clippy tools in the pre-push hook,
  without changing its gate or PATH. This resolves a publication blocker: Cargo
  subcommand discovery reached a rustup proxy despite direct Cargo/Rustc selection.
- [ ] Complete current-revision validation and independent review.
- [ ] Move this plan to `completed/` before final review.

## Validation

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked`
- [x] `cargo test --locked`
- [ ] `cargo build --release --locked`, last after other checks pass
- [x] Identity reproduction/regression and probe-target selection checks
- [ ] Independent review and coordinator assessment

## Post-completion

Merge and release actions remain subject to owner authority and current full CI.

## Validation record and limits

The genuine pre-push gate passes with direct Cargo/formatter/Clippy/Rust tools and
the external-compilation guard: 883 tests (514 app, 50 CLI unit, 24 CLI integration,
36 core, 97 DSP, 12 editing, 150 I/O), zero failed/ignored. Execve tracing confirms
no rustup or Cargo/Rust proxy invocations in this gate. Ordinary no-run discovery
has no benchmark/example artifact; guard records confirm both relocated bench
sources were checked. Their source bytes are unchanged.

Current fixture inventory contains all five CLI matrix files, the half-hour
capture and six repository captures. The corresponding full-gate tests pass;
their successful captured stdout was not separately expanded. The runtime-path
regression independently checks distinct current binary and fixture markers.

A follow-up `-p argand-cli` command narrowed feature unification and requested
uncached external dependency variants; the guard refused them before compilation.
No permission to rebuild them was used. Optional repeated filtered testing was
not required to establish the full-workspace gate result.

The early plan-only push was blocked by automatic approval review over hook
omission. Publication is deferred until the genuine code gate and release build;
the hook is now capable of using the exact installed tools without bypasses.
An earlier direct-Cargo invocation discovered a subcommand rustup proxy and logged
an attempted rust-src download. No rust-src directory/component was installed;
the installed-component manifest retained its earlier timestamp. Subsequent
commands use direct subcommand executables and preserve PATH/toolchain settings.
