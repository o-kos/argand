impl OpenFileState {
    /// Simulate an accepted picture for a test that has no delivery of its
    /// own.
    #[cfg(test)]
    pub(crate) fn record_for_test(&mut self, settings: &Settings) {
        self.displayed_settings = Some(*settings);
    }
}
