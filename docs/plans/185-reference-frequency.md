# Issue #185: Reference frequency and baseband presentation

Resolves #185.

## Overview

Edit the capture's reference frequency directly in File information, retain it
through ordinary Save/Save As and Recent, and optionally show baseband frequencies
without changing samples or recomputing FFTs.

## Roles and risk

- Class: A: frequency interpretation, asynchronous display, history and safe saving.
- Coordinator: `gpt-6-astra` medium.
- Executor: `gpt-6.1-sol` medium.
- Reviewer: `gpt-6.1-sol` high in a separate read-only context.

## Decisions

- The owner approved an inline Reference frequency numeric editor in the upper
  File information metadata section, including zero and baseband mode. No separate
  dialog, Edit button or duplicate Edit menu command.
- Enter/blur applies a valid value; Escape restores the prior value. Invalid
  input shows an inline error without changing the capture.
- A reference change is an undoable metadata edit, dirty until explicitly saved.
  Save and Save As write the effective reference, without sample modulation.
- View > Baseband frequencies is a checked presentation preference, persisted
  with other session display preferences. The physical reference is retained.
- Conversion's Reference is renamed Carrier frequency and keeps its DSP meaning.
- Source metadata and analysis coordinates remain immutable. Display labels use
  an explicit source-to-display offset. Selections retain source-frequency
  coordinates; processing and export translate once into effective physical Hz.
- Metadata and sample edits share one history serial and saved-version guard.
  Reference/baseband changes and metadata-only Undo/Redo do not request FFTs.
- Recent distinguishes a missing override from an explicit zero. Legacy zero
  hints remain absent overrides during session migration.

## Context and boundaries

WAVE already reads/writes exact `argd` reference metadata and compatible `auxi`;
FLAC uses Argand Vorbis comments. Opening hints override file metadata, including
explicit zero. Frequency navigation is normalized while worker caches, display
grids and selections currently carry source physical Hz. Source stamps, staged
publication, cancellation and texture retirement must remain intact.

No toolkit/toolchain/dependency changes, custom controls, spectral inversion or
frequency correction of sample values are included. Use the existing NumericEditor
and standard menu controls. The installed SDK and local debug/test/release/check
baseline must be reused; dependency rebuilds require explicit owner approval.

## Acceptance criteria and evidence

| Criterion | Verification / scenario | Result | Limits or unperformed checks |
| --- | --- | --- | --- |
| Inline editor commits valid reference, preserves invalid drafts, restores on Escape | Production File panel pointer/keyboard, zero/signed/fractional values | Pending | No runtime evidence yet |
| Reference changes are undoable, dirty and saved exactly | Mixed sample/metadata history, Save/Save As/reopen, stale-save guard | Pending | No checks run yet |
| Baseband changes all frequency presentation while preserving samples, selected bins, zoom and levels | Real/IQ, both orientations, rulers/cursor/badges/typed selection | Pending | No checks run yet |
| Neither metadata edits nor baseband require FFTs | Worker generation/cache and metadata-only Undo regressions | Pending | No checks run yet |
| Processing/export uses the effective reference exactly once | Extraction/suppression/conversion and metadata output regressions | Pending | No checks run yet |
| Save rebind preserves coordinates and rejects obsolete deliveries | Changed source-reference basis, selection/view and pending-worker cases | Pending | No checks run yet |
| Explicit zero survives Recent/restart and old sessions migrate | CLI/file/Recent precedence and session migration regressions | Pending | No checks run yet |
| Shared controls retain their existing behavior | File/Analysis, nested Select, focus, narrow window, themes in production Xvfb | Pending | Native Wayland/Windows/macOS unperformed |

## Implementation steps

- [ ] Add versioned reference metadata and effective output metadata.
- [ ] Add source/display coordinate mapping and baseband presentation.
- [ ] Preserve processing, saves, asynchronous deliveries and rebind semantics.
- [ ] Add inline reference editor and distinguish conversion carrier.
- [ ] Migrate session/Recent persistence and add checked View command.
- [ ] Add focused invariant and UI regressions.
- [ ] Update architecture and changelog.
- [ ] Complete isolated production Xvfb acceptance and independent review.
- [ ] Move this plan to `docs/plans/completed/` before final review.

## Validation

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --locked`
- [ ] `cargo test --locked`
- [ ] `cargo build --release --locked`, last after the other checks pass.
- [ ] Production application acceptance on isolated Xvfb, with exact revision and
  local-diff identity recorded outside the historical plan.

The initial plan-only publication uses diff/reference checks; it does not compile
code or claim UI acceptance. Code pushes require the full local gate. Source
inspection and fingerprints alone do not prove dependency artifact reuse.

## Post-completion

No release or merge is authorized by this plan.
