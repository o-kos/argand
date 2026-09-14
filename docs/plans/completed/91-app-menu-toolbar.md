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


## Owner feedback: centered title and stronger controls

- Center the filename on the whole window using balanced layout margins that
  include the platform title-bar padding and native controls. Keep truncation.
- Give toolbar controls a slightly darker resting appearance, a visible border
  and stronger theme-aware hover foreground/background, including active toggles.
- Retain standard buttons and their focus/disabled behavior; use one custom
  palette for the application button and both toolbar actions.

Validation passed again: formatting, strict Clippy, all 546 tests and a rebuilt
release. Linux GPU screenshots verified full-window centering, short/long titles,
640x400 truncation, dark/light hover backgrounds and the active grid control.
The shared-action and title-drag checks also passed. Independent review found that
Icon captured its foreground before hover painting; applying group hover directly
to the SVG fixed it. The final review round was clean; no findings were declined.


## Owner feedback: application identity and muted glyphs

- Use muted foreground for resting SVG controls and the brighter accent on hover.
- Combine the artwork and Argand label into one application-menu button, measuring
  the label to include its full width in the balanced title layout.
- Show only the filename in the centered client title; leave it empty at startup.
- Explicitly set the native title to `Argand` at startup and `filename – Argand`
  on every file opening, including replacement without restarting the application.

Validation passed: formatting, strict Clippy, all 546 tests and a fresh release.
Linux GPU checks confirmed muted/accent SVG colours in both themes, icon and label
activation of the same menu, centered filename-only text, and narrow-window
truncation. Compositor window titles were verified at startup, after opening a
file, and after replacing it through Recent without restarting. Independent review
was clean, with no findings to accept or decline.
