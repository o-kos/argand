//! Bounded full-range data, rebinned in the value domain before shading.

use super::*;

const TIME_COLUMNS: usize = 4096;
const PREVIEW_COLUMNS: u64 = 1024;
const FREQUENCY_ROWS: usize = 2048;
const WAVEFORM_COLUMNS: usize = 65536;

/// Analyze every frame once and retain a bounded overview for later rendering.
/// Only Max and MeanPower commute with the frequency reduction used here.
/// Published previews borrow the same state that will continue refining.
pub fn analyze_overview(
    source: &mut dyn SampleSource,
    request: &AnalysisRequest,
    options: ProgressiveOptions,
    control: &dyn Fn() -> Flow,
    publish: &mut dyn FnMut(&mut Overview, Coverage) -> Flow,
) -> Result<Overview, DspError> {
    analyze_overview_with_refresh(source, request, options, control, &|| false, publish)
}

/// As `analyze_overview`, with an explicit display refresh checked between bounded batches.
/// This bypasses the periodic snapshot interval without changing transform coverage.
pub fn analyze_overview_with_refresh(
    source: &mut dyn SampleSource,
    request: &AnalysisRequest,
    options: ProgressiveOptions,
    control: &dyn Fn() -> Flow,
    refresh: &dyn Fn() -> bool,
    publish: &mut dyn FnMut(&mut Overview, Coverage) -> Flow,
) -> Result<Overview, DspError> {
    continuing(control)?;
    check_request(request)?;
    if request.reduce == Reduce::Mean {
        return Err(DspError::BadOverviewReduction);
    }
    let meta = source.meta();
    let range = request.range.clamped_to(meta.len_samples);
    if range.len < request.cfg.fft_size as u64 {
        return Err(DspError::TooShort {
            samples: range.len,
            fft_size: request.cfg.fft_size,
        });
    }
    let frames = (range.len - request.cfg.fft_size as u64) / request.cfg.hop as u64 + 1;
    let bins = if meta.is_iq() {
        request.cfg.fft_size
    } else {
        request.cfg.fft_size / 2 + 1
    };
    let cached = AnalysisRequest {
        range,
        width: frames.min(TIME_COLUMNS as u64) as usize,
        height: bins.min(FREQUENCY_ROWS),
        waveform_columns: request
            .waveform_columns
            .filter(|&n| n > 0)
            .map(|_| range.len.min(WAVEFORM_COLUMNS as u64) as usize),
        ..*request
    };
    let mut state = Overview::with_cache(meta.clone(), cached, false)?;
    let preview_count = frames.min(PREVIEW_COLUMNS);
    state.preview_frames = (0..preview_count)
        .map(|i| ((u128::from(i) * u128::from(frames)).div_ceil(u128::from(preview_count))) as u64)
        .collect();
    refine(source, &mut state, options, control, refresh, publish)?;
    state.display_scale = None;
    Ok(state)
}

impl Overview {
    /// Change display settings without reading samples or changing the transform lattice.
    pub fn set_style(
        &mut self,
        colormap: Colormap,
        dynamic_range: DynamicRange,
    ) -> Result<(), DspError> {
        if let DynamicRange::Fixed(value) = dynamic_range
            && (!value.is_finite() || value <= 0.0)
        {
            return Err(DspError::BadDynamicRange(value));
        }
        if self.request.dynamic_range != dynamic_range {
            self.display_scale = None;
        } else if let Some(scale) = &mut self.display_scale {
            scale.shading.colormap = colormap;
        }
        self.request.colormap = colormap;
        self.request.dynamic_range = dynamic_range;
        Ok(())
    }

    /// Cache dimensions, independent of the initial and subsequent windows.
    pub fn dimensions(&self) -> (usize, usize) {
        (self.request.width, self.request.height)
    }

    /// Rebin cached extrema/powers, then shade. This takes no sample source.
    /// Max includes every overlapping cell. MeanPower weights linear power by
    /// overlap and observed frame counts. Sub-cell structure is not recoverable.
    /// The colour scale stays fixed across resizes, including during refinement.
    pub fn render(
        &mut self,
        width: usize,
        height: usize,
        waveform_columns: Option<usize>,
    ) -> Result<Analysis, DspError> {
        self.render_band(width, height, waveform_columns, self.meta.frequency_span())
    }

