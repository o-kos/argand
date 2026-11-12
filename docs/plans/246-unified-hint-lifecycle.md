# Issue #246: Unified tooltip and information-panel lifecycle

Resolves #246: <https://github.com/o-kos/argand/issues/246>.

## Overview

Information panels, passive tooltips and inline advice currently have separate
appearance and dismissal rules. An open Analysis panel can display its Range
warning balloon while the status-bar Range tooltip repeats the same explanation.
Establish one application-wide policy with independent state for each window,
retaining standard toolkit controls and the existing shared hint infrastructure.

The owner approved the lifecycle policy below for implementation. The original
explicit-panel snapshot passed its local gate, Xvfb acceptance and three review
rounds. Later owner-requested corrections below remain active pending owner
acceptance and independent-review disposition; the original clean review does not
cover those later changes. Work takes place in the explicitly requested linked
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

## Owner-requested File menu grouping follow-up

Reorder File as Open, separator, Recent entries when present, separator, Save /
Save as / Save selection as, separator, Analysis settings, separator, and final
File information. An empty Recent list contributes no empty group or duplicate
separator. Keep Analysis settings in File and preserve its existing action and
shortcut; it opens the panel containing FFT size. Tooltip copy is unchanged until
the owner approves revised wording. This remains local, without publication or a
new independent review. Validate populated/empty recent-list ordering and formatting;
current-production acceptance for this new menu snapshot remains pending until a
required fresh release build and isolated Xvfb check.

- [x] Apply the agreed menu grouping and verify populated/empty history cases.

Formatting and the targeted menu-order regression passed. Only the project crate
compiled under the existing refusal guard. No full gate or release was rerun for
this small intermediate change; combine fresh production validation with the
pending owner-approved tooltip wording change. The existing release binary does
not include this new menu order.

The owner approved File tooltip wording: “View file properties and conversion
options”. Apply that exact text without changing title, typography or shortcut.
The application menu is the logo/Argand button at the upper left (F10), whose File
submenu contains Analysis settings; FFT size is inside that panel, not a separate
FFT menu entry. Combine the approved copy and grouping in one fresh project-only
release and bounded Xvfb menu/tooltip acceptance. Reuse the previously passed full
lifecycle suite for this narrow delta; run formatting, Clippy and the populated/empty
menu-order regression, without claiming the earlier full gate covers this revision.
Independent review and owner acceptance remain pending.

Combined follow-up validation: formatting, all-target Clippy, populated/empty menu
model regression and the final project-only release passed under the dependency
refusal guard. The full suite was not repeated for this narrow menu/copy delta.
Fresh isolated production captures in dark/light show the approved tooltip and
empty-history File menu with exactly one separator between Open and Save. Actual
light-theme menu clicks opened Analysis settings (FFT size visible) and final File
information. Coordinator assessment confirmed the light captures. Populated Recent
ordering is covered by the model regression, not a new runtime history scenario.
Evidence and exact source/binary identity are retained in
`/tmp/argand-246-xvfb/menu-source-identity.json` and `menu-{light,dark}/`.
The delta remains local and uncommitted; no publication or additional independent
review. The plan remains active pending review disposition and owner acceptance.

## Owner correction: menu discoverability and populated-history accessibility

Move Analysis settings from File to a final separate View group. Preserve the
existing document-dependent applicability: the row stays visible but disabled
without a file or during conversion. Do not add a no-file status anchor or extend
global settings capability. Permanent File/Edit/View commands remain present;
only an empty Recent group is omitted. Verify actual rendered no-file and loaded
menus instead of inferring visibility from the menu model.

The standard PopupMenu defaults to half the window height with a 28.125rem cap.
File already enables native scrolling, but the populated-history case was missed
by prior runtime acceptance. Use public max_h from the viewport minus titlebar
and standard edge allowance, with native scrolling for every application-menu
branch. Preserve standard anchoring, scrollbar, pointer and keyboard selection.
Do not introduce custom controls or modify the toolkit.

- [x] Move Analysis to View and verify visible/disabled applicability.
- [x] Bound standard menu branches to available viewport with native scrolling.
- [x] Verify no-file/loaded, zero/eight Recent and small-window production menus,
  including pointer and keyboard access to final File information.

