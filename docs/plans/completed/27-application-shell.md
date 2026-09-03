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
  reversed what `AGENTS.md` and this Issue asked for at the time, on evidence
  gathered before any of the shell was written, and was agreed with the project
  owner. Both have since been corrected to say what is true.
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

- ➕ **The window position is not restored on Wayland, and not across displays
  on macOS.** Size and state are, wherever the backend reports the window state
  reliably -- which is not everywhere either; see the review section and Issue
  #38. Measured here:
  `window_bounds()` reports `x = 0, y = 0` however far the window has been
  dragged, while the compositor reports it at `(200, 150)`. Under plain
  xdg-shell that is correct -- a client has no absolute position to know or to
  set.

  `xdg-session-management-v1` exists for exactly this and has the compositor
  reapply the stored geometry. It entered wayland-protocols 1.48 on 2026-04-01
  as a **staging** protocol, not a stable one, and gpui does not implement it;
  `zed-industries/zed` has neither an issue nor a pull request for it. Raised as
  Issue #37 rather than worked around here. The acceptance criterion asking for
  position is narrowed in the Issue itself to where the toolkit reports enough
  to restore one, rather than left unmet with the Pull Request claiming
  otherwise.

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

A third round found six more, all accepted:

- **Reading the state from the `WindowBounds` variant alone is wrong on two
  backends.** Each answers with only the variants it tracks: X11 returns
  `Maximized` or `Windowed` and never `Fullscreen`, macOS returns `Fullscreen`
  or `Windowed` and never `Maximized`. The rectangle still comes from there,
  since that is what carries the size to restore to, but the state asks
  `is_fullscreen` and `is_maximized` as well.
- **The 8192 bound was in the wrong unit.** The driver compares device pixels
  and the saved size is logical, so a display at scale 3 asks for three times
  it. 4096 leaves that room.
- **The position is not restored across displays on macOS either.** gpui reports
  every display's origin as zero there and opens on the primary display unless
  told otherwise, so a window saved on a second display comes back on the first.
  The claims in `AGENTS.md` and the changelog are corrected, and Issue #37 now
  covers both platforms.
- **Four tests proved less than their names claimed.** The drag test now asserts
  that nothing is written inside the interval, rather than only checking what
  the file ends up holding; the failed-save test asserts what `save` answers;
  the search-order test checks the order `search_path` actually produces; and
  one test is renamed to what it checks.
- **The plan claimed to have been moved** to `completed/` while still sitting in
  `docs/plans/`.
- **A comment said gpui has no observer for a move or a resize.** It has one.
  The window's geometry is read from `observe_window_bounds` now instead of from
  every frame that is drawn.

A fourth round found three more, all accepted:

- **The saved rectangle became the screen once the window was maximized.**
  `WindowBounds` documents its payload as the size to restore to, but the
  backends that omit a variant have none to give: X11 hands back the bounds its
  last configure event set and macOS reads the live window frame, both of which
  are the screen while the window covers it. The shell keeps the last rectangle
  seen in the ordinary state instead, so leaving a maximized window no longer
  costs the size it would return to.
- **No logical size is provably safe.** The scale factor between logical and
  device pixels has no upper bound in any protocol -- Windows offers 500%,
  Wayland requires only that a scale be positive -- and Vulkan guarantees only
  4096 device pixels per edge. The guard is now a cap rather than a refusal, so
  a large window loses only what it must, and it binds only where no display is
  known, since a real display's own size is supportable by construction. The
  constant is 2048 and is described as a floor on the damage rather than a
  proof.
- **A session from a newer version was overwritten anyway.** It was read as
  defaults, and then a writer was built for the same path and the first
  notification wrote version 1 over it. Two things were wrong: the caller was
  never told, and the version could not be read at all, because a single parse
  of a newer layout fails on the fields this version does not know before it
  reaches the version field. The version is read on its own now, and a file
  this version cannot read is not written to.

A fifth round found one more, accepted; a sixth held it and rejected the fix,
correctly:

- **A rectangle reported as ordinary is not always an ordinary one.** gpui tells
  a client nothing about minimizing or about a transition, and two backends
  report a screen-sized window as ordinary because of it: X11 answers
  `is_maximized()` with false while a maximized window is minimized, and macOS
  calls a window maximized only once its size matches the visible frame exactly,
  so the frames of a maximize animation arrive as ordinary.

  Filtering on geometry -- refusing a report that covers a display -- was
  written, and then withdrawn on the sixth round's evidence. It misses both
  cases it was written for, because `Display::bounds()` is the full display on
  macOS while a maximized window is measured against `visibleFrame`, and on X11
  it is the whole root screen across every monitor. And it refuses an ordinary
  window whose size merely matches some *other* display -- a 1920x1080 window
  beside a 4K screen -- losing that resize outright. It was a worse answer than
  the problem, and the code is back to the plain state check.

  This is the one finding whose remedy is deferred rather than applied. Doing it
  properly means holding a candidate across a burst of reports and taking the
  previous maximized state into account, and doing it *correctly* wants gpui to
  expose the work area and the minimized state.

  A seventh round then rejected the *route* rather than the reasoning, and was
  right to: the milestone's own contract said size and state come back on every
  platform, so leaving this in the backlog would have left that claim false, and
  `CONTRIBUTING.md` reserves the backlog for findings that touch no acceptance
  criterion. So the contract is narrowed instead -- here, in `AGENTS.md`, in the
  changelog and in the Pull Request -- to say that size and state come back
  where the backend reports the state reliably, and the defect is Issue #38, a
  normal Issue rather than a backlog one, carrying the analysis and the reason
  the geometry filter cannot work.

