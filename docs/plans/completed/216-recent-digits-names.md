# Issue #216: Give the tenth recent file Alt+0 and tell recent files with the same name apart

Resolves #216.

Class B (two GUI modules and the toolkit-neutral recent list, local invariants, concrete acceptance criteria). Implementer: Claude in the session. Reviewer: `gpt-6-sol` medium, agreed with the owner.

## Overview

Up to ten recent files are kept and shown on the start page and in File, but only the first nine carry a digit. The tenth gets `0`, as a keyboard row runs 1 to 9, then 0: Alt+0 on the start page and `0` while the File branch holds the keyboard.

Recent rows are named by their file alone, so captures of the same name in different folders look identical. A row whose file name another shown row shares gets the nearest parent folder that tells it apart, the folders between it and the file elided with `…`, and `…` for the folders above it.

## Context

- `session::RECENT_LIMIT` is 10. `recent::RecentFiles::visible` gives the shown order and `RecentFiles::shortcut` refuses an index from 9 on.
- `session::recent_labels` names each row by its file name, used by the start page (`Shell::start_page`) and File (`Shell::file_menu` through `app_menu::file_items`).
- `app_menu::file_items` gives digits to the first nine rows (`DIGITS`). The start page binds Alt+1..Alt+9 to `OpenRecent { index }` in the `StartPage` context (`shell.rs`), File binds `1`..`9` to `OpenRecentRow` in the application menu context (`app_menu_ui.rs`), and the start page draws the digit itself (`Shell::recent_button`).

## Decisions

- One toolkit-neutral helper, `app_menu::digit(index)`, answers the digit of a shown row: `1`..`9` for the first nine, `0` for the tenth, none beyond. The start page, File, `RecentFiles::shortcut` and both key tables use it, so the ten keys and the drawn digits cannot drift apart.
- The labels move to `recent::labels` beside the list they name. For each group of shown files sharing a file name, a file's label goes outward through its parent folders to the first depth at which its folders up to there differ from every other file in the group. Of the folders up to that depth, those that every file of the group shares at the same depth are elided, consecutive elisions joining into one `…`. Folders beyond that depth are one leading `…`; a file whose whole path was needed starts from its root. A unique file name keeps the name alone. The separator is the platform's own.
- The start page hint keeps the file name over its full folder, so the whole path stays one hover away.
- ➕ A file name or a shown folder longer than 20 characters keeps its first and last characters around `…`, 8 on each side at first, as the owner chose after seeing long KiwiSDR folder names fill the row. The sides grow together, for all file names of the list at once and for the folders of one group at one depth at once, until every two different names, or folders, stay different once shortened. The 8 characters at the end keep a usual extension whole.

## Rejected alternatives

- Limiting the list to nine: the owner chose ten with `0`.
- Full paths for rows sharing a name: long, and they truncate in the middle exactly where the telling folder usually is.
- Telling rows apart by the folder nearest the file always: in the owner's example that folder is shared and tells nothing.

## Implementation steps

- [x] `app_menu::digit`, digits in `file_items`, `RecentFiles::shortcut`, Alt+0 and `0` bindings, the start page digit; tests.
- [x] `recent::labels` replacing `session::recent_labels`, used by the start page and File; tests for the owner's example, a differing nearest folder, different depths, three files, unique names.
- [x] ➕ Long file names and folders shortened around `…` while they stay apart (`recent::shortened`, `recent::side`), with tests on the owner's KiwiSDR folders, folders alike at both ends and long file names.
- [x] Review round 1 (`gpt-6-sol` medium, before the shortening): one finding, declined. A real folder named `…` reads like an elision, so `/a/a/a/f` and `/a/a/…/a/f` beside `/…/a/f` both read `…/a/…/f`. Such a folder name is contrived, the rows still open their own files, and the hint shows the whole path.
- [x] Review round 2 (same reviewer, after the shortening): no findings. The reviewer confirmed the shortening keeps labels distinct and matched, counts characters and handles the 20/21 character boundary, and did not contest the declined round 1 finding.
- [x] ➕ The start page hint names the bare file over its folder, since the row's label may now hold folders.
- [x] `AGENTS.md` and `CHANGELOG.md`.
- [x] Complete validation.
- [x] Move this plan to `docs/plans/completed/` before final review.

## Validation

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked` (warnings are denied in `[workspace.lints]`)
- [x] `cargo test --locked`
- [x] `cargo build --release --locked`, after the checks above pass
- [x] Manual, release binary: ten recent files show 1..9 and 0 on the start page and in File, Alt+0 and `0` open the tenth; files of the same name in different folders read as in the Issue's example.

The owner checked the release binary on 2026-10-16 and confirmed it works.

## Post-completion

None.
