#[cfg(test)]
use argand_core::{DbGrid, Psd, SpectrogramImage};
#[cfg(test)]
use argand_dsp::{Analysis, DynamicRangeResult};

#[cfg(test)]
pub(super) fn warned_analysis() -> Box<Analysis> {
    Box::new(Analysis {
        spectrogram: SpectrogramImage::new(1, 1),
        db: DbGrid {
            width: 1,
            height: 1,
            values: vec![-60.0],
            t0: 0.0,
            t1: 1.0,
            f0: 0.0,
            f1: 1.0,
        },
        psd: Psd {
            freqs_hz: Vec::new(),
            db: Vec::new(),
            segments: 0,
        },
        waveform: None,
        time_peak: 0.01,
        frames: 1,
        enbw_hz: 1.0,
        dynamic_range: DynamicRangeResult {
            requested: DynamicRange::Default,
            effective_db: 110.0,
            recommended_db: 42.0,
        },
    })
}