    /// Render a physical frequency interval from the retained cache without FFT work.
    pub fn render_band(
        &mut self,
        width: usize,
        height: usize,
        waveform_columns: Option<usize>,
        band: (f64, f64),
    ) -> Result<Analysis, DspError> {
        let full = self.meta.frequency_span();
        if !band.0.is_finite()
            || !band.1.is_finite()
            || band.0 < full.0
            || band.1 > full.1
            || band.0 >= band.1
        {
            return Err(DspError::BadFrequencyBand);
        }
        if width == 0 || height == 0 {
            return Err(DspError::BadOutputSize { width, height });
        }
        let request = self.request;
        if self
            .view_cache
            .as_ref()
            .is_none_or(|cache| cache.dimensions() != (width, height) || cache.band != band)
        {
            self.view_cache = Some(RenderCache::new(self, width, height, band));
            self.dirty.fill(true);
        }
        let cache = self.view_cache.as_mut().expect("initialized view cache");
        cache.refresh(&self.store, &self.dirty);
        let (f0, f1) = band;
        let db = DbGrid {
            width,
            height,
            values: cache.values.clone(),
            t0: request.range.start as f64 / self.meta.sample_rate,
            t1: request.range.end() as f64 / self.meta.sample_rate,
            f0,
            f1,
        };
        let psd = averaged_spectrum(&self.plan, &self.meta, &self.power, self.frames.max(1));
        let scale = self.display_scale.get_or_insert_with(|| {
            let (dynamic_range, db_min, db_max) =
                resolve_dynamic_range(request.dynamic_range, &db.values, &psd);
            DisplayScale {
                shading: Shading {
                    colormap: request.colormap,
                    db_min,
                    db_max,
                },
                dynamic_range,
            }
        });
        cache.shade(&db, scale.shading);
        self.dirty.fill(false);
        Ok(Analysis {
            spectrogram: cache.image.clone(),
            psd,
            waveform: finish_envelope(self.envelope.clone(), request.range, self.meta.sample_rate)
                .zip(waveform_columns.filter(|&n| n > 0))
                .map(|(envelope, columns)| rebin_waveform(envelope, columns, request.range.len)),
            db,
            time_peak: self.time_peak,
            frames: self.frames,
            enbw_hz: self.plan.window.enbw_hz(self.meta.sample_rate),
            dynamic_range: scale.dynamic_range,
        })
    }
}

#[derive(PartialEq)]
struct Overlap {
    cell: usize,
    units: f64,
    fraction: f64,
}

struct Overlaps(Vec<Vec<Overlap>>);

impl Overlaps {
    fn band(total: usize, source: usize, target: usize, band: (f64, f64)) -> Self {
        let edges: Vec<_> = (0..=source).map(|i| (total * i / source) as f64).collect();
        let boundary =
            |i: usize| (band.0 + (band.1 - band.0) * i as f64 / target as f64) * total as f64;
        Self(
            (0..target)
                .map(|i| {
                    let lo = boundary(i);
                    let hi = boundary(i + 1);
                    let first = edges
                        .partition_point(|&edge| edge <= lo)
                        .saturating_sub(1)
                        .min(source - 1);
                    let last = edges.partition_point(|&edge| edge < hi).min(source);
                    (first..last)
                        .map(|cell| {
                            let units = hi.min(edges[cell + 1]) - lo.max(edges[cell]);
                            Overlap {
                                cell,
                                units,
                                fraction: units / (edges[cell + 1] - edges[cell]),
                            }
                        })
                        .collect()
                })
                .collect(),
        )
    }

    fn new(total: u64, source: usize, target: usize, ceiling: bool) -> Self {
        let boundary = |index: usize, parts: usize| {
            let product = u128::from(total) * index as u128;
            if ceiling {
                product.div_ceil(parts as u128) as u64
            } else {
                (product / parts as u128) as u64
            }
        };
        let edges: Vec<_> = (0..=source).map(|i| boundary(i, source)).collect();
        Self(
            (0..target)
                .map(|i| {
                    let mut lo = boundary(i, target);
                    let mut hi = boundary(i + 1, target);
                    if lo == hi {
                        // Time uses the preceding populated column, frequency duplicates
                        // the bin at its lower edge, as in the ordinary analysis path.
                        lo = if ceiling { lo.saturating_sub(1) } else { lo };
                        hi = (lo + 1).min(total);
                    }
                    let first = edges
                        .partition_point(|&edge| edge <= lo)
                        .saturating_sub(1)
                        .min(source - 1);
                    let last = edges.partition_point(|&edge| edge < hi).min(source);
                    (first..last)
                        .map(|cell| {
                            let units = (hi.min(edges[cell + 1]) - lo.max(edges[cell])) as f64;
                            Overlap {
                                cell,
                                units,
                                fraction: units / (edges[cell + 1] - edges[cell]) as f64,
                            }
                        })
                        .collect()
                })
                .collect(),
        )
    }
}

pub(super) struct RenderCache {
    band: (f64, f64),
    time: Overlaps,
    repeats: Vec<usize>,
    frequency: Overlaps,
    values: Vec<f32>,
    image: SpectrogramImage,
    dirty: Vec<bool>,
    shading: Option<Shading>,
}

