# Issue #209: Ignore the local VS Code workspace folder

Resolves #209.

Class C (configuration, no behavioural consequence). Implementer: Claude in the session.

## Overview

`.vscode/` holds personal editor settings and appeared as untracked in every checkout. `.gitignore` now ignores it at the repository root.

## Implementation steps

- [x] Add `/.vscode/` to `.gitignore`.
- [x] Move this plan to `docs/plans/completed/` before final review.

## Validation

- [x] `git status` no longer lists `.vscode/`.
