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

#[test]
fn waveform_preserves_each_channel_and_short_bursts_in_the_shared_columns() {
    let dir = TempDir::new("waveform-bursts");
    let mut values = vec![0.0; SAMPLES * 2];
    values[65 * 2] = 0.75;
    values[65 * 2 + 1] = -0.25;
    values[(SAMPLES - 1) * 2 + 1] = 0.5;
    let path = write_wav(
        &dir.join("bursts.wav"),
        SampleType::new(Domain::Iq, SampleFormat::F32),
        RATE as u32,
        &values,
        1.0,
    );
    let (analyst, updates) = open(path, OpenHints::default());
    assert!(matches!(next(&updates), Some(Update::Opened(_))));
    let mut request = request();
    request.waveform_columns = Some(request.width);
    assert!(analyst.request(request));
    let Some(Update::Ready { analysis, .. }) = next_result(&updates) else {
        panic!("analysis should finish");
    };
    let waveform = analysis.waveform.as_ref().expect("requested envelope");
    assert_eq!(waveform.columns, analysis.spectrogram.width);
    assert_eq!(waveform.t0, analysis.spectrogram.t0);
    assert_eq!(waveform.t1, analysis.spectrogram.t1);
    for column in 0..64 {
        for channel in 0..2 {
            let samples = (column * 64..(column + 1) * 64).map(|sample| values[sample * 2 + channel]);
            let low = samples.clone().fold(f32::INFINITY, f32::min);
            let high = samples.fold(f32::NEG_INFINITY, f32::max);
            assert_eq!(waveform.column(column, channel), Some((low, high)));
        }
    }
}

/// The next update, or `None` once the thread has stopped.
///
/// Nothing here can wait for ever: the thread owns the only sender, so the
/// channel closes behind it however it ends.
fn next(updates: &async_channel::Receiver<Delivery>) -> Option<Update> {
    updates.recv_blocking().ok().map(|delivery| delivery.update)
}

/// The next `Ready` or `Failed`, skipping whatever progress arrives first.
fn next_result(updates: &async_channel::Receiver<Delivery>) -> Option<Update> {
    loop {
        match next(updates)? {
            Update::Progress { done, total } => assert!(done <= total, "{done} of {total}"),
            Update::Snapshot { .. } => {},
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

#[test]
fn deferred_open_does_not_touch_the_file_until_the_first_frame_releases_it() {
    let dir = TempDir::new("deferred-open");
    let (_analyst, updates, start) = prepare(dir.join("missing.wav"), OpenHints::default(), crate::execution::Settings::default());
    assert!(updates.try_recv().is_err());
    start.start();
    assert!(matches!(next(&updates), Some(Update::Failed(_))));
}

#[test]
fn superseded_deliveries_cannot_replace_the_current_view() {
    let dir = TempDir::new("superseded");
    let (analyst, updates) = open(capture(&dir), OpenHints::default());
    assert!(matches!(next(&updates), Some(Update::Opened(_))));
    analyst.request(request());
    let old = updates.recv_blocking().unwrap();
    assert!(analyst.accepts(&old));
    let mut latest = request();
    latest.width = 17;
    analyst.request(latest);
    assert!(!analyst.accepts(&old));
    loop {
        let delivery = updates.recv_blocking().unwrap();
        if !analyst.accepts(&delivery) { continue; }
        if let Update::Ready { analysis, .. } = delivery.update {
            assert_eq!(analysis.db.width, 17);
            break;
        }
    }
}

#[test]
fn full_snapshot_queue_does_not_prevent_cancelling_a_final_reply() {
    use std::cell::Cell;
    let (sender, receiver) = async_channel::bounded(2);
    let delivery = || Delivery { prepared_at: Instant::now(), generation: Some(1), update: Update::Progress { done: 0, total: 1 } };
    sender.try_send(delivery()).unwrap();
    sender.try_send(delivery()).unwrap();
    let polls = Cell::new(0);
    send_result(&sender, delivery(), &|| {
        polls.set(polls.get() + 1);
        if polls.get() == 1 { Flow::Continue } else { Flow::Stop }
    });
    assert_eq!(receiver.len(), 2);
    assert_eq!(polls.get(), 2);
}


#[test]
fn aggregation_replacement_rejects_stale_work_and_matches_the_requested_result() {
    let dir = TempDir::new("aggregation-replacement");
    let path = capture(&dir);
    let (analyst, updates) = open(path.clone(), OpenHints::default());
    assert!(matches!(next(&updates), Some(Update::Opened(_))));
    analyst.request(request());
    let stale = updates.recv_blocking().unwrap();
    assert!(analyst.accepts(&stale));
    let latest = AnalysisRequest { reduce: Reduce::MeanPower, ..request() };
    analyst.request(latest);
    assert!(!analyst.accepts(&stale));
    let mut source = argand_io::open(&path, &OpenHints::default()).unwrap();
    let expected = argand_dsp::analyze(&mut *source, &latest, &mut |_, _| {}).unwrap();
    loop {
        let delivery = updates.recv_blocking().unwrap();
        if !analyst.accepts(&delivery) { continue; }
        if let Update::Ready { analysis, .. } = delivery.update {
            for (actual, expected) in analysis.db.values.iter().zip(&expected.db.values) {
                assert!((actual - expected).abs() < 0.0001);
            }
            break;
        }
    }
}
