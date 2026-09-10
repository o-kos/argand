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
    assert!(matches!(next(&updates), Some(Update::Opened(_, _))));
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

    let Some(Update::Opened(meta, info)) = next(&updates) else {
        panic!("the first update should describe the file");
    };
    assert!(info.bytes.is_some_and(|bytes| bytes > (SAMPLES * 8) as u64));
    assert_eq!(meta.sample_rate, RATE);
    assert_eq!(meta.len_samples, SAMPLES as u64);
    assert!(meta.is_iq());
    drop(analyst);
}

#[test]
fn a_request_comes_back_as_a_picture_of_the_size_it_asked_for() {
    let dir = TempDir::new("analysis-request");
    let (analyst, updates) = open(capture(&dir), OpenHints::default());

    assert!(matches!(next(&updates), Some(Update::Opened(_, _))));
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
    assert!(matches!(next(&updates), Some(Update::Opened(_, _))));

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
fn resize_requests_coalesce_without_invalidating_analysis() {
    let (sender, receiver) = async_channel::bounded(1);
    let analyst = Analyst { requests: sender, mailbox: Arc::new(Mailbox::default()) };
    analyst.request(request());
    let first = analyst.mailbox.latest().unwrap();
    for width in 1..1000 {
        analyst.request(AnalysisRequest { width, ..request() });
    }
    let last = analyst.mailbox.latest().unwrap();
    assert_eq!(last.analysis.width, 999);
    assert_eq!(last.generation, first.generation);
    assert_eq!(receiver.len(), 1);
    analyst.request(AnalysisRequest { reduce: Reduce::MeanPower, ..request() });
    assert_ne!(analyst.mailbox.latest().unwrap().generation, first.generation);
}

#[test]
fn a_window_that_has_gone_stops_the_thread_rather_than_leaving_it_waiting() {
    let dir = TempDir::new("analysis-drop");
    let (analyst, updates) = open(capture(&dir), OpenHints::default());
    assert!(matches!(next(&updates), Some(Update::Opened(_, _))));

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
fn resize_keeps_analysis_and_eventually_delivers_the_latest_size() {
    let dir = TempDir::new("superseded");
    let (analyst, updates) = open(capture(&dir), OpenHints::default());
    assert!(matches!(next(&updates), Some(Update::Opened(_, _))));
    analyst.request(request());
    let old = loop {
        let delivery = updates.recv_blocking().unwrap();
        if delivery.view_revision.is_some() { break delivery; }
    };
    assert!(analyst.accepts(&old));
    let generation = analyst.mailbox.latest().unwrap().generation;
    let mut latest = request();
    latest.width = 17;
    analyst.request(latest);
    assert_eq!(analyst.mailbox.latest().unwrap().generation, generation);
    assert!(!analyst.accepts(&old), "queued old dimensions must be rejected");
    loop {
        let delivery = updates.recv_blocking().unwrap();
        if !analyst.accepts(&delivery) { continue; }
        if let Update::Ready { analysis, .. } = delivery.update {
            if analysis.db.width != 17 { continue; }
            assert_eq!(analysis.frames, 61);
            break;
        }
    }
}

#[test]
fn rapid_time_navigation_rejects_old_pictures_and_analyzes_only_the_latest_range() {
    let dir = TempDir::new("time-navigation");
    let (analyst, updates) = open(capture(&dir), OpenHints::default());
    let Some(Update::Opened(meta, _)) = next(&updates) else { panic!("opened") };
    let initial = AnalysisRequest { waveform_columns: Some(64), ..request() };
    analyst.request(initial);
    let old = loop {
        let delivery = updates.recv_blocking().unwrap();
        if delivery.view_revision.is_some() { break delivery; }
    };
    let original = analyst.mailbox.latest().unwrap().generation;
    for start in 1..=100 {
        analyst.request(AnalysisRequest { range: SampleRange::new(start, 2048), ..initial });
    }
    assert_eq!(analyst.mailbox.latest().unwrap().generation, original + 100);
    assert!(analyst.requests.len() <= 1, "only one wakeup may be queued");
    assert!(!analyst.accepts(&old));
    loop {
        let delivery = updates.recv_blocking().unwrap();
        if !analyst.accepts(&delivery) { continue; }
        if let Update::Ready { analysis, .. } = delivery.update {
            assert_eq!(analysis.db.t0, 100.0 / meta.sample_rate);
            assert_eq!(analysis.db.t1, 2148.0 / meta.sample_rate);
            let waveform = analysis.waveform.unwrap();
            assert_eq!((waveform.t0, waveform.t1), (analysis.db.t0, analysis.db.t1));
            break;
        }
    }
}

#[test]
fn full_snapshot_queue_does_not_prevent_cancelling_a_final_reply() {
    use std::cell::Cell;
    let (sender, receiver) = async_channel::bounded(2);
    let delivery = || Delivery { prepared_at: Instant::now(), generation: Some(1), view_revision: None, update: Update::Progress { done: 0, total: 1 } };
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
    assert!(matches!(next(&updates), Some(Update::Opened(_, _))));
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

#[test]
fn cached_resize_keeps_the_analysis_duration_and_never_enters_analyzing() {
    let dir = TempDir::new("cached-resize");
    let (analyst, updates) = open(capture(&dir), OpenHints::default());
    assert!(matches!(next(&updates), Some(Update::Opened(_, _))));
    analyst.request(request());
    let Some(Update::Ready { elapsed, .. }) = next_result(&updates) else { panic!("ready"); };
    for (width, height) in [(73, 51), (200, 300), (3, 2), (64, 32)] {
        analyst.request(AnalysisRequest { width, height, ..request() });
        let Some(Update::Ready { analysis, elapsed: cached }) = next(&updates) else { panic!("resize must only redraw"); };
        assert_eq!(cached, elapsed);
        assert_eq!((analysis.db.width, analysis.db.height), (width, height));
        assert_eq!(analysis.frames, 61);
    }
}

#[test]
fn display_dimensions_and_style_are_excluded_from_invalidation() {
    let base = request();
    assert!(same_analysis(base, AnalysisRequest { dynamic_range: DynamicRange::Auto, colormap: Colormap::Inferno, ..base }));
    assert!(same_analysis(base, AnalysisRequest { width: 5, height: 8, ..base }));
    for changed in [
        AnalysisRequest { cfg: StftConfig::new(128, Window::Hann), ..base },
        AnalysisRequest { range: SampleRange::new(1, 3000), ..base },
        AnalysisRequest { reduce: Reduce::MeanPower, ..base },

        AnalysisRequest { waveform_columns: Some(12), ..base },
    ] { assert!(!same_analysis(base, changed)); }
}

#[test]
fn returning_to_a_previous_size_still_delivers_the_new_display_revision() {
    let dir = TempDir::new("resize-return");
    let (analyst, updates) = open(capture(&dir), OpenHints::default());
    assert!(matches!(next(&updates), Some(Update::Opened(_, _))));
    analyst.request(request());
    let Some(Update::Ready { elapsed, .. }) = next_result(&updates) else { panic!("ready"); };
    analyst.request(AnalysisRequest { width: 13, ..request() });
    analyst.request(request());
    loop {
        let delivery = updates.recv_blocking().unwrap();
        if !analyst.accepts(&delivery) { continue; }
        let Update::Ready { analysis, elapsed: cached } = delivery.update else { panic!("only a cached redraw"); };
        assert_eq!(analysis.db.width, request().width);
        assert_eq!(cached, elapsed);
        break;
    }
}

#[test]
fn cached_style_changes_preserve_samples_psd_frames_and_duration() {
    let dir = TempDir::new("cached-style");
    let (analyst, updates) = open(capture(&dir), OpenHints::default());
    assert!(matches!(next(&updates), Some(Update::Opened(_, _))));
    let initial = AnalysisRequest { waveform_columns: Some(64), ..request() };
    analyst.request(initial);
    let Some(Update::Ready { analysis: original, elapsed }) = next_result(&updates) else { panic!("ready"); };
    let generation = analyst.mailbox.generation.load(Ordering::Acquire);
    for dynamic_range in [DynamicRange::Fixed(50.0), DynamicRange::Auto, DynamicRange::Default] {
        for colormap in [Colormap::Inferno, Colormap::Grayscale, Colormap::Oceanic] {
            analyst.request(AnalysisRequest { dynamic_range, colormap, ..initial });
            let Some(Update::Ready { analysis, elapsed: cached }) = next(&updates) else { panic!("style must only redraw"); };
            assert_eq!(analyst.mailbox.generation.load(Ordering::Acquire), generation);
            assert_eq!(elapsed, cached);
            assert_eq!(analysis.db.values, original.db.values);
            assert_eq!(analysis.psd.db, original.psd.db);
            assert_eq!(analysis.frames, original.frames);
            assert_eq!(analysis.waveform.as_ref().unwrap().min, original.waveform.as_ref().unwrap().min);
            assert_eq!(analysis.dynamic_range.requested, dynamic_range);
            let expected = argand_dsp::shade(&analysis.db, argand_dsp::Shading {
                colormap, db_min: analysis.spectrogram.db_min, db_max: analysis.spectrogram.db_max,
            });
            assert_eq!(analysis.spectrogram.rgba, expected.rgba);
            if dynamic_range == DynamicRange::Default && colormap == Colormap::Oceanic {
                assert_eq!(analysis.spectrogram.rgba, original.spectrogram.rgba);
            }
        }
    }
}

#[test]
fn changing_style_after_preview_keeps_the_generation_and_finishes_with_latest_style() {
    let dir = TempDir::new("preview-style");
    let (analyst, updates) = open(capture(&dir), OpenHints::default());
    assert!(matches!(next(&updates), Some(Update::Opened(_, _))));
    analyst.request(request());
    loop {
        let delivery = updates.recv_blocking().unwrap();
        if !matches!(delivery.update, Update::Snapshot { .. }) { continue; }
        let generation = analyst.mailbox.generation.load(Ordering::Acquire);
        analyst.request(AnalysisRequest { dynamic_range: DynamicRange::Fixed(40.0), colormap: Colormap::Inferno, ..request() });
        assert_eq!(analyst.mailbox.generation.load(Ordering::Acquire), generation);
        assert!(!analyst.accepts(&delivery));
        break;
    }
    loop {
        let delivery = updates.recv_blocking().unwrap();
        if !analyst.accepts(&delivery) { continue; }
        if let Update::Ready { analysis, .. } = delivery.update {
            assert_eq!(analysis.frames, 61);
            assert_eq!(analysis.dynamic_range.requested, DynamicRange::Fixed(40.0));
            break;
        }
    }
}

#[test]
fn required_delivery_acknowledges_only_a_successful_send_and_cancels_when_superseded() {
    let (sender, receiver) = async_channel::bounded(1);
    let delivery = || Delivery { prepared_at: Instant::now(), generation: Some(1), view_revision: Some(1),
        update: Update::Progress { done: 0, total: 1 } };
    assert!(send_result(&sender, delivery(), &|| Flow::Continue));
    assert!(!send_result(&sender, delivery(), &|| Flow::Stop));
    let pending = std::thread::spawn(move || {
        send_result(&sender, Delivery { prepared_at: Instant::now(), generation: Some(1), view_revision: Some(2),
            update: Update::Progress { done: 1, total: 1 } }, &|| Flow::Continue)
    });
    assert_eq!(receiver.recv_blocking().unwrap().view_revision, Some(1));
    assert_eq!(receiver.recv_blocking().unwrap().view_revision, Some(2));
    assert!(pending.join().unwrap());
}

struct ResizingSource {
    inner: Box<dyn argand_core::SampleSource>,
    analyst: Analyst,
    updates: async_channel::Receiver<Delivery>,
    stage: usize,
}
impl argand_core::SampleSource for ResizingSource {
    fn meta(&self) -> &SignalMeta { self.inner.meta() }
    fn seek(&mut self, sample: u64) -> Result<(), argand_core::SourceError> { self.inner.seek(sample) }
    fn read(&mut self, buf: &mut [f32]) -> Result<usize, argand_core::SourceError> {
        while let Ok(delivery) = self.updates.try_recv() {
            let Update::Snapshot { ref analysis, coverage, .. } = delivery.update else { continue; };
            let current = self.analyst.mailbox.latest().unwrap();
            match self.stage {
                0 => {
                    assert_eq!(coverage.refined_columns, 0);
                    self.analyst.request(AnalysisRequest {
                        colormap: Colormap::Inferno, dynamic_range: DynamicRange::Fixed(40.0),
                        ..current.analysis
                    });
                }
                1 => {
                    assert_eq!(coverage.refined_columns, 0);
                    assert_eq!(analysis.dynamic_range.requested, DynamicRange::Fixed(40.0));
                    assert!(self.analyst.accepts(&delivery));
                    self.analyst.request(AnalysisRequest { width: 17, ..current.analysis });
                    assert!(!self.analyst.accepts(&delivery), "resize invalidates the queued style snapshot");
                }
                2 => {
                    assert!(self.analyst.accepts(&delivery));
                    assert_eq!(coverage.refined_columns, 0, "replacement must arrive during sparse preview");
                    assert_eq!(analysis.db.width, 17);
                    assert_eq!(analysis.dynamic_range.requested, DynamicRange::Fixed(40.0));
                    assert_eq!(delivery.generation, Some(1), "refresh must retain the transform");
                    self.analyst.requests.close();
                }
                _ => panic!("unexpected snapshot"),
            }
            self.stage += 1;
        }
        self.inner.read(buf)
    }
}

#[test]
fn a_queued_style_preview_invalidated_by_resize_is_republished_during_preview() {
    let dir = TempDir::new("preview-style-resize");
    let samples = 2048 * 400;
    let path = write_wav(&dir.join("long.wav"), SampleType::new(Domain::Iq, SampleFormat::F32),
        RATE as u32, &iq_tone(samples, RATE, 6_000.0, 0.5), 1.0);
    let (requests, incoming) = async_channel::bounded(1);
    let (outgoing, updates) = async_channel::bounded(2);
    let mailbox = Arc::new(Mailbox::default());
    let analyst = Analyst { requests, mailbox: mailbox.clone() };
    analyst.request(AnalysisRequest { cfg: StftConfig::new(2048, Window::Hann),
        range: SampleRange::new(0, samples as u64), ..request() });
    let initial = mailbox.latest().unwrap();
    let mut source = ResizingSource { inner: argand_io::open(&path, &OpenHints::default()).unwrap(),
        analyst, updates, stage: 0 };
    let replies = Replies { requests: &incoming, updates: &outgoing, mailbox: &mailbox };
    let result = compute(&mut source, initial, crate::execution::Settings::default(), &replies);
    assert!(matches!(result, Err(DspError::Cancelled)));
    assert_eq!(source.stage, 3);
}

#[test]
fn zoom_publishes_only_a_compact_final_picture_and_keeps_display_cache_semantics() {
    let dir = TempDir::new("compact-zoom");
    let (analyst, updates) = open(capture(&dir), OpenHints::default());
    assert!(matches!(next(&updates), Some(Update::Opened(_, _))));
    let initial = AnalysisRequest { waveform_columns: Some(64), ..request() };
    analyst.request(initial);
    assert!(matches!(next_result(&updates), Some(Update::Ready { .. })));
    let zoom = AnalysisRequest { range: SampleRange::new(700, 256), ..initial };
    analyst.request(zoom);
    let elapsed = loop {
        let delivery = updates.recv_blocking().unwrap();
        if !analyst.accepts(&delivery) { continue; }
        match delivery.update {
            Update::Progress { .. } => {},
            Update::Ready { analysis, elapsed } => {
                assert_eq!(analysis.frames, 1);
                assert_eq!(analysis.spectrogram.width, 1);
                assert_eq!(analysis.waveform.unwrap().columns, 64);
                break elapsed;
            }
            _ => panic!("zoom must keep the held picture until the final result"),
        }
    };
    let generation = analyst.mailbox.latest().unwrap().generation;
    analyst.request(AnalysisRequest { width: 120, height: 44, waveform_columns: Some(120),
        colormap: Colormap::Inferno, ..zoom });
    assert_eq!(analyst.mailbox.latest().unwrap().generation, generation);
    let Some(Update::Ready { analysis, elapsed: resized }) = next_result(&updates) else { panic!("ready") };
    assert_eq!(resized, elapsed);
    assert_eq!(analysis.spectrogram.width, 1);
    assert_eq!(analysis.spectrogram.height, 44);
    assert_eq!(analysis.waveform.unwrap().columns, 120);
}

#[test]
fn unknown_length_flac_finishes_preview_and_refinement() {
    let dir = TempDir::new("flac-preview-refinement");
    let path = dir.join("levels.flac");
    let mut data = include_bytes!("../../io/tests/fixtures/levels.flac").to_vec();
    data[21] &= 0xf0;
    data[22..26].fill(0);
    std::fs::write(&path, data).unwrap();
    let (analyst, updates) = open(path, OpenHints::default());
    let Some(Update::Opened(meta, _)) = next(&updates) else { panic!("file must open") };
    assert_eq!(meta.len_samples, 16384);
    let mut request = request();
    request.range = SampleRange::new(0, meta.len_samples);
    request.cfg = StftConfig::new(2048, Window::Hann);
    assert!(analyst.request(request));
    match next_result(&updates) {
        Some(Update::Ready { analysis, .. }) => assert_eq!(analysis.frames, 29),
        Some(Update::Failed(error)) => panic!("{error:#}"),
        _ => panic!("analysis must finish"),
    }
}
