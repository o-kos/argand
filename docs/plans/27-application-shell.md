# Issue #27: Application shell: GPUI window, configuration and session state

Resolves #27.

## Overview

There is no application binary. `argand-app` does not exist, GPUI is not in the
dependency tree, and nothing reads `argand.toml`. Every later GUI milestone
rests on a toolkit pinned to a Git revision whose graphics backend is migrating
from Blade to wgpu, so whether it builds and runs on Linux, Windows and macOS is
the largest unproven assumption in the project. This milestone answers that
question and nothing else.

Add `crates/app` with the `argand` binary: a shell that opens, is configured,
and remembers itself. A window with the base theme, a placeholder panel area and
a placeholder status bar; tracing set up the way `aspec` does it; and the
persistence mechanism in full, because every later milestone adds settings to it
and retrofitting the mechanism costs more than starting with it.

Out of scope: signal handling, analysis, real panels, docking, menus.

## Context

- The workspace is `crates/{core,io,dsp,cli}`; `crates/app` is new and is the
  first member to depend on a GUI toolkit. `argand-core`, `argand-dsp` and
  `argand-io` must gain no dependency from this work.
- `aspec` sets its subscriber up in `init_tracing` in `crates/cli/src/main.rs`:
  an `EnvFilter` from the environment, falling back to a per-crate level built
  from the verbosity count, writing to stderr through `try_init`.
- Nothing in the repository reads a configuration file today, and no crate
  depends on `dirs` or an equivalent.
- The host this is developed on is Wayland with an Intel Iris Xe GPU, and the
  Linux packages GPUI needs (`libwayland-dev`, `libxkbcommon-dev`,
  `libvulkan-dev`, `libasound2-dev`, `libfontconfig-dev`) are already present.

## Decisions

- **Pins.** `gpui`, `gpui_platform` and `gpui_macros` at `zed-industries/zed`
  rev `f66ed399cdde86092af8af3dc7b418abf45f37f8`, and `gpui-component` at
  `longbridge/gpui-component` rev `f001d800867d941edce529cfb8e80b9b38ec5cb0`.
  The gpui revision is not chosen independently: it is the one that
  gpui-component's own `Cargo.lock` resolves to, so the pair is known to build
  together. gpui-component declares no `[patch]` section, so nothing has to be
  mirrored into this workspace.
- **Toolchain.** `rust-toolchain.toml` moves from `1.88.0` to `1.97.1`, which is
  what the pinned zed revision requires. Agreed with the project owner before
  the work started. The alternative, an older gpui that builds on 1.88, is
  recorded below.
- **Where the logic lives.** Configuration and session state are parsing,
  clamping and atomic writes -- no toolkit is involved, and CI cannot run a GPU
  application, so they must be testable without one. They live in `crates/app`
  as plain modules whose seam with GPUI is data: the geometry clamp takes the
  display rectangles as a slice of plain numbers and returns a rectangle, so
  every rule in the acceptance criteria is a unit test that needs no window.
- **Two files, one writer each.** `argand.toml` is a person's and the
  application only ever reads it; `session.toml` is the application's and a
  person is not expected to edit it. That is what keeps the application from
  rewriting a file someone has commented.

## Rejected alternatives

- **Pinning an older gpui that builds on Rust 1.88.** It keeps the toolchain
  where it is, but the milestone exists to find out whether *current* GPUI
  works; proving it about a revision a year old answers a question nobody asked,
  and the bump only moves to the next milestone.
- **A separate chore Pull Request for the toolchain bump.** Cleaner history and
  its own revert point, at the cost of a review cycle for a one-line change that
  nothing but this branch needs. Put to the owner, who chose to carry it here.
- **One state file.** Simpler, and wrong the first time the application saves a
  window position into a file a person has commented and ordered by hand.

## Implementation steps

- [ ] Prove the pinned toolkit builds and opens a window on this host before any
      of the rest is written; record what the spike needed.
- [ ] Raise `rust-toolchain.toml` to `1.97.1` and confirm the existing four
      crates still pass the whole gate under it.
- [ ] Add `crates/app` to the workspace with the `argand` binary, the pinned
      toolkit dependencies, and a window carrying the base theme, a title, a
      placeholder panel area and a placeholder status bar.
- [ ] Initialise tracing the way `aspec` does.
- [ ] Read `argand.toml`: beside the binary first, otherwise the platform
      configuration directory. Theme, STFT defaults, colour scheme, dynamic
      range mode and panel proportions, each falling back to a default.
- [ ] Read and write `session.toml` in the platform state directory: window
      geometry and state, written atomically through a temporary file and a
      rename, and debounced so a drag does not write per frame.
- [ ] Clamp restored geometry to the visible area of the current displays.
- [ ] Make a missing, unreadable or unknown-version state file log and fall back
      to defaults, so the application cannot fail to start because of its own
      configuration.
- [ ] Build the workspace on Linux, Windows and macOS in CI, with the Linux job
      installing the packages GPUI needs and the cargo directories cached.
- [ ] Update `AGENTS.md` and `docs/plans/IMPLEMENTATION_PLAN.md` where they
      describe `argand-app` as planned.
- [ ] Complete validation.
- [ ] Move this plan to `docs/plans/completed/` before final review.

Use `➕` for tasks discovered after implementation begins and `⚠️` for blocked tasks.

## Validation

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --locked` (warnings are denied in `[workspace.lints]`)
- [ ] `cargo test --locked`
- [ ] `cargo build --release --locked`, after the checks above pass
- [ ] `cargo run -p argand --locked` opens a window on this host in under a
      second, measured rather than asserted.
- [ ] The window reopens at its previous size, position and state after a
      restart.
- [ ] Geometry saved for a display that is gone, or that changed resolution,
      restores inside the visible area. Covered by unit tests over the clamp and
      checked once by hand.
- [ ] A deleted and a corrupted `session.toml` both start the application with
      defaults and log the reason.
- [ ] Values set in `argand.toml` are applied, and the file is unchanged after a
      run that saves session state.
- [ ] Killing the process during a window drag leaves a readable `session.toml`.
- [ ] `aspec` still renders a capture from `tests/signals/` byte-identically to
      `main`, so the toolchain bump moved no output.

## Post-completion

- Milestone 2 (#28) builds the first real panel on this shell; the settings it
  adds go into the mechanism this milestone establishes.
