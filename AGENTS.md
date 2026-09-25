# AGENTS.md — Argand

Argand is a cross-platform signal editor and analyzer focused on **I/Q (complex)** signals. Its UX is inspired by ocenaudio: waveform and spectrogram views with selection-based editing. Unlike an audio editor, Argand also handles complex samples and radio-frequency semantics.

> This file is shared project context for coding agents. Keep it synchronized with the code: update it whenever an architectural decision or invariant changes.

## Purpose and audience

Argand is designed for viewing, navigating, editing, and performing spectral analysis on recorded signals, including SDR/RF captures. Its target user works with custom or unusual formats and needs a fast, lightweight native application rather than a heavy framework.

## Technology stack

- **Language:** stable Rust organized as a Cargo workspace.
- **GUI:** GPUI Kit 0.6.x by Longbridge, the application facade over the GPUI 0.3.x family and gpui-component 0.6.x. All three are Apache-2.0 licensed.
- **Rendering:** wgpu, the GPUI 0.3.x graphics backend. Spectrograms are rendered as RGBA textures through image elements; waveforms use paths and quads. GPUI does not expose a public hook for a custom shader or wgpu pass within a frame; see "Rendering boundary."
- **DSP:** rustfft and realfft.
- **Configuration:** serde and TOML.
- **Logging:** tracing.

## Core requirements and invariants

1. Support Linux, Windows, and macOS.
2. Ship one binary plus configuration with minimal external dependencies. Take the GUI toolkit as one crates.io dependency, `gpui-kit`, and let `Cargo.lock` fix the graph.
3. Keep cold startup and the first waveform or spectrogram frame fast.
4. Provide the core display and editing operations available in ocenaudio.
5. Provide a separate detailed spectrum window.
6. A plugin system for formats and processing, with or without visualization, is planned around a separate worker process. It is currently deferred; see the end of `docs/plans/IMPLEMENTATION_PLAN.md`.

## Domain-specific rules

- A signal can be **real** or **complex** I/Q baseband.
- I/Q samples use interleaved `f32` values by convention: `I, Q, I, Q, ...`.
- A complex signal has a two-sided spectrum from `-Fs/2` to `+Fs/2` and requires `fftshift`. A real signal has a one-sided spectrum from `0` to `Fs/2`.
- Frequency axes use physical hertz derived from `center_frequency` and the true capture `sample_rate`. The sample rate may be in the megahertz range and must not be treated as an audio sample rate. Audio playback is not a project goal.
- The I/Q waveform view combines the I and Q channels; spectrogram mode is available separately.
- Editing operates on the underlying sample array. Cut, copy, and paste must keep I and Q together.

## Cargo workspace architecture

- **argand-core:** domain types such as `Signal`, real or complex sample metadata, `Selection`, and units. It also owns toolkit-independent render view models such as `WaveformEnvelope`, `DbGrid`, `SpectrogramTile`, and primitive lists, and the axis tick layout in `argand-core::axis`, which measures candidate labels through the `LabelMeasure` trait so that every front end places its marks by one policy. It must not depend on GUI or heavy DSP code.
- **argand-dsp:** STFT and spectrogram generation, Welch PSD, window functions, min/max pyramid construction, resampling helpers, and frequency shifting. Shading is a separate public step over a `DbGrid`, so changing the colour scheme or the dynamic range recolours values a caller already holds instead of running the transform again. It depends on rustfft and must not depend on GUI code.
- **argand-io:** WAV and other format readers behind the `FormatReader` interface: probe, open, and read through a lazy sample source. This is the future connection point for custom formats through the worker.
- **argand-edit:** a planned editing engine using a piece table over the memory-mapped original and inserted buffers, a command stack for undo and redo, and a clipboard.
- **argand-app:** the application binary, `argand`, using the GPUI Kit facade over GPUI and gpui-component. It opens a signal file, analyses it on a thread of its own, and draws the spectrogram with axes. Its modules divide by whether they name a toolkit: `config.rs`, `session.rs`, `document.rs` and `analysis.rs` do not and are tested without a window, while `shell.rs`, `chrome.rs`, `axes.rs` and `spectrogram.rs` do. It will further own cursors, selections, scrolling, transport, and the detailed spectrum window. This is the single shipped binary alongside `aspec`.
- **argand-worker:** a planned processing worker binary that loads C ABI libraries and communicates through a stdio protocol. It is deferred.
- **argand-abi:** planned C ABI contract and protocol types. It is deferred.

## Rendering boundary

GPUI types must never leak into `argand-core`, `argand-dsp`, `argand-io`, or `argand-edit`. Those crates return textures, envelopes, primitive lists, and other toolkit-neutral data. Only `argand-app` converts them into GPUI images, quads, and paths.

This boundary keeps the toolkit replaceable. If GPUI proves too restrictive for a custom GPU canvas, the fallback is egui/eframe with wgpu without rewriting the core crates.

## Performance strategy

