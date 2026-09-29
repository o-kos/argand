# #124 integration: native evidence map

Where each #124 validation case was verified natively, for
[#133](../../plans/completed/133-ui-integration.md). Linux only, by owner decision; Windows and
macOS rely on `ci/full`, and problems found there become their own Issues.

## Why the integrated build is not re-run

Issue #133 changes no shipped behaviour. Its code changes are the removal of the
example from #126 (never part of the binary), `chrome` moving from a one-module
library into the binary with the same code, the removal of `Frame::content_bounds`
(unused since #131), and a headless test. The integrated product is therefore the
sum of the stages the owner already verified natively, each on the release build of its final revision
on Ubuntu (Wayland). The owner decided on 2026-09-29 not to repeat the matrix.

## Evidence

| Case | What | Verified in |
| --- | --- | --- |
| R1 | One Root per window, one Argand frame, no second border or shadow | #127 ([plan](../../plans/completed/127-borderless-root.md), PR #139) |
| R2 | Edge and corner resize, maximize, restore, fullscreen, tiling, no stale resize zones | #127 (PR #139) |
| R3 | Toolbar controls act once without moving or maximizing; bare title drags and double-click maximizes | #127 (PR #139); toolbar controls again in #130 (PR #157) and #131 (PR #160) |
| F1 | Each navigation binding, both orientations, one view change and one analysis generation | #128 ([plan](../../plans/completed/128-plot-view.md), PR #143) |
| F2 | Settings editor inputs, ruler popup and application menu receive their keys, no plot navigation behind; Tab and Shift+Tab never land on toolbar or status buttons | #128 (PR #143) |
| F3 (menus) | Escape on each menu level and on the ruler menu returns focus; the next key reaches the plot | #129 (PR #155, cases C3 and C4); the stock menu's whole-chain Escape in #131 (PR #160), where Home and End no longer apply by owner decision |
| F3 (select and editor) | Escape closes an open select list first and reaches the settings editor only on the second press | Headless only: `settings_editor.rs` tests in #133; the same toolkit behaviour natively in the #126 dialog probe ([dialog results](../126-compatibility/dialog-results.md)), not in the production editor |
| P1 | Wheel, click and drag over the application and ruler menus leave the plot unchanged | #129 (PR #155, cases C1 and C2); the stock menu and its backdrop in #131 (PR #160) |
| P2 | Hints over the plot: arrow cursor, no readout or Alt guides, restored without a mouse move | #129 (PR #155, cases A1 to A6 and E1 to E3) |
| P3 | Drag released outside, or interrupted by a menu, the chooser, the settings window, a window switch or a document replacement, ends | #128 (PR #143), #129 (PR #155, cases B1 to B6) |
| O1 | Ready status clears on click, wheel and key without consuming them, including over a menu | #127 (PR #139), #129 (PR #155, case D1) |
| G1 | Progressive updates, resize, deep zoom and orientation leave no stale image or upload backlog, including a file opened mid-flight | #128 (PR #143) |
| S1 (settings) | Settings preview then Cancel restores view, frequency and labels; accept persists across a restart | #127 (PR #139), #128 (PR #143) |
| S1 (file replacement) | Opening another file while old frames are in flight replaces the picture cleanly | #128 (PR #143, G1) |
| Toolbar and zoom pairs | Standard buttons and their states, both orientations, as the PR #157 per-case table records them; themes are not recorded there | #130 (PR #157) |
| Application menu | Stock menu, keycaps, recent rows, placement; accepted by the owner without per-case rows | #131 (PR #160) |
| Minimap | Fixed size, no splitter, the default and a configured `minimap_size` natively; both units by tests | #132 (PR #162) |

## Not exercised

| Case | Status |
| --- | --- |
| IME composition in the settings editor's numeric fields | Not exercised on any stage; waived by the owner on 2026-09-29 |
| Clipboard, undo and redo, invalid numeric input and Enter or blur validation inside the numeric fields | Covered only as key routing in F2 (#128); not exercised as editing behaviour; waived on 2026-09-29 |
| Escape on an open select list and on the settings editor in the production window | Headless only (see F3); native waived on 2026-09-29 |
| Displayed and requested settings labels agreeing after a file replacement, and the range resetting to the configuration on opening a file | Covered by toolkit-free tests of `document.rs` and `settings.rs`; not recorded natively; waived on 2026-09-29 |
| Every orientation, theme and display-scale combination as a separate row | Stages checked both orientations and themes where their change touched them, on one display scale; separate rows per combination waived on 2026-09-29 |
| Resize and navigation responsiveness against the pre-migration baseline `c176ad2` | Not measured; each stage was accepted without a perceived regression; waived on 2026-09-29 |
| Windows and macOS native interaction, frame and appearance | Not exercised; owner decision on 2026-09-29, `ci/full` only |
