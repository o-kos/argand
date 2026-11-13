# Issue #185: Metadata and save completion feedback

Resolves [#185](https://github.com/o-kos/argand/issues/185), continuing Draft #250.

## Roles and risk

- Class A: capture metadata I/O and asynchronous save completion/data safety.
- Primary: `gpt-6.1-sol` medium.
- Reviewer: `gpt-6.1-sol` high, fresh read-only context.
- Consultant: none unless a concrete unresolved question needs advice.

## Owner decisions and scope

- Remove Argand-specific `argd` WAVE metadata. Use `auxi` reference frequency.
- Display/edit Reference frequency in whole Hz using the existing stock Input.
- Preserve IQW as a detected RIFF/WAVE container; do not classify it as RAW by extension.
- Investigate and fix persistent Saving 100% after a reference edit and Save.
- Inspect reported Convert availability after frequency edits; distinguish reference
  metadata commits from actual carrier/domain conversion before changing semantics.
- FLAC policy is pending the owner's clarification: it has no auxi chunk.
- Keep this work in the current Draft PR. Preserve unrelated policy edits.

## Acceptance

- WAVE/IQW save writes auxi without argd; reopening uses the whole-Hz reference.
- Existing foreign auxi captures continue to open, including the IQW extension.
- Reference editing uses whole Hz, including paste/Enter/blur/step/Escape paths.
- The reported edit-reference/Save sequence completes and removes Saving status;
  saved file can be reopened and its samples/reference verified.
- Failed/cancelled saves and publication/source identity protections remain intact.
- Convert behavior is explained and any agreed correction verified in production.

## Tasks

- [ ] Reproduce the stuck save and establish the cause before correcting it.
- [ ] Implement agreed metadata format and integer editor policy with regressions.
- [ ] Resolve FLAC policy and verify Convert availability.
- [ ] Update current architecture and changelog.
- [ ] Full local fmt/Clippy/test/release-last gate.
- [ ] Production acceptance on isolated Xvfb, with disposable data/settings.
- [ ] Independent review and current-snapshot assessment.

## Validation and limits

Record actual runtime observations separately from automated tests. No toolchain,
SDK, dependency graph or artifact layout changes. Do not alter the owner's files.

## Post-completion

Keep Draft for owner feedback; no Ready, merge or release transition authorized.
