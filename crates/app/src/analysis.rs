//! One background worker owns opening, preview and refinement for a document.
//! Replies are bounded and tagged so a superseded analysis cannot replace the view.

use std::path::{Path, PathBuf};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU64, Ordering},
};
use std::time::{Duration, Instant};

use argand_core::SignalMeta;
use argand_dsp::{
    Analysis, AnalysisRequest, Coverage, DspError, Flow, Overview, ProgressiveOptions,
    analyze_overview_with_refresh,
};
use argand_io::OpenHints;

#[path = "analysis_progress.rs"]
mod progress;

const THREAD_NAME: &str = "argand-analysis";
const LEVEL_SCAN_BYTES: usize = 64 << 20;

#[derive(Default)]
pub struct FileInfo {
    pub bytes: Option<u64>,
    pub sample_units: Option<(f64, f64)>,
    /// The file as it was opened, absent when it changed while it was being opened.
    pub stamp: Option<argand_io::write::SourceStamp>,
}

pub enum Update {
    Opened(SignalMeta, FileInfo),
    Progress {
        done: u64,
        total: u64,
    },
    Snapshot {
        analysis: Box<Analysis>,
        coverage: Coverage,
    },
    Ready {
        analysis: Box<Analysis>,
        elapsed: Duration,
    },
    Failed(anyhow::Error),
}

/// Build a delivery with a chosen generation and view revision for a test.
/// The fields are private so that production code cannot forge them.
#[cfg(test)]
pub(crate) fn for_test(
    update: Update,
    generation: Option<u64>,
    view_revision: Option<u64>,
) -> Delivery {
    Delivery {
        prepared_at: Instant::now(),
        generation,
        view_revision,
        update,
    }
}

pub struct Delivery {
    pub prepared_at: Instant,
    generation: Option<u64>,
    view_revision: Option<u64>,
    pub update: Update,
}

#[derive(Clone, Copy)]
struct Requested {
    generation: u64,
    view_revision: u64,
    /// The version of the edits this picture is of.
    edit: u64,
    analysis: AnalysisRequest,
    /// Replaces a picture already shown, so no preview or partial picture is published.
    replacement: bool,
    frequency: Option<(f64, f64)>,
}

pub struct Analyst {
    requests: async_channel::Sender<()>,
    mailbox: Arc<Mailbox>,
}

#[derive(Default)]
struct Mailbox {
    latest: Mutex<Option<Requested>>,
    generation: AtomicU64,
    view_revision: AtomicU64,
    edit: Mutex<Option<Arc<EditState>>>,
    /// Set for a worker started under a picture already shown, whose first answer must not preview.
    keep_picture: std::sync::atomic::AtomicBool,
}

impl Mailbox {
    fn latest(&self) -> Option<Requested> {
        *self
            .latest
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn edit(&self) -> Option<Arc<EditState>> {
        self.edit
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }
}

/// An edited capture and how to open each source it reads beyond the file itself.
pub struct EditState {
    pub version: u64,
    pub capture: argand_edit::Capture,
    /// Indexed by source id, the file itself being source 0 and left `None`.
    pub sources: Vec<Option<(SignalMeta, OpenHints)>>,
}

fn same_analysis(a: AnalysisRequest, b: AnalysisRequest) -> bool {
    let waveform = |r: AnalysisRequest| r.waveform_columns.is_some_and(|n| n > 0);
    waveform(a) == waveform(b)
        && AnalysisRequest {
            width: b.width,
            height: b.height,
            waveform_columns: b.waveform_columns,
            colormap: b.colormap,
            dynamic_range: b.dynamic_range,
            ..a
        } == b
}

pub fn prepare(
    path: PathBuf,
    mut hints: OpenHints,
    settings: crate::execution::Settings,
    lease: crate::release::Lease,
) -> (Analyst, async_channel::Receiver<Delivery>, Start) {
    hints.level_scan_bytes = Some(LEVEL_SCAN_BYTES);
    let (start, started) = async_channel::bounded(1);
    let (requests, incoming) = async_channel::bounded(1);
    let (outgoing, updates) = async_channel::bounded(2);
    let mailbox = Arc::new(Mailbox::default());
    let spawned = std::thread::Builder::new()
        .name(THREAD_NAME.to_owned())
        .spawn({
            let outgoing = outgoing.clone();
            let mailbox = mailbox.clone();
            move || {
                // Held until the thread ends, which is when the file is let go.
                let _lease = lease;
                worker(
                    &path, &hints, settings, &incoming, &outgoing, &mailbox, &started,
                )
            }
        });
    if let Err(error) = spawned {
        let _ = outgoing.try_send(Delivery {
            prepared_at: Instant::now(),
            generation: None,
            view_revision: None,
            update: Update::Failed(
                anyhow::Error::new(error).context("starting the analysis thread"),
            ),
        });
    }
    (Analyst { requests, mailbox }, updates, Start(start))
}

/// Release file opening only after the window has painted its initial frame.
pub struct Start(async_channel::Sender<()>);

impl Start {
    pub fn start(self) {
        let _ = self.0.try_send(());
    }
}

#[cfg(test)]
fn open(path: PathBuf, hints: OpenHints) -> (Analyst, async_channel::Receiver<Delivery>) {
    let (lease, _) = crate::release::lease();
    let (analyst, updates, start) =
        prepare(path, hints, crate::execution::Settings::default(), lease);
    start.start();
    (analyst, updates)
}

impl Analyst {
    #[cfg(test)]
    pub fn request(&self, analysis: AnalysisRequest) -> bool {
        self.request_view(analysis, None)
    }

