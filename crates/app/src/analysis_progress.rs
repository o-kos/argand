//! Progress-only notifications for navigation's sequential sample pass.

use super::*;
use argand_core::{AccessPattern, SampleRange, SampleSource, SourceError};

pub(super) struct Source<'a, 'b> {
    source: &'a mut dyn SampleSource,
    request: Requested,
    replies: &'a Replies<'b>,
    done: u64,
    last: Instant,
}

impl<'a, 'b> Source<'a, 'b> {
    pub fn new(
        source: &'a mut dyn SampleSource,
        request: Requested,
        replies: &'a Replies<'b>,
    ) -> Self {
        Self {
            source,
            request,
            replies,
            done: 0,
            last: Instant::now(),
        }
    }
}

impl SampleSource for Source<'_, '_> {
    fn meta(&self) -> &SignalMeta {
        self.source.meta()
    }
    fn seek(&mut self, sample: u64) -> Result<(), SourceError> {
        self.source.seek(sample)
    }
    fn access_pattern(&mut self, pattern: AccessPattern) {
        self.source.access_pattern(pattern);
    }
    fn prefetch(&mut self, range: SampleRange) {
        self.source.prefetch(range);
    }
    fn original_sample_units(&self) -> Option<(f64, f64)> {
        self.source.original_sample_units()
    }

    fn read(&mut self, buffer: &mut [f32]) -> Result<usize, SourceError> {
        let read = self.source.read(buffer)?;
        self.done = self
            .done
            .saturating_add((read / self.source.meta().channels()) as u64);
        if self.request.replacement && self.last.elapsed() >= Duration::from_millis(50) {
            self.last = Instant::now();
            let total = self
                .request
                .analysis
                .range
                .clamped_to(self.source.meta().len_samples)
                .len;
            let _ = self.replies.updates.try_send(Delivery {
                prepared_at: Instant::now(),
                generation: Some(self.request.generation),
                view_revision: None,
                update: Update::Progress {
                    done: self.done.min(total),
                    total,
                },
            });
        }
        Ok(read)
    }
}

#[cfg(test)]
#[path = "analysis_progress_test.rs"]
mod tests;