impl RenderCache {
    fn new(state: &Overview, width: usize, height: usize, band: (f64, f64)) -> Self {
        let time = Overlaps::new(state.columns.total_frames, state.request.width, width, true);
        let mut first = 0;
        let repeats = time
            .0
            .iter()
            .enumerate()
            .map(|(column, spans)| {
                if *spans != time.0[first] {
                    first = column;
                }
                first
            })
            .collect();
        let full = state.meta.frequency_span();
        let frequency = if band == full {
            Overlaps::new(state.plan.bins as u64, state.request.height, height, false)
        } else {
            let span = full.1 - full.0;
            Overlaps::band(
                state.plan.bins,
                state.request.height,
                height,
                ((band.0 - full.0) / span, (band.1 - full.0) / span),
            )
        };
        Self {
            band,
            time,
            repeats,
            frequency,
            values: vec![DB_FLOOR; width * height],
            image: SpectrogramImage::new(width, height),
            dirty: vec![true; width],
            shading: None,
        }
    }

    fn dimensions(&self) -> (usize, usize) {
        (self.time.0.len(), self.frequency.0.len())
    }

    fn refresh(&mut self, store: &ColumnStore, dirty: &[bool]) {
        let mut previous = 0;
        let filled: Vec<_> = store
            .counts
            .iter()
            .enumerate()
            .map(|(column, &count)| {
                if count > 0 {
                    previous = column;
                }
                previous
            })
            .collect();
        for (changed, spans) in self.dirty.iter_mut().zip(&self.time.0) {
            *changed = spans.iter().any(|span| dirty[filled[span.cell]]);
        }
        let height = self.frequency.0.len();
        self.values
            .par_chunks_mut(height)
            .zip(&self.time.0)
            .zip(&self.dirty)
            .enumerate()
            .filter(|(index, (_, changed))| **changed && self.repeats[*index] == *index)
            .for_each(|(_, ((column, spans), _))| {
                for (value, rows) in column.iter_mut().zip(&self.frequency.0) {
                    *value = reduced_value(store, &filled, spans, rows);
                }
            });
        for (column, &first) in self.repeats.iter().enumerate() {
            if first != column && self.dirty[column] {
                self.values
                    .copy_within(first * height..(first + 1) * height, column * height);
            }
        }
    }

    fn shade(&mut self, grid: &DbGrid, shading: Shading) {
        if self.shading != Some(shading) {
            self.dirty.fill(true);
        }
        shade_columns(
            grid,
            shading,
            &mut self.image,
            self.dirty
                .iter()
                .enumerate()
                .filter_map(|(i, &dirty)| (dirty && self.repeats[i] == i).then_some(i)),
        );
        let width = grid.width;
        self.image.rgba.par_chunks_mut(width * 4).for_each(|row| {
            for (column, &first) in self.repeats.iter().enumerate() {
                if first != column && self.dirty[column] {
                    row.copy_within(first * 4..first * 4 + 4, column * 4);
                }
            }
        });
        self.shading = Some(shading);
    }
}

fn reduced_value(
    store: &ColumnStore,
    filled: &[usize],
    time: &[Overlap],
    frequency: &[Overlap],
) -> f32 {
    let mut peak = 0.0f32;
    let mut power = 0.0;
    let mut weight = 0.0;
    for span in time {
        let column = filled[span.cell];
        for row in frequency {
            let index = column * store.height + row.cell;
            if store.reduce == Reduce::Max {
                peak = peak.max(store.values[index]);
            } else {
                let overlap = span.fraction * row.units;
                power += store.power[index] * overlap;
                weight += store.counts[column] as f64 * overlap;
            }
        }
    }
    if store.reduce == Reduce::Max {
        20.0 * peak.max(MAG_FLOOR).log10()
    } else {
        (10.0
            * (power / weight.max(f64::MIN_POSITIVE))
                .max(POWER_FLOOR)
                .log10()) as f32
    }
}

fn rebin_waveform(source: WaveformEnvelope, columns: usize, samples: u64) -> WaveformEnvelope {
    let spans = Overlaps::new(samples, source.columns, columns, true);
    let mut min = vec![f32::INFINITY; columns * source.channels];
    let mut max = vec![f32::NEG_INFINITY; min.len()];
    for (column, cells) in spans.0.iter().enumerate() {
        for span in cells {
            for channel in 0..source.channels {
                let target = column * source.channels + channel;
                let origin = span.cell * source.channels + channel;
                min[target] = min[target].min(source.min[origin]);
                max[target] = max[target].max(source.max[origin]);
            }
        }
    }
    WaveformEnvelope {
        columns,
        min,
        max,
        ..source
    }
}

#[cfg(test)]
#[path = "overview_test.rs"]
mod tests;

#[cfg(test)]
include!("overview_support_test.rs");

#[cfg(test)]
include!("overview_root_test.rs");
