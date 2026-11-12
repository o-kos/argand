# Issue #246: Unified tooltip and information-panel lifecycle

Resolves #246: <https://github.com/o-kos/argand/issues/246>.

## Overview

Information panels, passive tooltips and inline advice currently have separate
appearance and dismissal rules. An open Analysis panel can display its Range
warning balloon while the status-bar Range tooltip repeats the same explanation.
Establish one application-wide policy with independent state for each window,
retaining standard toolkit controls and the existing shared hint infrastructure.

The owner approved the lifecycle policy below for implementation. The full local
gate and isolated production Xvfb scenarios passed. Independent review completed
in three rounds with no substantive finding in the final round; the coordinator
accepted both lifecycle fixes and the final in-scope result. Work takes place in the explicitly requested linked
worktree, preserving the independent main-checkout work. Compact status-control geometry
accepted in #225 is outside this task.

## Roles and risk

- Class: A: shared scheduling, delayed delivery and lifecycle across more than
  five likely application modules.
- Coordinator: `gpt-6-astra` medium.
- Executor: `gpt-6.1-sol` medium.
- Reviewer: `gpt-6.1-sol` high in a separate read-only context.

## Context and verified framework evidence

- `shell/hints.rs` owns the existing explicit `PinnedHint` Popover composition and standard
  Tooltip view construction. The former hover timer and rehover grace are removed.
- `shell/settings_ui/mod.rs` coordinates File/Analysis ownership and deferred
  opens. External Range help follows shared eligibility while inline advice remains automatic.
- `shell/about_ui.rs` has separate deferred-open invalidation and suppresses its
  build tooltip while a panel is open; About remains click/menu-only.
- `shell/property_advice.rs` owns shared Range/format advice, dismissal and
  pointer-region reconciliation, with stock Tooltip fallback when space is short.
- Other consumers include selection details, conversion fields, toolbar actions,
  recent files, units and ready-status metadata. All external registrations use
  shared per-window eligibility; internal field help retains its existing policy.
- Preserve Analysis preview/keep/revert and File conversion-draft transactions,
  keyboard access, nested Select dismissal, focus restoration and plot isolation.
- The locked `gpui-kit`, `gpui-component` and `gpui-base` are 0.9.1. Registry source
  inspection confirms public controlled `Popover::open`, `track_focus`,
  `on_open_change` and `overlay_closable`. Public `HoverCard` exposes
  `open_delay`, `close_delay` and `on_open_change`, but hover panels are superseded.
- Public GPUI `Interactivity::tooltip` and `tooltip_show_delay` are available.
  Component `ManagedTooltipExt` is crate-private; do not depend on it. Verify the
  native suppression/cancellation uses builder removal and epoch-bound teardown,
  retaining existing standard controls. No replacement widget is implemented. No toolkit/dependency upgrade or registry patch is planned.

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
- [x] Complete the explicit-panel and external-tooltip consumer inventory.
- [x] Document explicit-opening ownership, dismissal, deactivation and tooltip suppression.
- [x] Verify public native-tooltip teardown and retain standard Popover composition.
- [x] Remove hover opening, harmonize names and implement shared external-tooltip eligibility
  and cancellation/invalidation of deferred panel requests.
- [x] Migrate external consumers; retain internal field help and automatic advice.
- [x] Add lifecycle and production-control regression coverage.
- [x] Synchronize current UI architecture/inventory and user-facing changelog.
- [x] Complete local validation, isolated Xvfb production-app acceptance and
  independent review; record evidence and limits for the exact submitted state.
- [x] Move this plan to `docs/plans/completed/` before final review.

## Acceptance criteria and evidence

| Criterion | Verification / scenario | Result | Limits or unperformed checks |
| --- | --- | --- | --- |
| One documented policy governs identified consumers | Consumer inventory and accepted explicit-opening policy matched to code | Implemented; automated and production scenarios passed | Independent review completed |
| Analysis and status Range do not compete | Warned Analysis, status Range hover, nested FFT Select | Passed in dark/light production Xvfb; automatic advice retained | Native Wayland and other platforms unperformed |
| Delayed requests cannot reveal stale surfaces | Visible/pending cancellation; rapid activation; stale epoch acknowledgement; first new motion | Automated regressions passed; actual two-window X11 activation showed no stationary resurrection and first-motion help | No-frame activation boundary covered by atomic regression; public deactivate helper itself drains frames |
| Windows and transactions remain independent | File/Analysis click switches; File type Select Escape; File rate draft; Analysis preview/revert/keep; two isolated windows | Production Xvfb passed; standard transaction/focus regression suite passed | Runtime windows used separate application processes/configuration; same-process ownership is covered by existing entity isolation and automated tests |
| Standard Popover and Tooltip use verified public APIs | Locked-source audit and independent review | Existing standard composition retained; accepted round 1/2 findings resolved | Final round clean; coordinator accepted |
| Explicit opening, names and geometry follow policy | File F11/menu/click; Analysis Ctrl+,/click; About click; descriptive tooltips and menu labels | Production Xvfb passed, dark/light 1536×1000 and narrow light 900×650 | Native Wayland, Windows and macOS UI unperformed |

