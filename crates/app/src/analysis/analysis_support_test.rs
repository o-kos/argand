/// Keep the real request mailbox alive without starting a reader or FFT worker.
/// The caller owns the receiver for the complete state-only test scope.
pub(crate) fn analyst_for_test() -> (Analyst, async_channel::Receiver<()>) {
    let (requests, incoming) = async_channel::bounded(1);
    let analyst = Analyst {
        requests,
        mailbox: Arc::new(Mailbox::default()),
    };
    (analyst, incoming)
}

/// A prepared test document never starts its inert reader handshake.
pub(crate) fn readers_for_test() -> (Analyst, async_channel::Receiver<()>, Start) {
    let (analyst, incoming) = analyst_for_test();
    let (start, _started) = async_channel::bounded(1);
    (analyst, incoming, Start(start))
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

#[cfg(test)]
fn open(path: PathBuf, hints: OpenHints) -> (Analyst, async_channel::Receiver<Delivery>) {
    let (lease, _) = crate::release::lease();
    let (analyst, updates, start) =
        prepare(path, hints, crate::analysis::execution::Settings::default(), lease);
    start.start();
    (analyst, updates)
}

impl Analyst {
    pub(crate) fn generation_for_test(&self) -> u64 { self.mailbox.generation.load(Ordering::Relaxed) }
    #[cfg(test)]
    pub fn request(&self, analysis: AnalysisRequest) -> bool {
        self.request_view(analysis, None)
    }
}
