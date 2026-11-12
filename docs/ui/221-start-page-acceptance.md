# Start-page centering acceptance (#221)

## Build and source identity

Validated on 2026-11-12, Linux x86-64, Rust 1.98.1, the installed GPUI Kit 0.9.1 SDK.
The production binary is the linked worktree's
`target/x86_64-unknown-linux-gnu/release/argand`, built with
`cargo build --release --locked` after formatting, Clippy and tests passed.

The binary was built from the plan commit plus the submitted layout/test diff.
Source checksums were recorded before the production run and rechecked afterwards;
no production or test source changed during acceptance. This binds the demonstrated
binary to source content independently of later documentation-only commits.

| File | SHA256 |
| --- | --- |
| `crates/app/src/shell/mod.rs` | `9b3679bec22dddf18430c02dc0e79603ce7de4e833baef5482bda942cf00c614` |
| `crates/app/src/shell/shell_start_page_test.rs` | `76bb94816207defd22dc7a02c0a1c83b194a410cf66d9429ebccd138d68cf09d` |

Production binary SHA256: `f5dab4c97ea90b3b0be1f994c97bd45755ddcaccf5329e3dfefbad965ac6ec26`.

## Automated checks

- `cargo fmt --all -- --check`: passed.
- `cargo clippy --all-targets --locked`: passed.
- `cargo test --locked`: 899 tests passed, no failures; doc tests passed.
- `cargo build --release --locked`: passed after the preceding checks.
- The new production-Shell regression checks equal group/page centers within one
  logical pixel, containment and chooser availability at 800×600, 420×260 and
  800×220 with zero, one and ten entries. It checks the shared number-column inset,
  content sizing for short names, and bounded long/disambiguated rows at 420×400.
  The sizes below the application's 640×400 minimum are deliberate headless stress
  cases, not a claim about a selectable native window size.

## Production rendering and interaction

All launches used a separately started, identity-checked Xvfb X11 server. No launch
or input used the owner's display. The child had its own XDG config, state, cache,
data and runtime directories and D-Bus session; Wayland selectors were removed.
Only generated one-second, 8 kHz mono WAVE files and disposable session histories
were opened. Pointer/key events were sent through XTest to this isolated display.
The actual release binary rendered the following scenarios; these are separate
from the headless regression above.

| Scenario | Observed result |
| --- | --- |
| Dark theme, three short recent files, 800×600 | Whole heading/list/or/chooser group centered in the content area; number and name columns remain aligned. |
| Light theme, ten files, 640×400 | Heading and chooser remain visible, with the list alone clipped to its available scroll height. Heading/or share their leading edge; chooser text and filenames share their leading edge. |
| Wheel scrolling at minimum size | Scroll reaches rows 5–9/0 while heading/or/chooser stay in place; clicking row 8 opens `capture-7.wav`. |
| Alt+0 | Opens the tenth displayed entry, `capture-9.wav`; title and actual signal view change accordingly. |
| Long duplicate filenames distinguished by parent paths, 640×400 | Group stays within its width bound; middle truncation keeps distinguishing folder text and the `.wav` extension. |
| Recent-row pointer activation | Clicking the first recent row opens `capture-0.wav`, with the expected one-second capture displayed. |
| Chooser pointer activation | Start-page chooser opens the standard portal file chooser. Escape cancels and leaves the page/history intact. |
| Ctrl+O and chooser confirmation | Reopens the chooser; selecting the generated `capture-1.wav` and confirming with Enter opens that capture. |
| Empty light-theme history | Only the centered chooser appears. Tab draws the standard focus outline; Enter opens the chooser and Escape dismisses it. |
| 125% X11 scale, three files | Supported `GPUI_X11_SCALE_FACTOR=1.25` in the child produces a 1000×750 physical window for 800×600 logical bounds; the group remains centered and internal columns aligned. |

## Review and limits

Independent `gpt-6.1-sol` high read-only review found no substantive layout,
overflow, behavior, component, test or policy defect in round 1. A final handoff
also covers this acceptance record and the exact submitted state.

An already visible recent-row tooltip can retain its former text when scrolling
moves another row beneath a stationary pointer. Leaving and re-entering refreshes
it; clicks and shortcuts still open the correct capture. The tooltip registration
and scroll mechanism are unchanged by this layout patch. The separate cosmetic
follow-up is [#253](https://github.com/o-kos/argand/issues/253); no claim of a
pre-change runtime reproduction or verified toolkit root cause is made.

This establishes production X11/Xvfb behavior at 100% and 125%, not native Wayland,
Windows, macOS, physical-display behavior or arbitrary rem/font overrides. Those
platforms were not run locally. CI results must be assessed independently on the
current PR revision. The PR remains Draft for owner feedback; this record does
not imply owner acceptance or authorize release publication.
