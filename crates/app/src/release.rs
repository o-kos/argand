//! Knowing when every thread reading a file has let it go.
//!
//! Each reader of an open file holds a [`Lease`] until its thread ends. The
//! window keeps the matching [`Released`] and waits on it before replacing the
//! file, which Windows refuses while any of them still maps it.

/// Held by a thread for as long as it may read the file.
pub struct Lease(
    async_channel::Sender<()>,
    Option<std::sync::Arc<crate::scratch::Scratch>>,
);

impl Lease {
    pub fn retaining(mut self, backing: std::sync::Arc<crate::scratch::Scratch>) -> Self {
        self.1 = Some(backing);
        self
    }
}

impl Clone for Lease {
    fn clone(&self) -> Self {
        Self(self.0.clone(), self.1.clone())
    }
}

/// Ends once every lease is gone.
#[derive(Clone)]
pub struct Released(async_channel::Receiver<()>);

/// A first lease and the wait for it and every copy of it.
pub fn lease() -> (Lease, Released) {
    let (sender, receiver) = async_channel::bounded(1);
    (Lease(sender, None), Released(receiver))
}

impl Released {
    /// Wait until no lease is left.
    pub async fn wait(self) {
        while self.0.recv().await.is_ok() {}
    }
}

#[cfg(test)]
#[path = "release_test.rs"]
mod tests;
