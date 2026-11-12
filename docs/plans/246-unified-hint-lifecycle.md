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

## Final owner policy

The final owner decision supersedes all earlier hover-preview and proposed custom
adapter designs. Keep the standard Popover and existing shared hint infrastructure.
No custom PopoverPanel, new control, toolkit patch or upgrade is authorized.

- File information, Analysis settings and About Argand open only through explicit
  click, menu command or keyboard command, transferring focus into the panel.
  Hover shows informative standard tooltips; it never opens a panel. Remove panel
  hover timers and the obsolete preview/pinning proposal and preview Tab behavior.
- Synchronize meaningful names across tooltip, panel accessibility/title and menu.
  File information describes sample rate, reference frequency, sample type and
  format/conversion (F11). Analysis settings describes FFT/window and display range
  (Ctrl+, or Cmd+,). About retains its branding and uses About Argand as its
  accessible/title/tooltip label, without a redundant visual brand heading.
- Move the opening File information entry from Edit > Convert to... into File;
  keep the real Convert action inside the panel. Rename File > Settings to
  Analysis settings. Keep other menu organization and shortcuts unchanged.
- Retain standard Popover focus, keyboard, nested Select and existing dismissal
  semantics. One information panel per window. Preserve Analysis keep/revert and
  File conversion drafts. Do not add status/toolbar Tab stops.
- Suppress external tooltips, including visible and pending requests, while a
  panel owns interaction. Keep internal field help and Range advice content and
  automatic visibility unchanged. Dismissal must not resurrect stale help without
  fresh interaction.
- Deactivation closes information panels/tooltips and invalidates deferred opens.
  Activation alone does not reopen them. Windows remain independent.

## Public toolkit evidence

Use the locked standard Popover controlled open/focus API. Earlier focus-free
preview limitations no longer apply because previews are out of scope. GPUI
`Interactivity::tooltip` conditionally registered on a stable control is public;
locked `gpui-pre 0.3.12` `elements/div.rs` explicitly removes active tooltip state
when a subsequent render has no tooltip builder. Verify visible and delayed
suppression and re-entry against the actual production controls. Component managed
provider internals remain private and must not be called. Keep standard nested
Select Escape propagation and existing scope traversal.

## Rejected alternatives

- Hover-open/close panels, preview Tab and pinning: superseded by the final explicit
  owner decision favoring informative tooltips and intentional opening.
- Custom lifecycle adapter over Popup: no longer necessary or authorized.
- Toolkit changes/private provider APIs: unnecessary for the accepted standard
  component approach; dependency graph remains fixed.

## Proposed implementation sequence

- [x] Read repository policy, inspect current consumers and locked public APIs.
- [x] Agree the final explicit-opening policy and synchronized product names.
- [ ] Complete the explicit-panel and external-tooltip consumer inventory.
- [ ] Document explicit-opening ownership, dismissal, deactivation and tooltip suppression.
- [ ] Verify the remaining public toolkit seams; select the smallest extension
  of existing hint infrastructure that expresses the accepted policy.
- [ ] Remove hover opening, harmonize names and implement shared external-tooltip eligibility
  and cancellation/invalidation of deferred panel requests.
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
| Standard Popover and Tooltip use verified public APIs | Locked-source audit and independent review of final implementation | Partial investigation | Suppression/cancellation seam still to verify |
| Explicit opening, switching, dismissal and permitted nested help follow policy | Current production binary in isolated Xvfb, appearance and pointer/keyboard interactions | Pending | Native Wayland, Windows and macOS UI evidence unperformed |

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
