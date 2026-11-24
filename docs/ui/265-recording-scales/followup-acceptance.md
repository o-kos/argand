# Recording scales: owner-feedback acceptance

The owner requested a visible aligned corner trigger, consistent fields, labelled
radio groups, standard datetime editing, file-creation default, consistent scale
terminology and restored shortcut labels. This report covers the complete approved
follow-up. Owner manual acceptance remains pending and the PR remains Draft.

## Exact snapshot and validation

Base HEAD is `0fd8ed5a6dedab57656d79e3655452fcac733b3d` plus the submitted local
follow-up. Review patch SHA256 is
`522630257299a02492618558978c0189b93c325762c90401f3375650f975c267`.
Code-file manifest SHA256 is
`4caf8e72bde3ceb2e052954e00b612cb6930ff12e2e32923b91b04272afc9c0d`.
The production binary `target/release/argand` is 7,048,576 bytes, SHA256
`42c45fade80f5a291e35ff495bd741a12d4c175345e128f4f5d15db0d3a939df`.
Exact file hashes, launch and isolation are retained in
`target/issue265-datepicker-ui/{identity,application,display}.json`.
The build's embedded revision label is not the local-diff identity. Product source
matches the reviewed packet. Later changes were tests and documentation only.

`cargo fmt --all -- --check`, `cargo clippy --all-targets --locked` and
`cargo test --locked` passed. The complete suite has 976 passing tests
(589 app, 50 CLI, 24 end-to-end, 42 core, 106 DSP, 12 edit, 153 I/O), plus doc tests.
`cargo build --release --locked` passed in 2m03s. Raw current logs are
`target/issue265-datepicker-{fmt,clippy,tests,release}.log`.
The final formatting/Clippy run includes the RAW fixture additions.

The first full run found two old RAW fixtures assuming no initial origin. They
now explicitly clear the effective default before exercising metadata-free RAW
saving. The existing source guard is unchanged. A new regression verifies that a
RAW creation origin followed by prefix trim requires Save As, preserves the RAW
source and round-trips the shifted start through WAVE. Other regressions cover
embedded priority, unsupported birthtime, clean default, explicit clear/save,
stable opened-handle creation time, precise no-op DatePicker and UTC conversion.

## Production observations

The current release ran only on verified task-owned Xvfb `:3`, 1280x800x24, using
X11 with child-only DISPLAY and unset WAYLAND_DISPLAY. Supported isolated XDG
config/state/cache/data and TMPDIR protected owner state. XTest pointer/keyboard
input was authorized. Pointer press/release were delivered separately with a
120ms hold. An earlier batched helper caused unreliable trigger clicks and was
corrected without product changes. The app quit normally and the verified
Xvfb was stopped. No owner desktop, packages, SDK or toolchain changed.

| Scenario | Actual observation | Evidence |
| --- | --- | --- |
| Inspect and open corner in both orientations | Standard outlined button is visible at rest and centered in its actual ruler/minimap corner. Panel uses common label columns, Scale and Format groups and standard field frames. | [Top-right](evidence/followup-top-right-panel.png), [bottom-left](evidence/followup-bottom-left-panel.png) |
| Open datetime by pointer and Enter | Standard calendar and seconds-precision time field open. Top-right popup opens below, bottom-left popup fits above. | [Datetime popup](evidence/followup-datetime-popup.png), [bottom popup](evidence/followup-bottom-datetime-popup.png) |
| Click day 25 outside parent panel bounds, then focus seconds and press Up | Parent and popup remain open. Start updates immediately from November 24 to 25 and from 00:00:00 to 00:00:01. Dirty indicator appears. | Same datetime popup, local captures 04/05 |
| Press Escape from time field | First Escape closes only calendar popup, retaining parent and committed date/time. Second Escape closes parent. | Local capture 06 |
| Undo date/time changes, reopen/dismiss unchanged picker, then Save | Clean original metadata remains. Direct WAVE auxi inspection afterward reads `(2026,11,2,24,0,0,0,321)`, preserving `.321` rather than applying displayed whole seconds. No-op sample-precise offsets are additionally covered by the panel regression. | Local capture 08 and raw auxi inspection in run |
| Use built-in inline clear, dismiss and Save, then reopen panel in same document | Date is unset, Absolute UTC unavailable and explanatory text present. Save completes cleanly without birthtime repopulation in this current document. | Local captures 09/10 |
| Restart with original embedded-date fixture | Original disposable fixture was restored before this separate launch. Embedded midnight date wins over filesystem birthtime. Captures 13 onward are a new launch, not resurrection of the cleared document in capture 10. | Both final panel captures |
| Inspect en-US grouping and 640px light layout | Supported isolated `number_format="en-US"` renders `100,100,000`. Fields, labels, radio choices and button fit at 640px. Earlier default C-locale captures intentionally show no grouping. | [Narrow light panel](evidence/followup-light-panel.png) |
| Main/ruler menu terminology and shortcuts | Main View and corresponding ruler entries use Relative/Absolute UTC or RF. Typed shared command wrapper supplies canonical shortcut labels while explicit selected actions remain idempotent. The earlier independent follow-up production captures exercise unchanged menu code, with rendered-menu tests in this final gate. | [Time menu](evidence/followup-time-menu.png), [frequency context](evidence/followup-frequency-context.png) |

