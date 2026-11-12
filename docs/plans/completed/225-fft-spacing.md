# Issue #225: Unify status control spacing

Resolves #225.

## Overview

Use one placement scheme for File, FFT and Range status items. The owner clarified
that highlighted button surfaces need equal external space to their separators,
in addition to equal FFT text insets. The earlier text-only correction did not
establish that common placement and is superseded by this scoped follow-up.
Preserve standard controls, IDs, pointer/focus/keyboard behavior and independent
Range actions. No dependency, toolkit or analysis change is intended.

## Roles and risk

- Class: B, local layout in one GUI module.
- Coordinator: `gpt-6-astra` medium.
- Executor: `gpt-6.1-sol` medium.
- Reviewer: `gpt-6.1-sol` high, read-only in a separate context.

## Context and decisions

The owner rejected the first common scheme's 8px horizontal outer gaps as too
loose relative to the unchanged vertical inset. Use a compact File/FFT/Range group
with the toolkit's `gap_0p5` and shared separator `pl_0p5` (2px at base scale).
Give the first File control the same leading inset through loaded-row `pl_0p5`.
Preserve the surrounding status row's other gaps, trailing padding and no-file
layout. Preserve Button `px_2`, `h_5` and row `h_6`; no custom control or pixel
nudge. Verify actual horizontal/vertical proportions from rendered surfaces,
not equality alone. Optional file-summary delivery must not hide FFT/Range while
metadata is loading.

Retain the standard actionable Range Button and nonactionable text container;
retain hover/pinned hints, focus wrappers, tooltips and action entry points.
No custom widget, optical pixel offset or architecture invariant change is needed.

The owner visually accepted the compact layout and initially requested a local
commit without push, then authorized publication, full CI, merge and safe cleanup.
Preserve the local UI skill
clarification and concurrent #241/#240 work. Use the agreed direct Rust 1.98.1,
shared intermediate cache and workspace-local final outputs. The dependency guard
must refuse unapproved external compilation. UI launches use verified isolated
Xvfb/X11 only, with child display/backend and config/session/cache isolation.

## Acceptance criteria and evidence

| Criterion | Verification / scenario | Result | Limits |
| --- | --- | --- | --- |
| Common placement | Measure File/FFT/Range highlight-to-separator space and vertical alignment, both themes at1x/2x | Observed2px at1x,4px at2x in both themes | Raw colored-surface bounds; vertical proportions separately measured |
| FFT text remains balanced | Measure resting/hovered/pinned FFT insets | Observed equal ink insets11/11px and22/22px | Glyph ink measurement, not direct content-box bounds |
| Existing interactions | Open File and FFT by pointer; Ctrl+,, Escape and shared hint transitions | File/FFT pointer panels, shared hover, Ctrl+, and Escape exercised | Enter follows focused standard-control behavior |
| Independent Range | Available/unavailable action, click/Ctrl+R and pending/busy when reproducible | Available click/Ctrl+R and inactive no-action observed | Transient pending/conversion-busy not reproduced |
| Constrained layout | Long FFT/window label and narrow window | Long label and800x600 in both1x themes observed | Xvfb does not establish Wayland/platform behavior |

## Implementation steps

- [x] Inspect the owner screenshots, current controls and locked Button composition.
- [x] Apply the compact shared group/insets and synchronize documentation.
- [x] Complete current-code local gate, release last.
- [x] Exercise actual production UI in verified isolated Xvfb; record unperformed transient cases.
- [x] Complete independent review and coordinator assessment for this revision.
- [x] Move this accepted plan to `completed/` for final publication.

## Validation

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked`
- [x] `cargo test --locked`
- [x] `cargo build --release --locked`, after other checks pass
- [x] Current production binary and exact local-diff identity recorded
- [x] Measured common geometry and actual pointer/keyboard acceptance evidence

The earlier full gate and clean review covered the text-only patch and do not
validate this follow-up. Refresh affected evidence. Do not add a test that merely
mirrors the wrapper construction; verify the actual rendered geometry and existing
interaction invariants at their appropriate boundary.

### Current validation results

Compact-code formatting, all-target Clippy, tests (883 reported passed) and final
release build passed, with no external dependency compilation allowed. Current
production Xvfb evidence covers both themes at1x/2x, File/FFT panels and switching,
keyboard entry, active/inactive Range and narrow long-label layout. Horizontal
outer spaces measure2px/4px consistently. Pre-hint frames separate actual vertical
clearance from popover shadow:1x inner top1px and bottom2px;2x top/bottom3px, with
the status top border accounted for separately. Existing heights were preserved.
The executor visually inspected the current compact surfaces and dividers. The
coordinator assessed the result; independent evidence review confirmed the geometry,
visual proportions and stated limits without substantive findings.

Exact source/binary identities, raw screenshots, measurements, stable image hashes
and command logs are retained in the external review handoff. Earlier8px evidence
is explicitly before-comparison material only. Task-owned application and Xvfb
processes were cleaned up. The compact code and evidence reviews by `gpt-6.1-sol` high returned no substantive
findings; no finding dispositions were required.
Transient pending/conversion-busy interactions remain unperformed; no unrelated
conversion was manufactured. Xvfb does not establish Wayland or platform-specific
behavior. The owner visually accepted the result with these recorded validation
limits, initially requested a local commit without a push, then authorized the
final publication lifecycle. Tooltip/panel lifecycle unification is separate
Issue #246; no new tooltip behavior is part of this completed spacing task.

## Post-completion

After current full CI succeeds, squash-merge the accepted PR and remove its branch
and task worktree with the documented exact-head checks. Preserve unrelated work
in the concurrent main checkout.