An eighth round found the narrowing incomplete, and it was:

- **Issue #27 itself still asked for the unqualified behaviour**, so narrowing
  only the repository left the milestone's own contract claiming it. Its
  acceptance criteria are narrowed in the Issue, with both gaps named and
  pointed at their Issues. The roadmap and the early half of this plan are
  qualified too.
- **Issue #37 should not carry the `backlog` label either.** Its Wayland half is
  blocked on the toolkit, but its macOS half is not: gpui 0.2.2 already exposes
  `Window::display(cx)`, `PlatformDisplay::uuid()` -- documented as a stable
  identifier to persist across restarts -- and `WindowOptions::display_id`, so
  that half is work in `crates/app` that anyone can pick up. The label is gone
  and the Issue no longer contradicts itself about how many platforms it
  concerns.

A ninth round found three more, all accepted, none of them in the code:

- **Issue #27 still asked for fixed Git revisions**, which is the deviation the
  owner agreed to before any of this was written and the one place the narrowing
  had not reached. Its problem statement, its proposed solution and that
  criterion are corrected in the Issue, with the reason.
- **The Wayland and macOS halves of Issue #37 were still described as one.** Its
  proposed solution said nothing in `crates/app` has to change and that gpui has
  to be waited for, which is true of Wayland and false of macOS, and one
  criterion asked for macOS behaviour to be unchanged while another asked for it
  to change. `AGENTS.md` also still called it a backlog Issue, and the module
  documentation of `main.rs` promised a window that "remembers where it was".
- **The Pull Request's review summary was two rounds out of date.**

➕ Separately, and not from the review: the project owner noticed that the window
had no border and could not be resized. The first attempt at this was wrong and
the tenth round caught it. `is_resizable` was never the problem -- it is true by
default -- and `Root` already wraps what it is given in
`gpui_component::window_border`, so adding another put two shadows, two frames
and two sets of resize edges on the platforms that decorate client-side. It is
reverted.

What the attempt did establish is that this cannot be diagnosed in the nested
compositor used for everything else here: sway negotiates *server-side*
decorations, so `window_border` correctly draws nothing there and the frame is
the compositor's, which the test configuration had told it not to draw. The
window now logs which side decorates it, because that is the first thing to know
when neither a frame nor a resize edge appears.

Running it on the owner's own session answered the rest. Both there and under
X11 the compositor decorates (`decorations=Server`), so the frame is GNOME's and
`window_border` is right to draw nothing; `xwininfo` shows `mutter-x11-frames`
wrapping the window with a 14px border and a 49px title bar. What *was* broken is
the window controls: `IconName` resolves to a path such as
`icons/window-close.svg`, loaded through an asset source, and none was
registered, so the buttons rendered as blank space that still answered a click.
That is the "close button you would not notice". `gpui-component-assets` is
registered now and they draw.

➕ Left for the owner to decide, not fixed here: under server-side decorations
the compositor draws a title bar and the shell draws another inside it, with its
own minimize, maximize and close. Requesting client-side decorations instead
would give one title bar and the same appearance on every platform, which is
what Zed does with this toolkit; keeping server-side decorations integrates with
the desktop. It is a question about how the application should look, so it was
the owner's, and they chose the client-side frame: one title bar, ours, with room
for the controls a later milestone puts in it.

➕ Using the application turned up three more things, all outside this milestone
and all raised as their own Issues rather than folded in: window controls that
respond to the pointer (#39), packaging for Windows and macOS as the desktop
entry now does for Linux (#40), and the icon (#41). The first was diagnosed here:
the toolkit's `Icon` resolves its colour when it is built, from the window rather
than from the computed style of the element around it, so a hover refinement
reaches the background and stops there.

➕ Three replacement icons were drawn and rejected. What the owner wants is the
current mark with its spectrum moved into the lower part; the parameters are in
#41.

## Validation

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked` (warnings are denied in `[workspace.lints]`)
- [x] `cargo test --locked`
- [x] `cargo build --release --locked`, after the checks above pass
- [x] `cargo run -p argand --locked` opens a window on this host in under a
      second, measured rather than asserted: 0.109 s from exec to a mapped
      window.
- [x] The window reopens at its previous size and state after a restart, where
      the backend reports the window state reliably, and at its previous
      position where the platform supplies one. Measured in a nested headless
      compositor: 1000x700 and fullscreen both survived a restart, and going
      fullscreen left the size to return to intact; position did not survive,
      for the reason recorded in Decisions. X11 and macOS report a window as
      ordinary while it is not, which is Issue #38.
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
