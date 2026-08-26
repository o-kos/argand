# argand

Cross-platform editor and analyser for recorded signals, with I/Q (complex)
captures as first-class citizens rather than an afterthought. See
[AGENTS.md](AGENTS.md) for the architecture and
[implementation roadmap](docs/plans/IMPLEMENTATION_PLAN.md). See
[CONTRIBUTING.md](CONTRIBUTING.md) before starting a change.

The GUI is not built yet. What exists today is `aspec`, a command line tool
that renders a signal file's spectrogram and averaged spectrum to a PNG. It is
not a throwaway: the domain model, the readers and the transforms live in
`argand-core`, `argand-io` and `argand-dsp`, which the GPUI front end will use
unchanged. `aspec` only turns their output into an image.

## Build

```sh
cargo build --release --locked        # target/release/aspec
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
```

`Cargo.lock` pins the complete dependency graph. Cargo downloads missing
sources into its local cache during a normal build. To prepare that cache and
then build without network access:

```sh
cargo fetch --locked
cargo build --frozen
```

## aspec

```sh
aspec capture.iqw -o spec.png
aspec dump.bin --raw iq_i16@24k --center 12.579M
aspec quiet.wav --normalize auto --ref peak -d 40
aspec long.iqw --start 5m --duration 30s --orientation v
```

The container is detected from the file's content, not its extension, so
`.wav`, `.iqw` and `.wavs` all work. A file with no header needs `--raw`.

### Formats

| token | stored as | notes |
|---|---|---|
| `rl_u8` / `iq_u8` | uint8 | offset binary, silence at 128 |
| `rl_i16` / `iq_i16` | int16 LE | |
| `rl_i32` / `iq_i32` | int32 LE | |
| `rl_f32` / `iq_f32` | float32 | scaled to [-1, 1] |
| `rl_f16x8` / `iq_f16x8` | float32 | arbitrary scale (CoolEdit "16x8") |

`rl_` is a real signal (one channel), `iq_` is complex with I and Q
interleaved (two channels). Containers: WAV for all ten, FLAC for the integer
ones, and headerless files for all ten via `--raw <token>[@<rate>]`.

Unnormalised float files are the odd one out. They are written with the
integer PCM format tag and 32 bits per sample, so nothing distinguishes them
from `i32` except a 20-byte `fmt ` chunk carrying the magic word `0x00010002`.
`aspec` detects that and, by default, measures the file's peak and divides by
it -- `--normalize` overrides the decision either way.

### Options

Short flags match the sgvr CLI (`-f -w -c -i -d`) on purpose.

```
  -f, --fft-size <N>        transform size, a power of two [2048]
      --hop <N>             frame advance [fft-size / 4]
  -w, --window-type <W>     hann, hamming, blackman-harris, rect [hann]
  -c, --color-scheme <C>    oceanic, grayscale, inferno, viridis, synthwave, sunset
  -d, --dynamic-range <DB>  range below the reference level [110]
      --ref <R>             fs (format full scale) or peak (this file) [fs]
      --reduce <R>          max or mean, when frames share a column [max]

  -i, --image-size <WxH>    [2048x512]
      --mode <M>            spectrogram, psd, both [both]
      --orientation <D>     horizontal (time across) or vertical (waterfall)
  -o, --output <PNG>        [<input>.png]
      --json                machine-readable report on stdout

  -t, --sample-type <TYPE>  override the detected type
  -r, --rate <HZ>           override the sample rate: 24000, 24k, 2.4M
      --center <HZ>         centre frequency for the axis [0, baseband]
      --offset <BYTES>      skip a header in a raw file
      --start, --duration   span to analyse: 12.5, 1m30, 01:30, 250ms
  -n, --normalize <MODE>    none, auto, or an explicit divisor
  -g, --gain <DB>           applied after normalization
```

The report goes to stderr, the output path to stdout, so `aspec x.iqw` can be
piped. `--json` replaces the path with the full report, which is what the
tests assert against.

### What it gets right

Two things here differ from an ordinary audio spectrogram, and both matter for
I/Q:

* A complex signal goes through a complex FFT and is `fftshift`ed, giving a
  genuinely two-sided spectrum from -Fs/2 to +Fs/2 in which +2 kHz and -2 kHz
  are different places. Feeding the interleaved stream to a real transform
  instead -- the easy mistake -- collapses them and halves the frequency axis.
  A real signal takes the real transform and gets a one-sided spectrum.
* Frames fold into image columns as they are computed, so memory follows the
  output size rather than the length of the capture, and consumed pages are
  released back to the kernel. A 30-minute, 172 MB I/Q capture renders in
  about 0.8 s with the resident set flat at roughly 60 MB.

A full-scale tone reads 0 dBFS in either domain, so the colour scale means the
same thing for a real recording and a complex one.

## Layout

```
crates/core   domain types, render view-models. No IO, no DSP, no toolkit.
crates/io     container detection and readers. Implements core's SampleSource.
crates/dsp    windows, STFT, averaged spectrum. Consumes SampleSource.
crates/cli    the aspec binary: arguments, report, PNG composition.
```

Dependencies run one way: `core <- io`, `core <- dsp`, and all three into
`cli`. There is no edge between `io` and `dsp` -- the transform does not know
what a file is, and the readers do not know what a transform is. Keeping that
seam is what lets the GPUI front end replace `cli` without touching anything
below it.

## Tests

```sh
cargo test
```

Fixtures for all ten sample types are generated at test time rather than
committed. Tests that use real captures look for them in `tests/signals/` and,
for the wider format matrix, in `../sgvr/cli/tests` or wherever
`ARGAND_EXTRA_FIXTURES` points; they report and skip when those are absent.
