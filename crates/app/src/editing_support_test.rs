impl Editing {
    /// Record what was learned about the opened file off the window's thread.
    #[cfg(test)]
    pub fn describe_file(&mut self, stamp: Option<SourceStamp>, storage: Option<Storage>) {
        self.sources[0].stamp = stamp;
        self.sources[0].storage = storage;
        self.statistics_revision = next_statistics_revision();
    }
}
