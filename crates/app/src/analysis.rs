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
    analyze_overview,
};
use argand_io::OpenHints;

const THREAD_NAME: &str = "argand-analysis";
const LEVEL_SCAN_BYTES: usize = 64 << 20;

pub enum Update {
    Opened(SignalMeta),
    Progress {
        done: u64,
        total: u64,
    },
    Snapshot {
        analysis: Box<Analysis>,
        coverage: Coverage,
        waveform_peak: f32,
    },
    Ready {
        analysis: Box<Analysis>,
        elapsed: Duration,
    },
    Failed(anyhow::Error),
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
    analysis: AnalysisRequest,
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
}

impl Mailbox {
    fn latest(&self) -> Option<Requested> {
        *self
            .latest
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

fn same_analysis(a: AnalysisRequest, b: AnalysisRequest) -> bool {
    let waveform = |r: AnalysisRequest| r.waveform_columns.is_some_and(|n| n > 0);
    waveform(a) == waveform(b)
        && AnalysisRequest {
            width: b.width,
            height: b.height,
            waveform_columns: b.waveform_columns,
            ..a
        } == b
}

pub fn prepare(
    path: PathBuf,
    mut hints: OpenHints,
    settings: crate::execution::Settings,
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
    let (analyst, updates, start) = prepare(path, hints, crate::execution::Settings::default());
    start.start();
    (analyst, updates)
}

impl Analyst {
    pub fn request(&self, analysis: AnalysisRequest) -> bool {
        let mut latest = self
            .mailbox
            .latest
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if latest.is_some_and(|previous| previous.analysis == analysis) {
            return !self.requests.is_closed();
        }
        let generation = match *latest {
            Some(previous) if same_analysis(previous.analysis, analysis) => previous.generation,
            _ => self.mailbox.generation.fetch_add(1, Ordering::AcqRel) + 1,
        };
        let view_revision = self.mailbox.view_revision.fetch_add(1, Ordering::AcqRel) + 1;
        *latest = Some(Requested {
            generation,
            view_revision,
            analysis,
        });
        drop(latest);
        match self.requests.try_send(()) {
            Ok(()) | Err(async_channel::TrySendError::Full(())) => true,
            Err(async_channel::TrySendError::Closed(())) => false,
        }
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
    let mut source = match argand_io::open(path, hints) {
        Ok(source) => source,
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
            update: Update::Opened(source.meta().clone()),
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
    let mut cached: Option<Cached> = None;
    while requests.recv_blocking().is_ok() {
        let Some(request) = mailbox.latest() else {
            continue;
        };
        if !cached
            .as_ref()
            .is_some_and(|cache| cache.generation == request.generation)
        {
            cached = None;
            match compute(source.as_mut(), request, settings, &replies) {
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
        let update = match self
            .overview
            .render(view.width, view.height, view.waveform_columns)
        {
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
    let mut waveform_peak = None;
    let mut render_error = None;
    let result = analyze_overview(
        source,
        &request.analysis,
        ProgressiveOptions::new(settings.batch_frames).unwrap_or_default(),
        &|| replies.control(request.generation),
        &mut |overview, coverage| {
            let Some(latest) = replies.mailbox.latest() else {
                return Flow::Stop;
            };
            if replies.control(request.generation) == Flow::Stop {
                return Flow::Stop;
            }
            let view = latest.analysis;
            let analysis = match overview.render(view.width, view.height, view.waveform_columns) {
                Ok(analysis) => analysis,
                Err(error) => {
                    render_error = Some(error);
                    return Flow::Stop;
                }
            };
            if replies.view_control(latest) == Flow::Stop {
                return replies.control(request.generation);
            }
            tracing::debug!(?coverage, elapsed = ?started.elapsed(), "analysis snapshot");
            let delivery = Delivery {
                prepared_at: Instant::now(),
                generation: Some(request.generation),
                view_revision: Some(latest.view_revision),
                update: Update::Snapshot {
                    waveform_peak: *waveform_peak.get_or_insert(analysis.time_peak),
                    analysis: Box::new(analysis),
                    coverage,
                },
            };
            if coverage.refined_columns == 0 {
                send_result(replies.updates, delivery, &|| replies.view_control(latest));
            } else {
                let _ = replies.updates.try_send(delivery);
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
) {
    while control() == Flow::Continue {
        match updates.try_send(delivery) {
            Ok(()) | Err(async_channel::TrySendError::Closed(_)) => return,
            Err(async_channel::TrySendError::Full(pending)) => delivery = pending,
        }
        std::thread::sleep(Duration::from_millis(1));
    }
}

#[cfg(test)]
mod tests {
    include!("analysis_tests.rs");
}
