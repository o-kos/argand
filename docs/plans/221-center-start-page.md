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
| Whole group centered on both axes | Production bounds regression and Xvfb rendering with short/mixed names | Passed: production geometry regression and Xvfb scenarios | Native Wayland, Windows and macOS not available locally |
| Common internal alignment retained | Compare heading/or and filename/chooser coordinates | Passed: shared edges preserved | Pixel rounding tolerance: one rendered pixel |
| Long rows remain bounded and middle-truncated | Narrow window and long/disambiguated labels | Passed: production narrow-window rendering and bounds regression | Keep extension and recovery tooltip |
| Opening and keyboard paths unchanged | Recent click, Alt+digit, chooser and Ctrl+O, cancel and return | Passed: click, Alt+0, chooser, Ctrl+O, Enter and Escape | Production X11/Xvfb evidence separate from headless tests |
| Empty and full histories remain usable | Zero/one/ten rows, small height, scroll, both themes | Passed: empty/full history, both themes, scroll and 125% display scale | No new state or interaction machinery |

## Implementation steps

- [x] Inspect current layout, standard controls and existing contracts.
- [x] Compose one centered content-sized group with bounded list overflow.
- [x] Add a focused production layout regression where useful.
- [x] Update UI contract and changelog.
- [x] Exercise the current production binary on isolated Xvfb with disposable data.
- [x] Complete local validation and independent review; record findings/dispositions.
- [ ] Move this plan to `docs/plans/completed/` before final review.

## Validation

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked`
- [x] `cargo test --locked`
- [x] `cargo build --release --locked`, after the preceding checks
- [x] Production UI scenarios above and independent review

## Post-completion

After owner acceptance and current full CI, merge through the protected PR and
remove the accepted worktree/branch only after the prescribed data-safety checks.

## Results and owner feedback

Implementation and the full local gate passed. See the
[acceptance record](../ui/221-start-page-acceptance.md) for production scenarios,
source/binary identity and platform limits. Independent review rounds 1 and 2 were clean;
there were no substantive findings or rejected findings. Round 2 verified the full
gate, source/binary identity, production screenshots and the separate routing of
the tooltip observation. The separate inherited tooltip observation is
tracked in [#253](https://github.com/o-kos/argand/issues/253).

The PR stays Draft for owner feedback. Move this plan to completed and request
full CI only when the agreed implementation is ready for final review under the
normal PR lifecycle; no release or merge is performed by this preparation.

## Accepted integration and publication

On 2026-11-13, the owner accepted the start-page result and requested autonomous
publication and merge while required checks remain green. At the owner's request,
#254's accepted native SDK layout was integrated into this worktree and its extra
worktree removed. The combined publication is Class A because it includes the SDK
build integration; the retained reviewer assesses the final integration.

PR #252 will close #221 and #254; PR #255 is superseded. Merge #185 first, then
integrate current main here and refresh the affected gate, release and review
before requesting full platform CI. Do not publish or merge an intermediate #185
state while its independently requested corrections remain in progress.

The owner's shared session was already version 12 from #185; this pre-integration
binary understands version 11 and deliberately declines reading/writing newer
sessions. Owner verification used a separate compatible copy. Its real Recent list
rendered in the current release on isolated Xvfb, and hashing confirmed the shared
session was unchanged. Integrating #185 must restore ordinary shared-session use;
do not weaken the session version guard or downgrade the owner's file.
