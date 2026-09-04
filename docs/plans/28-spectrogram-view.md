# Issue #28: Show a spectrogram in the application window

Resolves #28.

## Overview

Milestone 1 left a window that shows nothing. This one proves the whole data
path end to end: open a file, run the transform off the UI thread, get an RGBA
buffer onto the GPU, and draw axes around it with the tick policy `aspec`
already uses.

Deliberately the simple version. `analyze()` is called blocking over the whole
file on a background thread and the result is displayed when it arrives. Making
the first frame fast is #29; this milestone is about the path, not the speed, and
a large capture will be slow to appear.

Out of scope: progressive or tiled analysis, zoom and pan, selection, the
waveform panel, editing.

## Context

- `argand_io::open(path, &OpenHints) -> Result<Box<dyn SampleSource>, IoError>`
  is the whole opening surface, and `OpenHints` already carries exactly the seven
  things the command line offers: `raw`, `sample_type`, `sample_rate`,
  `center_freq`, `byte_offset`, `normalize`, `gain_db`. Remembering how a file
  was opened is therefore remembering one existing struct.
- `argand_dsp::analyze(&mut dyn SampleSource, &AnalysisRequest, &mut dyn FnMut(u64, u64))`
  is blocking and reports progress through the callback. It returns `Analysis`,
  which holds `spectrogram: SpectrogramImage` -- RGBA plus the time and frequency
  extents -- and, since #26, `db: DbGrid`, which #32 will recolour without
  recomputing.
- `argand_core::axis` decides where ticks go, measuring candidate labels through
  the `LabelMeasure` trait. `aspec` implements it over ab_glyph; this milestone
  implements it over GPUI's text system, which is the second implementation the
  trait was extracted for.