## Lifecycle and limits

The standard picker commits real user changes immediately at seconds precision.
Escape dismisses popup first and does not undo an already committed date/time.
There are no extra panel footer actions. Numeric editing retains Enter/blur commit
and Escape for uncommitted drafts. Merely opening/dismissing the picker retains
exact sample offsets and millisecond metadata.

Embedded recording start takes priority. Otherwise a new user open adopts actual
filesystem creation time as a clean initial effective origin. Unsupported birthtime
stays unset, never using modification/change time or the clock. Explicit clear
stays unset in the current document through save rebinding/private conversion.
A later new user open of a file without embedded start again applies its creation
default, because the containers do not store a separate explicit-unset marker.
RAW cannot serialize this metadata: prefix trimming its effective creation origin
therefore keeps the existing Save As requirement.

Native Wayland, Windows and macOS execution and the owner's exact manual scenario
remain unverified. Screenshots and local automated checks do not replace owner
acceptance. No Ready, merge or release action is authorized.

## Narrow follow-up: absolute ruler suffix

The owner requested removal of `Z` only from absolute ruler tick labels. The
private shared tick formatter now omits that suffix, including date-rollover and
fractional zoom labels. UTC/date captions, cursor/selection timestamps, DatePicker,
and RFC3339 millisecond serialization are unchanged. The existing axis regression
covers both orientations, UTC caption and midnight prefix, with one additional
no-`Z` assertion. No public signatures, tick generation or storage code changed.

This separate scoped review is clean in round 1, with no declined findings.
Base is `2da1b2c21db5e7d23b47321780ec47c335f3d007`; reviewed patch SHA256 is
`b3894dc6f81b36b6a46a909f8fd356f19352714d3c17b2bac0dca776cd0d8b45`.
Formatting, all-target Clippy and all 976 tests plus doc tests pass, including the
existing WAVE/FLAC `.123Z` recording-start round-trip test. Current release build
passes in 2m10s. Logs are `target/issue265-no-z-{fmt,clippy,tests,release}.log`.
The tested binary is 7,048,552 bytes, SHA256
`ab2636d8218e6808fccf357a0250e006efde158e3426e1eacfd3d3df058ef23d`.
Source/file and binary identities are retained in
`target/issue265-no-z-ui/identity.json`.

The production binary ran in a fresh verified task-owned Xvfb using the same
supported isolated XDG directories and X11 selection described above. A disposable
23:59:59.900Z fixture crosses midnight: ruler marks show time without `Z`, retain
`11-24` rollover prefixes, and keep the `UTC` caption and date context. Real
Ctrl+Plus zoom changes labels to hundredths without restoring `Z`. The cursor
readout still includes its UTC timestamp suffix as intended. See the
[midnight ruler](evidence/followup-absolute-ruler-no-z.png) and
[zoomed ruler](evidence/followup-zoomed-ruler-no-z.png).
This bounded production check used horizontal orientation; the existing regression
checks both orientations through their shared tick formatter. App and verified
Xvfb were stopped. Owner manual/native-platform acceptance limits remain unchanged.


## Follow-up: millisecond human presentation

The owner requested at most millisecond precision for human time displays, numeric
locale fractions and a space between UTC date and time. GUI time ticks use a
minimum 1ms step, retaining real coordinates and bounds. Sample-mode counts remain
exact. Canonical RecordingTime and WAVE/FLAC RFC3339 metadata are unchanged.

