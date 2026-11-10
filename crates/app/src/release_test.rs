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
