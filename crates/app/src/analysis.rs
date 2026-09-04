//! The thread that owns one file's samples.
//!
//! [`argand_dsp::analyze`] is blocking and reads the whole capture, which on a
//! long one takes as long as it takes. So it runs on a thread of its own, and
//! that thread owns the [`SampleSource`] outright: nothing on the window's side
//! holds a reference to it, and no frame can be drawn on a thread that is also
//! transforming.
//!
//! Opening is on the same thread for the same reason. A header probe looks
//! cheap, but a capture opened with `--normalize auto` is scanned for its peak
//! before the first sample reaches a transform, and that is a pass over the
//! file.
//!
//! Nothing here knows about a toolkit. The queue between the two threads is an
//! `async_channel`, whose receiver a GPUI task can await and whose sender a
//! plain thread can push to, so this module needs neither an executor nor a
//! window to be run or tested.
//!
//! [`SampleSource`]: argand_core::SampleSource

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use argand_core::{SampleSource, SignalMeta, SourceError};
use argand_dsp::{Analysis, AnalysisRequest, analyze};
use argand_io::OpenHints;

/// Shortest gap between two progress updates.
///
/// `analyze` reports once per block read, which on a megahertz capture is
/// thousands of times a second. Nothing on screen can show that, and a channel
/// carrying it would spend more time on the progress than on the picture.
const PROGRESS_INTERVAL: Duration = Duration::from_millis(100);

/// The thread's name, as a debugger and `top` show it.
///
/// Fifteen bytes, which is what Linux accepts before it drops the name.
const THREAD_NAME: &str = "argand-analysis";

/// What the analysis thread says back.
///
/// The sequence is `Opened` once, then any number of `Progress` runs each
/// ending in `Ready` or `Failed`. A `Failed` before `Opened` is the file
/// refusing to open at all, and the thread stops after it; one after `Opened`
/// is a request the transform would not run, and another request may still
/// succeed.
pub enum Update {
    /// The file opened, and this is what it says about itself.
    Opened(SignalMeta),
    /// Samples read of samples to read, for the request in flight.
    Progress { done: u64, total: u64 },
    /// A finished analysis for the last request.
    ///
    /// Boxed because it is far the largest thing this enum carries, and every
    /// other variant would otherwise be moved around at its size.
    Ready(Box<Analysis>),
    /// Nothing came of a request, and this is why.
    Failed(anyhow::Error),
}

/// The window's end of one file's analysis thread.
///
/// Dropping it closes the request channel, which an idle thread reads as the
/// end of its work. A thread in the middle of a transform is stopped by
/// [`Stopping`] instead, and neither is joined: what a caller waits for here
/// it waits for on the thread drawing the window.
pub struct Analyst {
    requests: async_channel::Sender<AnalysisRequest>,
}

/// Open `path` on a new thread, and hand back the two ends of the queue.
///
/// The receiver is separate from the [`Analyst`] rather than reachable through
/// it, because the two are what stops the thread and each has to be able to do
/// it alone: dropping the receiver tells a thread mid-analysis that nobody is
/// waiting for the answer, and dropping the [`Analyst`] tells an idle one that
/// no request is coming.
pub fn open(path: PathBuf, hints: OpenHints) -> (Analyst, async_channel::Receiver<Update>) {
    let (requests, incoming) = async_channel::unbounded();
    let (outgoing, updates) = async_channel::unbounded();

    let spawned = std::thread::Builder::new()
        .name(THREAD_NAME.to_owned())
        .spawn({
            let outgoing = outgoing.clone();
            move || serve(&path, &hints, &incoming, &outgoing)
        });

    // A thread that will not start is reported like a file that will not open:
    // the window says so and stays usable. The channel is unbounded, so this
    // cannot block the caller.
    if let Err(error) = spawned {
        let _ = outgoing.try_send(Update::Failed(
            anyhow::Error::new(error).context("starting the analysis thread"),
        ));
    }

    (Analyst { requests }, updates)
}

