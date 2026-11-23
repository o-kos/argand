# Recording metadata and scales: production acceptance evidence

Issue [#265](https://github.com/o-kos/argand/issues/265). Local scenarios below passed
in the production application. Owner manual acceptance remains pending and the PR
stays Draft. This report does not establish native Wayland, Windows or macOS behavior.

## Build and isolation

The full local gate passed: `cargo fmt --all -- --check`,
`cargo clippy --all-targets --locked`, `cargo test --locked` (968 tests, plus doc
tests), followed by `cargo build --release --locked` (2m13s). No toolkit graph,
compiler, SDK or artifact-directory changes were made.

The tested binary was `target/release/argand`, SHA256
`2ec78b154b998a0b540405b8943c0c6898e91c392eb1d4a5d006e2e26104b62e`,
7,019,824 bytes. The build used HEAD
`bb0cfeca9767c7fd0368f4450c7f8b6c9c08a65f` plus the corrected Issue implementation.
The sorted code-file content manifest SHA256 was
`0bed0ef79b295e7081b401e39ecccbc740782a1610cc4809e1d653e2d03dbe01`.
The exact manifest and binary inode/mtime are retained locally in
`target/issue265-ui/identity.json`. The code was independently reviewed in two
rounds. All three first-round findings were fixed and the corrected snapshot was
clean in round two. No product source changed during production acceptance.

All launches used task-owned Xvfb `:3`, 1280x800x24, with an actual X11 window
confirmed by `xwininfo`. Only the child received that DISPLAY, an unset
WAYLAND_DISPLAY and supported isolated XDG config/state/cache/data directories.
The application log confirmed loading that configuration. The SDK's existing X11
backend rendered the production window. Pointer/keyboard input used the available
XTest library on that display. There was no desktop launch or package installation.
The app quit normally after each restart and the verified task-owned Xvfb was
stopped at the end. Owner settings and working files were untouched.

The disposable capture was complex 16-bit I/Q, 1 MHz, 2 seconds, reference
100,100,000 Hz, starting at 2026-11-23T23:59:59.800Z. It crosses UTC midnight.
Raw logs, full captures, isolated session and cleanup evidence remain under
`target/issue265-ui/`. Local gate logs are `target/issue265-{fmt,clippy,tests-final,release}.log`.

## Executed scenarios

| Scenario and actual input | Observed result | Evidence |
| --- | --- | --- |
| Click the corner control in both orientations | Top-right and bottom-left placements open the same panel. Frequency/time label order stays invariant. No status-bar metadata control or panel action buttons. | [Top-right](evidence/top-right-panel.png), [bottom-left](evidence/bottom-left-panel.png) |
| Choose Frequency Relative and Time Absolute UTC radios, then Escape | Corner changes A/R to R/A, frequency becomes baseband, UTC/date context appears without moving the signal. Grouped reference remains 100,100,000 Hz. | [UTC rollover](evidence/utc-rollover.png) |
| Use Ctrl+Shift+T/F, main View submenu and both respective ruler context menus in both orientations | Effective scales, corner label, checked menu state and panel radios agree. Relative Seconds remains selected independently of Absolute. | Full captures 04–07 and 20–26 in local evidence directory |
| Replace timestamp draft, Escape and reopen | Original .800Z is restored. A settled capture was used after confirming the timestamp draft actually received input. | Local capture 15 |
| Replace timestamp, Enter, dismiss and Ctrl+S | Axis date changes to Nov24, dirty indicator appears before save. Saved auxi fields read back as `(2026,11,2,24,0,0,0,321)`, representing 2026-11-24T00:00:00.321Z. | Local capture16 and `saved-start.json` |
| Empty timestamp field then outside click | Blur commits clear, effective Relative/R/R appears, Absolute radio is disabled with “Recording start is not set”. Restoring start returns to the retained Absolute preference. | [Missing start](evidence/missing-start.png) |
| Resize640x600, open panel in both orientations | Reference grouping, radios and complete timestamp remain readable and within the viewport. No critical control clipping. | [Narrow panel](evidence/narrow-panel.png), local capture19 |
| Quit normally, restart with the same saved capture and isolated session | Saved .321Z and100,100,000Hz reopen. Baseband, Absolute, retained Seconds and640x600 geometry restore. Session13 contains those exact preferences. | [Restarted panel](evidence/restarted-panel.png) |
| Restart using isolated light-theme configuration | New panel, radios, grouped reference and timestamp remain readable at640px. | [Light panel](evidence/light-panel.png) |

UTC rollover ticks include a repeated month/day prefix on the new date. They were
readable in both orientations; cosmetic refinement is left to owner feedback.
Native generation/history/selection invariants are additionally checked by GPUI
regressions, including untouched sub-millisecond trim origins, Escape/blur,
mutual-panel switching and unchanged analysis generation. Sample equality across
metadata save/reopen is covered by I/O tests rather than inferred from screenshots.
Busy-state coverage is automated, not claimed as a production interaction here.
