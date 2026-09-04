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

- Record decisions here as they are made.

## Rejected alternatives

- Record meaningful alternatives and why they were rejected.

## Implementation steps

- [ ] Add a document: a path, its open hints, the source, and the analysis last
      produced for it. Everything below hangs off this rather than off the shell.
- [ ] Run the analysis on its own thread, owning the `Box<dyn SampleSource>`,
      with requests and results over channels and results applied to the view
      from GPUI's async context. No frame is drawn on a thread that is also
      transforming.
- [ ] Turn `SpectrogramImage` into a GPU image and draw it, settling the channel
      order and the image API that the later milestones build on.
- [ ] Implement `LabelMeasure` over GPUI's text system and draw the time and
      frequency axes through `argand-core::axis`, two-sided around the centre
      frequency for I/Q and one-sided for real.
- [ ] Open a file from the command line, accepting the same hints as `aspec`.
- [ ] Open a file from a menu and by drag and drop.
- [ ] Show container, sample type, sample rate, duration and centre frequency in
      the status bar.
- [ ] Remember recent files and the hints each was opened with, so a headerless
      capture opened once as `iq_i16@2M` does not need those flags again.
- [ ] Report a file that cannot be opened in the window, leaving the application
      usable.
- [ ] Update `AGENTS.md` and the roadmap where they describe what the application
      does.
- [ ] Complete validation.
- [ ] Move this plan to `docs/plans/completed/` before final review.

Use `➕` for tasks discovered after implementation begins and `⚠️` for blocked tasks.

## Validation

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --locked` (warnings are denied in `[workspace.lints]`)
- [ ] `cargo test --locked`
- [ ] `cargo build --release --locked`, after the checks above pass
- [ ] The window shows, for a capture from `tests/signals/`, the same spectrogram
      `aspec` renders for the same parameters. Compared as pixels, not by eye.
- [ ] A real and a complex capture both display correctly, with the frequency
      axis one-sided and two-sided respectively.
- [ ] The window stays responsive while a large capture is analysed.
- [ ] A file opens by argument, by menu and by drag and drop.
- [ ] A raw file reopened from the recent list needs no layout flags.
- [ ] An unreadable and an unsupported file each show a message and leave the
      application working.
- [ ] `argand-core`, `argand-io` and `argand-dsp` gain no GPUI dependency.

## Post-completion

- #29 makes the first frame fast on a large capture; this milestone's analysis
  thread and view are what it replaces the inside of.
