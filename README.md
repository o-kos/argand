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

Two binaries share one core. `aspec` is a command line tool that renders a
signal file's spectrogram to a PNG, with a waveform strip above it. `argand` is
the graphical application: it opens a capture -- by argument, from its menu, by
drag and drop, or from the files it remembers -- analyses it on a thread of its
own, and shows a full-capture waveform minimap above the spectrogram, with
time and frequency marks around the spectrogram. A sparse preview appears before
the full analysis. The spectrogram refines from left to right with status-bar progress;
the minimap replaces its preview once the independent waveform scan completes. Time zoom and pan are supported; selection and editing remain deferred.

Neither binary is a throwaway. The domain model, the readers and the transforms
live in `argand-core`, `argand-io` and `argand-dsp`; both front ends call the
same `analyze`, place their axis marks with the same policy, and differ only in
what they turn the result into.

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
cargo build --release --locked        # target/release/{aspec,argand}
```

Full CI runs those four commands on Linux, and tests plus release builds on
Windows and macOS. Draft PRs run only formatting and Clippy on Linux, plus the
small CI-policy test. Ready PRs request full validation before merge.
See [CONTRIBUTING.md](CONTRIBUTING.md) for
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
aspec quiet.wav --normalize auto -d auto
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
  -d, --dynamic-range <DB|auto>
                            range below the measured peak, or auto
                            [default: absolute 0...-110 dBFS]
      --reduce <R>          max, mean (dB), or mean-power [max]

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

The report goes to stderr as one section per file: a header naming the file
and its facts indented under it, comma-separated and each said once.

```
12.579000_25_08_26_06_41_10.iqw:
  wav iq_i16, 24 kHz, 30m
  peak -53.3, bin -94.1 @ -9.387 kHz, floor -120.6 dBFS
12.579000_25_08_26_06_41_10.iqw.png:
  fft 2048, hann, hop 512, 84372 frames, range 110 dB, try -d 40 to fit the drawn range
  2048×512, 498.2 KiB, 1.246s