impl Analyst {
    /// Ask for an analysis, replacing whatever has not been started yet.
    ///
    /// `false` once the thread has gone, which is the caller's cue to stop
    /// expecting updates.
    pub fn request(&self, request: AnalysisRequest) -> bool {
        // Unbounded, so this never blocks the thread drawing the window. What
        // queues up here is bounded by how fast a person can resize a window,
        // and `newest` throws away everything but the last of it.
        self.requests.try_send(request).is_ok()
    }
}

/// Open the file, then answer requests until nobody is asking.
fn serve(
    path: &Path,
    hints: &OpenHints,
    requests: &async_channel::Receiver<AnalysisRequest>,
    updates: &async_channel::Sender<Update>,
) {
    let mut source = match argand_io::open(path, hints) {
        Ok(source) => source,
        Err(error) => {
            let _ = updates.send_blocking(Update::Failed(anyhow::Error::new(error)));
            return;
        }
    };
    if updates
        .send_blocking(Update::Opened(source.meta().clone()))
        .is_err()
    {
        return;
    }

    while let Ok(request) = requests.recv_blocking() {
        let request = newest(request, requests);
        let update = match run(source.as_mut(), &request, updates) {
            Ok(analysis) => Update::Ready(Box::new(analysis)),
            Err(error) => Update::Failed(error),
        };
        if updates.send_blocking(update).is_err() {
            return;
        }
    }
}

/// A source that reports the end of the signal once nobody is waiting.
///
/// [`analyze`] is one blocking call with no way to interrupt it, so a document
/// closed halfway through a half-hour capture would otherwise hold a thread, a
/// core and a mapping until the transform finished on its own -- and opening
/// several large files in a row would leave one such thread behind for each.
///
/// A read answered with "no more samples" is the one lever the transform does
/// expose: it closes over what it has and returns within a block. What it
/// returns goes nowhere, because the channel that would have carried it is the
/// very thing whose closing stopped it.
///
/// This does not reach the level scan a capture opened with `--normalize auto`
/// runs before the first transform: that happens inside `argand_io::open`,
/// before there is a source to wrap.
struct Stopping<'a> {
    inner: &'a mut dyn SampleSource,
    updates: &'a async_channel::Sender<Update>,
}

impl SampleSource for Stopping<'_> {
    fn meta(&self) -> &SignalMeta {
        self.inner.meta()
    }

    fn seek(&mut self, sample: u64) -> Result<(), SourceError> {
        self.inner.seek(sample)
    }

    fn read(&mut self, buf: &mut [f32]) -> Result<usize, SourceError> {
        if self.updates.is_closed() {
            return Ok(0);
        }
        self.inner.read(buf)
    }
}

/// One transform, reporting progress no oftener than the window can use it.
fn run(
    source: &mut dyn SampleSource,
    request: &AnalysisRequest,
    updates: &async_channel::Sender<Update>,
) -> Result<Analysis, anyhow::Error> {
    // `None` rather than a moment one interval ago: subtracting from a clock
    // that starts at the machine's boot is not guaranteed to have anywhere to
    // go, and the first report should be sent anyway.
    let mut last: Option<Instant> = None;
    let mut source = Stopping {
        inner: source,
        updates,
    };
    analyze(&mut source, request, &mut |done, total| {
        let now = Instant::now();
        if last.is_some_and(|last| now.duration_since(last) < PROGRESS_INTERVAL) {
            return;
        }
        last = Some(now);
        let _ = updates.try_send(Update::Progress { done, total });
    })
    .map_err(anyhow::Error::new)
}

/// The last request offered, discarding those overtaken while a transform ran.
///
/// A window being resized offers one request per step, and every one but the
/// last describes a picture nobody is waiting for any more. Running them all
/// would put the window minutes behind a drag that took a second.
fn newest(
    first: AnalysisRequest,
    queued: &async_channel::Receiver<AnalysisRequest>,
) -> AnalysisRequest {
    let mut latest = first;
    while let Ok(next) = queued.try_recv() {
        latest = next;
    }
    latest
}

#[cfg(test)]
mod tests {
    include!("analysis_tests.rs");
}
