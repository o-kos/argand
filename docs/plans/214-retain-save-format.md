# Issue #214: Save computed captures without their original file

Resolves #214.

## Overview

Retain the opened document's output format independently of the files containing
its current samples. Save as of a fully computed capture must succeed when the
original file has moved, disappeared or changed, while actual sample sources and
intentional replacement targets remain checked.

## Roles and risk

- Class: A, because source dependencies, encoding and safe replacement affect data safety.
- Coordinator: `gpt-6-astra` medium.
- Executor: `gpt-6.1-sol` medium.
- Reviewer: `gpt-6.1-sol` high in a separate agent and context.

## Context

`Editing::save_request` currently adds the opened file as a format provider even
when every piece reads a scratch file. The writer opens it and checks its stamp.
Initial storage description already runs off the UI thread and reopens the source
with its expected stamp; extend that verified reader operation to retain its exact
linear `fmt ` body or FLAC depth. This does not eliminate initial format discovery
or reads required by real sample dependencies.

## Decisions

- An immutable `argand_io::write::Format` owns the output description. Its private
  fields retain the bounded header data and derive the compact compatibility
  `Storage`; the document owns the format, while clipboard sources retain their
  own storage compatibility information.
- Document saves pass their retained format explicitly. Existing pure convenience
  constructors may defer format discovery to their checked source reader to avoid
  adding I/O or changing their return types. There is one resolved format per save.
- Save requests list only sources their segments read and remap source IDs to those
  entries. Original format provenance does not add a sample dependency.
- Preserve exact untouched bytes, computed-value encoding, sample kind, rate and
  reference frequency, FLAC fallback and existing format refusal rules.
- A missing initial format keeps saving unavailable rather than guessing it.
  Clipboard provenance, late description delivery and save rebinding retain their
  existing document/stamp checks.
- Save as of a raw capture writes WAVE and needs no original preamble. Save over
  with a nonzero raw preamble still reads and stamps the original. Intentional
  replacement still checks its expected target stamp.
- No platform-specific changes, changes to #199, module reorganization (#222), or
  persisted-session format changes.

## Rejected alternatives

- Reopen an otherwise unused original at save time: this is the dependency being removed.
- Always export computed values as float WAVE: this changes the capture's encoding.
- Keep complete source headers or samples in the session: unnecessary for this task.

## Implementation steps

- [ ] Capture an immutable output format from the existing checked description reader.
- [ ] Retain document format and pass it independently of remapped sample sources.
- [ ] Resolve writer output and compatibility checks from that format.
- [ ] Cover computed-only saves, actual dependencies, corruption and replacement.
- [ ] Update architecture documentation and changelog.
- [ ] Complete validation and rebuild the release executable.
- [ ] Move this plan to `docs/plans/completed/` before final review.

## Validation

The initial plan-only commit and Draft PR use diff/link checks; no code changes
exist at that point, so the code gate is deferred until implementation.

- [ ] Fully computed captures save after the original is moved, removed or replaced,
  preserving output metadata, format and sample data.
- [ ] Non-contiguous source IDs remap correctly; missing actual sample sources and
  corrupt scratch sources fail without changing the target or discarding edits.
- [ ] Linear/raw/FLAC outputs retain their encoding rules; Save as needs no raw
  preamble while intentional replacement and nonzero preambles remain checked.
- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --locked`
- [ ] `cargo test --locked`
- [ ] `cargo build --release --locked`, after all checks pass
- [ ] Independent review of the exact final revision and diff.

## Post-completion

Ready status, merge and branch cleanup require the owner's subsequent instruction.
