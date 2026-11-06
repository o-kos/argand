# Status-panel interaction and shared numeric editor

Issue: https://github.com/o-kos/argand/issues/189
Draft PR: https://github.com/o-kos/argand/pull/224

## Roles and scope

Class A: shared focus/event scheduling and the expected cross-module refactor.
Coordinator: gpt-6-astra medium; executor: gpt-6.1-sol medium;
independent reviewer: gpt-6.1-sol high.

The approved scope is conversion notices, status typography, adjacent status-panel
hover routing, one numeric editor used by both panels, and simpler file details.
Reference-conversion policy and integral sample statistics remain discussions;
do not change either until their contracts are agreed.

## Architecture and invariants

- Keep the exclusive status-panel owner. Its input backdrop must protect the plot
  while allowing pointer access to both adjacent status triggers. Hover must use
  the same common opening/closing lifecycle as commands and clicks.
- Use one numeric editor entity with standard Input and Button controls. It owns
  typed text, locale commit formatting, units, configured step/bounds, Up/Down,
  and the existing press/repeat contract. Panels configure it and receive a
  commit event: Analysis previews; File information changes only its draft.
- Reuse supported toolkit controls. The retained press/repeat exception is the
  existing one: stock Button click-only stepping cannot repeat while held.
  Do not introduce custom text editing or pointer controls.
- Successful conversion leaves no persistent notice unless scalar samples were
  clipped. Explain that I and Q are counted individually. Keep errors/cancellation
  and atomic unsaved-document adoption unchanged.
- Remove only the displayed Document: Unsaved row; internal unsaved/save identity
  remains intact. Total samples always counts logical samples (one I/Q pair).

## Work and acceptance

- [x] Remove zero-clipping success reports and use common small status typography.
- [x] Fix adjacent-trigger hover through the actual backdrop/hit-testing path;
      verify both directions and continued panel interaction/plot protection.
- [x] Replace parallel numeric behavior with one configured editor; preserve
      grouped f64 roundtrips, invalid drafts, keyboard/hold/Tab contracts, and
      Analysis preview versus conversion draft isolation.
- [x] Remove the unsaved row and use Total samples for real and I/Q counts.
- [x] Report accurate Peak/RMS accumulation feasibility and Reference-policy
      questions to the coordinator without implementing unapproved semantics.
- [x] Obtain independent review, complete the full local gate, rebuild current
      release, and verify actual panel rendering within the second-monitor-only
      diagnostic limits. No input emulation or RemoteDesktop requests.

## Review record

Independent review of the shared editor, hover routing and bounded unused-output
cleanup was clean, with no substantive findings or rejected findings. The coordinator
also assessed the numeric event/repeat logic and the distinct preview/draft callbacks.

The full local gate passed: formatting, all-targets Clippy, and 1,121 locked workspace
tests with zero failures and one ignored manual DSP benchmark. Existing numeric
hold/keyboard/preview tests and the new File draft Up/Down/Tab regression pass.
The release workspace rebuilt successfully after the gate.

Paired actual-size images of both panels used an isolated diagnostic executable on
the second monitor, with the production rendering unchanged and explicit ru-RU number
formatting. The coordinator accepted the common controls and visible fields. The
only temporary hooks opened the existing panels without desktop input emulation;
only the diagnostic process was terminated. Native Wayland pointer interaction is
not claimed; adjacent hover routing is verified through GPUI pointer hit-testing.

Approved items 1, 2, 3, 5, 7 and 8 are complete. Reference policy (item 4), whole-sample
Peak/RMS statistics (item 6), and the previously documented Open chooser limitation
remain discussions outside this completed iteration; none is represented as fixed.

The actual pointer test passes File → Analysis → File using rendered trigger bounds,
and verifies the closing plot press does not create a selection. The existing
held-stepper regression passes after retaining the repeat across own preview sync.
Reference policy and integral statistics remain unimplemented pending discussion.
