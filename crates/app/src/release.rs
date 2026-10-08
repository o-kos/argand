//! Knowing when every thread reading a file has let it go.
//!
//! Each reader of an open file holds a [`Lease`] until its thread ends. The
//! window keeps the matching [`Released`] and waits on it before replacing the
//! file, which Windows refuses while any of them still maps it.

/// Held by a thread for as long as it may read the file.
pub struct Lease(async_channel::Sender<()>);

impl Clone for Lease {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

/// Ends once every lease is gone.
#[derive(Clone)]
pub struct Released(async_channel::Receiver<()>);

/// A first lease and the wait for it and every copy of it.
pub fn lease() -> (Lease, Released) {
    let (sender, receiver) = async_channel::bounded(1);
    (Lease(sender), Released(receiver))
}

impl Released {
    /// Wait until no lease is left.
    pub async fn wait(self) {
        while self.0.recv().await.is_ok() {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_wait_ends_only_after_every_lease_is_dropped() {
        let (lease, released) = lease();
        let held = lease.clone();
        drop(lease);
        assert!(!released.0.is_closed(), "a copy of the lease is still held");
        let thread = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(20));
            drop(held);
        });
        block_on(released.clone().wait());
        assert!(released.0.is_closed());
        thread.join().unwrap();
    }

    /// Run a future to completion on this thread, as the test has no executor.
    fn block_on(future: impl std::future::Future<Output = ()>) {
        let waker = std::task::Waker::noop();
        let mut context = std::task::Context::from_waker(waker);
        let mut future = std::pin::pin!(future);
        while future.as_mut().poll(&mut context).is_pending() {
            std::thread::yield_now();
        }
    }
}
