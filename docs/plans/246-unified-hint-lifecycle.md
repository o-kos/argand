# Issue #246: Unified tooltip and information-panel lifecycle

Resolves #246: <https://github.com/o-kos/argand/issues/246>.

## Overview

Information panels, passive tooltips and inline advice currently have separate
appearance and dismissal rules. An open Analysis panel can display its Range
warning balloon while the status-bar Range tooltip repeats the same explanation.
Establish one application-wide policy with independent state for each window,
retaining standard toolkit controls and the existing shared hint infrastructure.

The owner approved the lifecycle policy below for implementation. Runtime
validation remains pending. Work takes place in the explicitly requested linked
worktree, preserving the independent main-checkout work. Compact status-control geometry
accepted in #225 is outside this task.

## Roles and risk

- Class: A: shared scheduling, delayed delivery and lifecycle across more than
  five likely application modules.
- Coordinator: `gpt-6-astra` medium.
- Executor: `gpt-6.1-sol` medium.
- Reviewer: `gpt-6.1-sol` high in a separate read-only context.

## Context and verified framework evidence

- `shell/hints.rs` owns `PinnedHint`, its 500 ms hover delay, 400 ms rehover
  grace, standard Popover composition and standard Tooltip view construction.
  Hover currently opens the same persistent panel as explicit invocation.
- `shell/settings_ui/mod.rs` coordinates File/Analysis ownership and deferred
  opens. Its Range tooltip does not share suppression with inline advice.
- `shell/about_ui.rs` has separate deferred-open invalidation and suppresses its
  build tooltip while a panel is open; About currently remains click/menu-only.
- `shell/property_advice.rs` owns shared Range/format advice, dismissal and
  pointer-region reconciliation, with stock Tooltip fallback when space is short.
- Other consumers include selection details, conversion fields, toolbar actions,
  recent files, units and ready-status metadata. Inventory their actual entry
  points before deciding which are governed or explicitly excluded.
- Preserve Analysis preview/keep/revert and File conversion-draft transactions,
  keyboard access, nested Select dismissal, focus restoration and plot isolation.
- The locked `gpui-kit`, `gpui-component` and `gpui-base` are 0.9.1. Registry source
  inspection confirms public controlled `Popover::open`, `track_focus`,
  `on_open_change` and `overlay_closable`. Public `HoverCard` exposes
  `open_delay`, `close_delay` and `on_open_change`, but it is not yet selected.
- Public GPUI `Interactivity::tooltip` and `tooltip_show_delay` are available.
  Component `ManagedTooltipExt` is crate-private; do not depend on it. Verify the
  complete public suppression/cancellation seam before committing to an adapter.
  Existing controls are the starting point, not evidence that a replacement
  widget is approved. No toolkit/dependency upgrade or registry patch is planned.

## Accepted owner policy

- File, Analysis and About share hover behavior: reveal after 600 ms without
  taking keyboard focus. The earlier provisional click-only About decision is
  superseded. There is no pinned state.
- Leaving the union of trigger and panel closes after 300 ms unless genuine
  focus remains inside the panel or its nested child. Re-entering cancels close.
- Explicit click, menu command or keyboard command opens and transfers focus
  into the panel. Tab on a visible hover preview enters its first enabled stop;
  Shift+Tab enters its last, then ordinary scoped traversal applies.
- Escape dismisses the topmost nested surface before the panel, preserving
  existing editor revert semantics and appropriate pre-entry focus restoration.
  Outside clicks close while allowing the target to take focus.
- Deactivation closes panels/tooltips and cancels pending reveals. Activation
  alone must not resurrect them; a new interaction is required.
- Neighboring triggers start the shared hover delay, with at most one panel.
  Focus genuinely inside the current panel prevents hover replacement; explicit
  invocation can switch. Each window owns independent state.
- Keep Range advice content and automatic visibility unchanged. While a panel
  owns pointer or focus, suppress external tooltips, dismiss visible external
  help and invalidate pending requests. No stale reveal without fresh interaction.
  Internal field help remains available.
- Preserve Analysis keep/revert, File conversion drafts, nested Select behavior,
  keyboard commands and source safety. No menu renaming, extra toolbar Tab stops,
  toolkit graph changes or new custom widget is approved.

## Toolkit limitation and proposed adapter (owner decision required)

The locked public Popover takes focus unconditionally when opened
(`gpui-base 0.9.1`, `popover.rs`, `PopoverState::transition_to`). Its surface owns
the dialog role, tab group and Escape dismissal. Redirecting `track_focus` to an
external handle does not express the contract, fails when no element is focused,
and is not an accepted workaround. HoverCard owns hover scheduling but exposes
only `HoverCardState::is_open`; its state mutation/cancellation methods are private.

The public `gpui_kit::base::Popup` is the standard anchored host used by both. Its
source explicitly delegates open state and interaction to callers. It supplies
measurement, placement, deferred rendering and occlusion, but not panel focus,
Escape, outside dismissal or hover cancellation. Using it would require replacing
those responsibilities of the current Popover composition in shared application
code. Under CONTRIBUTING's component-selection policy, extending the existing
shared element does not itself authorize that nonstandard behavior.