Current menu correction validation: formatting, all-target Clippy, all 891 tests
and the final release build passed; only project code compiled under the refusal
guard. The affected menu module has 25 passing tests. Isolated production Xvfb
compared File/Edit/View with no file and with a loaded capture, each with zero and
eight visible Recent rows at 960×640. Permanent rows remained rendered; no-file
unavailable rows were visibly disabled. Analysis was absent from File and present
in View, inert without a file and opening the editor with one. At 960×360, native
wheel scrolling revealed final File information for empty and loaded windows with
eight Recent; a pointer click and native keyboard wrap/Enter each opened it in the
loaded window. At 640px height all eight history rows and final File information
fit together. The loaded fixture stores nine entries because the current capture
is intentionally excluded from the visible Recent list; the initial seven-row
fixture was retained separately and not counted as eight-row acceptance.

This evidence covers menu reachability, not a new review of property-panel layout
at forced below-normal window heights. Current source/binary identity and captures
are retained under `/tmp/argand-246-xvfb/availability-source-identity.json`,
`availability-report.md` and `availability-{empty,loaded}{0,8}/`. No native Wayland,
Windows or macOS runtime claim. The corrected snapshot remains independently
unreviewed; owner acceptance and review disposition remain pending. No fourth
review, external publication or additional commit was performed for this delta.

## Owner correction: scrolling, no-file View applicability and Edit shortcuts

The previous blanket scrollable helper introduced native scrollbar layers on the
short root and fixed branches; a short File edge-hover also showed an unwanted
native scrollbar. These are standard toolkit layers, not new widgets. Remove
blanket scrolling and keep the variable-history File branch's public max-height;
zero-history File must not mount a scroll layer. Native overflow behavior for
populated but fitting history remains a required runtime check, not an assumption.
Fixed Edit/View/Theme/time branches fit the supported 640×400 minimum window.

Without a document, disable grid, scale controls, orientation and Time scale format
alongside the existing document commands. Keep Theme available. Their handlers
previously persisted session preferences without an immediate no-file visual
result; the owner now requires applicability-based menu disabling.

Edit bindings, focus_target, action_context and the existing focus-based keycap
lookup predate this Issue unchanged. Most Edit bindings belong to Plot, while an
empty window has Shell/StartPage context. Use standard registered Plot-context Kbd
lookup for Edit labels so disabled commands keep their documented shortcuts; do
not hardcode key strings or alter dispatch mappings. Keep PopupMenu.action_context
on the real focus target and verify loaded actual activation after panel focus.
Mouse application-menu opening after File and Analysis focus showed all registered
Edit keycaps in the previous binary. F10 inside these panels did not open the menu;
that existing Shell-scoped route is recorded separately, not expanded silently.

- [x] Apply the bounded source changes and meaningful regression tests (unexecuted).
- [ ] Validate short root/File edge-hover, populated File overflow, no-file/loaded
  View applicability and stable Edit labels/actual actions in current production.

Validation is blocked by the new owner resource policy. The common build-dir was
removed from the global Cargo configuration. This old worktree lacks the current
main branch's immutable SDK consumer integration and local dependency baseline;
its target directories contain final outputs from the earlier shared setup.
Do not run Rust, migrate caches, select SDK flags or change the dependency graph
until the owner/coordinator resolves integration onto the approved current basis.
Earlier 891 passing tests and release evidence are historical, not validation of
this new source delta. No fourth review or external publication is authorized.

Pending runtime cases explicitly include one through seven Recent entries when
all entries fit, in addition to zero-history and eight-history overflow cases.
The standard native scrollbar's observed false-overflow behavior must not be
called fixed for those fitting populated histories. The available change removes
scrolling from fixed short branches and zero-history File only. No newly edited
source was formatted, compiled, tested or demonstrated after the resource-policy
change; git diff --check is the only completed check for this latest delta.

The owner subsequently granted a temporary exception to continue using the old
shared-cache scheme for this correction, without authorizing toolchain/toolkit
changes or new dependency compilation. Read-only verification found that the
previously approved `/home/xokc/.cache/cargo-target` is no longer present. This
worktree has only final outputs and no local dependency/fingerprint baseline.
The direct installed compiler remains available and no SDK/flag overrides apply,
but there are no old intermediate artifacts to reuse. No Rust command was run
under the exception. Validation remains blocked until the owner supplies/restores
the approved cache or authorizes a different dependency/SDK baseline. Do not create
a fresh cache and describe its inevitable dependency compilation as ordinary reuse.


## SDK integration and owner-approved local dependency baseline

