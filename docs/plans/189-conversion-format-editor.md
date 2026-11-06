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

## Container proposal pending architecture assessment

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

- [ ] Share numeric input initialization/commit formatting without changing each
      editor's apply/draft transaction.
- [ ] Remove normalization from the panel and keep Preserve for adjusted samples.
- [ ] Remove disabled action hover/active ink and background through shared composition.
- [ ] Validate save chooser folders and test deleted nested-folder resolution.
- [ ] Resolve the output-container proposal before implementing format selection.
- [ ] Add meaningful parse/commit, format/save identity and disabled interaction coverage.
- [ ] Obtain independent review and run the full local gate; rebuild current release.
- [ ] Verify actual rendering within the approved second-monitor diagnostic limits,
      complete this plan, and publish the existing Draft update after acceptance.
