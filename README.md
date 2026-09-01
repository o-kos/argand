<p align="center">
  <img src="icons/argand.svg" width="160" height="160" alt="Argand logo">
</p>

<p align="center">
  <a href="https://github.com/o-kos/argand/actions/workflows/ci.yml"><img src="https://github.com/o-kos/argand/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://github.com/o-kos/argand/releases/latest"><img src="https://img.shields.io/github/v/release/o-kos/argand?sort=semver" alt="Latest release"></a>
  <a href="LICENSE"><img src="https://img.shields.io/github/license/o-kos/argand" alt="MIT licence"></a>
  <a href="rust-toolchain.toml"><img src="https://img.shields.io/badge/rust-1.88%2B-dea584" alt="Rust 1.88+"></a>
</p>

# Argand

A cross-platform editor and analyzer of recorded signals in both real and I/Q formats. See
[AGENTS.md](AGENTS.md) for the architecture and
[implementation roadmap](docs/plans/IMPLEMENTATION_PLAN.md). See
[CONTRIBUTING.md](CONTRIBUTING.md) before starting a change.

The GUI is not built yet. What exists today is `aspec`, a command line tool
that renders a signal file's spectrogram to a PNG, with a waveform strip above
it. It is not a throwaway: the domain model, the readers and the transforms
live in
`argand-core`, `argand-io` and `argand-dsp`, which the GPUI front end will use
unchanged. `aspec` only turns their output into an image.

## Why Argand?