    pub fn request_view(&self, analysis: AnalysisRequest, frequency: Option<(f64, f64)>) -> bool {
        let mut latest = self
            .mailbox
            .latest
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let edit = self.mailbox.edit().map_or(0, |state| state.version);
        if latest.is_some_and(|previous| {
            previous.analysis == analysis
                && previous.frequency == frequency
                && previous.edit == edit
        }) {
            return !self.requests.is_closed();
        }
        // Only a file's first analysis has no picture to keep, so every later one is delivered whole.
        // A new edit version has no picture to keep, so it previews as a first analysis does.
        let replacement = match *latest {
            Some(previous) => {
                previous.edit == edit
                    && (previous.replacement || !same_analysis(previous.analysis, analysis))
            }
            None => self.mailbox.keep_picture.load(Ordering::Acquire),
        };
        let generation = match *latest {
            Some(previous)
                if previous.edit == edit && same_analysis(previous.analysis, analysis) =>
            {
                previous.generation
            }
            _ => self.mailbox.generation.fetch_add(1, Ordering::AcqRel) + 1,
        };
        let view_revision = self.mailbox.view_revision.fetch_add(1, Ordering::AcqRel) + 1;
        *latest = Some(Requested {
            generation,
            view_revision,
            edit,
            analysis,
            replacement,
            frequency,
        });
        drop(latest);
        match self.requests.try_send(()) {
            Ok(()) | Err(async_channel::TrySendError::Full(())) => true,
            Err(async_channel::TrySendError::Closed(())) => false,
        }
    }

    /// Deliver even the first picture whole, the window already showing one of the same samples.
    pub fn keep_picture(&self) {
        self.mailbox.keep_picture.store(true, Ordering::Release);
    }

    /// Make `state` what every later request is analysed from.
    pub fn set_edit(&self, state: EditState) {
        *self
            .mailbox
            .edit
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(Arc::new(state));
    }

