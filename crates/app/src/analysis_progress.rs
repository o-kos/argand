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
        if self.request.navigation && self.last.elapsed() >= Duration::from_millis(50) {
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
mod tests {
    use super::*;

    #[test]
    fn progress_counts_iq_pairs_and_never_waits_for_a_full_reply_queue() {
        use argand_core::{Domain, SampleFormat, SampleType};
        use argand_io::testutil::{TempDir, iq_tone, write_wav};
        let dir = TempDir::new("navigation-progress");
        let path = write_wav(
            &dir.join("iq.wav"),
            SampleType::new(Domain::Iq, SampleFormat::F32),
            48000,
            &iq_tone(4096, 48000.0, 6000.0, 0.5),
            1.0,
        );
        let mut input = argand_io::open(&path, &OpenHints::default()).unwrap();
        let (analyst, updates, _start) = prepare(path, OpenHints::default(), Default::default());
        let analysis = crate::settings::Settings::from_config(&crate::config::Config::default())
            .analysis_request(input.meta(), 10, 10);
        analyst.request(analysis);
        let mut request = analyst.mailbox.latest().unwrap();
        request.navigation = true;
        let (_sender, requests) = async_channel::bounded(1);
        let (outgoing, receiver) = async_channel::bounded(1);
        let replies = Replies {
            requests: &requests,
            updates: &outgoing,
            mailbox: &analyst.mailbox,
        };
        let mut source = Source::new(input.as_mut(), request, &replies);
        let mut buffer = [0.0; 200];
        for _ in 0..2 {
            source.last = Instant::now() - Duration::from_millis(60);
            assert_eq!(source.read(&mut buffer).unwrap(), 200);
        }
        let delivery = receiver.try_recv().unwrap();
        assert!(analyst.accepts(&delivery));
        assert!(matches!(
            delivery.update,
            Update::Progress {
                done: 100,
                total: 4096
            }
        ));
        assert!(receiver.is_empty());
        drop(updates);
    }
}
