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
        prepare(path, hints, crate::execution::Settings::default(), lease);
    start.start();
    (analyst, updates)
}

impl Analyst {
    #[cfg(test)]
    pub fn request(&self, analysis: AnalysisRequest) -> bool {
        self.request_view(analysis, None)
    }
}