    pub fn accepts(&self, delivery: &Delivery) -> bool {
        delivery
            .generation
            .is_none_or(|id| id == self.mailbox.generation.load(Ordering::Acquire))
            && delivery
                .view_revision
                .is_none_or(|id| id == self.mailbox.view_revision.load(Ordering::Acquire))
    }
}

fn worker(
    path: &Path,
    hints: &OpenHints,
    settings: crate::execution::Settings,
    requests: &async_channel::Receiver<()>,
    updates: &async_channel::Sender<Delivery>,
    mailbox: &Mailbox,
    started: &async_channel::Receiver<()>,
) {
    let pool = match settings.pool() {
        Ok(pool) => pool,
        Err(error) => {
            let _ = updates.try_send(Delivery {
                prepared_at: Instant::now(),
                generation: None,
                view_revision: None,
                update: Update::Failed(error.into()),
            });
            return;
        }
    };
    if started.recv_blocking().is_err() || requests.is_closed() {
        return;
    }
    pool.install(|| serve(path, hints, settings, requests, updates, mailbox));
}

fn serve(
    path: &Path,
    hints: &OpenHints,
    settings: crate::execution::Settings,
    requests: &async_channel::Receiver<()>,
    updates: &async_channel::Sender<Delivery>,
    mailbox: &Mailbox,
) {
    let opening_started = Instant::now();
    let (source, stamp) = match argand_io::open_stamped(path, hints) {
        Ok(opened) => opened,
        Err(error) => {
            let _ = updates.try_send(Delivery {
                prepared_at: Instant::now(),
                generation: None,
                view_revision: None,
                update: Update::Failed(error.into()),
            });
            return;
        }
    };
    tracing::debug!(elapsed = ?opening_started.elapsed(), "file opened");
    if updates
        .try_send(Delivery {
            prepared_at: Instant::now(),
            generation: None,
            view_revision: None,
            update: Update::Opened(
                source.meta().clone(),
                FileInfo {
                    bytes: std::fs::metadata(path).ok().map(|meta| meta.len()),
                    sample_units: source.original_sample_units(),
                    stamp,
                },
            ),
        })
        .is_err()
    {
        return;
    }
    let replies = Replies {
        requests,
        updates,
        mailbox,
    };
    let meta = source.meta().clone();
    let whole = argand_edit::Capture::whole(argand_edit::SourceId(0), meta.len_samples);
    let mut source = match argand_edit::EditedSource::new(whole, vec![Some(source)], meta) {
        Ok(source) => source,
        Err(error) => {
            replies.send(0, Update::Failed(error.into()));
            return;
        }
    };
    let mut edit = 0;
    let mut cached: Option<Cached> = None;
    while requests.recv_blocking().is_ok() {
        let Some(request) = mailbox.latest() else {
            continue;
        };
        if request.edit != edit {
            if let Err(error) = follow_edit(&mut source, mailbox, request.edit) {
                replies.send(request.generation, Update::Failed(error));
                continue;
            }
            edit = request.edit;
        }
        if !cached
            .as_ref()
            .is_some_and(|cache| cache.generation == request.generation)
        {
            cached = None;
            match compute(&mut source, request, settings, &replies) {
                Ok(cache) => cached = Some(cache),
                Err(DspError::Cancelled) => continue,
                Err(error) => {
                    replies.send(request.generation, Update::Failed(error.into()));
                    continue;
                }
            }
        }
        if let Some(cache) = &mut cached {
            cache.deliver(&replies);
        }
    }
}

/// Bring `source` to the edits the mailbox holds, opening the sources they newly read.
fn follow_edit(
    source: &mut argand_edit::EditedSource,
    mailbox: &Mailbox,
    version: u64,
) -> anyhow::Result<()> {
    let Some(state) = mailbox.edit().filter(|state| state.version == version) else {
        anyhow::bail!("the edits this picture was asked for are gone");
    };
    for id in state.capture.sources() {
        if source.has_source(id) {
            continue;
        }
        let Some(Some((meta, hints))) = state.sources.get(id.0 as usize) else {
            anyhow::bail!("source {} of the edited capture cannot be opened", id.0);
        };
        source.insert_source(id, argand_io::reopen(meta, hints)?);
    }
    source.set_capture(state.capture.clone())?;
    Ok(())
}

struct Replies<'a> {
    requests: &'a async_channel::Receiver<()>,
    updates: &'a async_channel::Sender<Delivery>,
    mailbox: &'a Mailbox,
}

impl Replies<'_> {
    fn control(&self, generation: u64) -> Flow {
        if self.updates.is_closed()
            || self.requests.is_closed()
            || self.mailbox.generation.load(Ordering::Acquire) != generation
        {
            Flow::Stop
        } else {
            Flow::Continue
        }
    }

    fn view_control(&self, request: Requested) -> Flow {
        if self.mailbox.view_revision.load(Ordering::Acquire) != request.view_revision {
            Flow::Stop
        } else {
            self.control(request.generation)
        }
    }

    fn send_view(&self, request: Requested, update: Update) {
        send_result(
            self.updates,
            Delivery {
                prepared_at: Instant::now(),
                generation: Some(request.generation),
                view_revision: Some(request.view_revision),
                update,
            },
            &|| self.view_control(request),
        );
    }

    fn send(&self, generation: u64, update: Update) {
        send_result(
            self.updates,
            Delivery {
                prepared_at: Instant::now(),
                generation: Some(generation),
                view_revision: None,
                update,
            },
            &|| self.control(generation),
        );
    }
}

fn render_view(
    overview: &mut Overview,
    width: usize,
    view: AnalysisRequest,
    frequency: Option<(f64, f64)>,
) -> Result<Analysis, DspError> {
    match frequency {
        Some(band) => overview.render_band(width, view.height, view.waveform_columns, band),
        None => overview.render(width, view.height, view.waveform_columns),
    }
}

struct Cached {
    overview: Overview,
    generation: u64,
    started: Instant,
    elapsed: Option<Duration>,
    last_view: Option<u64>,
}