```

What was measured in the signal belongs to the input; what the picture shows,
and the transform that drew it, belong to the render. The second line gains
`, analysed <span>` when `--start` or `--duration` made the analysed span
differ from the file. Levels are dBFS, which is the scale the range decision
is made on. The render is written beside its input, so its own name locates
it; `-o` pointing anywhere else makes the header the whole path.

`-v` restores what the default drops -- the sample count, the divisor, the
scaling the caller asked for, the reduce mode, each level in the file's own
units, and the render's full path -- with the divisor named once, on the file
it is a property of.

The render's path also reaches stdout, one per line, whenever stdout is not a
terminal, so `aspec '*.iqw' | xargs feh` works and a terminal is not told the
same path twice. `--json` replaces that line with the full report, which is
what the tests assert against.

Without `-d`, colours keep the absolute `0...-110 dBFS` window, so captures
remain directly comparable. A numeric value, such as `-d 40`, spans that many
decibels below the measured spectral peak. `-d auto` measures the distance
from the peak to the median spectral floor, adds 50% headroom, rounds upward
to 10 dB, and clamps the result to `20...120 dB` before applying it.

When a default or numeric range is at least 10 dB wider than that calculated
range, the report ends the render's first line with `try -d N to fit the drawn
range`, beside the range it argues with, while the image footer places
`(sugg -d N)` immediately after its scale reference. Automatic mode already
applies the recommendation and therefore does not repeat it as a suggestion.
The JSON STFT block exposes `dynamic_range_mode`, the effective
`dynamic_range_db`, and `recommended_dynamic_range_db` separately.

A batch shrinks the report to one line per file, using the same field names,
order and units, with a processed/succeeded/failed/elapsed summary after it:

```
[1/3] 12.579000_25_08_26_06_09_10.iqw: wav iq_i16, 24 kHz, 30m, peak -51.9, bin -99.8 dBFS, try -d 40 → *.png, 212.8 KiB, 463ms
```

What a listing has no room for is what the line drops: the bin's frequency,
the floor, the whole transform -- fft size, window, hop, frames and range --
and the render's pixel size, all of which a batch sets identically for every
file. `*.png` is the render, named by what it adds to the input rather than by
spelling the whole path out a second time. `-v` brings the sections back for
every file, `-q` says nothing but still names any file that failed, and
`--json` prints one report object per file it rendered -- a stream that `jq`
and `serde_json::StreamDeserializer` both read as it arrives. A file that
failed has no report, so it appears on stderr and not in that stream.

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
reaches the edge of the strip instead of averaging away. Its linear scale uses
full scale when `-d` is omitted and the loudest sample when a numeric or
automatic range is selected. A complex signal is one track, spanning whichever
of I and Q reached further in that column.

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

The foot of the plot names the vertical scale as `dBFS, ENBW 17.578 Hz`.
Levels are referenced to full scale: a full-scale tone sitting on a bin centre
reads 0 dBFS at any transform size, and one falling between bins reads under
it by the window's scalloping loss -- up to 1.42 dB half a bin off centre
under Hann. The transform divides by the window's coherent gain and not by any
bandwidth, so a carrier's level does not drop by 3 dB every time the bin
halves, the way a noise floor does.

Noise is the part that moves with the bandwidth, so the bandwidth is named
beside it. `ENBW` is the window's equivalent noise bandwidth, which is what a
bin answers to noise across: 1.5 times the raw `Fs / N` spacing under Hann,
1.0 under a rectangular window, 2.0 under Blackman-Harris. Subtracting
`10 log10(ENBW)` from a level turns it into a density per hertz -- but only in
the spectrum panel, which averages. A spectrogram pixel is the loudest bin of
the rows folded into it and, under `--reduce max`, the loudest frame of its
column, so it estimates a maximum rather than a mean and no single bandwidth
converts it.

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

A full-scale tone on a bin centre reads 0 dBFS in either domain, so the colour
scale means the same thing for a real recording and a complex one.

## argand

```sh
argand                                              # an empty window
argand capture.iqw
argand dump.bin --raw iq_i16@24k --center 12.579M
```

The seven options that say how to read a capture -- `--raw`, `--sample-type`,
`--rate`, `--center`, `--offset`, `--normalize` and `--gain` -- are `aspec`'s
own, spelled the same way. Everything that decides how the picture looks comes from
`argand.toml` instead: the theme, the colour scheme, the dynamic range, the
transform size and window. It is read from beside
the binary first and from the platform configuration directory second, and a
missing or malformed one costs a log line rather than the application.

The **Aggregation** control in the analysis settings window switches between **Peak (MAX)** and
**Mean power** while a file is open. Peak preserves the strongest value in each
pixel's time/frequency region. Mean power averages squared spectral amplitudes
across the bins assigned to each row and the frames assigned to each column,
then converts the result to dB. It shows average squared amplitude, not the
integrated power of the displayed frequency band. Brief events become weaker in
proportion to their duration; combining more frequency bins can also dilute a narrow tone.
The waveform retains its min/max envelope under either mode.

Switching starts a cancellable background analysis and retains the current
picture until the replacement preview arrives. The choice applies to subsequent
files and survives restarts. Set the initial choice with the top-level configuration key
`aggregation = "max"` (default) or `aggregation = "mean-power"`. Live choices do not
rewrite `argand.toml`; effective settings are saved in `session.toml`.

The waveform is a full-capture minimap. Its content and amplitude scale remain
fixed during spectrogram navigation and analysis-setting changes. The visible
interval stays bright; the waveform outside it is darkened, without a border or
background fill. At full capture the whole waveform stays bright. Sub-pixel
intervals retain a one-device-pixel minimum width.
Click outside the interval to pan one time-ruler division toward the pointer;
Ctrl+click pans five divisions, matching the arrow shortcuts. Single and double
clicks inside it do nothing; drag it to pan across the capture. An outside
double-click steps on its first press and centres on its second only if the
pointer is still outside the updated interval. An open-hand cursor marks the
viewport and rulers; dragging uses a closed hand. Pointer time over the minimap refers to the full recording.

Choose **View → Time scale format**, or right-click the time ruler, to show
hours, minutes and seconds (hms, default), elapsed seconds, or zero-based sample
numbers. The unit (`hms`, `s` or `#`) appears once at the right of the ruler. Seconds and sample numbers
refer to the start of the capture; one complex sample is one I/Q pair. Labels keep
appropriate precision as you zoom, and the Alt time badge follows the selected
format. The choice survives restarts. Switching formats leaves the visible range
unchanged and does not recalculate the spectrum. Arrow keys follow the divisions
of the selected ruler; zoom and position still reset whenever a file is opened.

