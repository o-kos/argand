# Issue #248: Separately installed GPUI Kit SDK

Resolves #248: https://github.com/o-kos/argand/issues/248.

## Overview

Separate the explicit Linux development SDK build/install from Argand compilation.
Local ordinary Cargo commands consume an immutable ready SDK; portable source mode
retains the approved toolkit graph for release and CI. Wireroom adoption is deferred.
No automatic SDK build, toolchain changes, registry upgrades, compiler wrappers,
Git hooks, cache clearing, remote validation or publication are part of this task.

## Roles and risk

- Class A: compiled artifact identity, dependency graph and shared build cache.
- Coordinator: gpt-6-astra medium.
- Executor: gpt-6.1-sol medium.
- Reviewer: gpt-6.1-sol high, separate read-only context.

## Decisions and boundaries

- A standalone tools/gpui-sdk workspace builds exactly the approved toolkit with
  test support; it is never an application dependency or automatic build step.
- Publish an immutable versioned user-local SDK directory, including runtime and
  compilation artifact closure; derive identities from actual compiler, lock and
  artifact hashes rather than guessing Cargo fingerprint names.
- Target-scoped machine-local Cargo flags and an explicit native target separate
  host proc macros from SDK inputs. Source dependencies use explicit aliases and
  source-mode features; preflight alias precedence before adopting it.
- Missing/incompatible SDK fails clearly without rebuilding it. A source mode does
  not require an installed SDK and supports portable CI/static releases.
- Preserve common intermediates and workspace-local final outputs. No historical
  timing claims substitute for measurements of current implementation.
- Base main retains older test-path and metadata behavior; do not reimplement
  independent changes from PR243 in this SDK task.

## Implementation steps

- [ ] Verify alias/mode selection with a dependency-free bounded preflight.
- [ ] Add standalone locked SDK producer and explicit immutable installer.
- [ ] Verify installed compilation/runtime closure without temporary/cache paths.
- [ ] Add local/source mode dependencies, machine selection and clear validation.
- [ ] Document setup, compiler/toolkit identity, static mode and limitations.
- [ ] Validate ordinary workspace commands and actual captures on current code.
- [ ] Record isolated production-app rendering and independent review.
- [ ] Commit the reviewed result without push; publication remains out of scope.

## Acceptance criteria and evidence

| Criterion | Verification | Result | Limits |
| --- | --- | --- | --- |
| SDK compilation is separate | Missing SDK and normal command tests | Pending | Linux local mode |
| Installed SDK is self-contained | Hide producer/cache during a consumer compile/run | Pending | Matching installed compiler and OS libraries |
| Ordinary local Cargo works | Build, tests, all-target Clippy | Pending | No wrappers/hooks |
| Source mode remains portable | Manifest/selection checks and appropriate local check | Pending | Server CI not executed |
| Capture/UI behavior remains intact | Real-capture tests and isolated Xvfb app | Pending | No native Wayland/other OS claim |

## Validation

- [ ] Installer/mode/missing SDK regression checks.
- [ ] cargo fmt --all -- --check.
- [ ] cargo clippy --all-targets --locked.
- [ ] cargo test --locked, including original real-capture fixtures.
- [ ] Appropriate final release validation after preceding checks; source graph
  rebuilds beyond approved experiment require a concrete resource assessment.
- [ ] Independent review and coordinator assessment of exact snapshot.

## Post-completion

Push/PR lifecycle and eventual integration into main require the owner's separate
publication direction. No merge or release is authorized by this implementation.