impl Cached {
    fn deliver(&mut self, replies: &Replies<'_>) {
        let Some(request) = replies.mailbox.latest() else {
            return;
        };
        if request.generation != self.generation || self.last_view == Some(request.view_revision) {
            return;
        }
        let started = Instant::now();
        let view = request.analysis;
        let width = if request.replacement && self.overview.dimensions().0 == 1 {
            1
        } else {
            view.width
        };
        let rendered = self
            .overview
            .set_style(view.colormap, view.dynamic_range)
            .and_then(|()| render_view(&mut self.overview, width, view, request.frequency));
        let update = match rendered {
            Ok(analysis) => {
                let elapsed = *self.elapsed.get_or_insert_with(|| {
                    let elapsed = self.started.elapsed();
                    tracing::debug!(?elapsed, "analysis completed");
                    elapsed
                });
                tracing::debug!(elapsed = ?started.elapsed(), width = view.width, height = view.height,
                    frames = analysis.frames, "cached view prepared");
                Update::Ready {
                    analysis: Box::new(analysis),
                    elapsed,
                }
            }
            Err(error) => Update::Failed(error.into()),
        };
        self.last_view = Some(request.view_revision);
        replies.send_view(request, update);
    }
}

fn compute(
    source: &mut dyn argand_core::SampleSource,
    request: Requested,
    settings: crate::execution::Settings,
    replies: &Replies<'_>,
) -> Result<Cached, DspError> {
    let started = Instant::now();
    replies.send(
        request.generation,
        Update::Progress {
            done: 0,
            total: request.analysis.range.len,
        },
    );
    let mut render_error = None;
    let last_view = std::cell::Cell::new(None);
    let options = ProgressiveOptions::new(settings.batch_frames).unwrap_or_default();
    let options = if request.replacement {
        options.final_only()
    } else {
        options
    };
    let mut source = progress::Source::new(source, request, replies);
    let result = analyze_overview_with_refresh(
        &mut source,
        &request.analysis,
        options,
        &|| replies.control(request.generation),
        &|| {
            replies
                .mailbox
                .latest()
                .is_some_and(|latest| last_view.get() != Some(latest.view_revision))
        },
        &mut |overview, coverage| {
            let Some(latest) = replies.mailbox.latest() else {
                return Flow::Stop;
            };
            if replies.control(request.generation) == Flow::Stop {
                return Flow::Stop;
            }
            let view = latest.analysis;
            let rendered = overview
                .set_style(view.colormap, view.dynamic_range)
                .and_then(|()| render_view(overview, view.width, view, latest.frequency));
            let analysis = match rendered {
                Ok(analysis) => analysis,
                Err(error) => {
                    render_error = Some(error);
                    return Flow::Stop;
                }
            };
            if replies.view_control(latest) == Flow::Stop {
                return replies.control(request.generation);
            }
            let view_changed = last_view.get() != Some(latest.view_revision);
            tracing::debug!(?coverage, elapsed = ?started.elapsed(), "analysis snapshot");
            let delivery = Delivery {
                prepared_at: Instant::now(),
                generation: Some(request.generation),
                view_revision: Some(latest.view_revision),
                update: Update::Snapshot {
                    analysis: Box::new(analysis),
                    coverage,
                },
            };
            let delivered = if coverage.refined_columns == 0 || view_changed {
                send_result(replies.updates, delivery, &|| replies.view_control(latest))
            } else {
                replies.updates.try_send(delivery).is_ok()
            };
            if delivered {
                last_view.set(Some(latest.view_revision));
            }
            Flow::Continue
        },
    );
    if let Some(error) = render_error {
        return Err(error);
    }
    let overview = result?;
    tracing::debug!(dimensions = ?overview.dimensions(), "overview retained");
    Ok(Cached {
        overview,
        generation: request.generation,
        started,
        elapsed: None,
        last_view: None,
    })
}

/// A final result must arrive, but waiting for GUI capacity must remain cancellable.
fn send_result(
    updates: &async_channel::Sender<Delivery>,
    mut delivery: Delivery,
    control: &dyn Fn() -> Flow,
) -> bool {
    while control() == Flow::Continue {
        match updates.try_send(delivery) {
            Ok(()) => return true,
            Err(async_channel::TrySendError::Closed(_)) => return false,
            Err(async_channel::TrySendError::Full(pending)) => delivery = pending,
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    false
}

#[cfg(test)]
mod tests {
    include!("analysis_tests.rs");
}
