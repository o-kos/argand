# Native checkpoint results

Date: 2026-09-21. This is a limited Linux run, not the full architecture matrix.

## Build and environment

- Production source is unchanged from the main revision underlying PR #125.
- Fixture source is the initial #126 implementation, before independent-review fixes.
- GPUI 0.2.2 and gpui-component 0.5.1 from the existing Cargo.lock.
- Linux, Wayland, Sway 1.9 isolated headless output, scale 1, 1600 by 1000 pixels.
- Blade/Vulkan reported Intel Iris Xe Graphics (ADL GT2); the compositor opened
  `/dev/dri/renderD128`. This was not a software-rendering substitute.
- Private XDG runtime/config/state/data directories; no existing desktop windows
  or user configuration were used. Input came from virtual pointer/keyboard devices
  connected only to that seat; the keyboard used a US keymap.
- Initial fixture release SHA-256:
  `c966e67dafd36031aaa111e03058417ab615aff5894715acab340c111f2a6856`.
- Production release SHA-256:
  `ad4d473cb1b386c427982a2a06e47b70047cb0fc610a8364e7d42119fa951bc9`.

## Reproduced blocker: Select inside Popover

1. Run `cargo run -p argand --example ui_compatibility --release --locked`.
2. Click **Retained editor**. Standard NumberInput and Select render in the popup.
3. Click the **Reducer** Select to open its options.

Expected: the options open; the process stays alive and focus can enter the list.
Observed: process exits with status 101. Reproduced on two fresh launches in the
light, horizontal configuration; the second sequence did not touch NumberInput.

```text
gpui-0.2.2/src/window.rs:2184:9:
assertion `left == right` failed: cannot call defer_draw during deferred drawing
  left: 1
 right: 0
stack backtrace:
   0: __rustc::rust_begin_unwind
   1: core::panicking::panic_fmt
   2: core::panicking::assert_failed_inner
   3: core::panicking::assert_failed::<usize, usize>
   4: <gpui::window::Window>::draw
```

The locked `popover.rs` and `select.rs` each introduce a deferred overlay. This
source observation is consistent with the assertion; it is not a tested fix or
proof that every supported composition is impossible. Keep the reproducer intact
and investigate supported composition before requesting any dependency change.
Do not replace Select with custom rows to hide the failure.

## Initial-build observations and limits

- Production release opened `tests/signals/m39.wav` and rendered waveform,
  spectrogram, axes and status. This is a startup baseline, not an interaction pass.
- The fixture opened without Root lookup failure. Light/dark theme switching was
  observed. One title bar was visible; full frame hitbox validation is outstanding.
- Popover content rendered over the plot. See [open editor](evidence/editor-open.png).
  Opening alone does not establish keyboard, IME or complete pointer isolation.
- Horizontal resizing was repeated with multiple motion events to cross the stock
  drag threshold: press at (771,450), move to (740,450), (680,450), (640,450),
  with 250 ms between movements, then release. In dark/horizontal at scale 1,
  dimensions changed from 531/587 to 399/719 while held, with the callback count
  still 1 from an earlier drag. See [before](evidence/horizontal-before.png),
  [held](evidence/horizontal-held.png), [released](evidence/horizontal-released.png).
  This demonstrates live size observation in this case, not the full Z1 lifecycle
  or production worker scheduling. Plot move feedback also changed during this
  splitter drag; no complete interaction-isolation pass is claimed.
- A later keyboard check focused NumberInput, sent Ctrl+A, typed `4096`, Enter,
  Escape, and reopened the popover. The field and retained-state panel both showed
  `4096`: [reopened editor](evidence/number-reopened.png). This verifies that bounded
  editing/reopen sequence only, not clipboard, validation, IME or focus restoration.
- Focus restoration, nested Escape, clipboard, numeric validation and Alt guides
  remain unexercised. Windows/macOS and other Linux desktop compositors remain
  outstanding. Initial screenshots label bounds as `restore`; review identified
  that label as inaccurate for some backends, so they must not be used as R2 proof.

## Post-review build observations

Revised release SHA-256:
`2f94659f09dca55d5a0da7366a76c75a1375135cee26951db165850d50d7e594`.
Same Linux/GPU/compositor/scale, light theme, horizontal arrangement.

| Case | Input and expected result | Observed result / evidence |
| --- | --- | --- |
| Scoped plot key | Click plot, press P; one plot action | PASS for this sequence: plot focus and count 1, [frame](evidence/revised-focus.png) |
| Input isolation and invalid feedback | Open editor, type P; text changes, plot action count stays 1 | PASS for this sequence: `p2048`, visible invalid feedback, plot count 1, [frame](evidence/revised-input-p.png) |
| Escape focus return | From editor opened after plot focus, Escape then P should return to prior plot target | FAIL: popup closes, tracked focus remains number input, plot count stays 1, [frame](evidence/revised-after-escape.png); no custom restoration was added |
| NumberInput Step | Replace text with 2048, click stock plus once | PASS for this sequence: text 2112, valid feedback, step count 1, [frame](evidence/revised-step.png); boundaries/invalid input are additionally model-tested |
| Passive hint target | Focus plot, hold Alt, hover target; observe pointer and underlying guides independently | Arrow observed, but both plot/target hover and Alt guides remain active, [frame](evidence/revised-hint-alt.png); no P2 compatibility pass |
| Enter tooltip position | Move from (700,231) target to visible tooltip position (720,260), Alt held | Tooltip dismisses and crosshair/readout remain, [frame](evidence/revised-over-hint.png); sustained covered-tooltip click/wheel behavior is not established |
| Select in Popover | Reopen editor and click Reducer | FAIL: same `defer_draw` assertion and exit 101 on revised build; standard composition intentionally unchanged |

Reported-bounds text is correctly labelled in the revised build, but the long
toolbar summary clips at the default width. Enlarge the window before recording
geometry; no R2 measurement pass is claimed from these images. Compact layout
and remaining native cases stay outstanding in #126. The stock Tooltip case
above is not proof about every supported hoverable-tooltip composition.

## Decision

NO-GO for production migration (#127). The runnable failure is useful checkpoint
evidence, not a completed compatibility gate. Continue #126 with supported toolkit
composition investigation and explicit remaining native checks. Any required
dependency update/patch or UX exception needs a separate, concrete owner decision.
