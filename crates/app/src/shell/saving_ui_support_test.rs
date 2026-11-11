#[cfg(test)]
impl SaveOf {
    /// A whole save of `version` of `document` that goes on from the saved file.
    pub(super) fn whole_for_test(document: u64, version: u64) -> Self {
        Self {
            document,
            version,
            whole: true,
            rebind: true,
            then: None,
            over: false,
            _scratch: Vec::new(),
        }
    }
}
