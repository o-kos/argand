impl Rebinding {
    /// A rebinding of the same file under the same edits, as a failed replacement leaves.
    #[cfg(test)]
    pub(super) fn kept(shown: crate::document::Shown) -> Self {
        Self {
            to: Rebind::Keep,
            shown,
        }
    }
}
