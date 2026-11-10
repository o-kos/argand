# Signal processing

Band extraction and suppression, spectral inversion and gain. Read this before changing their DSP or encoding semantics. Selection interaction is described in [navigation and selection](navigation-selection.md).

## Band and rectangle selection (#202)

A band is selected over the whole capture by a left drag on the frequency ruler, and a
rectangle by a Ctrl+left drag on the spectrum; `Selecting` keeps a time anchor, a
frequency anchor or both, and an axis the gesture selects without extent selects nothing.
The shell keeps the band beside its time span (`Shell::band` next to `Shell::selection`),
clamped to the capture's band (`Shell::capture_band`: the reference ± Fs/2 for complex, the
reference to Fs/2 above it for real); every gesture replaces both, a file opening and an
edit clear the band, except a suppression, which keeps both (#212). Edit commands act on `Shell::time_selection` only, which is none while
a band is selected. The spectrum tints the band across the visible time, or the rectangle,
through `waveform::selected_rect`; the minimap tints only a time span. Selection badges show the band's edges and width in the frequency ruler's unit; the status bar shows no selection.

Save selection as… with a band computes the file (`processing/extraction.rs`, run on the save thread
by `saving::start_extraction`): each file the current version reads is reopened with
the normalization the window resolved (`argand_io::reopen_stamped`) and must keep its stamp, also after reading, and the span plus the filter's
margin is read through an `EditedSource`. `argand_dsp::extract::ExtractPlan` takes
D = ⌊Fs / (1.25·B)⌋ and a Kaiser windowed-sinc low-pass of 80 dB. Its transition is the
narrowest of three limits:
- Fs/D − B, or B/4 at D = 1, so whatever aliases misses the band;
- a quarter of Fs − B, so the stopband stays below the Nyquist rate;
- for a real capture, twice the band's distance to 0 Hz or Fs/2, so its mirror is stopped.

None is narrower than Fs/131072, which bounds the filter to about 660 k taps. A complex band
within Fs/32768 of the whole capture is kept unfiltered, and a band narrower than Fs/32768 is
refused. A real band starting at 0 Hz or ending at Fs/2 meets its own mirror there, so the cutoff
sits on that edge, where the doubled half gain keeps DC or the Nyquist component whole. The window only checks the band (`ExtractPlan::check`), and the save thread builds
the filter.

`Extractor` mixes the band centre to 0 Hz with an `f64` oscillator. It filters by
overlap-save FFT convolution, with an FFT of 4× the filter length rounded up to a power of
two, clamped to 1024..2^22, so the cost per input sample grows with the logarithm of the
filter length. It keeps every D-th output and compensates the filter delay, so output n is
input n·D of the span and there are ⌈len/D⌉ outputs. Real input has the analytic signal's
gain. The capture's own samples beyond the span feed the filter, zeros
elsewhere. For export, `ExtractPlan::for_export` produces an intermediate rate 2·Fs/D. Non-identity
filters double the inverse FFT size before integer decimation; the base FFT is at most
2^22 and the inverse can reach 2^23. Whole complex bands instead use a native-rate
half-sample-delay Kaiser convolution for odd outputs and copy native samples exactly for
even outputs, then mix at 2·Fs. Their FFT stays at most 2^22. The 80 dB interpolation
transition at native ±Fs/2 is Fs/131072 wide: the passband ends Fs/262144 before each
native edge. Inside that guard, a tone shares its amplitude with an image beyond the
opposite native edge; an exact native-Nyquist component splits equally between ±Fs/2.
Whole bands are not refused. `argand_dsp::resample` reconstructs the intermediate stream
with a bounded Kaiser sinc phase bank at `ceil(1.25·B/1000)·1000` Hz for I/Q or
`ceil(2.5·B/1000)·1000` Hz for real, including upsampling. It emits at most 4096 samples
per call for cancellation and retains only the filter neighbourhood. Both filter margins
are read from the capture. The output preserves the source domain: I/Q stays centred at
0 Hz, while real output mixes the analytic band up by B/2 and takes its real component,
placing the selected lower edge at 0 Hz. Reference metadata is zero. `write::FloatWave`
writes real or I/Q `f32` WAVE on the unit scale (RF64 past 4 GB), under the target rules
of `save`, and a cancel during its final sync leaves the target alone. The name is `<stem>[_<start>-<end>s]_<low>-<high><unit>.wav`, in the largest unit that keeps the edges apart in six decimals.

## Band suppression (#212)

Edit → Suppress band, and the same row in the spectrogram's context menu, takes the selected
band out of the capture and fills it with noise as loud as the frequencies beside it: over
the whole capture for a band, over its time span for a rectangle. Its key is Ctrl+Shift+M in
the `Plot` context (Cmd+Shift+M on macOS). `argand_dsp::suppress` computes
`x − (1 − cos(πw/2))·b + sin(πw/2)·σ·v`, where `A` is a band-pass whose transitions lie
outside the band (a quarter of its width, at least Fs/131072, at most the room beside it, a
fifth of it still passed so the band's own edges stay suppressed), `b` the input through
`1 − cos(πA/2)`, `v` unit Gaussian noise from a fixed seed through `sin(πA/2)`, `w` a
raised-cosine taper and `σ` the neighbours' noise amplitude. Cosine and sine keep a noise
background's power through the transitions and the fade, and both are smooth in `A`, so the
fill keeps `A`'s stopband (no square root near zero lifting it) and the input past the
transitions is left untouched far below `f32` precision. The two filters are derived from
`A` by frequency sampling and truncated to twice its length, because their responses reach
further in time than `A`'s. `NoiseFloor` measures `σ` first: the region is cut into at most 256
blocks, up to eight Hann-windowed segments of at most 4096 samples are read in each, and a
block's level is the median bin power over the neighbouring bands (as wide as the band, at
least 32 bins, starting 8 bins past its transitions so a tone at its edge does not leak into
them, and inside the capture), corrected from median to mean.
`NoiseFill` interpolates the levels between block centres, so the fill follows a background
that changes over time, and a band with no neighbours (the whole capture) is left empty. For a complex
capture the band-pass is the low-pass of half the band modulated to its centre; for a real
one it is `LP(high) − LP(low)`, the first a unit impulse at Fs/2 and the second absent at
0 Hz, so no mirror counts twice. A complex band within Fs/131072 of the whole capture
suppresses everything. A rectangle's span widens by the filter's half length, at least 256
samples (`SHORTEST_TAPER`), on each side, inside the capture, and the taper fades over that
widening, so the whole selected span is suppressed without a click. Convolution is overlap-save as in `extract`, with the delay
compensated, so output `n` is input `n`.

`processing/suppression.rs` computes it on a thread of its own (`saving::spawn`, the save thread made
generic) with progress, a cancel and the status-bar item `Suppressing the band…`
(`shell/suppression_ui.rs`). Edits and saves wait for it, and opening another file cancels it. The
changed span is written as unit-scale `f32` through `FloatWave` into a scratch file.
`scratch.rs` keeps those in `<cache>/argand/scratch/<pid>/`, the user's own cache folder and
mode 0700 on Unix, never the shared temporary folder, under a lock on `<pid>.lock`, and
every instance removes the folders of locks nobody holds when it starts, under `sweep.lock`
so no instance removes a lock file another is taking. `Scratch` removes its file when the
last `Arc` goes, held by `editing::Source::computed` in the document, its history and the
clipboard, and by a save of it (`SaveOf::_scratch`) until that save ends. The edit is `Editing::replace_computed`, applied only to the
document and version the computation started from: the length is kept and what was selected,
time span and band, stays selected. The edit also sets the version before it to that
selection (`History::set_state`), and every version keeps the band beside its time span
(`History<Selection>`), so neither the edit, its undo nor its redo changes the selection and
only the picture changes, once its analysis lands. `Editing::repicture` keeps the shown picture over
samples computed in place, or brought back by undo, until the new analysis lands, so a
suppression over the whole capture never blanks the spectrogram.

A save encodes computed samples into the document's retained output `Format` (#214).
`Editing::save_request` lists only the sample sources its segments read, remapping their IDs;
a fully computed capture no longer needs the original file solely to describe its format.
The encoding is `value * factor + offset` from the saving
document's own `original_sample_units` (`Editing::describe_units`), so computed samples pasted
into another capture are stored on its scale. Integers are rounded and clamped without a
warning (8-bit offset binary, and 16, 24 and 32 bits whose value fills its container, so an
extensible header with fewer valid bits is refused), floats are written as `f32` or `f64`, and FLAC gets integers clamped to its
depth. Computed samples of another kind than the capture are refused. Untouched samples are
still copied byte for byte. Actual sample sources must still be their stamped selves.
A computed source pastes into any
capture of the same rate and kind, and a paste checks only the sources its clip reads.

## Inverting and normalizing (#188, #190)

Edit → Invert spectrum (Ctrl+Shift+I in the `Plot` context, Cmd+Shift+I on macOS, a checked
row) and Edit → Normalize level (Ctrl+Shift+N, Cmd+Shift+N) act on the whole capture, never a
selection, and change no file until it is saved. Both are disabled while anything is selected,
a time span, a band or a rectangle, because they would read as acting on that alone, as the
owner decided. `argand_core::Adjustment { invert, gain }` is what a capture applies
to every sample as it is read: inverting conjugates a complex sample and multiplies a real one
by `(−1)^n`, and the gain scales I and Q alike (`Adjustment::factor`). It is named apart from
the FFT transform settings on purpose. `n` is the sample's index in its own source, not its
position in the capture, so a sample reads the same wherever a cut, paste or delete moves it;
counted in the capture, every odd shift would negate what follows it. `argand_edit::Capture`
carries one, kept by its edits and part of its equality, and `EditedSource` applies it to each
piece as it reads it, so pieces always hold their sources' own samples. Each command is a version in the history (`Editing::adjust`, with the
selection kept as suppression keeps it), marks the capture unsaved and leaves the pieces, so
`repicture` holds the picture until the new analysis lands. Suppression computes on the
adjusted capture, as shown, and stores its scratch samples through `Adjustment::inverse` at
their own indices, so the adjustment reads them back as computed. A real capture's scratch file
starts with one silent sample when that gives its samples the parity of the samples they
replace in the span's first piece (`Suppressed::offset`), so undoing the inversion leaves them
in phase with their neighbours. Where the span runs on into a piece of the other alignment, made
by edits under the inversion, the computed samples over that piece turn over once the
inversion is undone, as that piece itself turns over against the first. Extraction reads the adjusted capture. The minimap is the composed envelope adjusted (`minimap::adjusted`): scaled, the Q
channel's extremes negated and swapped, and a real inversion's cells taken as their symmetric
hull, which errs outward. The status bar shows `Inverted`, the gain in dB or both after the
analysis item (`adjustment_item`).