GUI numbers use the system numeric locale by default, including grouping, decimal marks,
axis labels, cursor badges, hints and analysis settings. Numeric fields accept
the same locale. Linux follows `LC_ALL`, then `LC_NUMERIC`, then `LANG`; Windows
and macOS use the system regional locale. Number conventions come from CLDR;
configuration, session serialization and CLI output retain their existing formats.

To override the numeric locale, set the top-level `number_format` option in
`argand.toml` (before any `[section]`), then restart the application:

```toml
number_format = "ru-RU"
```

The default is `"system"`, also used when the option is absent. Explicit values
are BCP 47 locale tags such as `"ru-RU"`, `"en-US"` or `"de-DE"`; `"C"` and
`"POSIX"` select a decimal point without digit grouping. Use tags such as `ru-RU`,
not environment spellings such as `ru_RU.UTF-8`. Invalid tags produce a log warning
and fall back to `system`, preserving the other configuration settings. This
option is only available in the configuration file and applies to both numeric
display and input; it does not change the interface language.

Scroll over the spectrogram to pan in time, or hold Ctrl to zoom about the pointer.
Left-drag also pans. The time ruler uses the same gestures: wheel pans horizontally, Ctrl+wheel zooms. Shift+wheel is reserved
for frequency panning (#80) and currently leaves the time view unchanged.
The crosshair appears only over the spectrogram. The View menu exposes the keyboard commands:

| Key | Action |
| --- | --- |
| Ctrl+`+` or Ctrl+`=` / Ctrl+`-` | Zoom in / out about the view centre |
| Left / Right | Pan by one time-ruler division |
| Ctrl+Left / Ctrl+Right | Pan by five time-ruler divisions |
| Home / End | Move to the capture's beginning / end |
| Ctrl+`0` | Fit the entire capture |

The minimum span is one FFT, with a screen-resolution representability floor
for extreme sample indices. This floor is rechecked when the plot width changes. Axes and held pictures move immediately; a new
completed image replaces the placeholder at the same physical coordinates. A retained
wider picture fills known time on zoom-out while the replacement is calculated.
Rapid navigation replaces pending requests instead of queuing transforms.
Every file opening starts at full capture, including reopening a recent file within
the same run. Legacy saved zoom and position are ignored. Panning retains the ruler
spacing and clock format; arrow steps round to the nearest sample without accumulating
fractional-sample drift. Capture boundaries limit the last step.

The status bar shows pointer time from the capture start, physical frequency in Hz
(including centre frequency), and the displayed grid cell's level in dBFS. Above
the waveform and time ruler it shows time only. Uncovered placeholder areas have no level.
Hold **Alt** over the spectrogram to project the cursor onto the time and frequency
rulers, with rounded coordinate badges. White/black/white guide lines remain
visible across palettes. Release Alt to hide the guides.
File min/max values stay file-wide; they become available after the full-capture
waveform scan, even if navigation interrupts the initial spectral analysis.

Resizing the window or dragging the panel separator redraws from retained analysis
without rereading the file or restarting FFT refinement. The GUI keeps up to 4096
time cells and 2048 frequency cells (native FFT bins whenever they fit), plus a
separate min/max envelope of up to 65536 sample cells. Every FFT frame still
contributes; the cache does not use sparse sampling for the final result.

The GUI reduces these cached values before applying colours. Peak includes every
cell overlapping a pixel; Mean power weights linear powers by overlap and frame
counts. An aligned boundary is exact apart from floating-point rounding; inside
a cache cell, Peak can widen a feature by one cell at each edge and Mean power
assumes uniform power. Waveform extrema conservatively include overlapping cells.
A window wider/taller than the cache cannot reveal extra detail within a cell.
The colour scale and reported analysis time remain stable during resizing.

The retained spectral accumulator uses at most 32 MiB for Peak or 64 MiB for Mean
power, independent of recording duration. Render buffers, waveform data, FFT
plans/scratch and the mapped file are additional. See the
[resize measurements](docs/performance/74-resize-cache.md) for costs and limits.

`aspec --reduce mean-power` uses the same power definition and reduces directly
to the requested pixel boundaries without the GUI overview approximation. Its
existing `--reduce mean` retains its original meaning: average frame levels in dB after a frequency-bin
maximum. All modes use the same FFT frame lattice; choosing Mean power does not
reduce the number of transforms. A repeated capture viewed in full still cannot
show the short source's timing detail at the same window width.

The compute budget is configurable independently of the UI:

```toml
[analysis]
workers = 0           # Automatic: at most 8 available logical CPUs
batch_frames = 1024   # 1..4096; additionally bounded by memory and FFT work
affinity = "none"    # "efficiency" optionally confines compute to detected E cores
```

Valid explicit worker counts (1..128) are clamped to the process CPU budget.
Out-of-range worker or batch values are logged and reset to their defaults.
Workers share a dynamic Rayon queue; FFT plans do not start nested threads.
Efficiency affinity currently supports Linux Intel hybrid x86-64 CPUs via CPUID,
within the inherited CPU mask. Other CPUs/platforms fall back to ordinary scheduling
with a warning. Only the compute pool is pinned; the UI retains its mask.
This mode is optional, not an automatic speed optimization. See
[the scheduling measurements](docs/performance/67-fft-scheduling.md) for results,
limitations and proposed automatic planning.

The title is centred between the window edges. Normal client-decorated windows
have subtly rounded corners. The waveform starts at 3 rem
(normally 48 logical pixels) high, with a subtle separator ending at the waveform’s right edge. Ruler borders match their tick marks; the frequency border joins the separator. Drag the boundary
to change the panel proportion; the application remembers it between runs.
Real and I/Q captures use one merged min/max trace, exactly as `aspec` does,
without grid lines, a zero-axis line, a legend or an amplitude caption. Dragging stretches the existing view and
rebins the cached view as the separator moves without restarting analysis. The older `panels.waveform_fraction`
setting is still accepted so existing files load, but no longer sizes this strip.
The spectral preview keeps its colour scale while refinement runs,
then resolves them once at completion. Replacing an analysis cancels its remaining
work. GUI automatic normalization samples at most 64 MiB; a sparse scan can miss
an isolated peak. `aspec` retains its existing normalization scan policy.
The status bar combines file metadata into one compact group, for example
`wav · real i16 · 7.2 kHz · 47m12.4s`. Its hint includes the exact duration,
sample count (I/Q pairs for complex signals), file size in bytes, centre frequency,
and decoded original sample minima/maxima, separately for I and Q. Extrema become
available after the complete waveform pass; decoding precision limits their precision.

FFT sizes are powers of two from 2 to 1,048,576; overlap is rounded to a whole-sample hop.
Hover over the analysis group, for example `2048 · hann · 110 dB`, for its details.
Click it, choose **Edit settings…** in the hint, or press Ctrl+, (Cmd+, on macOS)
to open the analysis settings window. Standard dropdowns select FFT size, window,
aggregation and colour scheme; numeric fields edit overlap and fixed dynamic range.
Tab and Shift+Tab move between controls, arrows navigate lists or step numbers,
Enter confirms a choice or numeric edit, and Escape closes a list before closing
the settings window and cancels its changes. Numeric edits also commit when focus leaves the field.
Changes preview immediately; **OK** keeps them, while **Cancel**, Escape or closing
the window restores the settings present when it opened. **Reset to defaults** previews
the defaults from `argand.toml`, or built-in defaults when no configuration is present.

Range modes are absolute full scale (0 to -110 dBFS), a fixed span below the measured
peak, and automatic. The effective range remains visible as a readout outside the fixed mode.
The status text is muted and brightens on hover. Only a nonzero signal whose spectral
peak falls in the lower half of the absolute scale produces a yellow range warning.
A narrower recommendation by itself is not a warning; silence and peak-relative modes
are excluded. The hover hint and settings window offer the measured recommended
range used by `aspec`, and apply it with one action or Ctrl+R (Cmd+R on macOS).
A yellow ⚠ accompanies the highlighted range. Opening the editor hides the hint.

Colour and range changes reuse cached values without a new FFT, including during
refinement. Transform changes cancel obsolete work and retain the previous picture
until a preview arrives. Style edits during that initial replacement interval apply
to the incoming preview; the retained picture keeps its own transform and style
until then. Invalid choices show an explanation.
FFT, window, overlap, aggregation and colour choices persist after OK in session
version 5; configuration defaults remain untouched. Range and its mode belong to
the current file: opening a file or restarting restores the configured range default.
Older saved range values are ignored.
The bar describes the displayed analysis while a replacement is pending.
The `ready in` hint explains the latest analysis time, including sample reading
and excluding file opening and display.

Files also open from the File menu, by dropping a capture on the window, and
from the list of files opened before. When launched without a file argument, the
start page lists only recent paths verified as existing files. All checks run in
independent background workers, so an unavailable network location cannot block
the window or other entries. Alt+1 through Alt+9 open the first nine displayed
links; other entries remain clickable. The compact rows highlight only their
text-sized area on hover, without underlines. Below the list, `or` aligns with
the heading and `Open a signal file…` aligns with the filenames. This action opens
the platform chooser and stays centered when no recent file is available. Ctrl+O (Cmd+O on macOS) opens the same chooser anywhere in the
window. Shortcut hints show the registered binding in a distinct colour at the
right edge. Hints fit their contents and wrap long paths within the window; the File menu also shows the Open accelerator.
Unavailable paths remain in history for later launches. The list keeps the options each file was
opened with, so a headerless capture opened once as `iq_i16@24k` does not need
those flags a second time. It lives in `session.toml` in the platform state
directory, along with the window's size and state. That file is written whole
by whichever instance writes it last, so what it remembers is what the last
instance to write it remembered.

On Wayland, dropping from a native desktop file manager is supported. Drops
from Double Commander through XWayland currently do not open a file
([#46](https://github.com/o-kos/argand/issues/46)); use the File menu instead. The status bar shows analysis progress and, on completion,
the time spent analysing the capture.

## Layout

```
crates/core   domain types, render view-models. No IO, no DSP, no toolkit.
crates/io     container detection and readers. Implements core's SampleSource.
crates/dsp    windows, STFT, averaged spectrum. Consumes SampleSource.
crates/cli    the aspec binary: arguments, report, PNG composition.
crates/app    the argand binary: window, document, analysis thread, axes.
```

Dependencies run one way: `core <- io`, `core <- dsp`, and all three into each
front end. There is no edge between `io` and `dsp` -- the transform does not
know what a file is, and the readers do not know what a transform is -- and no
GPUI type appears below `crates/app`. That seam is what lets either front end
change without touching anything under it, and what would let the toolkit be
replaced.

## Tests

```sh
cargo test
```

Fixtures for all ten sample types are generated at test time rather than
committed. Tests that use real captures look for them in `tests/signals/` and,
for the wider format matrix, in `../sgvr/cli/tests` or wherever
`ARGAND_EXTRA_FIXTURES` points; they report and skip when those are absent.

The GUI minimap reopens its own cancellable reader using the resolved sample count
and normalization divisor, without another count, normalization scan or FFT. It publishes a
bounded sparse full-width preview, then scans every sample in blocks of at most
256 KiB. The complete envelope retains at most 65536 min/max cells per channel
(1 MiB for I/Q), with a separate bounded read buffer. Its scale is resolved from
the complete capture peak. Initial completion can refine the preview and its
scale once; later zoom, pan, palette and FFT changes do not rebuild it. Resizing
conservatively rebins cached extrema. Closing or replacing the file cancels the
reader without waiting on the UI thread.
