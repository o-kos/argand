# Issue #221: Center the start-page recent files block

Resolves #221: https://github.com/o-kos/argand/issues/221.

## Overview

Center the complete visible start-page group horizontally and vertically in the
available content area. Preserve the existing number/name columns, heading/or
alignment, chooser/name alignment, middle truncation, scrolling and opening actions.

## Roles and risk

- Class: B; local layout correction in Shell, with no capture or processing changes.
- Primary: `gpt-6.1-sol` medium.
- Consultant: none unless a concrete unresolved question warrants consultation.
- Reviewer: `gpt-6.1-sol` high, read-only in a fresh separate context.

## Context

Shell currently centers a fixed-width column while its short rows occupy only the
leading portion, and only centers the empty-history page vertically. Standard
Button controls already own all interaction. The list owns overflow.

## Decisions

- Compose the existing standard controls in one content-sized, bounded flex column.
- Keep the existing width limit and shared number-column inset.
- Use the installed Rust 1.98.1 and selected immutable GPUI Kit SDK; local target only.
- The owner also requested removal of separate dependency-compilation approval.
  Synchronize the active policy documents; retain toolchain/toolkit restrictions.
- Plan/policy-only preparation uses diff/link checks, without Rust compilation or
  external review. The initial documentation-only push may omit the pre-push Rust
  hook; every code push requires the complete local gate.

## Rejected alternatives

- Centering only each row would destroy the common number/name columns.
- Manual offsets or retained measurement state would duplicate ordinary flex layout.
- A replacement button or list widget is unnecessary.

## Acceptance criteria and evidence

| Criterion | Verification / scenario | Result | Limits or unperformed checks |
| --- | --- | --- | --- |
| Whole group centered on both axes | Production bounds regression and Xvfb rendering with short/mixed names | Pending | Native Wayland, Windows and macOS not available locally |
| Common internal alignment retained | Compare heading/or and filename/chooser coordinates | Pending | Pixel rounding tolerance: one rendered pixel |
| Long rows remain bounded and middle-truncated | Narrow window and long/disambiguated labels | Pending | Keep extension and recovery tooltip |
| Opening and keyboard paths unchanged | Recent click, Alt+digit, chooser and Ctrl+O, cancel and return | Pending | Production X11/Xvfb evidence separate from headless tests |
| Empty and full histories remain usable | Zero/one/ten rows, small height, scroll, both themes | Pending | No new state or interaction machinery |

## Implementation steps

- [x] Inspect current layout, standard controls and existing contracts.
- [ ] Compose one centered content-sized group with bounded list overflow.
- [ ] Add a focused production layout regression where useful.
- [ ] Update UI contract and changelog.
- [ ] Exercise the current production binary on isolated Xvfb with disposable data.
- [ ] Complete local validation and independent review; record findings/dispositions.
- [ ] Move this plan to `docs/plans/completed/` before final review.

## Validation

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --locked`
- [ ] `cargo test --locked`
- [ ] `cargo build --release --locked`, after the preceding checks
- [ ] Production UI scenarios above and independent review

## Post-completion

After owner acceptance and current full CI, merge through the protected PR and
remove the accepted worktree/branch only after the prescribed data-safety checks.
