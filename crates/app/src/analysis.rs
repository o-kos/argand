//! One background worker owns opening, preview and refinement for a document.
//! Replies are bounded and tagged so a superseded analysis cannot replace the view.

use std::path::{Path, PathBuf};
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use std::time::{Duration, Instant};

use argand_core::SignalMeta;
use argand_dsp::{
    Analysis, AnalysisRequest, Coverage, DspError, Flow, ProgressiveOptions,
    analyze_progressive_with_options,
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
    pub update: Update,
}

struct Requested {
    generation: u64,
    analysis: AnalysisRequest,
}

pub struct Analyst {
    requests: async_channel::Sender<Requested>,
    generation: Arc<AtomicU64>,
}

pub fn prepare(
    path: PathBuf,
    mut hints: OpenHints,
    settings: crate::execution::Settings,
) -> (Analyst, async_channel::Receiver<Delivery>, Start) {
    hints.level_scan_bytes = Some(LEVEL_SCAN_BYTES);
    let (start, started) = async_channel::bounded(1);
    let (requests, incoming) = async_channel::unbounded();
    let (outgoing, updates) = async_channel::bounded(2);
    let generation = Arc::new(AtomicU64::new(0));
    let spawned = std::thread::Builder::new()
        .name(THREAD_NAME.to_owned())
        .spawn({
            let outgoing = outgoing.clone();
            let generation = generation.clone();
            move || {
                worker(
                    &path,
                    &hints,
                    settings,
                    &incoming,
                    &outgoing,
                    &generation,
                    &started,
                )
            }
        });
    if let Err(error) = spawned {
        let _ = outgoing.try_send(Delivery {
            prepared_at: Instant::now(),
            generation: None,
            update: Update::Failed(
                anyhow::Error::new(error).context("starting the analysis thread"),
            ),
        });
    }
    (
        Analyst {
            requests,
            generation,
        },
        updates,
        Start(start),
    )
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
        let generation = self.generation.fetch_add(1, Ordering::AcqRel) + 1;
        self.requests
            .try_send(Requested {
                generation,
                analysis,
            })
            .is_ok()
    }

    pub fn accepts(&self, delivery: &Delivery) -> bool {
        delivery
            .generation
            .is_none_or(|id| id == self.generation.load(Ordering::Acquire))
    }
}

fn worker(
    path: &Path,
    hints: &OpenHints,
    settings: crate::execution::Settings,
    requests: &async_channel::Receiver<Requested>,
    updates: &async_channel::Sender<Delivery>,
    generation: &AtomicU64,
    started: &async_channel::Receiver<()>,
) {
    let pool = match settings.pool() {
        Ok(pool) => pool,
        Err(error) => {
            let _ = updates.try_send(Delivery {
                prepared_at: Instant::now(),
                generation: None,
                update: Update::Failed(error.into()),
            });
            return;
        }
    };
    if started.recv_blocking().is_err() || requests.is_closed() {
        return;
    }
    pool.install(|| serve(path, hints, settings, requests, updates, generation));
}

fn serve(
    path: &Path,
    hints: &OpenHints,
    settings: crate::execution::Settings,
    requests: &async_channel::Receiver<Requested>,
    updates: &async_channel::Sender<Delivery>,
    generation: &AtomicU64,
) {
    let opening_started = Instant::now();
    let mut source = match argand_io::open(path, hints) {
        Ok(source) => source,
        Err(error) => {
            let _ = updates.try_send(Delivery {
                prepared_at: Instant::now(),
                generation: None,
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
            update: Update::Opened(source.meta().clone()),
        })
        .is_err()
    {
        return;
    }
    while let Ok(request) = requests.recv_blocking() {
        let request = newest(request, requests);
        let control = || {
            if updates.is_closed()
                || requests.is_closed()
                || generation.load(Ordering::Acquire) != request.generation
            {
                Flow::Stop
            } else {
                Flow::Continue
            }
        };
        let started = Instant::now();
        let _ = updates.try_send(Delivery {
            prepared_at: Instant::now(),
            generation: Some(request.generation),
            update: Update::Progress {
                done: 0,
                total: request.analysis.range.len,
            },
        });
        let mut waveform_peak = None;
        let result = analyze_progressive_with_options(
            source.as_mut(),
            &request.analysis,
            ProgressiveOptions::new(settings.batch_frames).unwrap_or_default(),
            &control,
            &mut |analysis, coverage| {
                if control() == Flow::Stop {
                    return Flow::Stop;
                }
                tracing::debug!(?coverage, elapsed = ?started.elapsed(), "analysis snapshot");
                let delivery = Delivery {
                    prepared_at: Instant::now(),
                    generation: Some(request.generation),
                    update: Update::Snapshot {
                        waveform_peak: *waveform_peak.get_or_insert(analysis.time_peak),
                        analysis: Box::new(analysis),
                        coverage,
                    },
                };
                if coverage.refined_columns == 0 {
                    send_result(updates, delivery, &control);
                } else {
                    let _ = updates.try_send(delivery);
                }
                Flow::Continue
            },
        );
        let update = match result {
            Ok(analysis) => {
                let elapsed = started.elapsed();
                tracing::debug!(?elapsed, "analysis completed");
                Update::Ready {
                    analysis: Box::new(analysis),
                    elapsed,
                }
            }
            Err(DspError::Cancelled) => continue,
            Err(error) => Update::Failed(error.into()),
        };
        send_result(
            updates,
            Delivery {
                prepared_at: Instant::now(),
                generation: Some(request.generation),
                update,
            },
            &control,
        );
    }
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

fn newest<T>(first: T, queued: &async_channel::Receiver<T>) -> T {
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