The owner approved rebasing this branch onto the current main SDK consumer
integration, preserving the UI changes. The rebase completed without conflicts;
the eight local commits retain identical patches. Upstream SDK integration and
its locked graph remain unchanged. Backup refs and a binary patch preserve the
pre-rebase state. The existing immutable GPUI Kit SDK and its 1,098 artifact hashes
were verified against the installed direct Rust 1.98.1 compiler.

The owner explicitly approved initial ordinary application/CLI dependency
compilation for debug/test, check/Clippy and release in this worktree's local
`target/`, consuming that SDK without rebuilding it. This supersedes the temporary
old shared-cache exception; no shared build-dir or scratch cache is used. Existing
SDK selection uses the upstream consumer configuration. No toolkit producer,
toolchain installation, dependency graph change, publication or fourth review is
authorized. Current source validation and production menu acceptance remain
pending; historical test counts do not validate this snapshot.


### Standard Recent files submenu correction

Production acceptance found a native scrollbar on a fitting one-history File
menu. The public scrollable switch couples its indicator, overflow and max-height,
so removing only the indicator is not supported. The coordinator approved the
standard Recent files submenu composition instead of scroll geometry heuristics.
Recent history is capped at ten single-line rows. File and Recent menus use native
submenu sizing without scrollable layers, preserving the frozen history snapshot,
1–9/0 shortcuts and native actions. Empty history omits that submenu and retains
one separator between Open and Save. Acceptance must cover ten entries at the
supported 640×400 minimum as well as the owner's eight-entry case.


Native production diagnosis of the complete menu matrix found one additional
applicability issue: the toolkit's disabled submenu row still renders children
on pointer hover. Without a document, Time scale format now uses a standard
disabled item with the same label; with a document it remains the standard
submenu. A regression asserts no child submenu is drawn on hover and unavailable
parent clicks remain inert. Recent 1/7/8/10 geometry and actions, including ten
rows at 640×400, and loaded Edit commands after File/Analysis focus passed the
bounded diagnosis before this final correction. Final current-source validation
and production confirmation follow below; no toolkit patch or new control exists.


### Current menu correction validation

- [x] Format, all-target locked Clippy and the full locked test gate passed:
  893 tests, including 524 application tests.
- [x] The last release build passed after all checks, recompiling only the
  application against the unchanged installed SDK and local baseline.
- [x] Isolated production X11/Xvfb checks confirmed root/File zero-history edge
  hover, fitting Recent 1/7, eight and ten rows at supported 640×400, native
  keyboard selection/digit 0 and actual File information pointer activation.
- [x] No-file View commands are visibly disabled and inert; Time scale format
  draws no child on hover; Theme remains functional. Edit registered labels
  remain visible without a file and after File/Analysis focus. Actual Selection
  menu activation and Ctrl+E after each panel both work.
- [ ] Owner acceptance and independent-review disposition for the corrected
  snapshot. No fourth reviewer round, publication or merge was performed.

The standard Recent submenu supersedes the earlier inline-history scrolling
proposal and its pending fitting-history checks. Production identity and bounded
evidence are retained in the task acceptance artifacts. These checks establish
Linux X11/Xvfb behavior, not native Wayland, Windows or macOS, below-minimum window
heights, or a new exhaustive review of every information-panel transaction. The
working source and demonstrated binary remain bound by their recorded production
diff and binary hashes; documentation-only acceptance recording does not require
another build.


### Owner follow-up: guarded Close and Quit (active)

Add Quit Argand after About and a separator, with registered Ctrl+Q (Cmd+Q on
macOS). The application currently opens one window per process; Quit requests
that window's existing close transaction rather than bypassing it with app quit.
Retained Shell focus dispatch addresses caption Close without moving actual
focus. App-level shortcut fallback is scoped to its registered owning window.
Only the existing caption controls remain live above the menu backdrop; toolbar
and drag surfaces remain protected. No toolkit or dependency changes are needed.

Acceptance: the first caption Close click reaches the guarded transaction with
each File/Analysis/About panel and application/File/Edit/View/Recent menu open;
Cancel preserves dirty document identity/version and edits. Clean Close, menu
Quit and Ctrl/Cmd+Q remove the window. Existing conversion cancellation and save
protection remain authoritative. Verify targeted regressions, the required local
gate, then the same cases in the isolated production release. This follow-up is
not covered by the earlier independent-review snapshot; publication remains
unauthorized.

