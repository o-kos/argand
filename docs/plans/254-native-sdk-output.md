# Issue #254: Native Cargo output paths with the installed SDK

Resolves #254: https://github.com/o-kos/argand/issues/254.

## Overview

Remove the SDK selection's explicit build target and global SDK path/extern flags. Normal
native builds must use target/debug and target/release with the same installed
immutable SDK. Preserve portable source mode, toolchain/graph checks, exact image
types, host compilation isolation and CLI independence.

## Roles and risk

- Class: A; build integration across six Rust/Python code/test files, including
  crate resolution and runtime search paths.
- Primary: `gpt-6.1-sol` medium.
- Consultant: `gpt-6-astra` medium; targeted investigation of the reproduced
  StableCrateId collision and additive rustc metadata proof.
- Reviewer: `gpt-6.1-sol` high, fresh separate read-only context.

## Context and decisions

SDK select currently injects extern/search/rpath flags through machine-local
configuration. The application already has a build script and SDK validator.
Import the SDK under its native crate name (gpui_kit_sdk), alias it at the app
crate root, and import its exact image crate. The app build script will emit
package-scoped Cargo library search/rpath directives. SDK selection will only set
its installation path in the environment and a fixed consumer metadata namespace.
Full workspace validation exposed colliding StableCrateIds between separately built
rustversion proc macros in the SDK and native consumer graph. A single appended
-Cmetadata namespace preserves Cargo's per-crate disambiguators while separating
consumer identities from SDK identities, including host tools. It carries no SDK
extern, search or linker flags into host compilation. A direct Rust probe already linked and
ran a GPUI RenderImage from the installed image::Frame without --target/--extern.

Use the installed Rust 1.98.1 and agreed toolkit 0.9.1 graph. No wrapper, new
dependency, SDK rebuild, shared Cargo directory or deletion. The owner authorized
the output-layout change and any ordinary dependency compilation it requires.
Existing uncommitted owner-policy edits are preserved outside this task's commits.
Plan-only preparation uses diff/link checks and may omit the Rust pre-push hook;
all code pushes require the complete local gate.

## Rejected alternatives

- Removing build.target while retaining global SDK flags would expose those flags
  to host build scripts/procedural macros.
- Compiler wrappers or output relocation introduce unnecessary machinery.
- Profile/debug-assertion tweaks or private Cargo environment knobs would hide the
  identity problem behind unrelated settings; use rustc metadata disambiguation.
- Rebuilding the SDK or selecting another dependency graph is unnecessary.

## Acceptance criteria and evidence

| Criterion | Verification | Result / limits |
| --- | --- | --- |
| Ordinary native output paths | Cargo JSON paths and actual debug/release binaries | Passed |
| No explicit target or global SDK path/extern flags | Config generation test and actual selection | Passed |
| SDK parameters scoped to app | Verbose compilation and bounded host fixture | Passed |
| Exact image identity and runtime link | RenderImage boundary, app builds/tests, runtime dependencies | Passed: shared image probe, full native gate and runtime smoke checks |
| SDK validation and source mode retained | Bounded integration/selection regressions | Passed: bounded source-mode/validation fixtures; toolkit was not rebuilt |
| Installed SDK unchanged | Installation identity and hashes | Passed |

## Implementation steps

- [x] Emit package-scoped SDK search/rpath directives from the existing app build script.
- [x] Use native SDK imports and native SDK selection with a documented consumer metadata namespace.
- [x] Add bounded layout, host-isolation, missing-SDK and source-mode tests.
- [x] Update current setup/architecture documentation and executable paths.
- [x] Complete local gate, app/CLI smoke checks and independent review.

## Validation

- [x] SDK Python unit/integration regressions
- [x] cargo fmt --all -- --check
- [x] cargo clippy --all-targets --locked
- [x] cargo test --locked
- [x] cargo build --release --locked after preceding checks
- [x] Native cargo build / cargo run --locked -p argand -- --help and CLI smoke checks

## Final review and post-completion

Keep Draft for owner feedback; move this plan to completed before final Ready/full
CI under the normal lifecycle. After acceptance/full CI, merge through the protected
PR. Preserve old target-triple artifacts used by other tasks.

## Results

See [native SDK validation](../development/254-native-sdk-validation.md). Native
Clippy, 898 tests, debug/release builds, cargo run help and real Xvfb startup passed.
SDK verification confirmed the same installation. Round 1 independent review was
clean; its factual plan correction about the consultant assignment was accepted.
Final evidence review follows before presentation. Full source-toolkit rebuilding
and other native platforms were not run locally.
