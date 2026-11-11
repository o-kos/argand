//! Per-document compute policy. Discovery and affinity never run on the UI thread.

use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Affinity {
    #[default]
    None,
    Efficiency,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Settings {
    /// Zero selects the conservative automatic worker budget.
    pub workers: usize,
    pub batch_frames: usize,
    pub affinity: Affinity,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            workers: 0,
            batch_frames: 1024,
            affinity: Affinity::None,
        }
    }
}

impl Settings {
    pub fn repair(&mut self) {
        if self.workers > 128 {
            tracing::warn!(
                found = self.workers,
                "analysis workers must be at most 128; using automatic budget"
            );
            self.workers = 0;
        }
        if !(1..=4096).contains(&self.batch_frames) {
            tracing::warn!(
                found = self.batch_frames,
                "analysis batch_frames must be from 1 to 4096; using default"
            );
            self.batch_frames = Self::default().batch_frames;
        }
    }

    pub fn pool(self) -> Result<rayon::ThreadPool, rayon::ThreadPoolBuildError> {
        let cpus = self.efficiency_cpus();
        let available = std::thread::available_parallelism().map_or(1, usize::from);
        let threads = worker_count(self.workers, available, cpus.as_deref());
        tracing::debug!(
            threads,
            ?cpus,
            batch_frames = self.batch_frames,
            "starting analysis pool"
        );
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .thread_name(|i| format!("argand-fft-{i}"))
            .start_handler(move |_| {
                if let Some(cpus) = &cpus
                    && let Err(error) = crate::analysis::cpu::pin_current(cpus)
                {
                    tracing::warn!(%error, "could not apply compute affinity; retaining inherited affinity");
                }
            })
            .build()
    }

    fn efficiency_cpus(self) -> Option<Vec<usize>> {
        if self.affinity == Affinity::None {
            return None;
        }
        match crate::analysis::cpu::efficiency_cpus() {
            Ok(Some(cpus)) => Some(cpus),
            Ok(None) => {
                tracing::warn!(
                    "efficiency affinity is unavailable on this CPU/platform; using unrestricted workers"
                );
                None
            }
            Err(error) => {
                tracing::warn!(%error, "CPU topology discovery failed; using unrestricted workers");
                None
            }
        }
    }
}

fn worker_count(requested: usize, available: usize, cpus: Option<&[usize]>) -> usize {
    let capacity = cpus
        .map_or(available, |cpus| available.min(cpus.len()))
        .max(1);
    let requested = if requested == 0 {
        capacity.min(8)
    } else {
        requested
    };
    requested.clamp(1, capacity.min(128))
}

#[cfg(test)]
#[path = "execution_test.rs"]
mod tests;

#[cfg(test)]
#[path = "execution_configuration_test.rs"]
mod configuration_tests;
