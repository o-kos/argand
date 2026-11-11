# Issue #245: Source-only build metadata invalidation

Resolves #245: https://github.com/o-kos/argand/issues/245.

## Overview

Git remote and checkpoint refs currently invalidate the application build script,
causing expensive application recompilation without source changes. Replace Git
watch inputs with application and owned dependency source inputs. Keep compile-time
Git identity reading and embedding unchanged.

## Roles and risk

- Class: A, build cache invalidation and embedded artifact identity.
- Coordinator: `gpt-6-astra` medium.
- Executor: `gpt-6.1-sol` medium.
- Reviewer: `gpt-6.1-sol` high in a separate agent and context.

## Context and decisions

- Use standard Cargo `rerun-if-changed` directory scanning, not generated file lists.
- Track app source/assets/build script and manifest; core/dsp/edit/io source and
  manifests; workspace manifest and lockfile. CLI is not an app dependency.
- Never track Git refs, HEAD, packed refs or a linked-worktree Git marker.
- Metadata is captured when the build script executes. Source changes in the app
  and its owned libraries refresh it. Git-only changes do not invalidate it.
  This does not promise a callback on every compiler invocation.
- Preserve source archive isolation and inherited Git environment sanitization.
- Preserve the unrelated UI-acceptance skill edit. At the owner's direction,
  include this fix in PR #243 together with #240/#241; retain this plan for the
  additional source-only invalidation scope.
- Use the installed toolchain and existing common intermediate build-dir; no
  dependency, feature, profile, cache-layout or toolchain changes.

## Rejected alternatives

- Narrower Git watches still cause rebuilds for metadata-only changes.
- Removing all rerun inputs misses dependency-only metadata refresh: a tiny
  app/path-dependency fixture produced a new binary with old metadata.
- No metadata sidecar, hook, wrapper or always-run mechanism is introduced.

## Acceptance criteria and evidence

| Criterion | Verification | Result | Limits |
| --- | --- | --- | --- |
| Git-only commit leaves app fresh | Dependency-free Cargo fixture | Passed, 0.01s; commit/tag/remote/checkpoint/packed refs changed | Linux installed toolchain |
| App source refreshes snapshot | Same fixture | Passed, 0.09s | Build-script snapshot semantics |
| Owned dependency source refreshes snapshot | Same fixture | Passed, 0.12s, changed dependency output and current snapshot | No external dependencies |
| Git identity reading remains correct | Existing focused metadata tests | 7 passed via direct rustc --test -Dwarnings | No GUI interaction changes |
| No Git rebuild inputs remain | Diff and source-input tests | Passed | No per-compiler-invocation guarantee |

## Implementation steps

- [x] Reproduce default Cargo behavior and validate short source-only inputs.
- [x] Replace Git inputs, retain Git reading and relevant tests.
- [x] Update current architecture documentation.
- [ ] Complete focused and required local validation within approved baseline.
- [ ] Independent review and coordinator assessment.
- [ ] Move plan to completed before final review.

## Validation

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --locked`
- [ ] `cargo test --locked`
- [ ] `cargo build --release --locked`, after required checks
- [x] Dependency-free source-only invalidation fixture.

The owner excludes CI investigation from this task. No server validation is claimed.
Publication/full remote validation is not a prerequisite for the current local
reviewable result; outstanding workflow requirements will be reported explicitly.

## Post-completion

Merge and branch cleanup require the normal accepted-PR workflow.

## Local review snapshot

- Direct installed rustfmt checked all changed Rust files.
- Direct installed rustc compiled the existing metadata test module with
  `--edition=2024 --test -Dwarnings`; all seven tests passed. Package/Git constants
  were supplied explicitly to this standalone harness; this is not a workspace gate.
- The dependency-free Cargo fixture copied the current production build script and
  metadata module, used the existing common build-dir, and checked fresh Git-only
  state plus changed app and path-dependency sources. No external crates compiled.
- Full workspace fmt/Clippy/tests and final release remain unrun for this local
  reviewable result. No hook, proxy installation or server CI was invoked.
- Early Draft publication remains deferred: the ordinary push hook runs the full
  gate. It has not been bypassed or run solely to publish the initial plan.

## Combined branch handoff

At the owner's direction, the implementation branch was merged locally into
`fix/241-validation-artifact-identity` for PR #243. Inclusion of the implementation
commits was verified before deleting the separate local branch. No corresponding
remote branch existed. Issue #245 remains open and is closed through PR #243.

The independent read-only review of the pre-merge implementation snapshot returned
no substantive findings. The exact patch/revision and content digests were recorded
in the external handoff packet. Production and test sources are unchanged after
the merge; the review conclusion therefore covers this source-only addition in the
combined branch. No findings were declined.

Direct release timing after the merge used the installed Cargo/rustc/rustdoc,
existing common build-dir and workspace-local outputs, without hooks, compiler
wrappers, new flags or configuration changes:

- First build: 181.171 seconds; only `argand` was compiled.
- Immediate repeat: 342.808 seconds, contaminated by waiting for the shared build
  lock while the #225 worktree built, then recompiling this checkout's application.
- One final unchanged repeat: 0.647 seconds wall time (Cargo reported 0.60s), with
  no compilation or lock-wait messages.

All three commands succeeded. No external dependency compilation was reported.
These are diagnostic release builds, not the required release-after-full-gate.
The measured source state precedes this documentation-only evidence commit.
The new commits remain local; the ordinary publication hook has not been bypassed.
PR #243 is Draft and describes this pending publication/validation state. No server
CI was inspected for this step. The unrelated skill edit remains uncommitted.
