# Standard Dialog comparison: native follow-up

Date: 2026-09-21. Scope: a supported modal composition in the #126 fixture, not
an approved replacement for an anchored editor. Production UI is unchanged.

## Exact build and environment

- GPUI 0.2.2 / gpui-component 0.5.1, unchanged Cargo.lock.
- Example source SHA-256:
  `21080027c2506bebe312ba5d106cec0e0b57e8c3bd31cedfc833e9dd7550222e`.
- Example model SHA-256:
  `412e701caf0a2d87c2aa3a415b6d7d8afe70439543d36093db36feebfd12676d`.
- Tested release SHA-256:
  `c95f01c0cd9dcc4dcdcbc99def10f30c62e32e00e5b617ff34b8c9becbc8e886`.
- Linux/Wayland, isolated Sway 1.9 headless output, 1600×1000, scale 1,
  hardware GPU: the fixture process held `/dev/dri/renderD128` descriptors.
  Same host/GPU setup as the [initial evidence](native-results.md).
- Private XDG runtime/config/state/data directories; a private virtual pointer
  and US-keymap keyboard. No existing user windows or clipboard were used.
- Detailed input sequence: light theme, horizontal panels, Sway content rectangle
  (240,120), 1120×760; GPUI reported viewport 1144×784. These are different
  measurements, not a claim about guaranteed restore bounds.
- Additional compact observation: content rectangle (440,240), 720×520,
  GPUI viewport 744×544. Dark compact testing covered opening the dropdown only.

Before native runs, formatting, all-target Clippy, all workspace tests, nine
example tests, and production/example release builds passed in that order.
The existing proc-macro-error2 future-compatibility notice remains unchanged.

## Composition and interpretation

The fixture has one top-level Root and one `Root::render_dialog_layer`. Dialog
uses its standard `confirm()` configuration and separate retained NumberInput
and Select entities. `WindowExt` owns focus save/restore; there is no custom focus
repair or input suppression. Validation is read from current text on render and
on OK. This is fixture numeric policy, not product FFT or commit/cancel semantics.

Locked Popover and Select both defer their overlay drawing; GPUI rejects nested
deferred draws. The Root dialog layer does not introduce that outer deferred draw.
The results below demonstrate this alternate composition only. They do not fix
the original Popover, approve modality, or satisfy the passive-hint contract.

## Witnessed input ledger

All PASS labels are limited to the stated sequence and environment.

| Case | Input / expected result | Observed result and evidence |
| --- | --- | --- |
| Dropdown opens | Click plot at (430,400), press P, click Modal Dialog at (600,179), click Select at (840,425); open options without a panic | PASS: plot P count 1, options visible and process alive; [open Select](evidence/dialog-select.png) |
| Nested Escape and focus return | Escape once closes only the list; Escape again closes Dialog; P reaches the previously focused plot | PASS: [first Escape](evidence/dialog-first-escape.png) leaves Dialog and Select focus; [second Escape then P](evidence/dialog-focus-return.png) shows plot focus and P count 2 |
| Input isolation / live invalid feedback | Reopen, click number at (800,350), Ctrl+A then P; plot count stays 2; OK must reject invalid text | PASS: field becomes `p`, live invalid message appears, OK at (989,505) leaves Dialog open; [invalid OK](evidence/dialog-invalid-ok.png) |
| Clipboard | In number field Ctrl+A, type 2048, Ctrl+A/Ctrl+C, type 4096, Ctrl+A/Ctrl+V | PASS: text returns to 2048 with valid feedback; [paste](evidence/dialog-paste.png). Clipboard is private to this compositor |
| Undo / redo | Ctrl+Z, then Ctrl+Shift+Z, Tab, Shift+Tab, Ctrl+Y | Ctrl+Z returns the coalesced edit batch to `p`, [undo](evidence/dialog-undo.png). Ctrl+Shift+Z has no effect here; the locked Linux redo binding is Ctrl+Y, which restores 2048, [redo](evidence/dialog-redo-shift-tab.png). PASS for Ctrl+Z/Ctrl+Y, not per-character undo granularity |
| Tab pair | Tab from number field, then Shift+Tab | PASS for this pair: [Tab](evidence/dialog-tab.png) reports Select scope; [Shift+Tab](evidence/dialog-redo-shift-tab.png) reports number input. This does not establish full cyclic traversal or a focus trap |
| Step and keyboard choice | Click plus (998,350), click Select (840,425), Down, Enter | PASS: 2048 becomes 2112; Mean power is selected and Dialog remains open, [choice](evidence/dialog-choice.png) |
| Covered pointer / wheel | Within number field drag (711,350) → (740,350) → (760,350); wheel once there, then once over modal background at (430,600); click background, then P | PASS for sampled isolation: text selection appears, [covered input](evidence/dialog-covered-input.png); outside click does not dismiss the standard confirm dialog; P edits the selected number, not the plot. After Ctrl+Z, Escape, P: plot presses/clicks remain 1/1, wheel remains 0, P becomes 3 only after closing, [counters](evidence/dialog-isolation-counters.png) |
| Retained state | Reopen after the preceding undo/close sequence | PASS: 2112 and Mean power remain, [reopened](evidence/dialog-retained.png). This is retained fixture state, not transactional Cancel behavior |
| Wheel positive control | Close Dialog, move to (430,600), wheel once | PASS: exposed plot wheel count becomes 1, [positive control](evidence/dialog-wheel-control.png). The blocked-wheel result above is not an undelivered-device artifact |
| Compact toolbar | Resize only the fixture to 720×520 content | PASS for toolbar/metrics layout: buttons wrap and metrics remain visible, [compact](evidence/dialog-compact.png). This is not an all-edge, maximize/restore or stock-frame acceptance test |
| Dark compact dropdown | Switch theme, open Modal Dialog, click Select | PASS for opening only: [dark dropdown](evidence/dialog-dark-select.png). The full input sequence was not repeated in dark mode |
| Original crash remains opt-in | Fresh launch with `--popover-crash-probe`; click Retained editor crash probe (806,179), then its Select (870,408) | Original composition still FAILS: exit 101, `gpui-0.2.2/src/window.rs:2184`, `cannot call defer_draw during deferred drawing`, left 1/right 0. [Explicit warning](evidence/crash-enabled.png). Normal launches expose only a disabled crash-probe button |

## Decision and remaining work

Standard in-window NumberInput/Select composition is viable in this tested modal
Dialog configuration without a toolkit patch or a custom ordinary control.
This narrows the failure to the tested composition; it does not prove that a
non-modal anchored editor is impossible or authorize replacing one with a modal.

The checkpoint remains **NO-GO for direct production adoption on the locked
graph**: anchored Popover composition/focus,
passive hints, full frame/gesture coverage, IME, complete keyboard traversal,
Windows/macOS and other required matrix cases remain unresolved or unexercised.
The owner rejected the stock frame and retained the Argand frame. #137 must repeat
these component checks on the migrated graph before #127 adopts borderless Root.
