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

- [x] Verify alias/mode selection with a dependency-free bounded preflight.
- [x] Add standalone locked SDK producer and explicit immutable installer.
- [x] Verify installed compilation/runtime closure without temporary/cache paths.
- [x] Add local/source mode dependencies, machine selection and clear validation.
- [x] Document setup, compiler/toolkit identity, static mode and limitations.
- [x] Validate ordinary workspace commands and actual captures on current code.
- [x] Record isolated production-app rendering and independent review (final round clean).
- [x] Commit the reviewed result without push; publication remains out of scope.

## Acceptance criteria and evidence

| Criterion | Verification | Result | Limits |
| --- | --- | --- | --- |
| SDK compilation is separate | Five dependency-free Cargo validation cases and ordinary app build | Passed | Linux local mode |
| Installed SDK is self-contained | Real macro/image consumer compiled and ran with producer/cache/tmp hidden | Passed | Matching installed compiler and OS libraries |
| Ordinary local Cargo works | Build, 883 workspace tests, all-target Clippy | Passed | No wrappers/hooks |
| Source mode remains portable | All-target source check without local SDK configuration | Passed; final static release pending | Server CI/other platforms not executed |
| Capture/UI behavior remains intact | Original capture inputs present in full tests; isolated Xvfb signal and About interaction | Passed | No native Wayland/other OS claim |

## Validation

- [x] Five installer safety regressions and five mode/missing SDK Cargo cases.
- [x] cargo fmt --all -- --check.
- [x] cargo clippy --all-targets --locked.
- [x] cargo test --locked, including original real-capture fixtures (883 passed).
- [ ] Appropriate final release validation after preceding checks; source graph
  rebuilds beyond approved experiment require a concrete resource assessment.
- [x] Independent review and coordinator assessment of exact snapshot.

## Post-completion

Push/PR lifecycle and eventual integration into main require the owner's separate
publication direction. No merge or release is authorized by this implementation.

## Review corrections

Accepted review findings: preflight installed Rust before proxy invocation, including
hardlinked proxies; track SDK removal for Cargo freshness; validate immutable build
identity and approved lock graph; package only explicitly linked native build
outputs. An initial task-created installation mistakenly included system-library
search-directory contents. It was replaced by a corrected closure installation and
removed after verifying no active process/config/final-binary references. No shared
cache or other SDK version was removed. The final independent review was clean after three rounds; all findings were accepted and resolved.

## SDK workspace release correction

The owner requested ordinary workspace release builds with the SDK selected. The
CLI had consumed the SDK debug image rlib, which lacks LLVM bitcode required by
Thin LTO. Argand itself can consume the SDK dylib unchanged. Keep CLI image as an
always-source dependency under its distinct Cargo alias; it shares no GPUI image
types. Remove the now-unneeded CLI SDK validation build script. Preserve the
release profile, toolkit graph, installed SDK and common intermediate directory.

- [x] Make CLI and integration-test image aliases unconditional.
- [x] Verify 74 CLI tests, workspace all-target Clippy, CLI all-features and
  CLI default all-target check without SDK configuration.
- [x] Independently review the focused correction and commit without push.
- [ ] Run ordinary workspace release as the final build; verify CLI has no SDK
  runtime dependency and app still consumes the selected installed SDK.

The focused release correction received a clean independent review. The final
normal workspace release is run after this correction is committed; its result
and binary provenance are recorded in the task handoff rather than another
source-changing commit. The installed SDK is not rebuilt.

### Release import-resolution follow-up

The initial always-source alias alone did not prevent the compiler from loading
the injected SDK image during nested `use image::...` resolution. Exact compiler
LTO logging identified the SDK archive object as the missing-bitcode input. A
dependency-free control reproduced the failure with a nested use declaration;
`use crate::image::...` passed under the same Thin LTO inputs. Both CLI module
imports now explicitly name the source image alias. No SDK/profile changes.
