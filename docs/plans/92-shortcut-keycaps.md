# Issue #92: Consistent framed shortcut hints

Resolves #92.

## Overview

Use the framed shortcut beside Edit settings as the shared presentation for
application menus, toolbar and start-page tooltips, and analysis hints.
Keep shortcut behavior and action bindings unchanged.

## Context

The reference uses gpui-component's Kbd with its default appearance. Application
menus and shortcut_tooltip currently disable that appearance. Menu width measures
plain text at the menu font size, which differs from the framed keycap typography.
The ruler context menu has no registered shortcuts to display.

## Decisions

- Reuse Kbd's frame, colours and platform-specific notation from crates.io.
- Centralize keycap styling and measurement so menu layout matches painting.
- Resolve shortcuts from registered bindings in the existing action context.
- Preserve compact rows, right-aligned shortcuts, and viewport-bounded hints.

## Rejected alternatives

- Hard-coded shortcut strings would drift from registered platform bindings.
- A custom keycap design would depart from the owner's chosen visual reference.

## Implementation steps

- [ ] Share framed keycap presentation across menus and hints.
- [ ] Measure framed shortcuts for menu widths and preserve label space.
- [ ] Update the changelog and architecture notes.
- [ ] Complete independent review and native validation.
- [ ] Move this plan to docs/plans/completed before owner review.

## Validation

- [ ] cargo fmt --all -- --check
- [ ] cargo clippy --all-targets --locked
- [ ] cargo test --locked
- [ ] cargo build --release --locked after the local gate
- [ ] Inspect light/dark themes, narrow menus, 100/125/200% display scale,
  multi-modifier shortcuts and matching tooltip/menu rendering.
- [ ] Verify keyboard/menu actions and focus still work.
