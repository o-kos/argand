//! Preview and sequential refinement using the ordinary analysis accumulators.

use super::*;
use argand_core::AccessPattern;
use std::time::{Duration, Instant};

const FIRST_PREVIEW_FRAMES: usize = 128;

const SNAPSHOT_INTERVAL: Duration = Duration::from_millis(50);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flow {
    Continue,
    Stop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Coverage {
    pub refined_columns: usize,
    pub width: usize,
}

struct DisplayScale {
    shading: Shading,
    dynamic_range: DynamicRangeResult,
}

struct Refinement {
    request: AnalysisRequest,
    meta: SignalMeta,
    plan: Plan,
    columns: Columns,
    store: ColumnStore,
    power: Vec<f64>,
    frames: u64,
    envelope: Option<EnvelopeBuilder>,
    time_peak: f32,
    preview_frames: Vec<u64>,
}

impl Refinement {
    fn new(meta: SignalMeta, mut request: AnalysisRequest) -> Result<Self, DspError> {
        check_request(&request)?;
        request.range = request.range.clamped_to(meta.len_samples);
        if request.range.len < request.cfg.fft_size as u64 {
            return Err(DspError::TooShort {
                samples: request.range.len,
                fft_size: request.cfg.fft_size,
            });
        }
        let total = (request.range.len - request.cfg.fft_size as u64) / request.cfg.hop as u64 + 1;
        let columns = Columns::new(total, request.width);
        let preview_frames = (0..request.width)
            .map(|column| (column as u64 * total).div_ceil(request.width as u64))
            .filter(|&frame| frame < total)
            .fold(Vec::new(), |mut frames, frame| {
                if frames.last() != Some(&frame) {
                    frames.push(frame);
                }
                frames
            });
        let plan = Plan::new(&request.cfg, &meta, request.height);
        Ok(Self {
            store: ColumnStore::new(request.width, request.height, request.reduce),
            power: vec![0.0; plan.bins],
            envelope: request
                .waveform_columns
                .filter(|&n| n > 0)
                .map(|n| EnvelopeBuilder::new(n, meta.channels(), request.range.len)),
            request,
            meta,
            plan,
            columns,
            preview_frames,
            frames: 0,
            time_peak: 0.0,
        })
    }

    fn absorb(&mut self, partial: &Partial) {
        self.store.absorb(partial, self.request.height);
        for (slot, add) in self.power.iter_mut().zip(&partial.power) {
            *slot += add;
        }
        self.frames += partial.cols.len() as u64;
        self.time_peak = self.time_peak.max(partial.time_peak);
    }

    fn preview(
        &mut self,
        source: &mut dyn SampleSource,
        control: &dyn Fn() -> Flow,
        frames: &[u64],
    ) -> Result<(), DspError> {
        let frame_len = self.request.cfg.fft_size * self.meta.channels();
        let batch = ((1 << 18) / frame_len).clamp(1, 64);
        for frames in frames.chunks(batch) {
            continuing(control)?;
            self.preview_batch(source, control, frames)?;
        }
        Ok(())
    }

    fn preview_batch(
        &mut self,
        source: &mut dyn SampleSource,
        control: &dyn Fn() -> Flow,
        frames: &[u64],
    ) -> Result<(), DspError> {
        let started = Instant::now();
        let channels = self.meta.channels();
        let frame_len = self.request.cfg.fft_size * channels;
        let mut samples = vec![0.0; frames.len() * frame_len];
        for &frame in frames {
            continuing(control)?;
            source.prefetch(SampleRange::new(
                self.request.range.start + frame * self.request.cfg.hop as u64,
                self.request.cfg.fft_size as u64,
            ));
        }
        for (&frame, buffer) in frames.iter().zip(samples.chunks_exact_mut(frame_len)) {
            continuing(control)?;
            let first = frame * self.request.cfg.hop as u64;
            source.seek(self.request.range.start + first)?;
            let mut filled = 0;
            while filled < buffer.len() {
                continuing(control)?;
                let got = source.read(&mut buffer[filled..])?;
                if got == 0 {
                    return Err(SourceError::Io(std::io::Error::new(
                        std::io::ErrorKind::UnexpectedEof,
                        "preview frame is incomplete",
                    ))
                    .into());
                }
                filled += got;
            }
            if let Some(envelope) = &mut self.envelope {
                envelope.fold(buffer, first);
            }
        }
        continuing(control)?;
        let reading = started.elapsed();
        let partial = frames
            .par_iter()
            .zip(samples.par_chunks_exact(frame_len))
            .with_min_len(16)
            .fold(
                || Partial::new(self.plan.bins, self.plan.fft_size),
                |mut acc, (&frame, samples)| {
                    self.plan.frame(samples, &mut acc, self.columns.of(frame));
                    acc
                },
            )
            .reduce(
                || Partial::new(self.plan.bins, self.plan.fft_size),
                Partial::merge,
            );
        self.absorb(&partial);
        tracing::trace!(?reading, transform = ?(started.elapsed() - reading), frames = frames.len(), "preview batch");
        Ok(())
    }

    fn snapshot(&self, frozen: Option<&DisplayScale>) -> Analysis {
        let request = self.request;
        let (f0, f1) = self.meta.frequency_span();
        let db = DbGrid {
            width: request.width,
            height: request.height,
            values: self.store.clone().finish(),
            t0: request.range.start as f64 / self.meta.sample_rate,
            t1: request.range.end() as f64 / self.meta.sample_rate,
            f0,
            f1,
        };
        let psd = averaged_spectrum(&self.plan, &self.meta, &self.power, self.frames.max(1));
        let resolved;
        let scale = match frozen {
            Some(scale) => scale,
            None => {
                let (dynamic_range, db_min, db_max) =
                    resolve_dynamic_range(request.dynamic_range, &db.values, &psd);
                resolved = DisplayScale {
                    shading: Shading {
                        colormap: request.colormap,
                        db_min,
                        db_max,
                    },
                    dynamic_range,
                };
                &resolved
            }
        };
        Analysis {
            spectrogram: shade(&db, scale.shading),
            db,
            psd,
            waveform: finish_envelope(self.envelope.clone(), request.range, self.meta.sample_rate),
            time_peak: self.time_peak,
            frames: self.frames,
            enbw_hz: self.plan.window.enbw_hz(self.meta.sample_rate),
            dynamic_range: scale.dynamic_range,
        }
    }

    fn transform(&mut self, block: &Block<'_>, frame_base: u64, frames: usize) {
        let cfg = self.request.cfg;
        let channels = self.meta.channels();
        let buffer = &block.buf;
        let partial = (0..frames)
            .into_par_iter()
            .with_min_len(8)
            .filter(|&k| {
                self.preview_frames
                    .binary_search(&(frame_base + k as u64))
                    .is_err()
            })
            .fold(
                || Partial::new(self.plan.bins, cfg.fft_size),
                |mut acc, k| {
                    let start = k * cfg.hop * channels;
                    self.plan.frame(
                        &buffer[start..start + cfg.fft_size * channels],
                        &mut acc,
                        self.columns.of(frame_base + k as u64),
                    );
                    acc
                },
            )
            .reduce(
                || Partial::new(self.plan.bins, cfg.fft_size),
                Partial::merge,
            );
        self.absorb(&partial);
    }
}

fn continuing(control: &dyn Fn() -> Flow) -> Result<(), DspError> {
    match control() {
        Flow::Continue => Ok(()),
        Flow::Stop => Err(DspError::Cancelled),
    }
}

/// Publish a sparse preview and rate-limited full snapshots, returning the final analysis.
/// The control callback is checked between bounded read/transform batches.
pub fn analyze_progressive(
    source: &mut dyn SampleSource,
    request: &AnalysisRequest,
    control: &dyn Fn() -> Flow,
    publish: &mut dyn FnMut(Analysis, Coverage) -> Flow,
) -> Result<Analysis, DspError> {
    continuing(control)?;
    let started = Instant::now();
    let mut state = Refinement::new(source.meta().clone(), *request)?;
    let planned = started.elapsed();
    source.access_pattern(AccessPattern::Sparse);
    let count = state.preview_frames.len().min(FIRST_PREVIEW_FRAMES);
    let first: Vec<u64> = (0..count)
        .map(|i| state.preview_frames[i * state.preview_frames.len() / count])
        .collect();
    state.preview(source, control, &first)?;
    let transformed = started.elapsed();
    continuing(control)?;
    let preview = state.snapshot(None);
    tracing::debug!(?planned, ?transformed, snapshot = ?(started.elapsed() - transformed), "preview prepared");
    let frozen = DisplayScale {
        shading: Shading {
            colormap: request.colormap,
            db_min: preview.spectrogram.db_min,
            db_max: preview.spectrogram.db_max,
        },
        dynamic_range: preview.dynamic_range,
    };
    if publish(
        preview,
        Coverage {
            refined_columns: 0,
            width: request.width,
        },
    ) == Flow::Stop
    {
        return Err(DspError::Cancelled);
    }
    let remaining: Vec<u64> = state
        .preview_frames
        .iter()
        .copied()
        .filter(|frame| first.binary_search(frame).is_err())
        .collect();
    if !remaining.is_empty() {
        state.preview(source, control, &remaining)?;
    }
    let cfg = request.cfg;
    let range = state.request.range;
    source.access_pattern(AccessPattern::Sequential);
    source.seek(range.start)?;
    let mut block = Block::new(source, cfg.fft_size, state.meta.channels(), range.len);
    block.capacity = cfg.fft_size + cfg.hop * 255;
    block.buf.resize(block.capacity * block.channels, 0.0);
    let mut frame_base = 0;
    let mut last_snapshot = Instant::now();
    while frame_base < state.columns.total_frames {
        continuing(control)?;
        block.fill()?;
        if let Some(envelope) = &mut state.envelope {
            block.fold_into(envelope);
        }
        if block.filled < cfg.fft_size {
            return Err(SourceError::Io(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "refinement frame is incomplete",
            ))
            .into());
        }
        let frames = ((block.filled - cfg.fft_size) / cfg.hop + 1)
            .min((state.columns.total_frames - frame_base) as usize);
        state.transform(&block, frame_base, frames);
        frame_base += frames as u64;
        let mut consumed = frames * cfg.hop;
        while consumed > block.filled && block.remaining > 0 {
            continuing(control)?;
            consumed -= block.filled;
            block.carry(block.filled);
            block.fill()?;
            if let Some(envelope) = &mut state.envelope {
                block.fold_into(envelope);
            }
        }
        block.carry(consumed.min(block.filled));
        if last_snapshot.elapsed() >= SNAPSHOT_INTERVAL && frame_base < state.columns.total_frames {
            continuing(control)?;
            let snapshot = state.snapshot(Some(&frozen));
            let coverage = Coverage {
                refined_columns: state.columns.of(frame_base),
                width: request.width,
            };
            if publish(snapshot, coverage) == Flow::Stop {
                return Err(DspError::Cancelled);
            }
            last_snapshot = Instant::now();
        }
    }
    continuing(control)?;
    block.drain(state.envelope.as_mut())?;
    state.time_peak = state.time_peak.max(block.peak);
    continuing(control)?;
    Ok(state.snapshot(None))
}
