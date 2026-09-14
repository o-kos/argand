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

- [ ] Add the menu hierarchy and keyboard/hover navigation state
- [ ] Render cascading panels with one-level Escape, outside dismissal and restored focus
- [ ] Replace File/View title buttons with the application icon and two toolbar toggles
- [ ] Preserve registered actions, persistence, file hints and navigation controls
- [ ] Verify narrow layouts, themes, window dragging and double-click isolation
- [ ] Update architectural documentation and complete independent review
- [ ] Move this plan to `docs/plans/completed/` before owner review

## Validation

- [ ] Meaningful menu navigation and panel geometry tests
- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --locked`
- [ ] `cargo test --locked`
- [ ] `cargo build --release --locked` after the local gate
- [ ] Native GPU checks of pointer/keyboard menus, shared toggles and focus restoration
- [ ] Native light/dark, narrow-window, drag/resize and double-click checks
