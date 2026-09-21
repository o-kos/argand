# Issue #135: System or integrated window appearance

Resolves [#135](https://github.com/o-kos/argand/issues/135).
Related: #124, #126, #127, PR #134.

## Overview

Implement the owner's approved choice between a system title bar with the same
application toolbar below it, and an integrated application title/toolbar row.
Persist the preference and apply it after restarting Argand. This is a focused
feature from current main, not the #127 Root migration. The #126 fixture and its
outstanding compatibility/appearance gate are not declared complete by this work.

Class **A**, declared before implementation (cross-platform persistence and more
than five expected code files). Implementer: `gpt-5.6-sol` high; reviewer:
`gpt-5.6-terra` high. Keep this pair for all rounds.

## Context and verified toolkit boundaries

- Cargo.lock uses GPUI 0.2.2 and gpui-component 0.5.1.
- `TitlebarOptions::appears_transparent` selects custom versus system title-bar
  content on macOS/Windows. Native macOS traffic lights remain in integrated mode.
- Linux has `WindowDecorations::{Server, Client}` and `request_decorations`.
  X11 can fall back to server decoration without a compositor; Wayland negotiates
  the final choice with the compositor.
- Important locked-GPUI limitation: Wayland `request_decorations` immediately
  stores the requested mode even when no decoration manager exists. Therefore
  `window_decorations()` alone cannot prove system decoration support. Detect
  protocol availability before requesting Server, and render using negotiated
  state where available. Do not edit registry sources, vendor or fork GPUI.
- gpui-component TitleBar always appends its window controls on Linux/Windows;
  do not reuse it as the plain toolbar under a system title bar.
- Main currently owns chrome::Frame and is not Root-wrapped. The analysis editor
  is Root-backed. Preserve these ownership arrangements for this bounded change.

## Decisions

- Values: `integrated` (backward-compatible default) and `system`, stored in the
  program-owned session. Bump the session version and retain old-version loading
  and future-version write protection. Do not write user argand.toml.
- Add a Window appearance choice reachable on the start page and with a file
  loaded, using existing menu composition and clear English labels. State
  "Restart required"; show pending selection separately from the active policy.
  No new custom menu/control or live-restart button.
- Freeze the startup policy separately from the stored pending preference. All
  newly opened analysis settings windows in the same process use the startup
  policy, never a newly selected pending preference.
- Reuse the existing title contents/toolbar in both modes. System mode does not
  attach client title dragging, double-click maximize, window controls or resize
  hitboxes to that toolbar. Preserve file-name centering and narrow-window layout.
- Keep existing integrated behavior and default geometry. Fullscreen/tiling and
  native frame state must not cause double insets or duplicate controls.
- Unsupported/refused System uses Integrated and explains the fallback in the UI,
  retaining the requested preference. Do not infer support from desktop names.
- A minimal Linux-only Wayland registry capability adapter is permitted if public
  GPUI APIs cannot provide support information. Prefer an already locked crate;
  no toolkit upgrade, protocol hand-encoding, shell-command dependency, indefinite
  UI-thread wait or unsafe foreign-display integration. Bound failure/timeout and
  fail safely to Integrated. Document the adapter and its actual coverage.
- Keep Root, plot, overlay and menu migrations in their existing issues. No DSP,
  analysis scheduling, texture lifetime or input-routing redesign here.

## Retained custom UI justification

- Existing main chrome/resize/control code remains only for Integrated pending
  #126/#127. The owner approved the mode choice, not adoption of the stock frame
  with known interaction failures. System mode bypasses that behavior entirely.
- Existing application-menu machinery is retained pending #131; this change adds
  ordinary command entries, not a new implementation of menu interaction. Verify
  pointer/keyboard selection, checked/pending state and focus restoration.
- A plain toolbar container is layout, not a replacement for a standard button.
  Reuse existing standard child controls and do not hand-write control behavior.

## Implementation steps

- [ ] Implement toolkit-neutral preference/startup/fallback policy and session tests.
- [ ] Wire safe platform options and Linux capability handling; keep existing
      dependencies pinned and document any direct use of already locked crates.
- [ ] Add the persisted UI choice and restart/fallback messaging.
- [ ] Share toolbar content and apply the startup policy to main/settings windows.
- [ ] Update README, CHANGELOG and current architectural invariants in AGENTS.md.
- [ ] Record native evidence and unavailable cases without claiming cross-platform
      native success from compile/unit tests.
- [ ] Complete independent review and act on findings (maximum three rounds).
- [ ] Move this plan to completed only when implementation/validation is complete.

## Validation

- Unit tests: default, round trip, legacy session, future-session protection;
  pending versus active mode; supported/unavailable/refused System; Linux backend
  decisions; platform title-bar options; controls/insets absent in system mode.
- Native Linux in an isolated test session: start-page selection, pending label,
  restart into both modes, settings-window consistency, shared toolbar/file name,
  no duplicate buttons, resize edges/corners, double-click/drag only on the actual
  title bar, maximize/restore/fullscreen, narrow layout, loaded plot/navigation.
- Verify no-protocol fallback if an available isolated compositor can exercise it;
  otherwise explicitly list it as unexercised beyond deterministic policy tests.
- Windows/macOS: cross-platform compile/test CI where available; never substitute
  that for unperformed native interaction checks.
- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --locked`
- [ ] `cargo test --locked`
- [ ] `cargo build --release --locked` after the other checks pass.

## Post-completion

Keep the PR Draft for owner feedback. Merge only after the owner accepts and full
three-platform CI succeeds. Reconcile the mode choice with #124/#127 at their next
implementation checkpoint; no early Root migration is authorized by this feature.