The name comes from the
[Argand diagram](https://mathshistory.st-andrews.ac.uk/Biographies/Argand/),
also known as the complex plane. It represents a complex number with its real
component on the horizontal axis and its imaginary component on the vertical
axis. Each I/Q sample is naturally the complex value `I + jQ`, so the name
reflects the project's defining idea: complex signals are first-class data
rather than a pair of unrelated audio channels. The logo combines the plane's
axes with a colored spectrum trace.

## Install

Every `vX.Y.Z` tag publishes archives for `x86_64-unknown-linux-gnu` and
`x86_64-pc-windows-msvc` on the
[releases page](https://github.com/o-kos/argand/releases/latest), each holding
`aspec`, this README, the licence and the changelog, alongside a `SHA256SUMS`
file covering both:

```sh
sha256sum --check --ignore-missing SHA256SUMS
```

## Build

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --locked
cargo test --locked
cargo build --release --locked        # target/release/aspec
```

Those four commands, in that order, are what CI runs on Linux; Windows runs
the test and the release build. See [CONTRIBUTING.md](CONTRIBUTING.md) for
the full validation and release procedure, and [CHANGELOG.md](CHANGELOG.md)
for what changed when.

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
aspec capture.iqw --panels waveform,psd,db
aspec '*.iqw' --center 12.579M
aspec /data/'[0-9]*.wav' --raw iq_i16@24k -q
```

The container is detected from the file's content, not its extension, so
`.wav`, `.iqw` and `.wavs` all work. A file with no header needs `--raw`.

### Inputs

An input is an exact path or a mask over the filenames of one directory:
`*` for any run of characters, `?` for exactly one, `[0-9]` and `[!0-9]` for
a set. Quote the mask so the shell hands it over intact -- that is what makes
the same command line work in `bash`, `zsh`, `fish` and PowerShell alike.
Paths the shell has already expanded are accepted just as well.

Masks are deliberately not recursive. `**` and a mask in a directory
component are refused, and a mask that matches nothing is an error rather
than a silent no-op. Matches are sorted by filename and de-duplicated, so the
same capture named twice is still rendered once.

```sh
aspec '*.iqw' --raw iq_i16@24k --center 12.579M   # every capture, same settings
aspec a.iqw '/data/2026-0[1-6]*.wav' b.iqw        # exact paths and masks mixed
```

Every option applies to every file. With more than one file, `-o` is refused
and each PNG is written beside its input as `<input>.png`. A file that fails
does not stop the rest, and the run exits non-zero once it has finished if
any file failed.

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
      --panels <P>          waveform, psd, db, none [waveform]
      --orientation <D>     horizontal (time across) or vertical (waterfall)
  -o, --output <PNG>        one input only [<input>.png]
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

A batch shrinks the report to one line per file, with a
processed/succeeded/failed/elapsed summary after it:

```
[1/3] 12.579000_25_08_26_06_09_10.iqw  iq_i16 · 24 kHz · 30m · peak -99.8 dBFS  →  *.png  212.8 KiB  463ms
```

`*.png` is the render, named by what it adds to the input rather than by
spelling the whole path out a second time. On a terminal that line is all a
finished file prints; the paths reappear on stdout the moment stdout is a
pipe or a file, one per line, so `aspec '*.iqw' | xargs feh` still works.
`-v` brings the full block back for every file, `-q` says nothing but still
names any file that failed, and `--json` prints one report object per file --
a stream that `jq` and `serde_json::StreamDeserializer` both read as it
arrives.

### Panels

The spectrogram is always drawn, so `--panels` selects only what joins it:

| panel | what it adds |
|---|---|
| `waveform` | a time-domain strip sharing the spectrogram's time axis |
| `psd` | the averaged spectrum, on the spectrogram's frequency axis |
| `db` | the colour bar explaining the spectrogram's colours |
| `none` | nothing; the spectrogram on its own |

The strip goes above the spectrogram when time runs across and to its right
when time runs down, always covering the same span of the time axis, so a
burst can be traced from one panel into the other. It is a min/max envelope
rather than a decimated one: a burst shorter than a single pixel column still
reaches the edge of the strip instead of averaging away. Its scale is linear
against the `--ref` level -- full scale, or this file's loudest sample under
`--ref peak`. A complex signal is one track, spanning whichever of I and Q
reached further in that column.

### Axes

How many labels an axis carries is decided from how long it is and how wide
its labels turn out to be, not from a count fixed in advance. Every candidate
step is formatted and measured with the font the plot draws with, and the
densest one whose labels still keep two digits of clear space wins. A wider
image therefore gets more coordinates rather than the same few spread further
apart, and a narrow one does not stack them on top of each other.

Values stay round. Numeric axes step by 1, 2 or 5 times a power of ten, and
zero is exact whenever it is on the axis. Time steps on a clock -- 1, 5, 15,
30 seconds, a minute, five, an hour -- and never finer than one second, so a
label never carries a fraction. Time reads as `1:02:09` over a span of an hour
or more and `3.07` -- minutes and seconds -- below that, whatever point of the
recording the span was taken from. A window shorter than a second shows the
whole seconds inside it and nothing else.

The unit is named once beside the axis rather than repeated on every tick:
`MHz` at the head of the frequency labels and `dB` above the colour bar, with
bare numbers under them. Repeating it costs a third of every frequency label
and half of every colour-bar label to say the same thing a dozen times, and it
is what used to put `999.999 Hz` and `1.000 kHz` on one axis -- two spellings
of neighbouring values. One unit for the whole axis ends that too.

The foot of the plot names the vertical scale as `dBFS, ENBW 17.578 Hz`. The
level is referenced to full scale -- a full-scale tone reads 0 dBFS whatever
the transform size, because the transform divides by the window's coherent
gain and not by any bandwidth. Noise is the part that moves with the
bandwidth, so the bandwidth is named beside it: the window makes a bin answer
to noise across `ENBW` hertz, half again the raw `Fs / N` spacing under a Hann
window. Two renders of one capture at different `--fft-size` therefore show
the same carrier level and different noise floors, and the `ENBW` field is
what makes those floors comparable -- subtract `10 log10(ENBW)` from either to
reach a density per hertz.

Every label drawn has a grid line or tick mark at the same value, and a label
that would not fit whole inside the canvas is dropped along with its line
rather than clipped. Coordinates stay inside the span being drawn, to within
the pixel that separating a rounding artefact from a real value costs. Panels sharing an axis are given one set of values, so
the waveform strip's grid lines fall on the spectrogram's, and the averaged
spectrum's on the spectrogram's frequencies. The gutters holding the labels
are sized by measuring the widest label the axis could print, which is why a
capture tuned to 12.579 MHz gets a wider left margin than a baseband one.

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