- `crates/app` holds `config.rs` (a person's settings), `session.rs` (the
  application's own state, atomically written and version-checked) and `shell.rs`
  (the only place a GPUI type appears). The session already stores window
  geometry and state; this milestone adds to that mechanism rather than building
  another.
- `aspec`'s `main.rs` shows the working sequence: build `OpenHints` from
  arguments, `argand_io::open`, `analyze` with a progress callback.

## Decisions

- **The analysis thread opens the file as well as transforming it.** Opening
  looks cheap and is not: a capture read with `--normalize auto` is scanned for
  its peak before the first sample reaches a transform, and that is a pass over
  the file. So `analysis::open` starts the thread and the thread does both,
  reporting what the file turned out to be as its first update.
- **`async-channel` for the queue between the two threads.** Its receiver can be
  awaited from a GPUI task and its sender pushed to from a plain `std` thread,
  which is what lets `analysis` and `document` stay free of any toolkit and be
  tested without an executor or a window. It is already in the graph under gpui,
  so this adds an edge rather than a dependency.
- **The document is a state machine, not a bundle of fields.** `Document::apply`
  folds one update in and answers with what the window must do about it, so the
  sequence a file goes through is decided in one tested place and merely drawn
  in `shell.rs`.
- **A request is sized to the plot, not to the panel.** The frequency labels take
  a gutter out of the panel, so the transform is asked for exactly the rectangle
  the picture ends up in and one transform column is one screen column.
- **A resize re-analyses, and the thread keeps only the last request.** A drag
  offers one request per step; `newest` discards those overtaken while a
  transform ran, so the window ends a drag one analysis behind rather than
  dozens.
- **Every digit is measured as the widest digit the font has.**
  `LabelMeasure::width` has to answer the same for any digit in any place,
  because the gutter is reserved from a row of zeros before a tick is chosen.
  A desktop font is not obliged to have tabular figures, so the substitution is
  made in the measure rather than assumed of the face.
- **The uploaded picture is released when it is replaced.** gpui keeps a
  `RenderImage` in the window's texture atlas until it is told to let go, and a
  resize produces one per step.
- **The menu is drawn in the title bar, not handed to a platform menu bar.**
  gpui's `set_menus` builds a real menu bar on macOS and stores the list
  unused on Linux and Windows. The window already draws its own title bar on
  all three, so the menu goes there and behaves the same everywhere.
- **The recent list stores hints as the strings the command line uses.**
  `session.toml` then needs to know nothing about `RawSpec`, `SampleType` or
  `Normalize` beyond how a person writes them, one grammar covers the command
  line, the report and the file, and a hint a newer version wrote is dropped
  with a log line rather than making the whole entry unreadable.
- **The shell holds the whole `Session` rather than assembling one per write.**
  Two unrelated things write to it -- the toolkit reporting a window move, and
  a person opening a file -- and a session built from whichever happened last
  would guess at the other.

## Rejected alternatives

- **A `[patch]` or Git revision for gpui.** `gpui-component` declares its own
  `gpui` with no revision, so pinning one puts two incompatible copies in the
  tree. crates.io releases (gpui 0.2.2, gpui-component 0.5.1) are current
  enough; `AGENTS.md` now records why.
- **Embedding DejaVu Sans for the axis labels, as `aspec` does.** It would make
  the marks identical on every platform, and it would also make them the only
  text in the window not drawn in the desktop's own font. The window already
  depends on host fonts for everything else, so the labels use the window's
  font and the measure is made robust instead.
- **`cx.background_spawn` instead of a thread per document.** It is the shorter
  route, but the background executor is a shared pool and a half-hour capture
  would hold one of its threads for the whole transform. A thread that owns the
  `SampleSource` also makes it structurally impossible for the window to touch
  a file a transform is reading.
- **Debouncing resize on the window's side.** A timer to add and a delay to
  tune, for what `newest` already achieves by discarding overtaken requests.
- **serde derives on `argand-io`'s hint types.** It would make `session.toml`
  shorter to write and would tie the file's format to the shape of types that
  exist to be parsed from a command line. The spellings are the stable surface;
  the structs are not.

## Implementation steps

- [x] Add a document: a path, its open hints, the source, and the analysis last
      produced for it. Everything below hangs off this rather than off the shell.
- [x] Run the analysis on its own thread, owning the `Box<dyn SampleSource>`,
      with requests and results over channels and results applied to the view
      from GPUI's async context. No frame is drawn on a thread that is also
      transforming.
- [x] Turn `SpectrogramImage` into a GPU image and draw it, settling the channel
      order and the image API that the later milestones build on.
- [x] Implement `LabelMeasure` over GPUI's text system and draw the time and
      frequency axes through `argand-core::axis`, two-sided around the centre
      frequency for I/Q and one-sided for real.
- [x] Open a file from the command line, accepting the same hints as `aspec`.
- [x] Open a file from a menu and by drag and drop.
- [x] Show container, sample type, sample rate, duration and centre frequency in
      the status bar.
- [x] Remember recent files and the hints each was opened with, so a headerless
      capture opened once as `iq_i16@2M` does not need those flags again.
- [x] Report a file that cannot be opened in the window, leaving the application
      usable.
- [x] Update `AGENTS.md` and the roadmap where they describe what the application
      does. ➕ `README.md` too: it said the GUI was not built yet, and its layout
      section had no `crates/app`.
- [x] Complete validation.
- [ ] Move this plan to `docs/plans/completed/` before final review.

Use `➕` for tasks discovered after implementation begins and `⚠️` for blocked tasks.

## Validation

- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --all-targets --locked` (warnings are denied in `[workspace.lints]`)
- [x] `cargo test --locked`
- [x] `cargo build --release --locked`, after the checks above pass
- [x] The window shows, for a capture from `tests/signals/`, the same spectrogram
      `aspec` renders for the same parameters. Compared as pixels, not by eye.

      Done twice over. In the suite,
      `spectrogram::tests::every_pixel_uploaded_is_one_the_transform_produced`
      opens a capture, builds the request through `Config::analysis_request`,
      runs the same `analyze` `aspec` runs, and holds every byte handed to the
      GPU against every byte the transform produced -- equal but for red and
      blue, which is the order gpui reads a texture in.

      And on screen: the release binary was run on
      `12.579000_25_08_26_06_09_10.iqw` under a nested compositor, the window
      captured with `grim`, and the plot rectangle held against the transform's
      own RGBA for the same 1499x634 request. 897,559 of 950,366 pixels are
      identical, and every one of the 52,807 that differ lies on one of the 30
      grid columns or 23 grid rows the window draws over the picture. Outside
      the grid the difference is exactly zero, so nothing is resampled,
      recoloured or shifted between the transform and the screen.
- [x] A real and a complex capture both display correctly, with the frequency
      axis one-sided and two-sided respectively. `rl_f16x8-hfdl.wav` labels
      0 to 4 kHz; `12.579000_25_08_26_06_09_10.iqw` labels 12.567 to 12.591 MHz
      about its centre.
- [x] The window stays responsive while a large capture is analysed. The
      30-minute capture reports its progress in the status bar and repaints
      throughout; the transform runs on `argand-analysis`, never on the thread
      drawing the window.
- [x] A file opens by argument. ⚠️ Opening by menu and by drag and drop could
      not be exercised here: the nested compositor used for GUI work has no
      input device, and neither a virtual-pointer tool nor a way to synthesise
      a data offer is available on this machine. Both paths end in the same
      `Shell::open` the command line uses, and the file dialog is the
      platform's own. They need the owner's real session.
- [x] A raw file reopened from the recent list needs no layout flags. The
      entry, its hints and their round trip are covered in `session_tests.rs`;
      choosing it in the menu is part of the item above.
- [x] An unreadable and an unsupported file each show a message and leave the
      application working. Shown in the window in the theme's danger colour,
      with the same wording `aspec` prints -- including the `--raw` suggestion
      for an unrecognised container -- while the menu stays usable.
- [x] `argand-core`, `argand-io` and `argand-dsp` gain no GPUI dependency.
      Their manifests are unchanged; the only new edges are `argand-app` on
      `argand-io`, `async-channel`, `clap` and `image`.

## Post-completion

- #29 makes the first frame fast on a large capture; this milestone's analysis
  thread and view are what it replaces the inside of.