Validation is currently blocked before a new production binary exists. Formatting
completed and git diff whitespace checks passed. The initial targeted compile
found local compile errors, which were corrected but have not been revalidated.
A matching workspace test command with the project-only refusal guard then stopped
on missing ordinary dependency artifacts (including console, moxcms and host
serde_json build scripts). The release attempt also stopped before dependency
compilation: all 131 target library artifacts exist, but the host release build
scripts and proc-macro support artifacts are absent. The stopped release requests
include unicode-ident, autocfg, quote, proc-macro2, crossbeam-utils/epoch/deque,
thiserror, serde/serde_core, version_check and libc.

No new Close/Quit regression or running-app acceptance has passed for this source.
The previous 893-test result and demonstrated binary belong to the previous menu
snapshot. Required owner approval is limited to restoring ordinary application
dependency artifacts in this worktree-local target for debug/test, check/Clippy
and release using the unchanged installed compiler, lock and SDK selection.
Rebuilding the installed GPUI Kit SDK, changing the dependency graph/toolchain or
using shared intermediate caches is excluded. Builds remain stopped pending that
decision; publication and another independent-review round remain unauthorized.

The owner explicitly approved restoring the ordinary application dependency
baseline for debug/test, check/Clippy and release in this worktree-local target.
The installed SDK remains excluded from compilation; the unchanged direct
compiler, lock, SDK selection and default workspace feature inputs are retained.
The prior blocked state above records the investigation, not the current owner
authorization. Validation now resumes under this bounded approval.

Production acceptance found that Ctrl+Q reached the unsaved guard while deferred
About/menu overlays could still cover the confirmation. All Pending::Close
routes now dismiss nested Select, menus and information panels with their usual
keep semantics before invoking the unchanged conversion/unsaved guards. The
regression matrix asserts those overlays are absent when the prompt opens, then
Cancel preserves document identity/version/length and does not reveal a second
prompt. The earlier runtime captures diagnose this failure; final acceptance
must use the rebuilt corrected snapshot.

Final Close/Quit gate passed: fmt, all-target Clippy, 896 workspace tests (527 app),
then release. SDK/toolkit source compilation invocations: zero. Production
X11/Xvfb verified clean first-caption-click Close for File/Analysis/About and
application/File/Edit/View/Recent menus; dirty caption and Ctrl+Q for each, plus
menu Quit, showed an unobscured confirmation. Escape/Cancel retained dirty and
Inverted state and unchanged source bytes; pointer Cancel also retained state
after its dismissal animation. Clean Ctrl+Q and menu Quit removed the window.
Tests additionally establish document identity/version/length, no second prompt,
owning-window isolation and conversion cancellation safeguards. File information
was verified through production readers and the existing nested Select regression,
not the synthetic metadata matrix.

Final binary SHA256: e3b02d84fcbddeece94927e8f7c9f54f11f48266ab5ba7069d2c341ed4df576b.
HEAD f20ba6c plus unstaged production patch SHA256:
411dae912c828e9dbaf4f754336131e381d5977b61ef75633afa48229d712b70.
Report: task artifact sdk-close-report.md; identity: sdk-close-source-identity.json.
Linux X11/Xvfb only; native Wayland/macOS/Windows runtime and live long-running
conversion are unverified (conversion guard is test-covered). This snapshot has
no new independent review; the original three rounds concern older snapshots.
Plan remains active pending owner acceptance/review disposition. No publication,
push, merge, new review round or local commit was performed.

### Accepted review follow-up: Windows caption hit testing

The owner directed immediate correction of the accepted P2: the menu backdrop
reserved caption controls on Linux only and occluded standard Windows native
window-control hitboxes. Use the locked TitleBar's actual TITLE_BAR_HEIGHT
control width and supported minimize/maximize flags plus Close. Preserve Linux
and macOS behavior, add a Linux-executable Windows-style hitbox regression, run
the required gate/release and relevant Linux first-click Close/dirty acceptance,
then resubmit the exact corrected snapshot to the retained reviewer. No toolkit
patch, new widget, graph change, commit or publication is in scope. Windows
runtime remains unavailable and must not be inferred from the regression.

Submitted P2 resolution: the shared caption-width selector now includes Windows
standard Close and supported minimize/maximize at TITLE_BAR_HEIGHT each, while
retaining Linux/macOS geometry. The existing backdrop composition is shared from
chrome geometry code. A Linux-executable Windows-style window_control_area
regression covers all four supported-control combinations and blocked adjacent
input. Fmt, all-target Clippy, all 897 tests (528 app) and release passed.
Windows runtime remains unavailable; current Linux menu acceptance evidence is
recorded separately with the demonstrated binary identity. The corrected snapshot
is submitted to the retained reviewer; it is not yet declared clean or accepted.
