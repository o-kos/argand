use super::*;

use argand_core::{DbGrid, Domain, Psd, SampleFormat, SampleType, SpectrogramImage};
use argand_dsp::{DynamicRange, DynamicRangeResult};

fn meta() -> SignalMeta {
    SignalMeta {
        sample_rate: 24_000.0,
        center_freq: 12_579_000.0,
        sample_type: SampleType::new(Domain::Iq, SampleFormat::I16),
        len_samples: 48_000,
        container: "wav",
        divisor: 32_768.0,
        source: PathBuf::from("/captures/hfdl.iqw"),
    }
}

/// An analysis carrying nothing but its size, which is all this module reads.
fn analysis(width: usize) -> Box<Analysis> {
    Box::new(Analysis {
        spectrogram: SpectrogramImage::new(width, 4),
        db: DbGrid {
            width,
            height: 4,
            values: vec![0.0; width * 4],
            t0: 0.0,
            t1: 1.0,
            f0: -12_000.0,
            f1: 12_000.0,
        },
        psd: Psd {
            freqs_hz: Vec::new(),
            db: Vec::new(),
            segments: 0,
        },
        waveform: None,
        time_peak: 0.5,
        frames: 1,
        enbw_hz: 1.0,
        dynamic_range: DynamicRangeResult {
            requested: DynamicRange::Default,
            effective_db: 110.0,
            recommended_db: 60.0,
        },
    })
}

fn origin(path: &str) -> Origin {
    Origin::new(PathBuf::from(path))
}

fn opening() -> Document {
    Document::opening(origin("/captures/hfdl.iqw"))
}

#[test]
fn a_document_says_nothing_about_a_file_it_has_not_opened_yet() {
    let document = opening();
    assert_eq!(document.status(), &Status::Opening);
    assert!(document.meta().is_none());
    assert!(document.analysis().is_none());
    assert!(
        document.summary().is_none(),
        "nothing is known about the file until it opens"
    );
}

#[test]
fn opening_a_file_is_what_lets_a_request_be_built_for_it() {
    let mut document = opening();
    // The span to analyse is the length the file just reported, so the window
    // is told to build a request rather than merely to redraw.
    assert_eq!(document.apply(Update::Opened(meta())), Effect::Opened);
    assert_eq!(document.meta().map(|m| m.len_samples), Some(48_000));
}

#[test]
fn a_finished_analysis_is_what_the_window_draws_from() {
    let mut document = opening();
    document.apply(Update::Opened(meta()));
    assert_eq!(
        document.apply(Update::Ready {
            analysis: analysis(64),
            elapsed: Duration::from_millis(1250)
        }),
        Effect::Analysis
    );
    assert_eq!(
        document.status(),
        &Status::Ready {
            elapsed: Duration::from_millis(1250)
        }
    );
    assert_eq!(document.status().message(), "ready in 1.25s");
    assert_eq!(document.analysis().map(|a| a.spectrogram.width), Some(64));

    document.apply(Update::Ready {
        analysis: analysis(128),
        elapsed: Duration::from_millis(2500),
    });
    assert_eq!(document.status().message(), "ready in 2.5s");
    assert_eq!(document.analysis().map(|a| a.spectrogram.width), Some(128));
}

#[test]
fn a_new_transform_leaves_the_previous_picture_up_while_it_runs() {
    let mut document = opening();
    document.apply(Update::Opened(meta()));
    document.apply(Update::Ready {
        analysis: analysis(64),
        elapsed: Duration::from_millis(1250),
    });

    // A resize asks for a wider picture. Blanking the window until it arrives
    // would be a worse answer than a slightly stale spectrogram.
    assert_eq!(
        document.apply(Update::Progress {
            done: 10,
            total: 40
        }),
        Effect::Status
    );
    assert_eq!(
        document.status(),
        &Status::Analyzing {
            done: 10,
            total: 40
        }
    );
    assert_eq!(document.analysis().map(|a| a.spectrogram.width), Some(64));
}

#[test]
fn a_failure_is_something_to_read_rather_than_something_to_crash_on() {
    let mut document = opening();
    let error = anyhow::anyhow!("unrecognised container").context("opening hfdl.iqw");

    assert_eq!(document.apply(Update::Failed(error)), Effect::Status);
    let Status::Failed(message) = document.status() else {
        panic!("the document should be showing a failure");
    };
    // Both halves of the chain: the step that failed and what went wrong.
    assert!(message.contains("opening hfdl.iqw"), "{message}");
    assert!(message.contains("unrecognised container"), "{message}");
    // The status bar names it rather than quoting it; the reason goes where
    // the picture would have been.
    assert_eq!(document.status().message(), "cannot be read");
}

#[test]
fn a_transform_that_has_not_reported_yet_has_no_fraction_to_show() {
    let mut document = opening();
    document.apply(Update::Opened(meta()));
    assert_eq!(document.status().message(), "analysing...");

    document.apply(Update::Progress { done: 1, total: 40 });
    assert_eq!(document.status().message(), "analysing... 2%");
}

#[test]
fn the_status_bar_separates_metadata_and_keeps_rf_context() {
    let mut document = opening();
    document.apply(Update::Opened(meta()));

    assert_eq!(
        document.summary().unwrap().iter().map(|field| (field.value.as_str(), field.hint)).collect::<Vec<_>>(),
        vec![("wav", "Container"), ("iq i16", "Sample format (complex I/Q)"), ("24 kHz", "Sample rate"), ("0:02.000", "Duration (minutes:seconds.milliseconds)"), ("12.579 MHz", "Centre frequency")]
    );
}

#[test]
fn a_baseband_capture_has_no_centre_frequency_worth_printing() {
    let mut document = opening();
    document.apply(Update::Opened(SignalMeta {
        center_freq: 0.0,
        ..meta()
    }));

    assert_eq!(
        document.summary().unwrap().iter().map(|field| field.value.as_str()).collect::<Vec<_>>(),
        vec!["wav", "iq i16", "24 kHz", "0:02.000"]
    );
}

#[test]
fn a_document_is_named_by_its_file_rather_than_by_its_whole_path() {
    assert_eq!(origin("/captures/2026/hfdl.iqw").name(), "hfdl.iqw");
}

#[test]
fn capture_duration_rounds_before_splitting_minutes_seconds_and_milliseconds() {
    for (seconds, expected) in [
        (0.0, "0:00.000"),
        (0.0004, "0:00.000"),
        (0.001, "0:00.001"),
        (59.9996, "1:00.000"),
        (221.34, "3:41.340"),
        (3599.9996, "60:00.000"),
        (3661.001, "61:01.001"),
    ] {
        assert_eq!(capture_duration(seconds), expected);
    }
}

#[test]
fn metadata_duration_counts_iq_pairs_and_real_samples_once() {
    for (domain, label) in [(Domain::Iq, "iq i16"), (Domain::Real, "real i16")] {
        let mut document = opening();
        document.apply(Update::Opened(SignalMeta {
            sample_type: SampleType::new(domain, SampleFormat::I16),
            len_samples: 5_312_160,
            ..meta()
        }));
        let fields = document.summary().unwrap();
        assert_eq!(fields[1].value, label);
        assert_eq!(fields[3].value, "3:41.340");
    }
}
