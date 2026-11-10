use super::*;
use crate::analysis::{FileInfo, Update};
use crate::document::Document;
use argand_core::{Domain, SampleFormat, SampleType, SignalMeta};
use std::path::PathBuf;
use std::time::Duration;

#[test]
fn control_foregrounds_keep_a_resting_colour_and_a_distinct_state_one() {
    let normal = gpui_kit::hsla(0.15, 0.8, 0.4, 1.0);
    let hovered = gpui_kit::hsla(0.15, 0.8, 0.8, 1.0);
    let foregrounds = ControlForegrounds::between(normal, hovered);

    assert_eq!(foregrounds.normal, normal);
    assert_eq!(foregrounds.active, normal.mix(hovered, 0.5));
    assert_ne!(foregrounds.active, normal);
    assert_ne!(foregrounds.active, hovered);
}

#[test]
fn range_state_applies_advice_then_restores_full_scale() {
    let advised = RangeState::from_request(Some(42.0), DynamicRange::Default)
        .next_range()
        .unwrap();
    assert_eq!(advised, DynamicRange::Fixed(42.0));

    let restored = RangeState::from_request(None, advised)
        .next_range()
        .unwrap();
    assert_eq!(restored, DynamicRange::Default);
    assert_eq!(RangeState::from_request(None, restored).next_range(), None);
}

#[test]
fn automatic_range_has_no_toggle_without_advice() {
    assert_eq!(
        RangeState::from_request(None, DynamicRange::Auto).next_range(),
        None
    );
    assert_eq!(
        RangeState::from_request(Some(38.0), DynamicRange::Auto).next_range(),
        None
    );
}

#[test]
fn pending_picture_suspends_an_otherwise_available_range_action() {
    let displayed = Settings::from_config(&Config::default());
    let requested = Settings {
        dynamic_range: DynamicRange::Fixed(42.0),
        ..displayed
    };
    let warned = Some(DisplayedRange {
        effective_db: 110.0,
        state: RangeState::Warned(42.0),
    });

    assert_eq!(
        next_range_action(Some(displayed), displayed, warned, false),
        Some(DynamicRange::Fixed(42.0))
    );
    assert_eq!(
        next_range_action(Some(displayed), requested, warned, false),
        None
    );
    assert_eq!(next_range_action(None, requested, warned, false), None);
}

#[test]
fn failed_analysis_suspends_the_retained_range_action() {
    let settings = Settings::from_config(&Config::default());
    let mut document = Document::opening(Origin::new(PathBuf::from("test.iqw")));
    document.apply(Update::Opened(
        SignalMeta {
            sample_rate: 24_000.0,
            center_freq: 0.0,
            sample_type: SampleType::new(Domain::Iq, SampleFormat::I16),
            len_samples: 48_000,
            container: "raw",
            divisor: 32_768.0,
            source: PathBuf::from("test.iqw"),
        },
        FileInfo::default(),
    ));
    document.apply(Update::Ready {
        analysis: warned_analysis(),
        elapsed: Duration::ZERO,
    });
    let ready = document_range_presentation(&document, Some(settings), settings);
    assert_eq!(
        ready,
        RangePresentation {
            displayed: Some(DisplayedRange {
                effective_db: 110.0,
                state: RangeState::Warned(42.0),
            }),
            next_action: Some(DynamicRange::Fixed(42.0)),
        }
    );

    document.apply(Update::Failed(anyhow::anyhow!("replacement failed")));
    assert_eq!(
        document_range_presentation(&document, Some(settings), settings),
        RangePresentation {
            displayed: ready.displayed,
            next_action: None,
        }
    );
}

#[test]
fn corrected_range_action_restores_full_scale_only_when_current() {
    let corrected = Settings {
        dynamic_range: DynamicRange::Fixed(42.0),
        ..Settings::from_config(&Config::default())
    };
    let range = Some(DisplayedRange {
        effective_db: 42.0,
        state: RangeState::Corrected,
    });

    assert_eq!(
        next_range_action(Some(corrected), corrected, range, false),
        Some(DynamicRange::Default)
    );
    assert_eq!(next_range_action(None, corrected, range, false), None);
}
