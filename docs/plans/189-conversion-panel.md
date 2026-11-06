# Capture conversion panel and unsaved results

Issue: https://github.com/o-kos/argand/issues/189
Draft PR: https://github.com/o-kos/argand/pull/224

Class A: document identity, scratch retention and save safety.
Roles: coordinator gpt-6-astra medium; executor gpt-6.1-sol medium;
independent reviewer gpt-6.1-sol high.

## Approved contract

This iteration supersedes the earlier conversion destination/open contract.
The File details status item is a standard Button. Hover opens a file information
panel with read-only current information and draft target sample rate, reference,
sample type and level. Editing retains the panel. Edit > Convert to... and F11
open and pin the same panel. Explicit rate/reference values never change because
the sample type changes. Escape closes an open list first, otherwise discards the
draft and closes the panel. Convert requires valid changed parameters (Normalize
counts as a requested change). Only meaningful validation/band-loss warnings are
shown, rather than an implementation report.

Convert processes the entire current edited capture, including unsaved edits.
It asks for no destination and no preliminary save/discard decision. Existing
bounded DSP, scale, physical-frequency, source-stamp and encoding invariants remain.
Cancellation/error preserves the complete current document. Success replaces it
without Undo by a genuinely unsaved document backed by retained temporary disk
storage. Original user files remain unchanged. Display names are generated without
suffix accumulation; temporary backing paths are never save destinations or user
file identities. First File > Save asks for a path with the generated name;
successful whole save makes the result an ordinary saved document. Normal dirty
close/open prompts apply. Scratch ownership survives readers, workers and clipboard
references as required.

## Implementation

- [x] Separate unsaved document identity/display name/save destination from backing.
- [x] Adopt conversion scratch result atomically after successful preparation;
      preserve input on cancellation, failure and stale completion.
- [x] Compose the standard file information button/panel and existing inputs,
      selects and button; support hover, retained edits and Escape/list ordering.
- [x] Route Edit > Convert to... and F11 to that panel; remove old File row/dialog.
- [x] Cover dirty result, first Save, repeat conversion, scratch retention,
      unchanged source, cancel/error and draft/frequency invariants.
- [ ] Complete fmt, Clippy, workspace tests, current release build and independent
      review; update active architecture documentation.

## Acceptance and evidence

Current metadata/data remain byte-for-byte unchanged while editing a draft.
Identity Preserve with unchanged parameters disables Convert; Normalize enables it.
Successful conversion replaces only the current version used to start it and is
immediately dirty even before further edits. No destination chooser runs during
conversion. First Save cannot implicitly use input/scratch path. Repeat conversion
retains a stable generated basename. Cancel/error does not change document id,
capture/version, dirty state or input bytes. All prior DSP numeric criteria remain
in the completed conversion plan; this iteration does not weaken them.

Native input emulation, XTest/libei and RemoteDesktop portals are prohibited.
Any visual inspection uses a temporary diagnostic launcher with unchanged panel
rendering on the second monitor and window-only capture; distinguish this from
release/native interaction acceptance. No unrelated FFT spacing fix is included.

## Focused validation

Six focused conversion/panel tests pass: explicit frequencies and band validation,
changed/Normalize enable state, repeat naming, edited capture -> unsaved -> repeat ->
first Save lifecycle, cancelled/failed worker preservation, and standard Root/Popover
F11/Input/Select Escape ordering with draft isolation. Additional history and encoded
backing clipboard-retention regressions accompany the full local gate.
