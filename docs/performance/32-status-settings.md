# Status-bar analysis settings validation

Local validation on 2026-09-09 for #32, #61 and the range-advice portion of #62.
The release executable was rebuilt after formatting, Clippy and all 439 tests passed.
No lint policy was relaxed. Cross-platform native interaction is not covered by this
Linux experiment; the normal Ready PR matrix remains the merge gate.

## Native environment and interaction

Intel Core i5-1235U, Linux, Sway 1.9, a separate 1600 × 1000 headless output using
GLES2 and the real `/dev/dri/renderD128` device. The application also opened that GPU
node. Each case had separate XDG configuration, state and cache directories, so the
owner's running desktop and saved preferences were unaffected.

Checked dark and light themes, 1000 × 700 and 640 × 400 windows, real WAV,
complex WAV and FLAC, and a repeated 1 GB WAV. Native interactions covered:

- FFT, window, overlap and aggregation changes producing replacement previews;
- all range modes, colour selection and the clickable yellow recommendation;
- an FFT larger than the file showing an error while retaining the picture;
- Escape dismissal, file-chooser dismissal of the settings panel, and scrolling
  expanded choices in a narrow window;
- restart restoring FFT 1024, Hamming, 50% overlap, Mean power, Inferno and Auto;
  configuration bytes remained unchanged;
- FFT 4, changing overlap from 75% to 95%: both resolve to hop 1. The selected
  percentage was saved, the popup stopped indicating an update, and the log still
  contained exactly one completed analysis.

The first popup prototype exposed a native toolkit panic when a nested dropdown
attempted another deferred draw. The final panel uses inline expanding choices in
one popover; the same interactions then completed without the panic.

[Light file hint](32-data/file-hint.png),
[style change during refinement](32-data/live-settings.png),
[whole-sample overlap alias](32-data/overlap-alias.png),
[restored settings in a narrow window](32-data/restored-narrow.png).

## File metadata evidence

| Fixture | Count | Bytes | Original extrema |
| --- | ---: | ---: | --- |
| `m39.wav` | 424703 real samples | 849450 | −32768 … 32767 |
| `iq_f32-ft8.flac` | 10238976 I/Q pairs | 24139172 | I: −12214 … 7522; Q: −10878 … 7776 |

WAV values were independently read with Python's `wave` module. FLAC values were
independently decoded through `libsndfile` using Python `ctypes`; the
[reference result](32-data/flac-reference.json) matches the final native hint.
Reader tests additionally cover unsigned 8-bit offset, signed 16-bit and raw
24-bit values, normalization and gain. Extrema are recovered from decoded values;
the hint and README disclose decoding precision rather than promise bit-exact
recovery for every integer depth.

## Cached style latency

After the FLAC analysis completed, five Oceanic/Inferno alternations and one
recommended-range application were measured in the 1000 × 700 window. All six
kept the original completed-analysis count at one.

| Change | Compositor presentation, ms | Screencopy receipt, ms |
| --- | ---: | ---: |
| Palette 1 | 27.332 | 34.104 |
| Palette 2 | 25.959 | 34.947 |
| Palette 3 | 26.045 | 33.156 |
| Palette 4 | 25.783 | 33.815 |
| Palette 5 | 25.776 | 32.616 |
| Recommended range | 25.620 | 32.596 |

[Raw measurements](32-data/cached-latency.json) and the
[measurement client](32-data/latency.c) are retained. The client moves the virtual
pointer to the desired choice, waits for hover to settle, records a baseline frame,
and injects a click. Its start timestamp is immediately before the release event
is flushed. A 24 × 24 region inside the spectrogram and outside the popup is hashed
until changed pixels appear. The client uses the clock advertised by
`wp_presentation.clock_id` and the `wlr-screencopy` ready event's presentation time.
The separate receipt measurement includes GPU readback and IPC, explaining why it
exceeds 30 ms. These are compositor results, not physical monitor latency.

To build the client, use `wayland-scanner client-header` and `private-code` on
`wlr-screencopy-unstable-v1.xml`, `wlr-virtual-pointer-unstable-v1.xml` and
`presentation-time.xml`, writing `screencopy.h`/`screencopy-protocol.c`,
`pointer.h`/`pointer-protocol.c` and `presentation.h`/`presentation-protocol.c`
into a temporary build directory beside a copy of the client. The first two XML
files are in the Cargo registry's `wayland-protocols-wlr` package; the third is in
`/usr/share/wayland-protocols/stable/presentation-time/`.

```sh
cc -O2 latency.c screencopy-protocol.c pointer-protocol.c presentation-protocol.c \
  -lwayland-client -o latency
# Run only against the isolated compositor; arguments are output coordinates.
./latency 560 736 350 350
```

The client assumes one 1600 × 1000 output and an in-bounds 24 × 24 probe region.
Open the palette choices before measuring; alternate a different palette on each
run. Popup placement and coordinates depend on the fixture and window dimensions.

## Active refinement

The 1 GB file used FFT 2048 and one worker so the actions occurred before analysis
completion. The palette edit arrived at 16.4% refinement; the range edit at 22.4%.
The new CPU snapshots took 5.572 and 11.284 ms, and CPU texture preparation for those
snapshots completed after 13.652 and 16.679 ms. These are trace-derived preparation
timings, not presentation measurements. The test completed exactly one transform
pass. [Raw results](32-data/refinement-latency.json).

DSP/worker tests cover style refresh between sparse preview batches as well as
refinement batches, queue saturation and cancellation, and cached colour/range
changes preserving transform generation, frame counts, dB values and PSD.

There is no universal 30 ms guarantee for blocked sample I/O or a single oversized
FFT. When a replacement transform has not produced its first preview, the retained
old paired picture keeps its original transform and style; style edits apply to
the incoming preview. Once that overview is available, edits re-shade it without
restarting the transform. This interval is explicitly documented in the README.
