use crate::analysis::{Analyst, Delivery};
use crate::document::{Document, Effect};
use crate::settings::Settings;

/// One open file's analysis-acceptance state: the document the deliveries
/// build, the worker gating which deliveries are current, and which settings
/// the picture on screen was produced with. The window-side parts of an open
/// file, such as the plot handle and the delivery tasks, stay in `Shell`'s own
/// `OpenFile`, because they name GPUI and cannot run in CI.
pub(crate) struct OpenFileState {
    pub(crate) document: Document,
    pub(crate) analyst: Analyst,
    displayed_settings: Option<Settings>,
}

impl OpenFileState {
    pub(crate) fn new(document: Document, analyst: Analyst) -> Self {
        Self {
            document,
            analyst,
            displayed_settings: None,
        }
    }

    /// The settings the picture on screen was produced with, if a picture has
    /// been accepted at all.
    pub(crate) fn displayed_settings(&self) -> Option<&Settings> {
        self.displayed_settings.as_ref()
    }

    /// Retake the recorded settings as these, when the picture on screen was
    /// produced with settings equivalent to them. The settings editor's apply
    /// uses this for a preview it has already seen and will not re-request,
    /// where the record must follow the applied settings without a picture.
    pub(crate) fn retake_equivalent(&mut self, settings: &Settings) -> bool {
        let matches = self
            .displayed_settings
            .as_ref()
            .is_some_and(|displayed| displayed.equivalent(*settings));
        if matches {
            self.displayed_settings = Some(*settings);
        }
        matches
    }

    /// Simulate an accepted picture for a test that has no delivery of its
    /// own.
    #[cfg(test)]
    pub(crate) fn record_for_test(&mut self, settings: &Settings) {
        self.displayed_settings = Some(*settings);
    }

    /// Gate the delivery, apply it, and record the settings behind the picture
    /// when the delivery carried one. A superseded delivery is rejected with
    /// `None` and leaves every field untouched, so a failed analysis leaves the
    /// last good settings in place and a cancelled editor preview keeps them
    /// until the restored settings produce their own picture.
    pub(crate) fn accept(&mut self, delivery: Delivery, settings: &Settings) -> Option<Effect> {
        if !self.analyst.accepts(&delivery) {
            return None;
        }
        let effect = self.document.apply(delivery.update);
        if effect == Effect::Analysis {
            self.displayed_settings = Some(*settings);
        }
        Some(effect)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::Update;
    use argand_core::{DbGrid, Psd, SpectrogramImage};
    use argand_dsp::{Analysis, DynamicRangeResult};
    use std::path::PathBuf;
    use std::time::Duration;

    /// A one-cell analysis, which is all the acceptance bookkeeping reads.
    fn analysis() -> Box<Analysis> {
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
                requested: Settings::from_config(&crate::config::Config::default()).dynamic_range,
                effective_db: 110.0,
                recommended_db: 42.0,
            },
        })
    }

    /// A real analyst over a file that will not open, which is the cheapest
    /// one to build and gates exactly like any other.
    fn state() -> OpenFileState {
        let (analyst, _updates, _start) = crate::analysis::prepare(
            PathBuf::from("missing.iqw"),
            Default::default(),
            crate::execution::Settings::default(),
        );
        let document =
            Document::opening(crate::document::Origin::new(PathBuf::from("missing.iqw")));
        OpenFileState::new(document, analyst)
    }

    fn ready(generation: u64) -> Delivery {
        crate::analysis::for_test(
            Update::Ready {
                analysis: analysis(),
                elapsed: Duration::ZERO,
            },
            Some(generation),
        )
    }

    fn failed(generation: u64) -> Delivery {
        crate::analysis::for_test(
            Update::Failed(anyhow::anyhow!("transform failed")),
            Some(generation),
        )
    }

    #[test]
    fn a_superseded_delivery_is_rejected_and_records_nothing() {
        let settings = Settings::from_config(&crate::config::Config::default());
        let mut state = state();
        assert_eq!(state.accept(ready(7), &settings), None);
        assert_eq!(state.displayed_settings(), None);
        assert!(
            matches!(state.document.status(), crate::document::Status::Opening),
            "and the document never saw it"
        );
    }

    #[test]
    fn a_failed_analysis_leaves_the_last_good_settings_in_place() {
        let settings = Settings::from_config(&crate::config::Config::default());
        let mut state = state();
        assert_eq!(
            state.accept(ready(0), &settings),
            Some(Effect::Analysis),
            "the picture lands first"
        );
        assert_eq!(state.displayed_settings(), Some(&settings));

        assert_eq!(state.accept(failed(0), &settings), Some(Effect::Status));
        assert_eq!(
            state.displayed_settings(),
            Some(&settings),
            "and the failure keeps the settings behind the picture"
        );
    }

    #[test]
    fn a_cancelled_preview_records_the_restored_settings_on_the_next_picture() {
        let settings = Settings::from_config(&crate::config::Config::default());
        let mut preview = settings;
        preview.dynamic_range = argand_dsp::DynamicRange::Fixed(42.0);
        let mut state = state();

        assert_eq!(state.accept(ready(0), &preview), Some(Effect::Analysis));
        assert_eq!(state.displayed_settings(), Some(&preview));

        // The editor's cancel restores the opening values, and the picture the
        // restored settings then produce records them.
        let restored = settings;
        assert_eq!(state.accept(ready(0), &restored), Some(Effect::Analysis));
        assert_eq!(state.displayed_settings(), Some(&restored));
        assert_ne!(
            state.displayed_settings(),
            Some(&preview),
            "the cancelled preview is gone"
        );
    }

    #[test]
    fn a_second_file_starts_with_no_recorded_settings_while_the_first_keeps_its_own() {
        let settings = Settings::from_config(&crate::config::Config::default());
        let mut first = state();
        let mut preview = settings;
        preview.dynamic_range = argand_dsp::DynamicRange::Fixed(42.0);
        assert_eq!(first.accept(ready(0), &preview), Some(Effect::Analysis));

        let second = state();
        assert_eq!(second.displayed_settings(), None);

        assert_eq!(
            first.displayed_settings(),
            Some(&preview),
            "and the first file keeps its own record"
        );
    }
}
