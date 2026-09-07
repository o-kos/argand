use super::*;

use argand_core::{Colormap, Domain, SampleFormat, SampleRange, SampleType};
use argand_dsp::{DynamicRange, Reduce, StftConfig, Window};
use argand_io::testutil::{TempDir, iq_tone, write_wav};

/// Long enough for several frames at the transform size below, short enough
/// that every test here runs in milliseconds.
const SAMPLES: usize = 4096;
const RATE: f64 = 48_000.0;

fn request() -> AnalysisRequest {
    AnalysisRequest {
        cfg: StftConfig::new(256, Window::Hann),
        range: SampleRange::new(0, SAMPLES as u64),
        width: 64,
        height: 32,
        reduce: Reduce::Max,
        colormap: Colormap::Oceanic,
        dynamic_range: DynamicRange::Default,
        waveform_columns: None,
    }
}

/// A small I/Q capture on disk, removed when the directory goes out of scope.
fn capture(dir: &TempDir) -> PathBuf {
    let values = iq_tone(SAMPLES, RATE, 6_000.0, 0.5);
    write_wav(
        &dir.join("capture.wav"),
        SampleType::new(Domain::Iq, SampleFormat::F32),
        RATE as u32,
        &values,
        1.0,
    )
}

/// The next update, or `None` once the thread has stopped.
///
/// Nothing here can wait for ever: the thread owns the only sender, so the
/// channel closes behind it however it ends.
fn next(updates: &async_channel::Receiver<Update>) -> Option<Update> {
    updates.recv_blocking().ok()
}

/// The next `Ready` or `Failed`, skipping whatever progress arrives first.
fn next_result(updates: &async_channel::Receiver<Update>) -> Option<Update> {
    loop {
        match next(updates)? {
            Update::Progress { done, total } => assert!(done <= total, "{done} of {total}"),
            other => return Some(other),
        }
    }
}

#[test]
fn a_file_that_opens_reports_what_it_is_before_anything_is_asked_of_it() {
    let dir = TempDir::new("analysis-open");
    let (analyst, updates) = open(capture(&dir), OpenHints::default());

    let Some(Update::Opened(meta)) = next(&updates) else {
        panic!("the first update should describe the file");
    };
    assert_eq!(meta.sample_rate, RATE);
    assert_eq!(meta.len_samples, SAMPLES as u64);
    assert!(meta.is_iq());
    drop(analyst);
}

#[test]
fn a_request_comes_back_as_a_picture_of_the_size_it_asked_for() {
    let dir = TempDir::new("analysis-request");
    let (analyst, updates) = open(capture(&dir), OpenHints::default());

    assert!(matches!(next(&updates), Some(Update::Opened(_))));
    assert!(analyst.request(request()), "the thread should be listening");

    let Some(Update::Ready { analysis, elapsed }) = next_result(&updates) else {
        panic!("a picture should come back");
    };
    assert!(!elapsed.is_zero(), "the worker should time the analysis");
    assert_eq!(analysis.spectrogram.width, 64);
    assert_eq!(analysis.spectrogram.height, 32);
    // A complex capture is two-sided about its centre frequency.
    assert!(
        analysis.spectrogram.f0 < 0.0 && analysis.spectrogram.f1 > 0.0,
        "{}..{} is not a two-sided spectrum",
        analysis.spectrogram.f0,
        analysis.spectrogram.f1
    );
}

#[test]
fn a_file_that_will_not_open_says_so_instead_of_taking_the_application_with_it() {
    let dir = TempDir::new("analysis-bad");
    let path = dir.join("not-a-signal.bin");
    std::fs::write(&path, b"this is not a capture").expect("write fixture");

    let (_analyst, updates) = open(path, OpenHints::default());

    let Some(Update::Failed(error)) = next(&updates) else {
        panic!("an unrecognised container should fail");
    };
    let message = format!("{error:#}");
    assert!(
        message.contains("--raw"),
        "the message should say how to read it anyway: {message}"
    );
    // Nothing follows a failure to open: the thread ends and closes the
    // channel rather than leaving the window waiting for an update.
    assert!(next(&updates).is_none());
}

#[test]
fn a_request_the_transform_will_not_run_leaves_the_file_open_for_the_next_one() {
    let dir = TempDir::new("analysis-retry");
    let (analyst, updates) = open(capture(&dir), OpenHints::default());
    assert!(matches!(next(&updates), Some(Update::Opened(_))));

    // More points than the capture holds, which the transform refuses.
    let mut impossible = request();
    impossible.cfg = StftConfig::new(1 << 16, Window::Hann);
    assert!(analyst.request(impossible));
    assert!(matches!(next_result(&updates), Some(Update::Failed(_))));

    // The thread is still there, and a request it can run still works.
    assert!(analyst.request(request()));
    assert!(
        matches!(next_result(&updates), Some(Update::Ready { .. })),
        "a workable request after a refused one should still produce a picture"
    );
}

#[test]
fn only_the_last_of_a_run_of_requests_is_transformed() {
    // The policy, not the race: a transform short enough to test against would
    // finish between two offers as often as not, and the question here is what
    // happens to the offers that pile up while a long one runs.
    let (sender, receiver) = async_channel::unbounded();
    for width in [16, 32, 48] {
        let mut sized = request();
        sized.width = width;
        sender.try_send(sized).expect("the queue is open");
    }

    let first = receiver.try_recv().expect("the first request");
    assert_eq!(newest(first, &receiver).width, 48);
    assert!(receiver.is_empty(), "the overtaken requests should be gone");
}

#[test]
fn a_window_that_has_gone_stops_the_thread_rather_than_leaving_it_waiting() {
    let dir = TempDir::new("analysis-drop");
    let (analyst, updates) = open(capture(&dir), OpenHints::default());
    assert!(matches!(next(&updates), Some(Update::Opened(_))));

    drop(analyst);
    // With no sender left there is no request to wait for, so the thread ends
    // and closes the channel behind it.
    assert!(updates.recv_blocking().is_err());
}
