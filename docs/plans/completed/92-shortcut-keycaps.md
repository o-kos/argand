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

- [x] Share framed keycap presentation across menus and hints.
- [x] Measure framed shortcuts for menu widths and preserve label space.
- [x] Update the changelog and architecture notes.
- [x] Complete independent review and native validation.
- [x] Move this plan to docs/plans/completed before owner review.

## Validation

- [x] cargo fmt --all -- --check
- [x] cargo clippy --all-targets --locked
- [x] cargo test --locked
- [x] cargo build --release --locked after the local gate
- [x] Inspect light/dark themes, narrow menus, 100/125/200% display scale,
  multi-modifier shortcuts and matching tooltip/menu rendering.
- [x] Verify keyboard/menu actions and focus still work.

## Verification notes

Formatting, strict Clippy and all 554 tests passed, followed by a fresh release
build. Independent review was clean, with no findings to accept or decline.
Linux GPU checks covered both themes at 100%, 125% and 200% output scale, nested
menus in a 640x400 window, multi-modifier keycaps, analysis and toolbar hints,
long-path start-page hints, menu/keyboard grid activation and settings focus.
Platform-specific notation remains the toolkit formatter; native Windows/macOS
appearance was reviewed through its source contract, not runtime-tested here.
