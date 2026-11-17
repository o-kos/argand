# Issue #259: Linux Display Stack

Resolves #259 (https://github.com/o-kos/argand/issues/259).

## Overview

Keep the Linux host display libraries coherent with its graphics drivers instead of placing older display libraries ahead of them in the application launcher.

The owner requested Issues #259 and #242 together in one separate linked worktree, based on the main checkout's local branch. No merge or release is authorized by this implementation task.

## Roles and risk

- Class: A, release authority and cross-host runtime safety.
- Coordinator: `gpt-6.1-sol` medium, owner communication and final assessment.
- Executor: `gpt-6.1-sol` medium, investigation, implementation, validation and PR operations.
- Reviewer: `gpt-6.1-sol` high, fresh read-only context.
- Consultant: `gpt-6-astra` medium only if a concrete uncertainty requires it.

## Context and decisions

- Preserve the approved Rust/toolkit graph and immutable installed SDK. All Cargo artifacts stay in this worktree's target directory.
- Preserve main branch protection and Release tags ruleset. No secrets, new bypass actors, force tag updates or production release during development.
- Existing authenticated administrator git push triggers the tag workflow. A GITHUB_TOKEN-generated push is not a supported handoff.

## Acceptance criteria and evidence

| Criterion | Verification | Result | Limits |
| --- | --- | --- | --- |
| Host display libraries excluded throughout closure | Ten packaging tests, including synthetic transitive SONAME closure and real packaged ELF | Passed | Host GPU runtime is separate |
| Native notices and other bundled libraries retained | Real archive closure/notices regression | Passed | CI baseline packaging still pending |
| Vulkan ICD instance/device/desktop extensions work under launcher environment | probe-linux-backend.py on extracted archive | Pending | Probe does not create a rendering surface |
| Baseline Ubuntu 24.04 production GUI opens capture | Existing Ubuntu24 image identified with Wayland1.22/Mesa24 | Pending | Desktop runtime not exercised yet |
| Newer Wayland/Mesa production GUI opens capture | Ubuntu26.04 host with ICDs and isolated Xvfb available | Pending | Startup/real frame evidence not exercised yet |
| Existing application behavior | fmt, Clippy, workspace tests | Passed | Local SDK gate, portable build separate |

## Implementation steps

- [x] Inspect native dependencies and define the host display-stack boundary with explicit baseline requirements.
- [x] Exclude host display-stack libraries from every transitive bundle closure and retain native notices for bundled libraries.
- [x] Add regression checks and a real Vulkan loader/device probe using the packaged library environment.
- [ ] Document and record baseline/newer-host backend evidence and limitations.

- [ ] Complete independent review and relevant validation.
- [ ] Move this plan to completed before final review when all acceptance evidence exists.

## Owner-approved validation cleanup

The owner expanded this same PR to fix the analyzer/thread accumulation observed
in the existing GUI test harness, without a new Issue. Installed GPUI Kit's test
macro delegates teardown to App::quit on a no-op test platform. Its genuine
TestAppContext::quit cleanup is not called, leaving raw VisualTestContext owners
and document readers retained. This is a verified static retention path consistent
with the observed analyzer/pool multiplicities, not per-instance heap attribution.

The repository correction is test-only: one shared RAII cleanup scope, created
before each GPUI test's other locals, calls the existing genuine test-context quit
API after those locals drop. It also runs during unwinding, preserves the original
panic, and propagates cleanup failures on otherwise successful tests. No registry,
immutable SDK, dependency graph, production worker policy or test assertions change.

- [ ] Add shared test-context cleanup and first-local guards to all app GPUI tests.
- [ ] Prove real Shell/capture ownership and reader release across repeated normal and panic scopes.
- [ ] Observe a bounded serial subset returning analyzer/pool threads to baseline before any full gate.
- [ ] Repeat full gate and independent review only after focused lifecycle evidence passes.

## Validation

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked`
- [x] `cargo test --locked`
- [ ] `cargo build --release --locked`, last
- [ ] Change-specific policy and integration checks.

## Post-completion

Only the owner authorizes merge and production release. Baseline and newer-host runtime evidence are unfinished implementation acceptance work, not post-merge substitutions.