Use the accepted policy above as the expected result for each interaction; tests
and observations must not silently change product behavior.

## Resources and validation

Preserve the installed toolchain, agreed toolkit graph and common intermediate
build directory, with final outputs local to the workspace. Read-only inspection
found the installed `1.98.1-x86_64-unknown-linux-gnu` toolchain and
`/home/xokc/.cargo/config.toml` configuring
`/home/xokc/.cache/cargo-target` as `build.build-dir`. The project pin is 1.98.1.
Validation uses the installed toolchain's direct executable paths without changing
PATH, pins or the toolkit graph. The existing dependency-compilation guard is
adapted only to the current project crate source root; it refuses external crate
compilation. Debug/test, check/Clippy and release use the approved common cache,
with final outputs in this worktree's target directory. No dependency rebuild is
authorized by this plan.

The initial plan-only publication used diff/link checks under a scoped workflow
decision. Implementation uses the full local gate below and independent code
review; production Xvfb acceptance remains required before completion.

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked`
- [x] `cargo test --locked`
- [x] `cargo build --release --locked`, after the checks above pass
- [x] Automated lifecycle regressions and current-production-app scenarios
- [x] Independent read-only review and coordinator assessment

Run production UI acceptance only in isolated Xvfb/X11, never on the owner's
desktop. Verify the child display/backend and supported configuration/session/cache
isolation before launch; preserve the owner's settings and files. Missing permitted
interaction or Xvfb capabilities leave acceptance incomplete. Record native
Wayland and other unperformed platform checks separately.

## Post-completion

After owner acceptance and successful current-revision full CI, follow the normal
Pull Request merge and conditional branch-cleanup workflow. Do not publish a release.

## Review dispositions

- Round 1 accepted: rapid deactivation/reactivation could retain native visible or
  pending tooltip state if no inactive frame retired its builder. External help
  now remains suppressed until an epoch-matched acknowledgement after all native
  registrations and deferred controls retire.
- Round 2 accepted: releasing suppression on the first pointer motion missed that
  same event in newly registered native tooltip listeners. After confirmed teardown,
  the active window now restores fresh builders before future input; an inactive
  window waits for activation. Native registration alone starts no tooltip timer.
  Regression coverage requires stationary cancellation, first-motion help after
  panel closure and rapid activation, pre-teardown input rejection, actual platform
  activation wiring and rejection of stale epoch acknowledgements.

## Executed acceptance summary

The exact production build passed after fmt, Clippy and all 887 tests. Guarded
compilation reused the approved dependency baseline; only project crates compiled.
Task-owned Xvfb/X11 runs used disposable captures and isolated configuration, state
and cache directories. The owner desktop and open source files were untouched.

Actual pointer/keyboard checks covered descriptive hover without panel opening,
explicit click/shortcut/menu routes, mutual exclusion, File draft cancellation,
Analysis preview/revert/keep, nested Select Escape, outside-click dismissal and
automatic Range advice without duplicate external help. About retained its logo,
version, tag, commit and date. Two independently configured windows retained their
own analysis values. Deactivation closed File and cancelled visible help; stationary
reactivation showed no stale tooltip, and the first fresh pointer motion restored
normal help. Dark/light and narrow-window captures were visually inspected.

Raw logs, binary/source identity and screenshots are supplied with the external
review packet. No native Wayland, Windows or macOS runtime success is claimed.

- Round 3: no substantive findings. The coordinator accepted the two earlier
  lifecycle resolutions and the final implementation/acceptance evidence within
  the documented platform and runtime-window limits.

## Owner-requested tooltip presentation follow-up (included in correction)

The owner requested a local copy/typography correction after the lifecycle review:
use `Version` instead of the repeated application name in About's secondary build
line, and render File information / Analysis settings descriptions with the same
small muted typography as About. Preserve titles, build details, registered
shortcuts and all lifecycle/interaction behavior. Use standard `Tooltip::element`
composition; no widget, dependency or toolkit change. The prior lifecycle review
remains historical; this scoped presentation delta is assessed separately.

- [x] Apply the two tooltip presentation corrections.
- [x] Run the mandatory code gate with the existing dependency refusal guard.
- [x] Inspect actual corrected tooltips in dark/light isolated production Xvfb.
- [ ] Record coordinator assessment and local commit; publication remains blocked.

## Reopened: owner correction of incomplete lifecycle acceptance

The owner observed differing panel show/hide behavior and authorized correction.
The earlier acceptance overstated uniformity: only Popover composition, tooltip
suppression and deactivation had been shared; File/Analysis/About still differed
in trigger phase, repeated activation, internal clicks, focus-out and backdrop.
The plan is active again. Earlier validation/review remains historical evidence,
not approval of this correction. Class A and the original role assignments remain.

Use one per-window information-panel owner and deferred-open generation for all
three panels. The existing standard Popover adapter owns pointer activation and
common open/change delivery; explicit menu/key actions use the common owner path.
Use the standard left trigger rather than independent button handlers plus a
right-button Popover bypass. Preserve toolkit geometry, focus and nested dismissal.
Remove About's blanket internal-click/nonleft dismissal and extra focus-out close.
Use the same backdrop area with all information triggers live for switching.
Keep File drafts, Analysis keep/revert and internal automatic advice unchanged.

| Action | File information | Analysis settings | About Argand |
| --- | --- | --- | --- |
| Trigger / menu / existing shortcut | Open with standard focus | Open with standard focus | Open with standard focus; no new shortcut |
| Repeat own entry | Close | Close keeping preview | Close |
| Other information trigger | Switch to requested panel | Switch to requested panel | Switch to requested panel |
| Internal empty click / controls | Stay open; fields retain semantics | Stay open; fields retain semantics | Stay open; repository Link activates normally |
| Outside press | Close keeping draft policy | Close keeping preview | Close |
| Escape | Nested Select first, then panel | Nested Select first, then revert panel | Close with standard focus restoration |
| Focus elsewhere / window deactivation | Standard focus policy; deactivation closes | Same | Same; no special About focus-out listener |
| Hover only | Tooltip, no panel | Tooltip, no panel | Tooltip, no panel |

- [x] Implement common standard-Popover entry/ownership/dismissal policy.
- [x] Verify meaningful shared regression matrix, including six switch directions.
- [x] Complete full guarded local gate and current-production dark/light Xvfb matrix.
- [ ] Obtain coordinator assessment and owner disposition on exhausted review cap;
  do not start a fourth review or relabel it as a new cycle.
- [ ] Commit locally; publication remains blocked by the prior approval rejection.

Correction verification notes: the shared actual-button regression covers all
three repeat/internal-click policies and all six switch directions, checking the
owner, open entities, keyboard focus and an additional frame after each switch.
Linux About's right trigger may reach the native titlebar window menu, outside
this information-panel policy; TestWindow cannot implement that OS operation.
No new non-left consumption is added. Right inside panels and right status triggers
remain covered; About middle trigger is covered without claiming OS menu acceptance.
The corrected snapshot has not received independent review. The earlier clean third
round applies only to the previous implementation; no fourth round is started.

### Correction validation and remaining acceptance

The corrected production source passed formatting, Clippy, all 888 tests and the
release build, in that order, using the approved direct toolchain and dependency
refusal guard. No external dependency compilation was permitted. The production
patch relative to `5713f140933f0927f7013d1f1d359208f7ee64f1` has SHA-256
`276d2f9e07d95604da439ee1b040fff28568b18e281c40a97ba1e6e846f836b1`;
the demonstrated pre-commit release binary has SHA-256
`592582c2af484304e9790088cc7bbda7247b4491cdb81760f2b7cbf4097d492d`.
Its embedded Git metadata names the base commit; the recorded patch binds the
additional corrected production source. Documentation/commit updates do not imply
a post-commit rebuild.

Actual isolated Xvfb production checks exercised all three buttons, repeat toggles,
internal empty clicks, all six adjacent switches, outside dismissal and Escape in
dark and light themes. File/Analysis existing shortcuts and all three menu entries
opened the expected panel; the File menu input was retried after waiting for the
submenu to render. File draft cancellation, Analysis preview/revert/keep and nested
Select-first Escape remained intact. Each panel closed on window deactivation;
stationary reactivation did not restore the panel or visible tooltip, and the first
fresh pointer motion restored normal help. These activation checks used two
separate processes, not same-process windows. Automatic internal Range advice
remained visible without duplicate outer hints. The coordinator assessed the
corrected dark tooltips and light File tooltip as readable and consistent.

The raw current report and captures are retained outside the repository under
`/tmp/argand-246-xvfb/correction-report.md` and `correction-{dark,light}/`.
Native Wayland, Windows, macOS, the OS titlebar context menu and an actual external
browser launch were not runtime-validated. Link keyboard activation remains covered
by the automated URL-opening mock tests. The corrected snapshot has no independent
review; coordinator/owner disposition and owner acceptance remain pending. The plan
stays active. External publication remains blocked; no push or PR edit is attempted.
