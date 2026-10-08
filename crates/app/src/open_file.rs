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

    /// Two analysis requests that differ, so each gets its own generation.
    fn request_a() -> argand_dsp::AnalysisRequest {
        argand_dsp::AnalysisRequest {
            cfg: argand_dsp::StftConfig::new(256, argand_dsp::Window::Hann),
            range: argand_core::SampleRange::new(0, 1000),
            width: 64,
            height: 32,
            reduce: argand_dsp::Reduce::Max,
            colormap: argand_core::Colormap::Oceanic,
            dynamic_range: argand_dsp::DynamicRange::Default,
            waveform_columns: None,
        }
    }

    fn request_b() -> argand_dsp::AnalysisRequest {
        // Changing the reduction changes the transform itself, so this request
        // takes its own generation, unlike a style-only change.
        argand_dsp::AnalysisRequest {
            reduce: argand_dsp::Reduce::Mean,
            dynamic_range: argand_dsp::DynamicRange::Fixed(42.0),
            ..request_a()
        }
    }

    /// The settings the second request carries.
    fn requested_b(settings: &Settings) -> Settings {
        let mut requested = *settings;
        requested.dynamic_range = argand_dsp::DynamicRange::Fixed(42.0);
        requested
    }

    /// A real analyst over a file that will not open, which is the cheapest
    /// one to build and gates exactly like any other.
    fn state() -> OpenFileState {
        let (analyst, _updates, _start) = crate::analysis::prepare(
            PathBuf::from("missing.iqw"),
            Default::default(),
            crate::execution::Settings::default(),
            crate::release::lease().0,
        );
        let document =
            Document::opening(crate::document::Origin::new(PathBuf::from("missing.iqw")));
        OpenFileState::new(document, analyst)
    }

    fn ready(generation: u64, view_revision: u64) -> Delivery {
        crate::analysis::for_test(
            Update::Ready {
                analysis: analysis(),
                elapsed: Duration::ZERO,
            },
            Some(generation),
            Some(view_revision),
        )
    }

    fn failed(generation: u64, view_revision: u64) -> Delivery {
        crate::analysis::for_test(
            Update::Failed(anyhow::anyhow!("transform failed")),
            Some(generation),
            Some(view_revision),
        )
    }

    #[test]
    fn a_superseded_delivery_is_rejected_and_records_nothing() {
        let settings = Settings::from_config(&crate::config::Config::default());
        let mut state = state();

        state.analyst.request_view(request_a(), None);
        assert_eq!(state.accept(ready(1, 1), &settings), Some(Effect::Analysis));
        assert_eq!(state.displayed_settings(), Some(&settings));

        // A second request for a different picture advances the generation,
        // so anything the first request's pipeline still delivers is
        // superseded: the record and the document must not move.
        state.analyst.request_view(request_b(), None);
        assert_eq!(state.accept(ready(1, 2), &settings), None);
        assert_eq!(state.accept(failed(1, 3), &settings), None);
        assert_eq!(
            state.displayed_settings(),
            Some(&settings),
            "superseded deliveries never touch the record"
        );

        assert_eq!(
            state.accept(ready(2, 2), &requested_b(&settings)),
            Some(Effect::Analysis),
            "and the newer request's picture lands"
        );
        assert_eq!(state.displayed_settings(), Some(&requested_b(&settings)));
    }

    #[test]
    fn a_failed_analysis_leaves_the_last_good_settings_in_place() {
        let settings = Settings::from_config(&crate::config::Config::default());
        let mut state = state();

        state.analyst.request_view(request_a(), None);
        assert_eq!(state.accept(ready(1, 1), &settings), Some(Effect::Analysis));
        assert_eq!(state.displayed_settings(), Some(&settings));

        // A preview request carries different settings, and its analysis
        // fails. The failure is accepted as a status, the record stays with
        // the picture on screen, and the document says Failed.
        state.analyst.request_view(request_b(), None);
        let preview = requested_b(&settings);
        assert_eq!(state.accept(failed(2, 2), &preview), Some(Effect::Status));
        assert_eq!(
            state.displayed_settings(),
            Some(&settings),
            "the failure never records the requested settings"
        );
        assert!(matches!(
            state.document.status(),
            crate::document::Status::Failed(_)
        ));

        // Restoring the opening settings changes the transform back, which
        // takes its own generation, whose first view revision records them.
        state.analyst.request_view(request_a(), None);
        assert_eq!(state.accept(ready(3, 3), &settings), Some(Effect::Analysis));
        assert_eq!(state.displayed_settings(), Some(&settings));
    }

    #[test]
    fn a_cancelled_preview_is_superseded_and_the_restored_settings_record_their_own_picture() {
        let settings = Settings::from_config(&crate::config::Config::default());
        let preview = requested_b(&settings);
        let mut state = state();

        state.analyst.request_view(request_a(), None);
        assert_eq!(state.accept(ready(1, 1), &settings), Some(Effect::Analysis));

        // The editor previews the advised settings, which requests their
        // analysis; cancelling then restores the opening settings, which
        // returns to the original picture's generation and advances only the
        // view revision.
        state.analyst.request_view(request_b(), None);
        state.analyst.request_view(request_a(), None);

        // The preview that lands late lost its view revision to the restored
        // request and must not record.
        assert_eq!(state.accept(ready(2, 2), &preview), None);
        assert_eq!(state.displayed_settings(), Some(&settings));

        // The restored settings produce their own picture and record them.
        assert_eq!(state.accept(ready(3, 3), &settings), Some(Effect::Analysis));
        assert_eq!(state.displayed_settings(), Some(&settings));
        assert_ne!(
            state.displayed_settings(),
            Some(&preview),
            "the cancelled preview is gone"
        );
    }

    #[test]
    fn a_style_only_preview_is_rejected_by_its_view_revision_alone() {
        let settings = Settings::from_config(&crate::config::Config::default());
        let mut style = settings;
        style.dynamic_range = argand_dsp::DynamicRange::Fixed(42.0);
        let mut state = state();

        state.analyst.request_view(request_a(), None);
        assert_eq!(state.accept(ready(1, 1), &settings), Some(Effect::Analysis));

        // A style-only request changes nothing the transform reads, so the
        // generation stays and only the view revision advances. Cancelling
        // advances it once more.
        let style_request = argand_dsp::AnalysisRequest {
            dynamic_range: argand_dsp::DynamicRange::Fixed(42.0),
            ..request_a()
        };
        state.analyst.request_view(style_request, None);
        state.analyst.request_view(request_a(), None);

        // The preview's own delivery still matches the generation, so the
        // view revision is the only thing that rejects it.
        assert_eq!(state.accept(ready(1, 2), &style), None);
        assert_eq!(state.displayed_settings(), Some(&settings));
        assert_eq!(state.accept(ready(1, 3), &settings), Some(Effect::Analysis));
        assert_eq!(state.displayed_settings(), Some(&settings));
    }

    #[test]
    fn a_second_file_starts_with_no_recorded_settings_while_the_first_keeps_its_own() {
        let settings = Settings::from_config(&crate::config::Config::default());
        let preview = requested_b(&settings);
        let mut first = state();

        first.analyst.request_view(request_a(), None);
        assert_eq!(first.accept(ready(1, 1), &settings), Some(Effect::Analysis));

        // The editor requests a preview whose analysis has not landed, and a
        // second file is opened in that moment.
        first.analyst.request_view(request_b(), None);
        let second = state();
        assert_eq!(second.displayed_settings(), None);

        // The pending preview belongs to the first file alone: it lands
        // there, and the second file's record stays independent.
        assert_eq!(first.accept(ready(2, 2), &preview), Some(Effect::Analysis));
        assert_eq!(first.displayed_settings(), Some(&preview));
        assert_eq!(second.displayed_settings(), None);
    }
}