Normalize level asks nothing: the normal form is one, the peak at `levelling::TARGET`, −0.1
dBFS, just below full scale, as the owner chose. `processing/levelling.rs` measures the peak of the unadjusted
capture on the save thread's shape (`saving::spawn`, `Measuring the level…` with a cancel in
the status bar; edits and saves wait, opening a file cancels it). The peak is the largest
`|I + jQ|`, or `|x|` for a real capture, taken piece by piece in stored units, each source
through its own `original_sample_units` (a computed one through its encoding), because pasted
files may be read on other scales while the gain acts on stored values. dBFS is relative to the
stored format's full scale (`Storage::full_scale`, `Editing::full_scale`), not to the read unit
scale that the normalization and gain hints move. The gain replaces any before it,
applies only to the document and version measured, adds no version within 0.001 dB of the one
in force, and a silent capture is refused.

A save with any adjustment but the identity encodes every sample again
(`SaveRequest::adjustment`, its parity from each segment's own source index): linear stored values are decoded, scaled around the format's zero (128 for 8-bit
offset binary) and pushed through `Linear::push`, computed samples are encoded first, and FLAC
integers are adjusted, all rounded and clamped without a warning, so inverting −32768 gives
32767. A linear layout `Linear::of` refuses is refused. After Save over or a whole Save as the
document reads the saved file, which holds the adjusted samples, with the identity. A
`Clipboard` remembers the adjustment it was copied under and pastes only into a capture under
the same one (`PasteError::Adjustment`). Copies of a file saved adjusted cannot follow it onto
the saved file, nor can copies made under a real inversion that the save moves to the other
parity.

## Capture conversion (#189)

`argand_dsp::convert::ConversionPlan` intersects native input-local bounds
with output bounds shifted by the reference difference, preserving exact native
endpoints at large physical references. Reconstruction width is bounded by both
declared domain bandwidths; the final oscillator subtracts that same reference delta.
Physical edges are for metadata/reporting. The plan validates the output count; `ExtractPlan::for_conversion` uses bounded overlap-save filtering,
integer decimation and 2x interpolation with transitions inside the intersection (1% of
its width per edge), except native real DC/Nyquist half-gain endpoints. The fractional
reconstructor removes intermediate sampling images; conversion's extractor alone owns
output-rate anti-aliasing. It must not apply a second target-Nyquist low-pass that
would discard native real endpoints. The final oscillator preserves the requested
reference. Exactly coincident IQ Nyquist endpoints cannot represent independent tones.
Identity domain/rate/reference skips filtering; eligible unchanged WAVE
captures copy bytes through the existing writer, preserving i32 low bits.

Preserve converts each source's read values back to nominal full-scale units, computed
sources through their document Encoding, and applies the Capture adjustment once.
The File information panel offers no normalization; it preserves adjustments already
applied. The internal Normalize mode measures the transformed output peak first, then
repeats the bounded stream with the same -0.1 dBFS/common IQ-magnitude gain as `levelling`.

Conversion Reference is editable only when the target real/IQ domain differs from the source. Across domains it is the carrier C locating I/Q zero in the real array, not the physical metadata origin. Domain-changing conversion writes output center_freq=0, so real results span 0..Fs/2 and IQ results span -Fs/2..Fs/2 independently of the source physical reference. The application passes the exact signed carrier separately into the DSP input-local reference-delta constructor; it does not recover the shift from output metadata. Band-loss warnings report retained input-array frequencies. The existing metadata-based DSP constructor remains available. Filters, oscillator and band-loss decisions use local bounds rather than subtracting rounded absolute origins. Returning to the source domain restores its Fs and physical reference; numeric-encoding-only changes preserve active drafts.

See [File information UI](ui.md#file-information-and-conversion-189) and
[conversion publication and saving](editing-io.md#conversion-publication-and-saving-189).
