# Issue #27: Application shell: GPUI window, configuration and session state

Resolves #27.

## Overview

Before this branch there was no application binary. `argand-app` did not exist,
GPUI was not in the dependency tree, and nothing read `argand.toml`. Every later
GUI milestone rests on a toolkit whose graphics backend is migrating from Blade
to wgpu, so whether it builds and runs on Linux, Windows and macOS was the
largest unproven assumption in the project. This milestone answers that question
and nothing else. It is answered: the window opens on this host in 0.109 s and
the workspace builds on all three platforms in CI.

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

- ➕ **The toolkit comes from crates.io, not from Git.** `gpui = "0.2.2"` and
  `gpui-component = "0.5.1"`, with `Cargo.lock` committed as always. This
  reverses what `AGENTS.md` and this Issue both ask for, on evidence gathered
  before any of the shell was written, and was agreed with the project owner.
  Three things decided it:

  - **Pinning both to fixed revisions does not build.** A spike pinning `gpui`
    to zed rev `f66ed39` alongside `gpui-component` from Git produced two copies
    of `gpui` in the tree -- `f66ed39` from this workspace and `97b1e64` from
    gpui-component -- because gpui-component declares
    `gpui = { git = ".../zed" }` with no revision and so floats on the default
    branch. Cargo treats Git sources at different revisions as different
    packages, and the build failed on `Styled` from one not being `Styled` from
    the other. The Issue's wording cannot be satisfied literally.
  - **`gpui` has no repository of its own.** It lives in the zed monorepo, so a
    Git dependency clones all of zed: over 400 MB, on which cargo's own Git
    transport already timed out here and needed `net.git-fetch-with-cli`. That
    is a fragile fetch in front of every clone and every CI job, for a
    dependency whose exact version `Cargo.lock` pins either way.
  - **The rule's premise has expired.** `AGENTS.md` pins to Git *because*
    "their crates.io releases lag behind development". Both are published now:
    `gpui` at 0.2.2 and `gpui-component` at 0.5.1, which requires `gpui ^0.2.2`
    from the same registry, so one source resolves the whole graph.

  `AGENTS.md` is corrected in this branch to say what is now true and why.
  Moving to Git later, if a fix lands only at tip, is an edit to one manifest.

- ➕ **`gpui_platform` is not published**, so the entry point is `gpui`'s own
  `Application::new()` rather than `gpui_platform::application()`. That is what
  gpui-component's own example uses at the `v0.5.1` tag.

- ➕ **The window position is not restored on Wayland, and cannot be by this
  toolkit.** Size and state are, on every platform. Measured here:
  `window_bounds()` reports `x = 0, y = 0` however far the window has been
  dragged, while the compositor reports it at `(200, 150)`. Under plain
  xdg-shell that is correct -- a client has no absolute position to know or to
  set.

  `xdg-session-management-v1` exists for exactly this and has the compositor
  reapply the stored geometry. It entered wayland-protocols 1.48 on 2026-04-01
  as a **staging** protocol, not a stable one, and gpui does not implement it;
  `zed-industries/zed` has neither an issue nor a pull request for it. Raised as
  backlog Issue #37 rather than worked around here, because the missing piece is
  in the toolkit. The acceptance criterion asking for position is met where the
  platform supplies one, which is X11, Windows and macOS.

  Two claims made while chasing this were wrong and are recorded so they are not
  made again. `wayland/window.rs:419` zeroes the *surface-local* geometry passed
  to `set_window_geometry`; it is not where the screen origin is discarded. And
  `wl_output` is a registry global advertised at connection, not something a
  client learns only once a surface has entered an output -- the empty early
  `cx.displays()` is gpui's event ordering, not a protocol rule.

  Two things came out of chasing this and are fixed on this branch. `place()`
  discarded the whole saved rectangle when no display was known, losing the
  *size* along with the position -- and on Wayland that is the ordinary case,
  because gpui has not processed the display globals by the time the placement is
  decided, so the first window is always placed before any display is known. And
  the window is opened from a spawned task rather than straight from `run`,
  following the toolkit's own examples.
- **Toolchain.** `rust-toolchain.toml` moves from `1.88.0` to `1.97.1`, which is
  what the toolkit requires -- the zed revision `gpui-component` builds against
  pins that channel. Agreed with the project owner before the work started, and
  the declared `rust-version` follows it, because a lower minimum than the
  pinned toolchain is a claim nothing here builds or tests. Two lints that did
  not exist in 1.88 fire on existing code and are fixed rather than suppressed.
  The renders are unchanged: every capture in `tests/signals/`, in both
  orientations and all eight panel combinations, is byte-identical to what
  1.88.0 produced. The alternative, an older gpui that builds on 1.88, is
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
- **Git sources with no revision in the manifest**, declared exactly as
  gpui-component declares them, so that Cargo unifies the source and
  `Cargo.lock` holds the revisions. It builds and it tracks tip, but it keeps
  the 400 MB zed fetch in front of every clone and every CI job, and a manifest
  that pins nothing is a worse record of intent than a version requirement.
- **Git sources plus `[patch]`** to force a single `gpui`. It satisfies the
  letter of the Issue, at the cost of resolution machinery that is itself a
  source of surprises, and it still clones zed.
- **A separate chore Pull Request for the toolchain bump.** Cleaner history and
  its own revert point, at the cost of a review cycle for a one-line change that
  nothing but this branch needs. Put to the owner, who chose to carry it here.
- **One state file.** Simpler, and wrong the first time the application saves a
  window position into a file a person has commented and ordered by hand.

## Implementation steps

