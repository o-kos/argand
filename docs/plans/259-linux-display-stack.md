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
| Issue acceptance criteria | Targeted policy/integration tests and exact-revision review | Pending | Native desktop and real remote publication evidence are separate from fixtures |
| Existing application behavior | Full local gate | Pending | Local SDK mode is not portable source packaging |

## Implementation steps

- [ ] Inspect native dependencies and define the host display-stack boundary with explicit baseline requirements.
- [ ] Exclude host display-stack libraries from every transitive bundle closure and retain native notices for bundled libraries.
- [ ] Add regression checks and a real Vulkan loader/device probe using the packaged library environment.
- [ ] Document and record baseline/newer-host backend evidence and limitations.

- [ ] Complete independent review and relevant validation.
- [ ] Move this plan to completed before final review when all acceptance evidence exists.

## Validation

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --locked`
- [ ] `cargo test --locked`
- [ ] `cargo build --release --locked`, last
- [ ] Change-specific policy and integration checks.

## Post-completion

Only the owner authorizes merge and production release. After merge, verify the installed workflow's handoff under administrator authority. Native desktop evidence unavailable during implementation must remain an explicit acceptance limit.
