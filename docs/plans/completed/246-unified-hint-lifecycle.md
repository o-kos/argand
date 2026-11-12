# Issue #246: Explicit information panels and unified hint lifecycle

Resolves [Issue #246](https://github.com/o-kos/argand/issues/246) through
[PR #247](https://github.com/o-kos/argand/pull/247).

## Outcome and scope

File information, Analysis settings and About Argand open explicitly through
standard Popovers. Status controls explain their purpose through informative
tooltips, while one per-window policy coordinates panels, external help and
deactivation. Internal field help and automatic Range advice retain their
content and visibility policy.

The final scope includes menu discoverability and guarded window closing:

- File information is the final File command. Analysis settings is in View.
- Recent files use a standard submenu, omitted when empty, preserving the ten
  history entries and existing digit shortcuts. No blanket scrollbar is added.
- Permanent commands stay visible and are disabled when inapplicable. No-file
  View document commands are disabled; Theme remains available. Unavailable Time
  scale format is a disabled row without an expandable child.
- Edit labels resolve registered Plot bindings; actual dispatch retains the real
  focus/action context. No replacement keymap is introduced.
- About is followed by a separator and Quit Argand, Ctrl+Q or Cmd+Q on macOS.
  Quit and caption Close use the current window's existing close transaction.

## Roles and risk

- Class A: delayed lifecycle delivery, per-window ownership and more than five
  application code files.
- Coordinator: gpt-6-astra medium.
- Executor: gpt-6.1-sol medium.
- Retained independent reviewer: gpt-6.1-sol high, read-only.

## Accepted interaction contract

Panel triggers use standard Popover activation. Repeating the same trigger or
registered command toggles that panel; an adjacent explicit trigger switches
ownership. One information panel is active per window. Ordinary internal clicks
and links retain standard behavior. Nested Select, Escape/topmost dismissal,
focus restoration, File drafts and Analysis preview/keep/revert remain intact.

External tooltip builders are suppressed while a panel/menu owns interaction.
Suppression invalidates pending and visible native help. Deactivation closes
panels and cancels help; reactivation must not resurrect old help. An epoch-bound
acknowledgement after deferred prepaint teardown releases fresh builders only
in an active window, so the first subsequent real motion starts normal help.

Close first dismisses nested Select, menus and information panels with keep
semantics, then uses the unchanged conversion/replacement/unsaved guards. A
confirmation cannot be covered by a deferred overlay. Cancel preserves edits;
there is no unconditional application quit or second delivery to another window.
Linux caption controls retain their existing width. Windows reserves the locked
TitleBar's Close plus supported minimize/maximize controls, each TITLE_BAR_HEIGHT
wide; native window-control hitboxes remain accessible above the menu backdrop.
macOS receives no additional app-painted caption reservation.

## Decisions and rejected alternatives

The initial hover proposal and later custom Popup/PopoverPanel exploration were
superseded by the owner's explicit-opening decision. No hover reveal/close
timers, hover-preview Tab interception, pinned state, custom widget, toolkit
patch or dependency upgrade was introduced. Existing shared hints composition
and public toolkit focus/actions/Popover/Tooltip APIs are retained.

Owner feedback refined the shared trigger/dismissal policy, secondary tooltip
typography, File tooltip text, menu placement and disabled states. About's
secondary line identifies Version; File explains properties and conversion
options. Inline Recent scrolling was replaced with a standard submenu. Enabling
Analysis before a file loads was excluded because it would extend settings
mount/apply behavior beyond this task.

## Implementation

- [x] Inspect repository contracts, existing consumers and locked public toolkit.
- [x] Agree final explicit-opening policy and synchronized product names.
- [x] Coordinate per-window information panels through existing shared elements.
- [x] Remove hover opening and apply external-tooltip eligibility/cancellation.
- [x] Preserve internal help, automatic Range advice and editor transactions.
- [x] Harmonize descriptive tooltips and standard menu composition/availability.
- [x] Preserve registered shortcuts and actual dispatch contexts.
- [x] Route caption Close and Quit through guarded, owning-window transactions.
- [x] Preserve native Windows caption hitboxes while menus are open.
- [x] Add meaningful lifecycle, native-control, transaction and menu regressions.
- [x] Synchronize architecture, control inventory and Unreleased notes.
- [x] Complete the local gate and production Linux UI acceptance.
- [x] Resolve accepted review findings and obtain coordinator assessment.

## Validation and limits

The installed Rust 1.98.1, unchanged locked graph and selected immutable GPUI Kit
SDK are retained. The owner approved restoring ordinary application dependencies
for debug/test, Clippy/check and release in this worktree-local target. The SDK
was not rebuilt. After current-main integration, formatting, all-target Clippy,
898 workspace tests and the final release build passed.

Production Linux X11/Xvfb acceptance exercised File/Analysis/About explicit
entry/switch/dismissal, nested Select, transactions, dark/light tooltip typography,
automatic Range advice, tooltip cancellation and deactivation. Menu scenarios
covered no-file/loaded applicability, empty and populated Recent up to ten entries
at supported 640×400, registered Edit labels and actual commands.

Close/Quit acceptance covered eight clean first-caption-click overlay cases,
sixteen dirty caption/Ctrl+Q cases and menu Quit, with Cancel retaining state. The
Windows caption correction additionally passed five relevant Linux clean-menu
Close cases and eleven dirty confirmation/Cancel cases. A Linux-executable
Windows-style hitbox regression covers all four minimize/maximize support
combinations, first caption presses and blocked adjacent drag/body input.

The current-main integration changed build-metadata invalidation and runtime
checkout paths, not the panel/menu implementation. Refreshed gate/release bind the integrated source to the demonstrated artifact;
About identity is checked again before publication, and
unchanged UI runtime evidence remains identified separately.

Native Wayland, Windows and macOS UI were not exercised. Windows support has
locked-source and executable hitbox evidence, not native runtime verification.
Final conversion cancellation evidence is automated; no live long-running
conversion was claimed. Runtime two-window activation used separate processes;
same-process ownership is covered by automated isolation checks.

## Review and acceptance

Accepted findings: rapid reactivation retaining native tooltip state; first-motion
registration requiring a second motion; and Windows menu occlusion of caption
controls. They were resolved through epoch-bound teardown/fresh registration and
standard caption geometry respectively. No substantive finding was declined.

The original third round was clean. The owner explicitly requested additional
review after later menu/Quit/Close feedback. The retained reviewer found the
Windows P2, then returned clean after its correction; coordinator accepted the
resolution. The current-main integration is submitted to the retained reviewer; its outcome is recorded in PR #247 before Ready.

The owner requested completion after the updated binary and clean review. This
authorizes publication and completion under the mandatory current full-CI gates.
No release tag or version publication is part of this Issue.

## Post-completion

After current full PR CI succeeds on Linux, Windows and macOS and review
conversations are resolved, squash-merge PR #247. Delete accepted remote/local
branch refs conditionally against the accepted head. Preserve any unique work
and any worktree still used by the owner's running application; remove the
accepted linked worktree normally when safe. Leave the independent main checkout
and unrelated work untouched.
