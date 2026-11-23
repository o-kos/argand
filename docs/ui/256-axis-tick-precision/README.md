# Issue #256: frequency tick presentation evidence

Overall UI acceptance remains **incomplete**. These are direct production rendering
observations, not an exercised zoom/pan or orientation-switching scenario.

## Build and environment

- Source revision: `d270ca7267da8bff77f1d7910d1d873c702e0cae`, clean working tree at build/run.
- Binary: `/home/xokc/dp/argand.worktrees/256-axis-tick-precision/target/release/argand`.
- Binary SHA-256: `59357dfdd758cf6d06c22096f743fb9fe0df2fe09dd8d31fbe064cb3798afb00`.
- Build: `cargo build --release --locked`, exit 0 after current formatting,
  all-target Clippy and full workspace tests passed. Ordinary production app,
  linked to the existing immutable SDK, without source/test feature changes.
- Linux X11 on task-owned Xvfb `:256`, 1280×900 display and 1100×760 window.
  Xvfb identity/readiness and each child's actual `DISPLAY` were verified.
  `WAYLAND_DISPLAY` was removed only from application children. X11 selection
  matches the locked toolkit's backend policy and the native X11 window tree.
- Isolated absolute `XDG_CONFIG_HOME`, `XDG_STATE_HOME`, `XDG_CACHE_HOME` and
  `XDG_DATA_HOME` beneath a disposable task directory. There was no portable
  configuration beside the executable to shadow the isolated config. Numeric
  locale was explicitly `ru-RU`. Existing desktop settings and captures were
  not used or changed.
- Synthetic disposable interleaved f32 I/Q: 24,000 samples of a 0.5-amplitude
  complex tone, 1,500 Hz at 24 kHz. Startup options below choose the physical rate
  and reference. No signal-processing change is claimed.
- Xvfb emitted DRI3 acceleration warnings. The application rendered successfully;
  hardware acceleration, Wayland, Windows and macOS were not verified.

## Production observations

Each view was opened as a separate process through supported configuration/session
fields, allowed to render, captured and inspected. This did not exercise changing
orientation or scale within an already running application.

| View | Startup and saved presentation | Observed result | Evidence |
| --- | --- | --- | --- |
| Horizontal, dark | `--raw iq_f32@24k --center 12.59M`, horizontal session | Right MHz ruler includes `12,589`, `12,590` and `12,600` with three fractional digits and readable spacing | [Horizontal](horizontal.png) |
| Vertical, light | Same data/options, vertical session | Bottom MHz ruler includes `12,588`, `12,590` and `12,600`, with the same fractional precision | [Vertical](vertical.png) |
| Integer crossing, dark | `--raw iq_f32@200k --center 13M`, horizontal session | Exact `13` has no fraction, beside `13,01` and `12,99`, with MHz caption | [Integer crossing](integers.png) |

The displayed time rulers and status-bar standalone frequency readouts retain
independent formatting. Only shared frequency tick labels changed.

## Automated checks and review

Formatting, `cargo clippy --all-targets --locked`, `cargo test --locked` and the
subsequent release build passed on the final source. The suite passed 958 tests
and all doc tests. Core tests cover Hz/kHz/MHz/GHz, fractional/negative/zero/integer
labels, automatic and retained schemes, collision measurement, non-ladder
0.9996 Hz and very large fractional retained steps. GUI number tests confirm
English and Russian localization. Existing other-ruler/readout tests passed.

Three independent read-only review rounds accepted two P2 findings, both corrected:
rounding a non-ladder step before choosing precision, and losing large fractional
steps through a relative tolerance or scaled multiplication. Both have explicit
regressions. No findings were rejected. The final round was clean.

## Unperformed checks and cleanup

Interactive zoom, pan and live orientation switching remain **unperformed**.
Native computer input APIs were disabled in this session, and no installed
`xdotool`, `xte`, `ydotool` or `wmctrl` input route was available. No packages were
installed and no input was injected through an alternate mechanism. Automated
held-scheme tests and these screenshots do not establish interactive acceptance.
The PR stays Draft and its plan stays active until this scenario can be exercised.

Only verified task-owned application processes and the task-owned Xvfb server
were stopped. The owner's running applications and physical displays were not
used. Host-local raw logs, launch environment records and fixture remain in the
disposable evidence directory for the current session.
