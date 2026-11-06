# Conversion numeric editor and output-format follow-up

Issue: https://github.com/o-kos/argand/issues/189
Draft PR: https://github.com/o-kos/argand/pull/224

## Roles and risk

Class A: output-container/save identity and writer validation affect data safety.
Coordinator: gpt-6-astra medium; executor: gpt-6.1-sol medium;
independent reviewer: gpt-6.1-sol high.

## Agreed corrections

- Remove the Level/Normalize choice from File information. Conversion preserves the
  currently adjusted capture; this does not undo normalization already applied.
- Reuse numeric editing infrastructure for locale parsing, grouped presentation
  after confirm/blur, units and standard Inputs. Preserve typed text until commit;
  keep Analysis preview-on-commit and conversion draft isolation distinct.
- Disabled Convert must have no pointer-dependent text or background changes.
  Keep standard Button disabled behavior and the shared Reset presentation.
- Resolve save chooser directories to an existing ancestor. Do not reset desktop
  chooser state or replace a supported native chooser without an explicit decision.

## Approved container architecture

Keep private encoded-WAVE backing independent of the unsaved document's requested
user container. First Save must use the explicit container and extension. WAV supports
all ten current types. The locked FLAC encoder supports the current U8/I16 domains,
not I32 or floating types; unsupported pairs must remain visible validation failures,
never silently change encoding or fall back to WAV. Existing exact-rate/reference
FLAC metadata must remain intact. PCM24 remains separate work.

The current toolkit Open File API exposes no initial directory, and its Linux portal
request supplies none. The application therefore cannot currently validate or replace
the desktop's remembered Open directory through that API. Record this limitation;
do not claim that the application supplied the stale path.

## Work and acceptance

- [x] Share numeric input initialization/commit formatting without changing each
      editor's apply/draft transaction.
- [x] Remove normalization from the panel and keep Preserve for adjusted samples.
- [x] Remove disabled action hover/active ink and background through shared composition.
- [x] Validate save chooser folders and test deleted nested-folder resolution.
- [x] Resolve the output-container proposal before implementing format selection.
- [x] Add meaningful parse/commit and format/save identity coverage; retain stock
      disabled interaction handling and remove enabled-only child hover styling.
- [ ] Obtain independent review and run the full local gate; rebuild current release.
- [ ] Verify actual rendering within the approved second-monitor diagnostic limits,
      complete this plan, and publish the existing Draft update after acceptance.

## Implementation contract

The unsaved document retains its target container independently of the encoded WAV
backing. First Save and all selection exports use it. An explicit .wav/.flac extension
in Save as overrides that container; other extensions retain the explicit selection.
Band/rectangle FLAC export stages extraction and integer encoding privately before
atomic FLAC publication, retaining existing extraction rate/reference semantics.
The adapter reports clipped scalar values and never publishes an intermediate.

Open chooser remediation remains pending a separate owner decision: the locked API
cannot pass an initial directory. No desktop chooser state is reset by this work.

## Review record

Round one accepted four findings: synchronous chooser-directory metadata on the UI
thread, missing original-source checks after staged FLAC band encoding, lost FLAC
band-export defaults after first Save, and premature FLAC validation blocking an
explicit WAV export override. Directory preparation now runs in the background with
a document/version guard before prompting. Final FLAC publication checks original
stamps; saved metadata supplies the default container; final target compatibility is
validated after the chooser. Focused regressions cover all four. No findings rejected.
