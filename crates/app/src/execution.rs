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
                    && let Err(error) = crate::cpu::pin_current(cpus)
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
        match crate::cpu::efficiency_cpus() {
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
mod tests {
    use super::*;

    #[test]
    fn budgets_respect_process_limits_and_selected_cpus() {
        assert_eq!(worker_count(0, 12, None), 8);
        assert_eq!(worker_count(4, 12, None), 4);
        assert_eq!(worker_count(32, 12, None), 12);
        assert_eq!(worker_count(0, 1, None), 1);
        assert_eq!(worker_count(8, 12, Some(&[7, 19, 31])), 3);
        assert_eq!(worker_count(8, 2, Some(&[7, 19, 31])), 2);
    }

    #[test]
    fn invalid_fields_are_repaired_independently() {
        let mut settings = Settings {
            workers: 999,
            batch_frames: 0,
            affinity: Affinity::Efficiency,
        };
        settings.repair();
        assert_eq!(settings.workers, 0);
        assert_eq!(settings.batch_frames, Settings::default().batch_frames);
        assert_eq!(settings.affinity, Affinity::Efficiency);
    }
}

#[cfg(test)]
mod configuration_tests {
    use super::*;

    #[test]
    fn partial_analysis_configuration_keeps_independent_defaults() {
        let config: crate::config::Config =
            toml::from_str("[analysis]\nworkers = 4\naffinity = 'efficiency'").unwrap();
        assert_eq!(config.analysis.workers, 4);
        assert_eq!(
            config.analysis.batch_frames,
            Settings::default().batch_frames
        );
        assert_eq!(config.analysis.affinity, Affinity::Efficiency);
        assert!(toml::from_str::<crate::config::Config>("[analysis]\naffinity = 'cpu4'").is_err());
    }
}
