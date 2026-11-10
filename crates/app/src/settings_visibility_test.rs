use super::*;
use argand_core::{DbGrid, Psd, SpectrogramImage};

fn analysis(
    requested: DynamicRange,
    effective_db: f32,
    recommended_db: f32,
    peak_db: f32,
) -> Analysis {
    Analysis {
        spectrogram: SpectrogramImage::new(1, 1),
        db: DbGrid {
            width: 1,
            height: 1,
            values: vec![peak_db],
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
        dynamic_range: argand_dsp::DynamicRangeResult {
            requested,
            effective_db,
            recommended_db,
        },
    }
}

#[test]
fn a_narrow_recommendation_does_not_make_a_normal_signal_a_warning() {
    let range = argand_dsp::DynamicRangeResult {
        requested: DynamicRange::Default,
        effective_db: 110.0,
        recommended_db: 30.0,
    };
    assert_eq!(visibility_advice(range, -6.0, 0.9), None);
    assert_eq!(visibility_advice(range, -60.0, 0.002), Some(30.0));
    assert_eq!(visibility_advice(range, -180.0, 0.0), None);
    assert_eq!(visibility_advice(range, f32::NEG_INFINITY, 0.01), None);
    for requested in [DynamicRange::Fixed(110.0), DynamicRange::Auto] {
        assert_eq!(
            visibility_advice(
                argand_dsp::DynamicRangeResult { requested, ..range },
                -60.0,
                0.002
            ),
            None
        );
    }
}

#[test]
fn displayed_value_and_state_advance_as_one_analysis_snapshot() {
    let warned =
        DisplayedRange::from_analysis(&analysis(DynamicRange::Default, 110.0, 40.0, -60.0));
    assert_eq!(
        warned,
        DisplayedRange {
            effective_db: 110.0,
            state: RangeState::Warned(40.0),
        }
    );

    let corrected =
        DisplayedRange::from_analysis(&analysis(DynamicRange::Fixed(40.0), 40.0, 40.0, -60.0));
    assert_eq!(
        corrected,
        DisplayedRange {
            effective_db: 40.0,
            state: RangeState::Corrected,
        }
    );
}