- Memory-map input files and parse them lazily instead of loading complete captures into RAM. `SampleSource` exposes optional access-pattern and bounded prefetch hints; they never change its cursor or decoded values. The mmap reader uses Unix advice to overlap sparse preview reads.
- Build a multi-level min/max peak pyramid for waveform level-of-detail selection.
- The GUI retains a toolkit-neutral `Overview` in its document worker: at most 4096 time cells by 2048 frequency cells, keeping native FFT bins when they fit, and up to 65536 sample min/max cells for the waveform. Every STFT frame is accumulated. `Overview::render` rebins values before shading: conservative overlapping maxima or overlap- and frame-count-weighted linear powers. Cache boundaries are the declared resolution; sub-cell structure cannot be recovered. The spectral accumulator is bounded to 32 MiB (Peak) or 64 MiB (Mean power), excluding display buffers, waveform, PSD/FFT storage and input mapping. Ordinary DSP/CLI rendering still reduces directly to requested pixels; legacy dB Mean is not an overview reducer.
- Display dimensions use a coalescing mailbox and do not change the analysis generation while the sample range stays unchanged, including during splitter dragging and progressive refinement. At extreme sample counts, growing the plot width can widen the range to preserve representable time coordinates (#30). File/range/FFT/reducer changes invalidate analysis. Colour and dynamic-range changes advance only the display revision; explicit style refreshes bypass the periodic snapshot interval between bounded batches. The worker renders the newest requested size from the same overview; completed redraws preserve analysis duration and resolved colour range. Only image-sized spectral snapshots cross to the UI; the independent minimap retains its bounded full-capture envelope.
- Use a piece table so cuts and pastes remain `O(1)` for multi-hour files.
- Keep expensive work off the UI thread by using rayon or dedicated worker threads.

## Build and run

- After the GUI scaffold exists, run it with `cargo run -p argand --locked`; build releases with `cargo build --release --locked`.
- Take the GUI toolkit as one crates.io dependency, `gpui-kit`, with the version requirement in `Cargo.toml`, and let the committed `Cargo.lock` fix the exact graph. `gpui-kit` selects and re-exports the compatible GPUI 0.3.x family, gpui-component and the toolkit assets; do not add direct `gpui`, `gpui-component` or `gpui-component-assets` dependencies beside it, because a type from one copy of the graph is not that type from the other.

  This rule used to name `gpui` and `gpui-component` separately on crates.io, and before that, fixed Git revisions. The separate selections drifted out of compatibility, which is the mistake `gpui-kit` exists to prevent. Do not go back to naming the layers, and do not `[patch]` the registry source without a reason that names what is only available at tip.
- Commit the complete dependency graph in `Cargo.lock`; do not commit `vendor/`. Run `cargo fetch --locked` before an offline build, then use `cargo build --frozen`.
- Run rustfmt and Clippy for every change: `cargo fmt --all -- --check` and `cargo clippy --all-targets --locked`.
- Clippy warnings are errors, and that is a property of the repository, not of the command line. `[workspace.lints]` sets `warnings = "deny"`, so a plain `cargo clippy` fails locally exactly as it fails in CI. Never rely on a `-D warnings` flag to make a check strict; a check that is strict only when someone remembers a flag is not a check.
- Run the full local gate before every push, not only before a Pull Request: `cargo fmt --all -- --check`, `cargo clippy --all-targets --locked`, `cargo test --locked`. Install the repository hook once with `git config core.hooksPath .githooks` and it runs for you.
- A real GPU is required. Software rendering and some virtual machines may degrade performance.

## Engineering conventions

- Prefer standard gpui-component controls and GPUI interaction mechanisms. Try supported styling, composition and configuration before implementing an ordinary control yourself; appearance differences alone do not justify replacing its behavior. Every new or retained custom UI control must have a separate justification in the active plan: name the standard alternative, the verified limitation in the locked version, why composition is insufficient, and the required focus, keyboard, pointer, disabled-state and regression coverage. Obtain the owner's agreement before introducing a custom replacement or changing product behavior to avoid a toolkit limitation. Domain-specific canvases (spectrograms, waveforms, axes and minimaps) are legitimate custom drawing, not permission to reimplement buttons, text editing or menus. Review this inventory when the relevant UI is reorganized; remove superseded custom implementations rather than maintaining parallel paths.
- Use `thiserror` in libraries and `anyhow` in binaries. Non-test code must not use `unwrap` or `expect` on external data. Panics must not unwind across FFI boundaries.
- Read `argand.toml` at startup from beside the binary or from the platform-specific configuration directory.
- The default `theme = "system"` follows GPUI's OS appearance at startup and through a retained main-window appearance subscription. Theme changes refresh all windows, including the settings editor. Explicit `dark` / `light` stay fixed; interface theme does not change the spectral palette.
- `crates/app/assets/argand.toml` is the single complete, commented default configuration for distribution. Keep it synchronized with `Config::default()` and all supported keys; its test parses without repairs. Package delivery is tracked in #86. Installations must preserve existing user configuration and must not place a default beside a system-installed executable, which would shadow the user file under the portable-first search policy.
- Express units and axes in physical hertz and seconds. Never confuse audio sample rates with the true RF capture sample rate.
- Direct conversation with the project owner must be in Russian only.
- All repository content and project communication outside that direct conversation must be in English. This includes source comments, user-facing messages, logs, documentation, branch names, commit messages, Pull Request titles and descriptions, issues, and release notes.
- Write code comments only when the intent is not evident from the code itself, and keep them concise.
- Do not write nested, multi-level, opaque `if` chains. A reader must be able to tell what a branch does without holding three conditions and a later early return in their head at once. No lint catches this: it is a review obligation, and the two shapes below are the ones that keep appearing.
  - **One flag tested twice in two different roles.** Here `args.quiet` gates stdout in the `else if`, then gates stderr as an early return three lines later, so it reads like a duplicate that has to be proven not to be one:

    ```rust
    if args.json {
        println!("{}", report.to_json());
    } else if !args.quiet && reporting.echo_paths {
        if let Some(output) = &report.output {
            println!("{}", output.path);
        }
    }
    if args.quiet {
        return;
    }
    ```

    Decide once, in one place. Name the outcome — an enum, a `match`, or a single early return at the top — and let each branch do one thing.
  - **A condition repeated inside its own `else`.** Here `reporting.batch` is tested in the `if` and again in the `else`, which forces the reader to work out that the second test is not the same question as the first:

    ```rust
    if reporting.batch && args.verbose == 0 {
        report.write_compact(&mut stderr, index, reporting.total).ok();
    } else {
        if reporting.batch && index > 1 {
            writeln!(stderr).ok();
        }
        report.write_human(&mut stderr).ok();
    }
    ```

    Hoist the shared condition, or split the function so each half has one job.
- Every lint suppression must be agreed with the project owner before it is pushed. This covers `#[allow(...)]`, `#[expect(...)]`, `-A` flags, and lint levels relaxed in `Cargo.toml` or `clippy.toml`. Refactor first; a suppression is the last resort, never the quick one. When one is genuinely unavoidable, ask for it explicitly, state what was tried, and use `#[expect(..., reason = "...")]` so it fails once it stops being needed. A suppression that nobody re-reads turns the whole gate into a formality.
- Never state or imply in commit messages, Pull Requests, issues, or release notes that an AI tool was used during development.

## Agent roles and model selection

Claude plans and arbitrates; the `codex` CLI implements and reviews. The two roles
never share a model, so a review is never the implementer agreeing with itself.

- Claude owns the plan in `docs/plans/`, the Issue and Pull Request text, the
  architecture and the acceptance reasoning, and talks to the owner. It runs the
  local gate itself and reads the diff before showing anything to the owner.
- Codex implements against that plan and reviews the Draft Pull Request. It commits
  code, tests and the plan checkboxes its own work completes.
- Claude arbitrates a technical disagreement between implementer and reviewer and
  records the decision in the Pull Request. Anything that changes product behaviour,
  UX or an Issue's acceptance criteria goes to the owner instead.
- Three review rounds are the limit. Substantive findings still standing after the
  third mean the task was stated badly, so Claude rewrites the plan rather than
  starting a fourth round.

Declare the class in the plan before the implementer is given the task, never after
seeing the result.

| Class | Applies to | Implementer | Reviewer |
| --- | --- | --- | --- |
| A | Concurrency and work scheduling, analysis generations, retention, caches, GPU texture lifetime, DSP correctness, public `argand-core` / `argand-dsp` API, security or data safety; or an expected diff above roughly 400 lines or 5 code files | `gpt-5.6-sol` high | `gpt-5.6-terra` high |
| B | A feature or fix in one or two GUI modules with local, known invariants and concrete acceptance criteria | `gpt-5.6-sol` high | `gpt-5.6-terra` high |
| C | Documentation, README, configuration, renames, single-file changes with no behavioural consequence | `gpt-5.6-terra` high | `gpt-5.6-luna` high |

`gpt-6-astra` is not scheduled by class. Ask the owner for it, with the reason, and
use it only once agreed. It costs roughly 1.4M tokens per run against 0.9M for
`gpt-5.6-sol` and 0.55M for `gpt-5.6-terra`, measured over this repository's own
sessions, and the model is not what decides most outcomes: a run wasted on a mistaken
plan costs the same as a useful one.

A change that touches no code -- documentation, a plan, configuration, release notes
-- is reviewed by `gpt-5.6-luna`. Reserve the stronger reviewers for code.

Any of class A's signals puts the Issue in class A. The file count considers code
only: the plan, `CHANGELOG.md` and `AGENTS.md` change in nearly every Pull Request
and say nothing about how hard the work is. Use the same implementer and reviewer
model and reasoning effort for every round of one Pull Request.

The cheapest saving is not the model. Read the parts of a framework a plan depends on
before writing it, so an implementer does not spend a full run discovering the plan
was wrong, and do not open a review round on a change with nothing to review.

## Git workflow

- Follow the complete process in `CONTRIBUTING.md`.
- Every implementation must have a GitHub Issue, a focused branch, and a versioned plan in `docs/plans/`.
- Open a Draft Pull Request early and link it to the Issue with a closing keyword. Keep it Draft during iterative owner feedback, including small UI changes; present locally validated work without waiting for remote CI. Ready requests the final full gate, and every subsequent push to a Ready PR repeats it. Return to Draft before another iteration.
- Linux CI uses Ubuntu 24.04 and installs GPUI dependencies through `.github/scripts/install-ubuntu-packages.sh`, using only the runner's `ubuntu.sources`, fresh indexes and a private package archive cache. Signature/hash checks remain enabled; any required-source update error is fatal. Signed local-repository regression tests run before installation.
- Draft CI runs Linux formatting and Clippy plus CI-policy tests, publishing `ci/quick`. Ready PRs, main pushes and manual runs run the full three-platform matrix. Require `ci/full` on the current PR revision before merge; it must reflect explicit success from Linux, Windows and macOS, never skipped jobs. Preserve strict up-to-date branch protection.
- Update plan checkboxes in the commits that complete the corresponding work. Do not record commit hashes in plans.
- Route findings discovered during implementation or review in this order, as detailed in `CONTRIBUTING.md`: first keep branch regressions and anything required by the active Issue in the current work; otherwise raise material or urgent unrelated problems as normal Issues; only otherwise create a separate Issue labelled `backlog` for a minor-impact, pre-existing, non-urgent problem that is unrelated to the active objective and does not affect current functionality or acceptance criteria. Link the source Issue or Pull Request, and never use backlog to defer security, correctness, or data-safety work.
- Move a finished plan to `docs/plans/completed/` before final review.
- Before the owner is asked to review a Pull Request, put it through an external review with the `codex` CLI and act on the findings. Iterate until a round returns nothing substantive. When asking the owner to review the Pull Request, always provide a brief summary of the automatic review: the findings, which were accepted and how they were addressed, which were rejected and why, and whether the final round was clean. A second reviewer that never disagrees is worth nothing: ask it to challenge the reasoning behind anything you decline, rather than to confirm it.
- Take the implementer and the reviewer for the Issue's class from "Agent roles and model selection".
- Rebuild the release binary once the standard checks pass and before the owner is asked to accept the Pull Request. Any behaviour shown to the owner must come from a binary built from the current code, never from a stale `target/release/`.
- `main` is protected. Merge only through a Pull Request using squash merge after all checks pass and all review conversations are resolved.

## Current status

- Planned UI architecture (#124): after #126 rejected the locked stock frame, the owner selected migration through GPUI Kit 0.6.x to its aligned GPUI 0.3.x family (#137), a borderless standard `Root` around the retained Argand frame (#127), focused plot ownership, and maximum reuse of standard controls. #137, #127 and #128 have landed: the graph runs on `gpui-kit` 0.6.x / GPUI 0.3.6, the production window root is `Root::bordered(false)` strongly owning `Shell` as its child, and a per-document `PlotView` owns plot focus and gestures (see "Plot ownership (#128)"). The compatibility fixture demonstrates the same composition beside the stock-Root baseline. The interactive FFT hint remains separate work in #108.
- The B0 application icon uses closed cyan/coral I/Q ribbons. The selected full-size raster and 16/24-pixel optical SVG sources live in `crates/app/assets/icons/source/`. `crates/app/assets/icons/generate.py` reproduces PNG, ICO, ICNS, scalable SVG, monochrome SVG and tray PNG assets using Pillow and rsvg-convert. The toolbar embeds the 24-pixel PNG for its 22-pixel slot. The scalable SVG embeds the approved raster; it is intentionally not a vector tracing. Monochrome/tray artwork derives from the same optical SVG paths with transparent crossing gaps. The desktop installer includes the 24-pixel theme directory. Windows resource/macOS bundle packaging is tracked separately in #40.
- The name and icon assets live in `crates/app/assets/icons/`, beside the desktop entry that names them. The set includes `argand.svg`, `argand.ico`, `argand.icns`, PNG sizes, and a monochrome glyph. A desktop environment learns the application's name and icon from `crates/app/assets/io.github.o_kos.argand.desktop`, matched against the window's application id; `install-desktop-entry.sh` beside it installs both into the user's own directories for a development build.
- `crates/app` opens a signal file, analyses it, and shows the spectrogram with time and frequency axes and a linear waveform above it. The spectrum publishes a sparse full-width preview and left-to-right refinement snapshots; the independent waveform minimap spans the full capture; time navigation is implemented by #30; selection remains deferred.
- Window composition: `Shell` owns theme font setup and Tab traversal. `chrome.rs` provides the single client frame: resize cursors belong to separate edge hitboxes measured against `Window::viewport_size()`, never the restore bounds in `window_bounds()`. Expanded windows have no resize hitboxes; free corners are rounded by 8 logical pixels. The window root is `Root::bordered(false)` strongly owning `Shell`; callbacks reach the shell through `WeakEntity<Shell>` (`WeakEntity::update_in`), and the pointer/wheel and keystroke observers that must resolve the shell of whichever window an event hit go through the root in a single centralized helper, since GPUI's `observe_keystrokes` and `intercept_keystrokes` both fire for every window rather than only the one that registered them. The retained `chrome.rs` frame stays the sole chrome, and the separate analysis settings window uses one gpui-component `Root` for standard select/input focus, Tab traversal and its own frame.
- Linux title-bar controls paint their own rounded hover/press backgrounds; Windows and macOS retain toolkit controls. File uses the same small text size as popup menus. The status bar has a combined file group and an analysis hover hint and a separate settings window, with compact duration units in the bar and h:mm:ss.mmm in the file hint. The worker reports file bytes and the reader's optional `original_sample_units` affine conversion (including gain and codec bit depth); complete waveform extrema are converted back to decoded original units, separately for I/Q. Settings and validation live in toolkit-neutral `settings.rs`; `settings_ui.rs` renders the status and hover hints; `settings_editor.rs` owns standard dropdowns and numeric fields in a separate Root-backed window. Ctrl+, (Cmd+, on macOS) opens it, Tab/Shift+Tab traverse controls, arrows/Enter operate them, and Escape closes a dropdown before cancelling the form. Ctrl+R (Cmd+R on macOS) applies available range advice, restores the full range from an explicit range when no advice remains, and does nothing on the full or automatic range, from either the main window or editor. The analysis tooltip is disabled while the editor is open. Numeric edits preview on Enter or blur; OK also validates current text in both fields before accepting. The analysis tooltip observes its owner so recommendations and effective values refresh while it remains open. `document.rs` supplies toolkit-neutral hint titles, current values and optional explanations; `shell.rs` preserves heading typography, renders aligned file-hint values with muted labels, and uses smaller muted explanations of one sentence without terminal periods. File hints use a viewport-bounded 330-pixel content width with aligned rows. Other metadata hints measure their definite width with GPUI using the rendered font sizes and weights; keep text blocks non-shrinking so wrapped text contributes its full height. The ready-state timing hint describes the latest analysis including sample reads, transforms and CPU image preparation, excluding file opening and window drawing; it is absent for other states. The whole ready status and its timing hint are transient and disappear on a mouse button press, a key press, a wheel event or the cursor readout appearing. Pointer motion outside the readout area, activation and focus changes do not dismiss them. Each accepted ready update resets dismissal without discarding the elapsed time. Window-level capture listeners observe mouse presses and wheel events, and a retained keystroke subscription covers bound actions with direct dismissal in the two existing key interceptors. Dismissal never consumes input. The cursor readout shows frequency before time and places the level in its own status-bar field. No status group is shown before a file is open. Cursor coordinates share the Alt badge formatter, following the time-ruler mode, frequency-axis unit and device-pixel precision. The minimap uses full-capture time and sample extents.
- The waveform starts at 3 rem (48 logical pixels with the default font), with a 1-logical-pixel theme-border separator spanning only the waveform plot. The time and frequency ruler baselines use the same colour as their ticks; the spectrum starts directly below the waveform separator without a top inset. The draggable boundary persists the adjusted proportion in session version 3; older sessions retain the font-relative default. `panels.rs` bounds the split, while `waveform.rs` draws one merged real or I/Q envelope without grid lines, a zero-axis line or labels, using the exact horizontal geometry measured for the spectrogram. `WaveformEnvelope::pixel_spans` shares channel merging, pixel rounding and continuity with `aspec`; The minimap amplitude scale comes from its full-capture peak, independently of spectral dynamic range. Its envelope survives spectrum replacement. Splitter gestures submit coalesced display sizes throughout the drag; they never restart analysis. The old `[panels].waveform_fraction` configuration remains readable for compatibility but does not size the panel. Progressive waveform integration is implemented by #29; synchronized time navigation is implemented by #30.
- The application's logic is deliberately free of GPUI wherever it can be, because CI has no GPU and cannot run the application. What a file is doing (`document.rs`), the thread that reads and transforms it (`analysis.rs`), what the settings ask a transform for (`config.rs`) and what is remembered between runs (`session.rs`) name no toolkit type at all. Where the axes put their marks does name one -- `axes::Frame::measure` takes a `gpui_kit::Size<Pixels>` -- but decides everything through `argand_core::axis` and a `LabelMeasure`, so it is tested against the fixture font with no window in sight. What does need a running window is `shell.rs`, frame setup in `chrome.rs` and the drawing half of `axes.rs`: a font to measure and paint labels with comes from one. `spectrogram.rs` needs neither -- it turns a transform's RGBA into a `RenderImage` and nothing else -- so it is tested like the rest.
- The analysis settings window selects Peak (MAX) or Mean power, including subsequent files; saved effective settings override configuration defaults after a restart. Session version 9 persists transform/palette settings, the time-ruler format, grid visibility and orientation, and reads versions 1 through 8, ignoring legacy dynamic range. Range and its mode reset to configuration defaults on file opening. Editor changes preview without altering saved session settings; OK commits, while Cancel/Escape/window close restore the opening values. Reset uses configuration defaults. Opening another file cancels and closes the editor before replacing the document. Changes use the same cancellable worker and retain the old picture until the preview arrives. Mean power accumulates squared, tone-calibrated spectral amplitudes in cached bin/frame cells and weights their overlap with display pixels before taking dB; it is not integrated band power. Temporal power sums use f64 and count every frame once, including preview frames. CLI `mean` retains its legacy dB average after frequency MAX. Neither reducer changes the waveform envelope, PSD, transform lattice, or normalization. The displayed settings are tracked separately from requested settings so pending work cannot mislabel the retained picture. The recommended range is the same measured value as the CLI; the GUI warning specifically requires a nonzero signal whose spectral peak is in the lower half of the absolute full-scale colour window. It is suppressed for peak-relative/auto modes; obsolete or failed deliveries never create a warning, while a failed re-analysis retains the presentation of the last accepted picture without an action. A narrower recommendation alone is not a warning. The transform summary is the settings button and owns the analysis hint. The range is a separate hinted status item with warned, corrected and full states. Its effective number, requested mode and recommendation are derived together when the accepted analysis snapshot enters the document, so the displayed state never combines parts of different updates. Warned and corrected states use the same default cursor, hover background and shared toggle action as Ctrl+R / Cmd+R; full and automatic ranges are informational only. The action retains the stricter requested-settings check. While the displayed and requested settings differ, or while re-analysis is failed, the level keeps the displayed appearance but offers no action, keycap or hover affordance.
- File opening starts after an intervening GUI frame; ruler boundaries appear before metadata, with physical labels added once the file is described. The per-document worker uses a configurable Rayon pool (by default at most eight threads), bounds automatic normalization scans to 64 MiB, and bounds its reply queue to two snapshots. Analysis changes and replies carry generations; size-only mailbox updates retain the generation but advance a separate display revision, so stale sizes are rejected before sending and on receipt. The GUI rejects obsolete analysis deliveries and the DSP cancels superseded passes between batches. The first coarse preview uses at most 128 evenly spread frames, followed by the remaining preview frames (at most 1024 overall in the GUI, independent of cache width) and sequential refinement. Preview frames belong to the final frame lattice and are accumulated once. Preview reads/transforms are batched to at most 1 MiB of sample values (or one FFT frame when larger), with cancellation between batches. GPU uploads coalesce at shell render time, so paused frames retain one pending analysis rather than accumulating retirement callbacks. Whole spectral snapshots are published at most every 50 ms, with the preview colour scale held until completion. Incremental rendering updates only output columns affected by changed cache cells or their empty followers; size changes rebin the full view. Measured sample peaks remain independent of that frozen display scale. Refinement is shown through the changing picture and status-bar progress; do not overlay a moving indicator on either panel. `SampleSource::access_pattern` is an optional I/O hint: mmap preview disables sequential readahead and page retirement, then restores them for refinement. CLI analysis and unbudgeted normalization retain their existing behaviour.
- A replaced spectrogram texture is retired after an intervening redraw, using two `on_next_frame` callbacks. The backend can destroy a released texture immediately while the preceding GPU frame still samples it -- observed on Blade, and assumed until wgpu retirement is verified; the intervening draw waits for that frame. Never replace this with an immediate `drop_image`, and always name the current window when releasing the image.
- The start page and the inline recent rows in File share `recent.rs`'s toolkit-neutral availability snapshot, displaying only verified regular files in recent order. The shared snapshot excludes the currently loaded path, recorded only after a successful open and normalized with the same function as session history. Filtering preserves saved entries and opening hints, and the previous file returns in recent order when another file opens. Availability refreshes on startup (including a file argument), successful file opening, window activation and File-menu opening. Each refresh considers at most ten saved entries and coalesces already pending paths. Distinct new paths are checked independently even when old paths are blocked; there is no global worker cap that could let old network mounts starve the current history. Results identify paths, so reordered history cannot misapply a result or restore old opening hints. The menu takes the latest completed snapshot when opened and keeps its entries stable until dismissal; pending availability changes appear on reopening. Both numbered start-page links and Alt+1 through Alt+9 use the same filtered order. These shortcuts apply only while the start page is visible. The shell owns the initial focus handle; File returns focus through `Shell::focus_target`, which is the plot while one is shown and the shell handle otherwise. The result receiver lives with the shell, across file openings; blocked filesystem checks are never joined on the UI thread or at shutdown. Failed checks do not delete history. Recent rows and the chooser size their hover backgrounds to the text, bounded by the list width; the chooser follows the list content height, with `or` aligned to the heading and chooser text aligned to the filename column. Empty history keeps the chooser centered. Ctrl+O (Cmd+O on macOS), File > Open and the start-page chooser dispatch one `ChooseFile` action. Its binding and application-level handler also cover popup focus; the handler dismisses the tracked title-bar popup and restores focus through `Shell::focus_target` before opening the chooser. All shortcut hints size to their content with a viewport-aware maximum and use `shortcut_tooltip`, which resolves the registered action binding and places it at the right edge in the same framed keycap used by menus and analysis hints. `shortcuts.rs` applies shared, slightly stronger text and border contrast to `Kbd` and measures that same element for menu widths. Keep the original Edit settings geometry and typography: a text-colour override replaces the entire built-in text refinement, so the helper explicitly preserves its size, alignment, line height and whitespace defaults. Reserve its full width and keep platform notation from registered bindings. Keep every metadata check off the UI thread, including paths that look local but may be network mounts.
- The window measures its axis labels through `argand_core::axis::LabelMeasure`, implemented over GPUI's text system. That is the second implementation of the trait, `aspec`'s ab_glyph one being the first, and it is why the trait exists. The GUI sizes the right ruler's gutter from the labels it actually places plus every unit caption (#148). `PlotView` holds the widest gutter measured (`axes::Held::gutter`), so zooming in, panning and zooming the other axis never narrow it. `PlotView::view_intent` releases the hold when the axis shown on the right zooms out or is fitted, so the ruler refits its new labels (it can still widen when a format or unit grows). A new file, an orientation change and a time-format change with time on the right also release it. The GUI starts time labels after their ticks; shared axis layout accounts for that anchor while retaining centered labels by default for CLI rendering. Numeric label centering uses the shaped line's baseline metrics. The axis layout keeps a 4-logical-pixel outer margin at the sides and bottom and meets the minimap separator at the top. The frequency unit occupies the right gutter beside the minimap, with the bottom of its text aligned to the top of the frequency ruler; only time labels reserve a full text row outside the plot height.
- The window's size and state are restored wherever the toolkit reports the window state reliably; X11 and macOS both report a window as ordinary while it is not, which can cost the size a maximized window returns to (Issue #38). Its position is restored only where the toolkit reports enough to restore it: not on Wayland, where plain xdg-shell gives a client no absolute position, and not across displays on macOS, where gpui reports every display's origin as zero and opens on the primary one (Issue #37; its macOS half needs nothing gpui does not already provide).
- The current implementation focus is display, editing, and spectrum analysis: phases 0 through 6 in `docs/plans/IMPLEMENTATION_PLAN.md`.
- The plugin and worker design is deferred to the final section of `docs/plans/IMPLEMENTATION_PLAN.md`.

- Progressive Max reduction retains linear row amplitudes until time-column reduction; Mean retains its existing dB arithmetic. The refinement cache refreshes changed columns and their empty followers, invalidates colours when shading changes, and still publishes owned whole snapshots. FFT scratch belongs to each partial accumulator and is reused across its frames.

## FFT execution policy (#67)

`execution.rs` owns the per-document Rayon budget: configurable workers (zero means
at most eight available logical CPUs), bounded batches (default 1024 frames), and
optional efficiency affinity. Work stealing distributes frame groups dynamically;
realfft/rustfft plans remain single-threaded. Linux Intel hybrid discovery uses a
short-lived probe thread and CPUID core classes inside the inherited CPU mask.
Only compute workers are pinned; UI affinity and system scheduler settings are
untouched. Unsupported discovery falls back to ordinary scheduling. Max reduction
combines consecutive frames for the same image column within each partial;
independent frame counts preserve PSD normalization. Mean retains individual rows.
Progressive batches bound decoded data to 4 MiB and estimated FFT work to 1024
2048-point transforms, allowing one oversized FFT. Buffers are allocated at that
bounded capacity, not allocated large and then truncated.
`argand::ui_latency=trace` enables UI timer, delivery-age and CPU upload-preparation
probes; these are not input-to-display measurements.

## Time navigation (#30)

`navigation.rs` owns bounded sample ranges and held-grid level lookup without GPUI.
The shell starts every opened file at its full sample range, clamps its span to
one FFT or the shorter capture, and sends range changes through the existing
cancellable mailbox. Size and style changes retain their previous cache semantics.
`navigation_ui.rs` provides Ctrl+wheel zoom about the pointer, left-drag pan (including the time ruler), identical Ctrl+wheel zoom and wheel horizontal pan over plots and the time ruler, keyboard
commands and the View menu. The crosshair is restricted to the spectrogram. `plot_ui.rs` immediately maps held spectral pictures into
the requested time interval and clips the spectrogram to its plot. Cursor levels
come from the displayed grid's own extents; uncovered time has no reported level.
The completed full-capture minimap updates the file hint's extrema independently
of spectral navigation; partial minimap previews never supply complete extrema.
At zoom ratios above 1024, the few visible source columns are cached as one-pixel-wide textures and drawn
with clipped bounds to avoid losing the viewport in large GPU f32 image coordinates.
`AxisKind::PreciseTime` extends the shared clock ladder with fractional seconds
for the GUI; CLI whole-second clock formatting remains unchanged.
Recent entries contain no time view; deserialization ignores legacy saved ranges. Each file opening starts with the full capture, even within the same run. Navigation gestures perform no filesystem writes.
The minimum span also reserves two floating-point ULPs per display column at
extreme sample counts (at least ten columns for keyboard pans). Width changes
reapply this floor before requesting analysis. The settings editor restores automatic view expansion on
Cancel; explicit navigation during editing becomes the new view to restore.
Cursor time precision follows the visible time per pixel, up to nanoseconds.

Navigation range replacements use `ProgressiveOptions::final_only`: sequential sample
reading, no sparse previews or intermediate pictures, and cancellation between bounded
batches. `analysis_progress.rs` forwards source hints and sends bounded, nonblocking
progress-only notifications from sequential reads every 50 ms. At most 128 frames fitting one configured memory/work-bounded batch use a
single FFT accumulator; larger workloads retain parallel frame groups. Navigation
textures use one logical column for a single FFT frame; multi-frame pictures retain
pixel-width time cells so toolkit filtering cannot blend different FFT frames.
The overview computes and shades identical adjacent time-cell overlaps once and
copies their pixels. Waveform columns still match the plot width. Spectrogram
textures replicate their edge texels and clip the padding at paint time to prevent
linear atlas sampling from bleeding neighbouring allocations into stretched images.
`backdrop.rs` keeps one additional wider picture (CPU grid/image, GPU
texture) to fill only time uncovered by the foreground. It is scoped
to the displayed transform and file. It keeps the widest extent, upgrades a matching
preview on completion, and reports cursor levels only from completed backdrops.
A held preview can seed it when navigation interrupts initial analysis; it remains
an approximate visual placeholder. Style changes shade the retained grid in a
background task and coalesce the pending foreground delivery; both pictures change
style together after delivery-generation validation. Retention defers wider-picture
replacement and completion upgrades while that delivery is parked for shading, so
releasing a backdrop cannot discard a valid foreground result. Equivalent rounded-hop settings
retain it. Both
foreground and backdrop use bounded source-column textures above 1024x stretch.
This spectral navigation placeholder is separate from the full-capture minimap.

`axes::CursorGuides` paints transient Alt-held cursor-to-ruler lines and coordinate
badges from the requested physical extents, with precision based on device pixels. Badges use axis font metrics and remain
within the spectrum panel; they do not change layout or submit analysis. Modifier
and window-activation events repaint the shell, with current window modifier state
read during painting. Guides are absent outside the spectrum, during panning, with
an open menu or while the window is inactive. Vertical frequency navigation is
tracked separately in #80.

`axis::TickScheme` retains the selected division spacing and clock-format span while
panning. Zoom, file opening and width changes release that scheme for fresh layout.
If wider labels collide after a pan, the grid remains stable and colliding label text
is omitted. Left/Right pan by one measured division; Ctrl+Left/Right by five.
`navigation::TickPan` accumulates divisions before sample rounding, preventing drift
for nonintegral samples per division; hitting either capture edge resets its origin.
Shift+wheel is reserved for vertical frequency panning in #80 and currently does
not navigate in time. Unmodified wheel pans horizontally; Ctrl+wheel zooms on both
plots and the time ruler.

All time zoom keys use Control: Ctrl+Plus/Equals, Ctrl+Minus and Ctrl+0 (fit).
Alt guides use a three-logical-pixel stroke in the theme's paper colour with a
one-pixel core in its ink colour (#148). That is the same sequence as a badge (ink
fill inside a paper ring), black–white–black in the dark theme. The guides are
clipped to the plot, and the coordinate badges are slightly rounded. Guide colours
follow the interface theme, independent of the spectrogram palette. Badges (#141) wrap the digits' ink with
3 logical pixels on every side. A one-pixel ring in the paper colour (#148) surrounds
each badge so its edge stays visible over picture areas of its own shade. Rings are
painted before the guide lines, which therefore meet the badge fill without a gap. The bottom badge is centred on the bottom
ruler's text row, so its text shares the labels' row and clears the ruler line.
The right badge is centred on the guide line and stands as far from the panel's
right edge as the bottom badge stands from the bottom ruler's edge (#148). Clamping moves a badge only where the
panel has no room; `cursor_guides::badge_rects` holds the geometry.

## Full-capture waveform minimap (#79)

`minimap.rs` owns a toolkit-neutral, separately cancellable document task. After
metadata arrives it opens an independent cursor through `argand_io::reopen`,
reusing the already resolved sample count and normalization divisor. FLAC seeks
recreate the parser with those known values because the decoder can otherwise
retain old packets at aligned frame offsets. Reopening
never recounts or rescans levels and does not enter the global Rayon pool. A preview reads at most 128 evenly spaced blocks of
256 samples, then a sequential no-FFT scan visits every sample in blocks of at
most 65536 scalar values (256 KiB). The retained envelope has at most 65536 cells
per channel (1 MiB for I/Q). A bounded two-item channel carries only the preview
and final envelope; dropping its receiver cancels between reads without a UI join.
Initial completion replaces the preview and resolves the full-capture peak scale.
Range, transform, reducer and palette changes neither cancel nor rebuild this task.
The shell no longer requests or retains per-range spectral waveforms. File hints
accept extrema from the complete minimap; a read failure clears its picture and
appears in the file hint, without discarding the spectrum.
`waveform.rs` caches conservatively rebinned pixel spans by device width and height.
Minimap ink derives from the displayed spectral colormap and the interface theme,
falling back to the requested colormap before the first spectral picture arrives.
Navigation changes only waveform ink, active inside the requested time viewport
and muted outside, with no border or fill. Full capture has no dimmed portion. Sample
bounds retain a one-device-pixel minimum visible width. The minimap has no grid.
An outside click invokes the same one-division pan as Left/Right; Ctrl+click invokes
five divisions. Inside clicks, including double-clicks, only arm dragging. An outside double-click
centres at the pointer. Dragging uses full-capture coordinates. When the time view
can pan, the viewport and time ruler use an open-hand cursor; active drags use a
closed hand. The frequency ruler uses an arrow. Frequency navigation remains
reserved for #80. Wheel gestures belong to the spectrum and time ruler. Pointer
time over the minimap uses full-capture coordinates.

## Time ruler modes (#81)

`time_ruler.rs` owns the toolkit-neutral clock/seconds/samples presentation.
`View > Time scale format` and the time-ruler context menu share the mode items; session version 7 remembers it, while older
sessions default to clock and file openings still reset the navigation range.
`AxisKind::Seconds` uses decimal second steps with matching fractional labels;
`AxisKind::Samples` uses integer decimal steps and exact integer-multiple labels
prefixed with `#`. The ruler receives native sample bounds so a complex sample
counts one I/Q pair. Held schemes use the selected ruler's units; keyboard pan
converts its step to samples once. Changing mode clears the held scheme, preserves
the view and performs no analysis request. The Alt time badge uses the same mode;
its sample index is computed relative to the integer view start and clamped to an
existing sample. Alt badge widths are measured from the current extent endpoints
with the same widest-digit font policy and pixel precision as readouts; pointer
motion only changes centered text and position. The shell retains a lazy measurement
cache keyed by extents, plot geometry, display scale and font; it is accessed only
while Alt guides are visible. Numeric locale is immutable after startup.
Guide lines overlap the rounded
badge backgrounds, including across the gap below the spectral image.
Status-bar coordinates share the Alt badge formatter and follow both ruler units. CLI defaults and
CLI frequency axes are unchanged.

`numbers.rs` formats GUI numbers with ICU/CLDR using the regional numeric locale
read by `numeric_locale.rs` at startup. Top-level `Config::number_format` defaults
to `system` and accepts an explicit BCP 47 tag or C/POSIX. It has no visual editor
or session override. Configuration loads before numeric initialization, which
precedes session restoration and window creation; invalid locale strings reset
only this field to `system`. Numeric settings use the same formatter
and validate localized grouping when parsing. `LabelMeasure::localize` runs before
tick measurement; CLI implementations keep the default identity hook. GUI HMS
labels use colons between clock fields, and fractions use the locale decimal mark.
Time units are painted once at the right; space for all three captions is reserved
independently of the mode. In horizontal orientation switching the mode therefore
cannot resize the spectral image. In vertical orientation the right gutter refits
the new mode's time labels (#148).
Unit hints use measured caption rectangles from the axis frame. The time hint
belongs to the ruler context-menu element so right-click handling is preserved;
the frequency hint occupies only its visible caption beside the minimap, not the frequency ticks. Hints observe the shell and report the current view span per physical device pixel, in seconds/samples and an independently selected frequency unit (Hz through GHz)
based on resolution magnitude, without redundant decimal zeros. Hand cursors require an available pan range; the frequency ruler remains non-draggable until #80.
Machine-readable persistence and CLI number formatting do not use this module.

The time-ruler context menu retains its popup entity after closing (a toolkit
reference cycle, #144). `PlotView` tracks its DismissEvent explicitly, clears only the matching menu, and restores
the pointer from the owning window through the ordinary plot filter. This keeps
Alt guides and cursor feedback available without a mouse move after dismissal.


## Frequency navigation (#80)

`frequency.rs` owns a normalized, bounded frequency viewport independent of the time
range. Each file opening resets it. Its minimum span is one retained frequency
cell, with at most 2048 cells across the capture band and a physical-coordinate
precision floor for extreme centre frequencies. The document mailbox treats
frequency bounds as a display revision, never a new analysis generation.
`Overview::render_band` rebins overlapping native-bin/cache-cell intervals in the
value domain on the worker; full-band rendering retains its existing reduction
policy. Held textures, the wider backdrop and numeric level lookup use both time
and frequency extents. Frequency panning holds the tick scheme until zoom or a
height change. Shift+wheel pans frequency; Ctrl+Shift+wheel zooms it. The frequency
ruler supports drag/wheel pan and Ctrl+wheel zoom. Frequency keyboard zoom uses
Ctrl+Shift+plus/minus and Ctrl+Shift+0; Up/Down and Ctrl+Up/Down pan one/five divisions. No navigation
state is persisted, and the full-capture minimap is independent of both viewports.


## Ruler marks and grid visibility (#71, #72)

GUI time labels request `LabelMetrics::keep_edge_marks`: spacing is selected using
readable labels as before, then edge-clipped labels become empty strings while
their valid marks remain. Default CLI layout is unchanged. Both GUI ruler ticks
extend six logical pixels. Right-hand ruler labels start nine pixels after the tick.
Bottom-ruler labels in both orientations (#145, #148) start four clear pixels after
their tick: time in horizontal orientation, frequency in vertical. Their ink sits at
least six pixels under the ruler line and at least eight above the band's bottom edge
(`BOTTOM_LABEL_FOOT`), which keeps them optically clear of the status bar. An Alt
badge wraps the ink with 3 pixels, so it clears the ruler line by at least 3. `session.show_grid`
(version 8, default true for older files) controls only grid strokes; View > Show
grid saves the choice and notifies the UI without requesting analysis.


## Spectrogram orientation (#82)

`orientation::Mode` maps physical time/frequency fractions and rectangles to the
screen; spectral grids retain their original time-column/frequency-row order.
Horizontal mode is the default. Vertical mode puts time downwards, frequency to
the right and the independent minimap on the left. Both the vertical spectrum
and minimap meet the panel top without an outer inset. Uploaded BGRA pixels rotate
clockwise in vertical mode, including padded deep-preview strips; held pictures
and backdrop coverage use the same orientation mapping. GPU retirement still
waits two frame callbacks. Switching mode reuploads retained images and requests
a cached display size, preserving physical ranges unless the existing time ULP
floor requires expansion for a longer time axis. That exceptional range change
restarts analysis just as an ordinary resize does.
`PlotSize` always counts time columns and frequency rows; axes, hints, gestures,
minimap rebinning and splitter layout use the corresponding screen dimension.
The mode button and Ctrl+T share one orientation action and persist session version 9.
Its shortcut hint resolves the registered binding; plot bindings match only while
the plot surface itself has focus, so they cannot toggle behind a popup or input. Arrow bindings use Horizontal/Vertical
key contexts so panning follows the screen; named time/frequency zoom shortcuts
retain their axes. In vertical mode time labels occupy the right gutter, frequency
labels the bottom row, with time units at the top of the right gutter and frequency
units at bottom-right. The time ruler fits numeric ink between the top time-unit and bottom frequency-unit
captions with four logical pixels of clearance, including the painted half-pixel
center offset, without moving the spectrum top or removing grid marks. Unit hitboxes retain an
arrow cursor and exclude pan/zoom gestures; the time unit retains its context menu. The grid and time-format settings apply
to both layouts.

Spectrum left-drag pans both physical axes, including frequency-only zoom when
time remains fitted. Rulers and minimap constrain dragging to their own axis.
A pointer event updates both viewports before submitting one display request.
Wheel handling uses the dominant nonzero delta component because Linux GPUI
backends remap Shift+wheel to horizontal deltas; modifiers still select the axis.

Symbolic zoom keys are normalized by a keystroke interceptor that each `PlotView`
owns and drops with itself. It runs before GPUI binding match, which is the only hook
that early, and acts only when its window is the event's window and its own plot
surface is exactly focused; otherwise it neither matches nor consumes anything. Grid
and orientation keys are ordinary bindings. It combines the keystroke Shift flag with the
window's physical Shift state, which Linux symbol normalization otherwise loses.
Ctrl plus/minus targets time; adding Shift targets frequency, including
underscore and keypad aliases; a top-row `=` keystroke is accepted silently as
the same physical key as `+`, and the keycaps show the four zoom commands
uniformly as Ctrl+Plus, Ctrl+Shift+Plus, Ctrl+Minus, Ctrl+Shift+Minus
(`shortcuts::zoom_keycap`). Ctrl+Shift+0 fits frequency; the interceptor handles
both `0` and the US shifted `)` with physical Shift state, leaving unshifted Ctrl+0
to the time-fit binding. Named menu actions retain their explicit axes.
Ctrl+Shift+Up/Down are not zoom bindings; arrow pan bindings stay unchanged.


## Plot ownership (#128)

`plot_view.rs` defines `PlotView`, one entity per document created when the file
describes itself and dropped with it. It owns the plot focus handle and the
`Plot Horizontal`/`Plot Vertical` key context on its surface element. It also owns
pointer and readout position, pan, frequency-pan, splitter and pressed-zoom state,
measured geometry and badge metrics, and the ruler context menu. Focusable overlays
are siblings of the surface, never descendants, so they inherit no plot bindings.
Navigation actions and gestures become `PlotIntent`s that Shell handles in one
subscription. Shell validates views against the capture and keeps view ranges,
tick schemes, analysis requests, session writes and settings rollback. Session
commands (grid, scale controls, orientation, ruler mode) stay Shell handlers,
reached by bubbling from plot focus. While a drag is active, a tracker with its
own hitbox matching the plot's handles every move the plot's hitbox does not see:
beyond the plot and over a hint lying on it (#129). The plot's own listener
handles the rest, so no move is handled twice.

Shell remains the only texture owner. It rebuilds an immutable `PlotSnapshot` of
shared references in every render and gives it to the plot. `shell::retire` is the
only place images are released: it clears the plot's snapshot, then schedules the
two-callback release on the originating window. Backdrop and deep-preview code
return the images they drop rather than releasing them.

While a document is shown, the plot owns keyboard focus. Toolbar and status-bar
buttons are not tab stops, and their clicks never take focus. Menus and popups
take focus temporarily and return it through `Shell::focus_target`, which the
application menu also uses to resolve keycaps. Headless GPUI tests
(`gpui-kit` `test-support`, dev-only) cover key routing, focus isolation, window
isolation, plot replacement, retirement and drags beyond the plot.

## Overlay surfaces (#129)

Surfaces over the plot own the input they cover through toolkit layering; the
plot never learns which overlays are open or where. It decides its cursor,
readout and Alt guides from its own hitbox hover state, and its handlers are
hover-based, so any blocking layer above it wins.

- Passive hints: `shortcut_tooltip`, `metadata_tooltip` and `unit_tooltip`
  return their view through `hints::passive`, never a bare `Tooltip::build`. A
  passive hint (`.tooltip`) lives only while its trigger is hovered, so the
  pointer can be inside it only over the trigger, which keeps the pointer and its
  clicks. Blocking it would take clicks from its own trigger, because a GPUI
  tooltip opens 13 pixels from the pointer and can cover the trigger.
- The analysis hint is a pinned hint (`hints::PinnedHint`), following the
  contract agreed in #108 for the future settings popover. Hovering the FFT
  summary for 500 ms opens it as a standard gpui-component `Popover` anchored
  above the summary, drawn with the standard `Tooltip` look (`appearance(false)`).
  A right click on the summary also toggles it; the left click still opens
  the settings window. Moving the pointer away does not close it. Only a click
  outside (consumed), Enter (keeps the values), Escape (reverts to the settings
  at opening), Ctrl+, (opens the settings window), F10, Ctrl+O and opening a
  file close it. While it is open the plot is frozen: a transparent deferred
  backdrop (`hints::backdrop`) covers the window, so the plot gets no pointer,
  readout, Alt guides, clicks or wheel, and the popover holds keyboard focus,
  so `Plot` bindings do not match. Tab does not leave it, and Space, which the
  popover would treat as Enter, does nothing (`NoAction` in the `PinnedHint`
  key context). Values changed from
  it (Ctrl+R, its recommendation button) preview live. `PinnedHint` emits
  `Pinned::Opened` / `Pinned::Closed { revert }`; Shell interrupts the plot on
  opening and restores `hint_opening` on a reverting close. While the settings
  window is open the hint is disabled (`PinnedHint::set_enabled`): the trigger
  renders without its popover, so neither hover nor a right click opens a second
  surface with its own rollback.
- A resting pointer: `on_mouse_move` and `on_hover` fire only on moves, so a
  plot that appears under a still pointer (a file opened from the start page) or
  is uncovered under it (a hint or menu closed by a key) would not know it is
  there. `pointer_probe` inserts the plot's own hitbox each frame and, while the
  plot has no pointer and the mouse lies within it, asks after the frame
  (`on_next_frame`) whether `Hitbox::is_hovered_at` the mouse position and the
  window is hovered; `PlotView::pick_up_pointer` then takes the pointer up.
  Nothing is marked dirty unless a pointer is found.
- The application sets `CursorHideMode::Never`: GPUI's default hides the mouse
  pointer on Tab and on every key bound to an action, and on the plot the
  pointer is the working tool.
- The corner zoom buttons suppress the crosshair, the Alt guides and the
  status-bar readout (`PlotGeometry::over_scale_buttons`).
- Menus: the application menu and the ruler `PopupMenu` `occlude()`, taking wheel,
  clicks and drags.
- Gestures: `PlotView::interrupt` ends drags, a pressed zoom half, the ruler menu
  and the pointer. Shell calls it before the application menu, the settings
  window and the file chooser open. Opening the ruler menu and deactivating the
  window call `PlotView::end_gestures`, which keeps the pointer.
- Tests cover the pinned hint's lifecycle and isolation, a passive hint's
  trigger click, a drag crossing a blocking layer, a menu-like occluding layer
  and interruption. The ruler context menu cannot be opened in a headless
  test because of the toolkit's `PopupMenu` retention cycle (#144), so its cases
  are native checks.

## Application menu and toolbar (#91, #99)

File contains Open file, availability-filtered recent rows directly in the menu,
and Settings (`EditAnalysis`). Empty recent lists leave one separator between
Open and Settings. The first nine recent rows carry digit keycaps, and pressing
that digit while the File list is open activates the row
(`app_menu::Menu::activate_numbered`). View appears only with a file and
contains grid, scale
controls, orientation, Fit time, Fit frequency and Time scale format; pan/zoom
commands remain available through keys and gestures rather than menu rows.
The menu anchor wraps the application button (`toolbar` in `app_menu_ui.rs`),
not its padded interior, so the dropdown drops from the button's outer edge
and never covers it.
`plot_ui.rs` draws translucent `[+|-]` zoom pairs over the spectrogram itself,
eight logical pixels clear of the picture's edges (`corner_zones`). Each pair
runs along the ruler of the axis it zooms (#150): a row in the bottom-left
corner beside the bottom ruler, a column in the top-right corner beside the
right ruler. Horizontal mode puts time bottom-left and frequency top-right;
vertical mode swaps them. Each pair is one framed container with
rounded corners (22-pixel squares, one-pixel divider) dispatching the
registered zoom actions with shortcut tooltips. View → Show scale controls
(`ToggleScaleUi`, Ctrl+U, session version 10, default on) hides or shows
every pair; pairs vanish when either spectrum side is too small. Pair zones use
an arrow cursor and exclude pan, drag and wheel navigation.

The application icon and Argand label form one button that opens a cascading File / View menu; F10 opens the same menu
from the shell. `app_menu.rs` owns toolkit-neutral branch selection, hover and
keyboard navigation. `app_menu_ui.rs` renders the overlay and dispatches existing
actions after returning focus to the shell. Escape closes one level, outside
clicks close the chain, and the independent ruler context menu retains the stock
PopupMenu. Toolbar buttons share the grid and orientation actions and persistence.
The orientation icon depicts the current time-axis direction; its tooltip names
the next mode. `assets.rs` adds embedded application artwork to the toolkit icons.
Title-bar content centers the shrinking filename region on the full window with
symmetric margins that include the toolbar and native controls;
interactive controls consume drag and double-click gestures.

The centered client title contains only the filename. Set the native window title
explicitly at startup and on each file opening: `Argand` or `filename – Argand`,
so application switchers and window managers receive the same identity.
