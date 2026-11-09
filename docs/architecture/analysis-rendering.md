# Analysis and rendering

Analysis scheduling, bounded retention and display resources. Read this before changing worker delivery, caches, minimap scans or texture lifetime.

## Rendering boundary

On Linux, `shell::run` creates the GPUI Kit platform once, seeds its cursor with
Arrow before opening any window, and passes that same platform to
`Application::with_platform`. Wayland pointer Enter can then apply the cached
cursor with its valid entry serial even without pointer motion (#152); normal
frame hit-testing still selects resize cursors only over their edge regions.

GPUI types must never leak into `argand-core`, `argand-dsp`, `argand-io`, or `argand-edit`. Those crates return textures, envelopes, primitive lists, and other toolkit-neutral data. Only `argand-app` converts them into GPUI images, quads, and paths.

This boundary keeps the toolkit replaceable. If GPUI proves too restrictive for a custom GPU canvas, the fallback is egui/eframe with wgpu without rewriting the core crates.

## Performance strategy

- Memory-map input files and parse them lazily instead of loading complete captures into RAM. `SampleSource` exposes optional access-pattern and bounded prefetch hints; they never change its cursor or decoded values. The mmap reader uses Unix advice to overlap sparse preview reads.
- Build a multi-level min/max peak pyramid for waveform level-of-detail selection.
- The GUI retains a toolkit-neutral `Overview` in its document worker: at most 4096 time cells by 2048 frequency cells, keeping native FFT bins when they fit, and up to 65536 sample min/max cells for the waveform. Every STFT frame is accumulated. `Overview::render` rebins values before shading: conservative overlapping maxima or overlap- and frame-count-weighted linear powers. Cache boundaries are the declared resolution; sub-cell structure cannot be recovered. The spectral accumulator is bounded to 32 MiB (Peak) or 64 MiB (Mean power), excluding display buffers, waveform, PSD/FFT storage and input mapping. Ordinary DSP/CLI rendering still reduces directly to requested pixels; legacy dB Mean is not an overview reducer.
- Display dimensions use a coalescing mailbox and do not change the analysis generation while the sample range stays unchanged, including during progressive refinement. At extreme sample counts, growing the plot width can widen the range to preserve representable time coordinates (#30). File/range/FFT/reducer changes invalidate analysis. Colour and dynamic-range changes advance only the display revision; explicit style refreshes bypass the periodic snapshot interval between bounded batches. The worker renders the newest requested size from the same overview; completed redraws preserve analysis duration and resolved colour range. Only image-sized spectral snapshots cross to the UI; the independent minimap retains its bounded full-capture envelope.
- Use a piece table so cuts and pastes remain `O(1)` for multi-hour files.
- Keep expensive work off the UI thread by using rayon or dedicated worker threads.

## Analysis and display delivery

`crates/app` opens a signal file, analyses it, and shows the spectrogram with time and frequency axes and a linear waveform above it. The spectrum publishes a sparse full-width preview and left-to-right refinement snapshots; the independent waveform minimap spans the full capture; time navigation is implemented by #30; time selection by #178.

The application's logic is deliberately free of GPUI wherever it can be, because CI has no GPU and cannot run the application. What a file is doing (`document.rs`), the thread that reads and transforms it (`analysis.rs`), what the settings ask a transform for (`config.rs`) and what is remembered between runs (`session.rs`) name no toolkit type at all. Where the axes put their marks does name one -- `axes::Frame::measure` takes a `gpui_kit::Size<Pixels>` -- but decides everything through `argand_core::axis` and a `LabelMeasure`, so it is tested against the fixture font with no window in sight. What does need a running window is `shell.rs`, frame setup in `chrome.rs` and the drawing half of `axes.rs`: a font to measure and paint labels with comes from one. `spectrogram.rs` needs neither -- it turns a transform's RGBA into a `RenderImage` and nothing else -- so it is tested like the rest.

File opening starts after an intervening GUI frame; ruler boundaries appear before metadata, with physical labels added once the file is described. The per-document worker uses a configurable Rayon pool (by default at most eight threads), bounds automatic normalization scans to 64 MiB, and bounds its reply queue to two snapshots. Analysis changes and replies carry generations; size-only mailbox updates retain the generation but advance a separate display revision, so stale sizes are rejected before sending and on receipt. The GUI rejects obsolete analysis deliveries and the DSP cancels superseded passes between batches. The first coarse preview uses at most 128 evenly spread frames, followed by the remaining preview frames (at most 1024 overall in the GUI, independent of cache width) and sequential refinement. Preview frames belong to the final frame lattice and are accumulated once. Preview reads/transforms are batched to at most 4 MiB of sample values (or one FFT frame when larger), with cancellation between batches. GPU uploads coalesce at shell render time, so paused frames retain one pending analysis rather than accumulating retirement callbacks. Whole spectral snapshots are published at most every 50 ms, with the preview colour scale held until completion. Incremental rendering updates only output columns affected by changed cache cells or their empty followers; size changes rebin the full view. Measured sample peaks remain independent of that frozen display scale. Refinement is shown through the changing picture and status-bar progress; do not overlay a moving indicator on either panel. `SampleSource::access_pattern` is an optional I/O hint: mmap preview disables sequential readahead and page retirement, then restores them for refinement. CLI analysis and unbudgeted normalization retain their existing behaviour.

A replaced spectrogram texture is retired after an intervening redraw, using two `on_next_frame` callbacks. The backend can destroy a released texture immediately while the preceding GPU frame still samples it -- observed on Blade, and assumed until wgpu retirement is verified; the intervening draw waits for that frame. Never replace this with an immediate `drop_image`, and always name the current window when releasing the image.

Progressive Max reduction retains linear row amplitudes until time-column reduction; Mean retains its existing dB arithmetic. The refinement cache refreshes changed columns and their empty followers, invalidates colours when shading changes, and still publishes owned whole snapshots. FFT scratch belongs to each partial accumulator and is reused across its frames.

## FFT execution policy (#67)

`execution.rs` owns the per-document Rayon budget: configurable workers (zero means
at most eight available logical CPUs), bounded batches (default 1024 frames), and
optional efficiency affinity. Work stealing distributes frame groups dynamically;
realfft/rustfft plans remain single-threaded. Linux Intel hybrid discovery uses a
short-lived probe thread and CPUID core classes inside the inherited CPU mask.
Only compute workers are pinned; UI affinity and system scheduler settings are
untouched. Unsupported discovery falls back to ordinary scheduling. Max reduction
combines consecutive frames for the same image column within each partial;
independent frame counts preserve PSD normalization. Mean retains individual rows.
Progressive batches bound decoded data to 4 MiB and estimated FFT work to 1024
2048-point transforms, allowing one oversized FFT. Buffers are allocated at that
bounded capacity, not allocated large and then truncated.
`argand::ui_latency=trace` enables UI timer, delivery-age and CPU upload-preparation
probes; these are not input-to-display measurements.

## Full-capture waveform minimap (#79)

`minimap.rs` owns a toolkit-neutral, separately cancellable document task. After
metadata arrives it opens an independent cursor through `argand_io::reopen`,
reusing the already resolved sample count and normalization divisor. FLAC seeks
recreate the parser with those known values because the decoder can otherwise
retain old packets at aligned frame offsets. Reopening
never recounts or rescans levels and does not enter the global Rayon pool. A preview reads at most 128 evenly spaced blocks of
256 samples, then a sequential no-FFT scan visits every sample in blocks of at
most 65536 scalar values (256 KiB). The retained envelope has at most 65536 cells
per channel (1 MiB for I/Q). A bounded two-item channel carries only the preview
and final envelope; dropping its receiver cancels between reads without a UI join.
Initial completion replaces the preview and resolves the full-capture peak scale.
Range, transform, reducer and palette changes neither cancel nor rebuild this task.
The shell no longer requests or retains per-range spectral waveforms. File hints
accept extrema from the complete minimap; a read failure clears its picture and
appears in the file hint, without discarding the spectrum.
`waveform.rs` caches conservatively rebinned pixel spans by device width and height.
Minimap ink derives from the displayed spectral colormap, drawn in its dark-theme form on that colormap's darkest colour in both interface themes,
falling back to the requested colormap before the first spectral picture arrives.
Navigation changes only waveform ink, active inside the requested time viewport
and muted outside, with no border or fill. Full capture has no dimmed portion. Sample
bounds retain a one-device-pixel minimum visible width. The minimap has no grid.
An outside click invokes the same one-division pan as Left/Right; Ctrl+click invokes
five divisions. Inside clicks, including double-clicks, only arm dragging. An outside double-click
centres at the pointer. Dragging uses full-capture coordinates. When the time view
can pan, the viewport and time ruler use an open-hand cursor; active drags use a
closed hand. The frequency ruler uses an open hand when its viewport can pan, and a closed hand during a drag. Wheel gestures belong to the spectrum and both rulers; the minimap does not take wheel navigation. Pointer
time over the minimap uses full-capture coordinates.
