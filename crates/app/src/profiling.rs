//! Opt-in UI scheduling diagnostics, separate from input-to-display latency.

use std::time::{Duration, Instant};

use gpui::Context;

pub fn watch_ui<T: 'static>(cx: &Context<T>) {
    if !tracing::enabled!(target: "argand::ui_latency", tracing::Level::TRACE) {
        return;
    }
    let executor = cx.background_executor().clone();
    cx.spawn(async move |entity, cx| {
        let period = Duration::from_millis(20);
        loop {
            let due = Instant::now() + period;
            executor.timer(period).await;
            if entity
                .update(cx, |_, _| {
                    tracing::trace!(target: "argand::ui_latency",
                        late_us = Instant::now().saturating_duration_since(due).as_micros(),
                        "UI timer dispatched");
                })
                .is_err()
            {
                break;
            }
        }
    })
    .detach();
}
