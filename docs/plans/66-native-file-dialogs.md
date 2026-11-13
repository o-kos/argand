# Issue #66: Native file dialog starting directory and filters

Resolves #66. [Issue](https://github.com/o-kos/argand/issues/66).

## Overview

Use rfd for native open/save dialogs, with a remembered open directory and signal
format filters. The owner approved a trial in this linked worktree, superseding
the previous rejection of a second dialog implementation. Preserve the installed
GPUI Kit SDK and the approved toolkit graph.

## Roles and risk

- Class: A, because this crosses the SDK type boundary and async window lifetime.
- Primary: `gpt-6.1-sol` medium, owns investigation, implementation and validation.
- Consultant: `gpt-6-astra` medium, targeted SDK/native-window handle advice.
- Reviewer: `gpt-6.1-sol` high, fresh read-only context before owner review.
- Current owner-provided role rules override the older roles in this base revision.

## Decisions

- Use rfd with the Linux XDG Portal backend and no direct GTK dependency.
- Reuse persisted successful-open history for the initial directory. Check its
  parent off the UI thread, falling back to the current working directory.
- Keep All files available for raw captures with arbitrary extensions.
- Preserve the existing save request, source checks and staged publication.
- Do not rebuild the immutable SDK, select a new toolkit graph or install Rust.
- Exercise production UI only on isolated Xvfb as required by project policy.

## Acceptance criteria and evidence

| Criterion | Verification | Result | Limits |
| --- | --- | --- | --- |
| Open starts in last successful file directory across restarts | Native dialog scenario, session round-trip | Pending | Linux first |
| First open and unavailable remembered directory use cwd | Directory policy tests and native scenario | Pending | Checks off UI thread |
| Cancellation/failed open preserves directory history | Session and action tests | Pending | Native cancellation also needed |
| Menu, shortcut and start page share the policy | Shared ChooseFile action and UI tests | Pending | Keyboard/focus acceptance needed |
| Supported signal filters and All files | Native dialog scenario | Pending | Raw extension is unrestricted |
| Save still validates sources and publishes safely | Existing save tests and native save/cancel | Pending | No publication changes |
| Parent/window lifetime and focus remain correct | SDK compile probe and native scenario | Pending | KDE, Wayland, Windows/macOS unverified locally |

## Implementation steps

- [ ] Verify native-parent integration with the installed SDK.
- [ ] Add the shared native dialog boundary and dependency without toolkit drift.
- [ ] Implement initial open directory and filters, preserving cancellation.
- [ ] Use the same dialog implementation for Save as and selection export.
- [ ] Add focused directory/history and action regression coverage.
- [ ] Update current architecture documentation and changelog.
- [ ] Run local checks, production UI acceptance and independent review.
- [ ] Move the plan to completed only after implementation and acceptance.

## Validation

- [ ] cargo fmt --all -- --check
- [ ] cargo clippy --all-targets --locked
- [ ] cargo test --locked
- [ ] cargo build --release --locked after checks pass
- [ ] Production Xvfb dialog interaction with isolated state
- [ ] Fresh independent review of the exact diff

## Post-completion

Ready/full CI, merge and worktree cleanup require the ordinary acceptance workflow.
