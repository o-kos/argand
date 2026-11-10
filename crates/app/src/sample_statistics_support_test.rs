type ReadCounter = std::sync::atomic::AtomicU64;

fn record_read_for_test(cache: &Cache, frames: u64) {
    cache
        .1
        .fetch_add(frames, std::sync::atomic::Ordering::Relaxed);
}