- [x] Prove the toolkit builds and opens a window on this host before any of the
      rest is written. ➕ The first spike failed, and that failure is why the
      dependency source changed; see Decisions.
- [x] Raise `rust-toolchain.toml` to `1.97.1` and confirm the existing four
      crates still pass the whole gate under it.
- [x] Add `crates/app` to the workspace with the `argand` binary, the pinned
      toolkit dependencies, and a window carrying the base theme, a title, a
      placeholder panel area and a placeholder status bar.
- [x] Initialise tracing the way `aspec` does.
- [x] Read `argand.toml`: beside the binary first, otherwise the platform
      configuration directory. Theme, STFT defaults, colour scheme, dynamic
      range mode and panel proportions, each falling back to a default.
- [x] Read and write `session.toml` in the platform state directory: window
      geometry and state, written atomically through a temporary file and a
      rename, and debounced so a drag does not write per frame.
- [x] Clamp restored geometry to the visible area of the current displays.
      ⚠️ Exercised by unit tests rather than on Wayland, where the displays are
      not known yet when the first window is placed; see Decisions.
- [x] Make a missing, unreadable or unknown-version state file log and fall back
      to defaults, so the application cannot fail to start because of its own
      configuration.
- [x] Build the workspace on Linux, Windows and macOS in CI, with the Linux job
      installing the packages GPUI needs and the cargo directories cached.
- [x] ➕ Correct the dependency rule in `AGENTS.md`, which tells a reader to pin
      the toolkit to Git revisions for a reason that no longer holds.
- [x] Update `AGENTS.md` and `docs/plans/IMPLEMENTATION_PLAN.md` where they
      describe `argand-app` as planned.
- [x] Complete validation.
- [x] Move this plan to `docs/plans/completed/` before final review.

Use `➕` for tasks discovered after implementation begins and `⚠️` for blocked tasks.

## Review

One round with `codex exec -s read-only` so far, six findings, all accepted:

- **A corrupt session could leave the application without a window.** A finite,
  positive, absurd size passed the usability check and, with no display to clamp
  against, reached the toolkit; and the spawned task's error was dropped by
  `detach()`, so a failed `open_window` ended the run silently. Geometry is now
  bounded, and the task reports and quits.
- **The configuration surface was smaller than the Issue promised** and the plan
  had already ticked it. Colour scheme, dynamic range and the transform defaults
  are now read, by the same names the CLI parses, through the same `FromStr`.
  Values that parse but cannot be used -- a transform size that is not a power
  of two, a waveform share of zero or one -- are replaced individually with a
  log line, so one bad value does not cost the rest of the file.
- **A failed write was recorded as a successful one.** `Session::save` swallowed
  its error while `Writer` cleared what it had not written, so a transient
  failure lost the update for good. The result is answered and the value stays
  pending.
- **Three pieces of evidence about Wayland were misstated.** All three are
  corrected above and in Issue #37; the conclusion they were offered for is
  unchanged.
- **The last offer did not always win.** A window dragged away and back inside
  one interval left the position it had already left pending, and that is what
  the next write recorded.
- **The plan contradicted its own branch.** Synchronised.

A second round found six more, all accepted:

- **The geometry guard was still too loose to prevent a crash.** The graphics
  backend does not refuse an oversized surface with an error: it logs that the
  request is outside the surface capabilities and then unwraps the swapchain it
  could not create, so the panic happens inside the call that opens the window.
  The edge bound came down to 8192 and the origin is bounded too.
- **The wider configuration was parsed and then dropped.** It is carried on the
  shell now, and the panel split is applied to the layout, so what has somewhere
  to act is applied and what has not is retained rather than lost. The plan says
  which is which instead of claiming both.
- **A failure that persisted retried on every frame.** The interval ran from the
  last success, which a failing write never updates. It runs from the last
  attempt.
- **Window state and restore geometry were read from two places** that can
  disagree, since `is_maximized` and a separately fetched rectangle are not
  updated together on every platform. Both now come from the one `WindowBounds`,
  which already pairs the state with the rectangle to restore to.
- **The roadmap still told a reader to pin the toolkit to Git**, and the
  validation checkboxes were unticked. Both synchronised.
- **No changelog entry**, for a Pull Request that adds a whole binary.

## Validation

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked` (warnings are denied in `[workspace.lints]`)
- [x] `cargo test --locked`
- [x] `cargo build --release --locked`, after the checks above pass
- [x] `cargo run -p argand --locked` opens a window on this host in under a
      second, measured rather than asserted: 0.109 s from exec to a mapped
      window.
- [x] The window reopens at its previous size and state after a restart, and at
      its previous position where the platform supplies one. Measured in a
      nested headless compositor: 1000x700 and fullscreen both survived a
      restart; position did not, for the reason recorded in Decisions.
- [x] Geometry saved for a display that is gone, or that changed resolution,
      restores inside the visible area. Covered by unit tests over the clamp and
      checked once by hand.
- [x] A deleted and a corrupted `session.toml` both start the application with
      defaults and log the reason.
- [x] Values set in `argand.toml` are read, the theme and the panel split are
      applied, and the file is unchanged after a run that saves session state.
      The settings with nothing yet to act on are carried on the shell rather
      than dropped; see the review note.
- [x] Killing the process during a window drag leaves a readable `session.toml`.
- [x] `aspec` still renders every capture in `tests/signals/` byte-identically to
      `main`, in both orientations and all eight panel combinations, so the
      toolchain bump moved no output.

## Post-completion

- Milestone 2 (#28) builds the first real panel on this shell; the settings it
  adds go into the mechanism this milestone establishes.
