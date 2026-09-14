# Issue #91: Application menu and title-bar toolbar

Resolves #91.

## Overview

Replace the horizontal File/View buttons with a cascading application menu opened
from the Argand icon. Place grid and orientation icon controls beside it, followed
by a truncating filename in the remaining title-bar space.

## Context

The accepted #89 tree provides orientation and grid actions. This branch is
stacked on it while the accepted PR chain awaits integration. #84 remains a
separate unfinished investigation. Keep the registry toolkit dependencies.

The stock popup menu dismisses its parent chain on Escape. The application menu
needs explicit level-by-level dismissal, hover switching, keyboard selection and
focus restoration. Own that navigation state in Argand, using GPUI elements and
the existing theme, icons and registered actions. Existing ruler context menus
remain separate.

## Decisions

- Reuse the Argand asset for the menu trigger and compact line icons for controls
- Keep all existing File, Recent, View, time-format and frequency commands
- Use the same registered actions for menu entries, toolbar buttons and shortcuts
- Keep menu navigation independently testable without a GPU
- Reserve title-bar space through layout, including native platform controls
- Consume control gestures so they cannot start window dragging or double-click zoom
- Preserve session persistence and existing shortcut presentation; #92 is separate

## Rejected alternatives

- A permanently expanded tree does not match the agreed cascading menu design
- Patching toolkit popup internals is unnecessary for application-owned menu state
- A fixed-position filename overlay cannot reliably avoid controls in narrow windows

## Implementation steps

- [x] Add the menu hierarchy and keyboard/hover navigation state
- [x] Render cascading panels with one-level Escape, outside dismissal and restored focus
- [x] Replace File/View title buttons with the application icon and two toolbar toggles
- [x] Preserve registered actions, persistence, file hints and navigation controls
- [x] Verify narrow layouts, themes, window dragging and double-click isolation
- [x] Update architectural documentation and complete independent review
- [x] Move this plan to `docs/plans/completed/` before owner review

## Validation

- [x] Meaningful menu navigation and panel geometry tests
- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked`
- [x] `cargo test --locked`
- [x] `cargo build --release --locked` after the local gate
- [x] Native GPU checks of pointer/keyboard menus, shared toggles and focus restoration
- [x] Native light/dark, narrow-window, drag/resize and double-click checks


## Review and verification notes

Independent review identified two issues, both accepted and corrected:

- Windows native title-bar hit testing must exclude toolbar controls before
  client mouse handlers run. The toolbar now occludes the underlying drag hitbox.
- A flipped submenu can overlap its parent in a narrow window. Each panel now
  occludes covered rows, and placement leaves a 32-pixel strip of the immediate
  parent reachable. A regression test covers the placement constraint.

The second review round was clean. Native testing then found that popup placement
included the client shadow area; the final placement uses the frame's content
bounds, including asymmetric padding on tiled windows.

Linux GPU checks exercise 1500x900 and 640x400 windows, short and long filenames,
dark and light themes, nested File/Recent and View submenus, sibling hover,
one-level Escape, outside dismissal, shared toolbar/menu/keyboard actions and
session persistence. Windows/macOS native behavior is checked against the toolkit
contracts; runtime checks on those platforms remain part of downstream validation.


Final verification passed: 546 tests, strict Clippy, formatting and a release
build from the final sources. The final focused review of content coordinates was
clean. Startup checks also confirmed disabled toolbar controls before opening a
capture, Recent activation, keyboard focus restoration, and frame resizing.
Because Sway ignores maximize requests for floating windows, double-click testing
verified the Wayland request itself: title text sends one maximize request;
the application icon and both toolbar controls send none.