Automated coverage includes relative/UTC cursor and selection presentation,
midnight rounding, dot/comma inverse entry and stepping, both ruler orientations,
large-coordinate sample fallback, retained frequency precision and actual panel
Enter/blur with unchanged rounded drafts at 1MHz. Independent review identified
one accepted correction: canonical dot UTC drafts in comma locales must step by
their fractional digit, just as localized drafts do. A regression follows the
changed-draft parse then step sequence for ru-RU and de-DE with both spellings.

Formatting, all-target Clippy and all 980 tests plus doctests pass, including
593 application tests. Review by `gpt-6.1-sol` high accepted the UTC draft stepping
correction in round 1, then returned no substantive findings in round 2. There
were no declined findings. Final logs are `target/issue265-ms-{fmt,clippy,tests}-final.log`.

The first two production launch requests timed out in automatic permission review
before process creation. After explicit owner authorization, the bounded rerun
launched successfully. The current code revision is
`1bc701e120d277481c5c8d0292927deea203616c`, with a clean tree before launch.
The existing validated production binary is 7,049,968 bytes, SHA256
`8408021da445ff4c52c77e61fce5f2b580eff2c5a54b57f0878f4252dcba0d6c`.
No product code changed, so the existing gate and build were retained.

The rerun used verified task-owned Xvfb `:3`, 1280x800x24, X11, unset
WAYLAND_DISPLAY and supported isolated XDG config/state/cache/data and TMPDIR.
The isolated configuration selected ru-RU and dark theme. The disposable 1MHz
I/Q WAVE starts at `2026-11-23T23:59:59.900Z` and lasts two seconds. XTest supplied
real pointer/keyboard events. Captures below show settled frames; intermediate
captures taken before the next render are not used as outcome evidence.

| Criterion and actual interaction | Result | Evidence |
| --- | --- | --- |
| Inspect UTC ruler date rollover, selection badges and pointer/status readout | Pass: numeric fractions use comma, endpoints use a date/time space and at most three fraction digits. UTC caption/date context remain visible. | [UTC status](evidence/millisecond-utc-status.png), [UTC endpoint badges](evidence/millisecond-utc-badges.png) |
| Enter exact `#1001` start and `#8002` inclusive end in the panel, reopen, Tab twice and Enter without changing the rounded UTC drafts, then switch Relative/Samples and reopen | Pass: fields show `#1 001`, `#8 002`, count `7 002`. Actual Enter/blur preserves the exact 1MHz selection. | [Exact samples after unchanged drafts](evidence/millisecond-exact-samples-after-noop.png) |
| In UTC mode replace the end draft with canonical dot `2026-11-24 00:00:00.001Z`, press Up then Enter | Pass: settled duration is `0:00,101`, with start offset 0.001001s and end offset 0.102s. The following relative view shows end `0:00,102`, establishing a 1ms step. | [Canonical UTC step](evidence/millisecond-canonical-utc-step.png), [relative result](evidence/millisecond-relative-vertical.png) |
| Toggle Relative and turn orientation, inspect labels, selection and status | Pass: relative hms values use comma with at most three digits. Frequency readout retains four digits where needed (`1,0000 MHz`). | [Relative vertical](evidence/millisecond-relative-vertical.png) |
| Read complete UTC endpoints in resting selection-panel fields | **Failed**: the fixed 104px endpoint fields show only date/hour, hiding minutes, seconds and milliseconds. The complete timestamps remain visible in plot badges, but that does not make panel editing readable. | [UTC panel clipping](evidence/millisecond-utc-panel-clipping.png) |

The panel-readability defect remains in active Issue #265. `FIELD=(104,24)` and
`AREA=(220,104)` in the existing selection-panel drawing are shared by all six
fields; the input and painted frame both use that fixed width. A correction needs
coordinated endpoint/frame/drawing sizing in the existing standard Input
composition, preserving orientation and the 640px minimum. No layout change was
implemented in this verification run. Width estimates alone do not establish a
fitting accepted layout.

The application quit normally with Ctrl+Q and the verified task-owned Xvfb was
stopped. Local launch/display/cleanup identities are in `target/issue265-ms-ui/`.
This bounded run did not repeat the earlier 640px/light/menu/DatePicker matrix,
exercise actual localized comma typing separately, or establish native Wayland or
other-platform behavior. Automated localized-input regressions remain separate.

Owner manual acceptance and native Wayland/Windows/macOS verification remain
pending. The PR remains Draft, with the related panel-readability criterion failed.
Current automated/build identities and exact review disposition are recorded in
the follow-up handoff packet.
