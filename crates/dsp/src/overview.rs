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
        if width == 0 || height == 0 {
            return Err(DspError::BadOutputSize { width, height });
        }
        let request = self.request;
        if self
            .view_cache
            .as_ref()
            .is_none_or(|cache| cache.dimensions() != (width, height))
        {
            self.view_cache = Some(RenderCache::new(self, width, height));
            self.dirty.fill(true);
        }
        let cache = self.view_cache.as_mut().expect("initialized view cache");
        cache.refresh(&self.store, &self.dirty);
        let (f0, f1) = self.meta.frequency_span();
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
    time: Overlaps,
    repeats: Vec<usize>,
    frequency: Overlaps,
    values: Vec<f32>,
    image: SpectrogramImage,
    dirty: Vec<bool>,
    shading: Option<Shading>,
}

impl RenderCache {
    fn new(state: &Overview, width: usize, height: usize) -> Self {
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
        Self {
            time,
            repeats,
            frequency: Overlaps::new(state.plan.bins as u64, state.request.height, height, false),
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

#[cfg(test)]
fn rebin(store: &ColumnStore, time: &Overlaps, frequency: &Overlaps) -> Vec<f32> {
    let filled: Vec<_> = (0..store.width).collect();
    time.0
        .iter()
        .flat_map(|spans| {
            frequency
                .0
                .iter()
                .map(|rows| reduced_value(store, &filled, spans, rows))
        })
        .collect()
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
mod tests {
    use super::*;

    #[test]
    fn fractional_time_cells_use_frame_counts_and_keep_boundary_peaks() {
        let mut store = ColumnStore::new(3, 1, Reduce::MeanPower);
        store.counts = vec![4, 3, 3];
        store.power = vec![4.0, 27.0, 75.0];
        let time = Overlaps::new(10, 3, 2, true);
        let freq = Overlaps::new(1, 1, 1, false);
        let db = rebin(&store, &time, &freq);
        for (actual, power) in db.iter().zip([2.6f64, 18.6]) {
            assert!((f64::from(*actual) - 10.0 * power.log10()).abs() < 1e-5);
        }
        store.reduce = Reduce::Max;
        store.values = vec![1.0, 3.0, 5.0];
        let db = rebin(&store, &time, &freq);
        assert!((db[0] - 20.0 * 3.0f32.log10()).abs() < 1e-5);
        assert!((db[1] - 20.0 * 5.0f32.log10()).abs() < 1e-5);
    }

    #[test]
    fn unequal_frequency_groups_are_weighted_by_bin_overlap() {
        let mut store = ColumnStore::new(1, 2, Reduce::MeanPower);
        store.counts = vec![2];
        store.power = vec![2.0, 18.0];
        let values = rebin(
            &store,
            &Overlaps::new(2, 1, 1, true),
            &Overlaps::new(5, 2, 3, false),
        );
        for (actual, power) in values.iter().zip([1.0f64, 5.0, 9.0]) {
            assert!((f64::from(*actual) - 10.0 * power.log10()).abs() < 1e-5);
        }
        let whole = rebin(
            &store,
            &Overlaps::new(2, 1, 1, true),
            &Overlaps::new(5, 2, 1, false),
        );
        assert!((f64::from(whole[0]) - 10.0 * 5.8f64.log10()).abs() < 1e-5);
    }

    #[test]
    fn envelope_rebin_keeps_a_peak_crossing_a_cache_boundary() {
        let source = WaveformEnvelope {
            columns: 3,
            channels: 2,
            min: vec![0.0, 0.0, -0.8, -0.4, 0.0, 0.0],
            max: vec![0.0, 0.0, 0.9, 0.3, 0.0, 0.0],
            t0: 0.0,
            t1: 10.0,
        };
        let view = rebin_waveform(source, 2, 10);
        assert_eq!(view.min, vec![-0.8, -0.4, -0.8, -0.4]);
        assert_eq!(view.max, vec![0.9, 0.3, 0.9, 0.3]);
    }
}

#[cfg(test)]
#[test]
fn incremental_overview_render_matches_a_fresh_render_with_empty_followers() {
    use argand_core::{Domain, SampleFormat, SampleType};
    for reduce in [Reduce::Max, Reduce::MeanPower] {
        let meta = SignalMeta {
            sample_rate: 1000.0,
            center_freq: 0.0,
            sample_type: SampleType::new(Domain::Iq, SampleFormat::F32),
            len_samples: 1000,
            container: "test",
            divisor: 1.0,
            source: "memory".into(),
        };
        let request = AnalysisRequest {
            cfg: StftConfig::new(8, Window::Hann),
            range: SampleRange::new(0, 1000),
            width: 7,
            height: 4,
            reduce,
            colormap: Colormap::Oceanic,
            dynamic_range: DynamicRange::Auto,
            waveform_columns: None,
        };
        let mut state = Overview::with_cache(meta, request, false).unwrap();
        for (column, value) in [(0, 0.1), (6, 0.8), (0, 0.6), (3, 0.2), (3, 0.7)] {
            let mut partial = state.partial();
            partial.cols = vec![column];
            partial.rows = vec![value, value * 0.5, value * 0.1, 0.0];
            partial.frames = 1;
            state.absorb(&partial);
            let incremental = state.render(5, 3, None).unwrap();
            state.view_cache = None;
            let fresh = state.render(5, 3, None).unwrap();
            assert_eq!(incremental.db.values, fresh.db.values);
            assert_eq!(incremental.spectrogram.rgba, fresh.spectrogram.rgba);
        }
        state.display_scale = None;
        let final_view = state.render(5, 3, None).unwrap();
        state.view_cache = None;
        let fresh = state.render(5, 3, None).unwrap();
        assert_eq!(final_view.db.values, fresh.db.values);
        assert_eq!(final_view.spectrogram.rgba, fresh.spectrogram.rgba);
    }
}