Proposed single adapter: extend the existing shared hint infrastructure around
standard Popup, keeping existing panel content and standard Input/Select/Button
and Tooltip. The application would own reveal/close timers and invalidation,
per-window ownership, focus entry/restoration, nested dismissal coordination,
Tab entry and external-help suppression. Validate all three panels, no-current-focus
opening, keyboard/pointer entry, nested Select, outside target focus, independent
windows and deactivation with automated regressions plus production Xvfb scenarios.
The retained coordinator belongs to the existing Shell/hint owner in each
window, never an application-global panel singleton. Requests carry monotonically
changing ownership/generation tokens; close, switch, trigger removal, deactivation
and owner drop invalidate delayed callbacks. Old close events cannot clear a newer
owner. Keep capture/history and processing state out of this coordinator.

Record pre-entry focus only when explicit invocation or Tab actually enters;
hover preview does not change focus. Restore only a still-live same-window handle
when dismissal owns focus; outside clicks must not overwrite focus gained by their
target. Read actual focus containment including standard nested Select content,
not a flag inferred from the last open action. Preserve topmost nested Escape
handling and the editor's existing transaction callbacks. Suppression invalidates
both visible and pending external help, and requires fresh eligible trigger entry
before revealing again.

This is a lifecycle maintenance obligation, not merely a visual wrapper.

**Owner decision: pending.** No adapter/application code has been implemented.
Alternative: owner-approved toolkit public-API work in the shared dependency graph;
no dependency change or registry patch is authorized by this plan. Do not weaken
the accepted UX to fit existing APIs without another owner decision.

## Proposed implementation sequence

- [x] Read repository policy, inspect current consumers and locked public APIs.
- [x] Agree the owner lifecycle decisions.
- [ ] Complete the consumer inventory.
- [ ] Document the accepted transition table, ownership, dismissal reasons,
  suppression boundaries and lifecycle acceptance scenarios.
- [ ] Verify the remaining public toolkit seams; select the smallest extension
  of existing hint infrastructure that expresses the accepted policy.
- [ ] Implement per-window coordination and cancellation/invalidation of delayed
  requests without per-control timing and suppression exceptions.
- [ ] Migrate all agreed consumers, resolving duplicate Analysis/Range advice.
- [ ] Add lifecycle and production-control regression coverage.
- [ ] Synchronize current UI architecture/inventory and user-facing changelog.
- [ ] Complete local validation, isolated Xvfb production-app acceptance and
  independent review; record evidence and limits for the exact submitted state.
- [ ] Move this plan to `docs/plans/completed/` before final review.

## Acceptance criteria and evidence

| Criterion | Verification / scenario | Result | Limits or unperformed checks |
| --- | --- | --- | --- |
| One documented policy governs identified consumers | Consumer inventory and accepted transition table matched to code | Accepted policy; implementation pending | Runtime checks pending |
| Analysis and status Range do not compete | Open warned Analysis, hover status Range, traverse permitted nested help | Pending | Production interaction not run |
| Delayed requests cannot reveal stale surfaces | Leave before delay; switch, dismiss, remove trigger, deactivate and close window; require fresh interaction | Pending | Automated and production scenarios required |
| Windows and transactions remain independent | Two windows; Analysis keep/revert; File draft; focus return; nested Select Escape before panel dismissal | Pending | No runtime validation yet |
| Standard controls use verified public APIs | Locked-source audit and independent review of final implementation | Partial investigation | Suppression/cancellation seam still to verify |
| Opening, switching, pinning and permitted nested help follow policy | Current production binary in isolated Xvfb, appearance and pointer/keyboard interactions | Pending | Native Wayland, Windows and macOS UI evidence unperformed |

Use the accepted policy above as the expected result for each interaction; tests
and observations must not silently change product behavior.

## Resources and validation

Preserve the installed toolchain, agreed toolkit graph and common intermediate
build directory, with final outputs local to the workspace. Read-only inspection
found the installed `1.98.1-x86_64-unknown-linux-gnu` toolchain and
`/home/xokc/.cargo/config.toml` configuring
`/home/xokc/.cache/cargo-target` as `build.build-dir`. The project pin is 1.98.1.
Cargo/Rust have not been invoked. Before code validation, verify direct executable
selection, the existing dependency-compilation guard and approved artifact
baseline; stop before dependency rebuilding outside that baseline.

The initial plan-only publication uses diff/Markdown-link checks; Rust checks,
runtime acceptance and code review do not apply to this non-behavioral commit.
The coordinator scoped this initial documentation publication accordingly.

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --locked`
- [ ] `cargo test --locked`
- [ ] `cargo build --release --locked`, after the checks above pass
- [ ] Automated lifecycle regressions and current-production-app scenarios
- [ ] Independent read-only review and coordinator assessment

Run production UI acceptance only in isolated Xvfb/X11, never on the owner's
desktop. Verify the child display/backend and supported configuration/session/cache
isolation before launch; preserve the owner's settings and files. Missing permitted
interaction or Xvfb capabilities leave acceptance incomplete. Record native
Wayland and other unperformed platform checks separately.

## Post-completion

After owner acceptance and successful current-revision full CI, follow the normal
Pull Request merge and conditional branch-cleanup workflow. Do not publish a release.
