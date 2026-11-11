# Issue #225: Balance FFT summary spacing

Resolves #225.

## Overview

Give the FFT summary equal spacing to its neighboring status-bar separators.
Preserve the standard Button, existing hint and keyboard behavior, and the
independent Range action. Changes are limited to application layout and the
user-facing changelog; no toolkit, dependency or analysis changes are intended.

## Roles and risk

- Class: B, a local geometry correction in one GUI module.
- Coordinator: `gpt-6-astra` medium.
- Executor: `gpt-6.1-sol` medium.
- Reviewer: `gpt-6.1-sol` high, read-only in a separate context.

## Context and decisions

`analysis_control` adds an inter-item gap after the FFT summary while its leading
separator directly precedes a standard Button with symmetric horizontal padding.
Remove that group gap so the existing Button owns both text insets. Retain the
outer status-bar grouping and all standard control identities and behavior.
Do not add pixel offsets, replace controls or change shared Button styling.
No architecture invariant changes.

The owner authorized an isolated linked worktree for this task. Preserve the
concurrent #241/#240 work, the common intermediate cache and local final outputs.
Use the agreed installed Rust 1.98.1 and dependency baseline with the existing
external-compilation guard. Stop before unapproved dependency rebuilding.

## Acceptance criteria and evidence

| Criterion | Verification / scenario | Result | Limits or unperformed checks |
| --- | --- | --- | --- |
| Equal FFT text insets | Measure resting, hovered and pinned summary against both separators in light/dark themes and normal/high display scaling | Pending | Native appearance evidence required |
| Hint and keyboard behavior retained | Left click, Ctrl+,, Escape; compare the neighboring File control | Pending | Native interaction required |
| Independent Range action retained | Click recommended Range and use Ctrl+R; check pending/disabled state | Pending | Requires a suitable capture |
| Layout remains usable | Long FFT/window labels and narrow window | Pending | Native appearance required |

## Implementation steps

- [x] Inspect the current implementation, policy and toolkit behavior.
- [ ] Remove the redundant analysis-group gap and add an Unreleased changelog entry.
- [ ] Complete current-revision local validation and production UI acceptance.
- [ ] Complete independent review and coordinator assessment.
- [ ] Move this plan to `completed/` before final review.

## Validation

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --locked`
- [ ] `cargo test --locked`
- [ ] `cargo build --release --locked`, after the other checks pass
- [ ] Record production binary identity, native observations and remaining limits.

No implementation-mirroring test is planned for this local spacing correction.
Existing interaction coverage, the full local gate and production UI checks remain
required. Unavailable native interaction leaves acceptance incomplete and the PR Draft.

## Post-completion

Merge requires current full CI and owner acceptance. Preserve unrelated work during
normal worktree and branch cleanup after acceptance.
