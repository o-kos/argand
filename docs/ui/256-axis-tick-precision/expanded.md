# Issue #256: shared frequency presentation evidence

The owner explicitly accepted PR #268 on 2026-11-24 with the previously reported
validation limits. Executor-run interaction evidence remains **incomplete**.
These are direct production rendering
observations from five separate launches, not interactive zoom/pan, live orientation
switching, cursor/Alt interaction or editing acceptance.

## Exact build and isolation

- Reviewed and built source: `5c6ae7514e08bf2cef4a5ac5c168e1d7ce355260`, product source clean at build and all runs.
  The last capture had only pending documentation/evidence changes.
- Binary: `/home/xokc/dp/argand.worktrees/256-axis-tick-precision/target/release/argand`.
- Binary SHA-256: `f3161a1b336a3ad66ba586d804f04b3d117b535ec3280b16216481e38940a362`.
- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --locked`,
  `cargo test --workspace --locked` and then `cargo build --release --workspace --locked`
  passed. The current suite passed 970 tests and all doc tests.
- Ordinary production application, unchanged features/toolchain/backend graph,
  workspace-local output and the existing immutable shared GPUI Kit SDK.
- Task-owned Xvfb `:256`, 1280×900 display, 1100×760 application window. The server
  identity/readiness and each application's actual `DISPLAY=:256` were verified.
  `WAYLAND_DISPLAY` was removed only from child environments. Native X11 window
  trees identified the application windows captured directly with `import`.
- Separate absolute `XDG_CONFIG_HOME`, `XDG_STATE_HOME`, `XDG_CACHE_HOME` and
  `XDG_DATA_HOME` under the task's disposable directory. Numeric locale `ru-RU`.
  Existing owner configuration/session/history and captures were not modified.
- The same disposable interleaved-f32 I/Q fixture contains 24,000 samples. Startup
  raw-rate/reference options define each physical display context below. Changing
  those startup interpretations is not a claim of signal conversion or live zoom.

## Static production observations

| View | Startup interpretation and presentation | Observed common unit | Evidence |
| --- | --- | --- | --- |
| Horizontal, dark | `--raw iq_f32@24k --center 12.59M` | kHz ruler, `24 kHz` sample rate and `12 590 kHz` reference; grouped tick integers remain bare | [Horizontal](expanded-horizontal.png) |
| Vertical, light | Same rate/reference, vertical session | kHz bottom ruler, `24 kHz` sample rate and `12 590 kHz` reference; labels and unit stay within the window | [Vertical](expanded-vertical.png) |
| Coarse integer crossing, dark | `--raw iq_f32@2M --center 13M` | MHz ruler, `2 MHz` sample rate and `13 MHz` reference; exact tick `13` is bare beside `12,9` and `13,1` | [Integer crossing](expanded-integers.png) |
| Fractional kHz, dark | `--raw iq_f32@1k --center 12.58255M` | kHz ticks include `12 582,55` and `12 582,50`, integer `12 583` is bare, sample rate `1 kHz` and reference `12 582,55 kHz` use the same unit | [Fractional kHz](expanded-fractional.png) |
| Fine Hz floor, dark | `--raw iq_f32@200 --center 12.59M` | Hz ruler with grouped values around `12 590 000`, `200 Hz` sample rate and `12 590 000 Hz` reference; long labels fit the measured gutter | [Hz floor](expanded-fine.png) |

All five images were inspected. No frequency-label clipping or mixed application
display units was observed. These views establish static startup rendering only.
Property panels, selection fields, cursor/Alt badges and lifecycle interaction
remain unexercised in the production application in this environment.

## Automated checks and independent review

The current 970-test gate includes approved unit examples, shared fractional
precision and bare integers, exact canonical frequency drafts, Hz bounds/steps,
common-unit selection and retained panning, first accepted geometry, bounded layout
near thresholds in both orientations, metadata/CLI context and JSON preservation.
Review corrections have regressions for selection draft `10.2 kHz = 10200 Hz`
across a 600-to-601-pixel unit threshold, reference resize/commit/revert/new-document
cleanup, stale same-document acquisition release, integer endpoint/interior
fractional Alt badge bounds and the public tick/caption pair.

Expanded-scope independent read-only review concluded clean in two rounds. One P1
and two P2 findings were accepted and fixed, none rejected: editing transaction
unit coherence, fractional Alt badge measurement bounds, and public axis caption
unit coherence. The earlier initial-scope three-round record is separate historical
evidence and does not substitute for this expanded review.

## Remaining acceptance and cleanup

Interactive production zoom/pan, live orientation switching, cursor/Alt and frequency
editing scenarios remain **unperformed**. Native computer input APIs are disabled,
and no installed permitted `xdotool`, `xte`, `ydotool` or `wmctrl` input route is
available. No packages or alternate input mechanism were introduced. Automated
interaction tests and static captures do not establish owner acceptance. The owner accepted these reported limits as the final task decision. This does
not establish that the unavailable interaction scenarios were executed. The
[accepted plan](../../plans/completed/256-axis-tick-precision.md) is complete.

All task-owned application processes and Xvfb were stopped after capture. DRI3
acceleration warnings did not prevent rendering. Hardware acceleration, Wayland,
Windows and macOS were not validated by these X11 captures.
