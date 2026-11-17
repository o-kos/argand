//! Opt-in pan diagnostics. No retained event buffer or capture identity.

use super::plot_view::PlotView;
use crate::navigation::{View, frequency};
use gpui_kit::{MouseButton, Pixels, Point};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::Instant;

static NEXT_INPUT: AtomicU64 = AtomicU64::new(1);
static STARTED: OnceLock<Instant> = OnceLock::new();

pub(super) fn input(
    plot: &PlotView,
    kind: &'static str,
    position: Point<Pixels>,
    button: Option<MouseButton>,
) -> u64 {
    if !tracing::enabled!(target: "argand::pan_trace", tracing::Level::TRACE) {
        return 0;
    }
    let input_id = NEXT_INPUT.fetch_add(1, Ordering::Relaxed);
    let elapsed_us = STARTED.get_or_init(Instant::now).elapsed().as_micros();
    tracing::trace!(
        target: "argand::pan_trace", input_id, elapsed_us = ?elapsed_us, kind,
        ?position, ?button, "input"
    );
    state(plot, "input_before", input_id);
    input_id
}

pub(super) fn state(plot: &PlotView, reason: &'static str, input_id: u64) {
    tracing::trace!(
        target: "argand::pan_trace",
        input_id,
        reason,
        pan = ?plot.pan,
        frequency_pan = ?plot.frequency_pan,
        pan_button = ?plot.pan_button,
        snapshot = ?plot.snapshot.as_ref().map(|snapshot| (
            snapshot.extents.time.view, snapshot.frequency,
            snapshot.extents.seconds, snapshot.extents.hertz,
        )),
        geometry = ?plot.geometry.map(|geometry| (
            geometry.orientation, geometry.spectrum,
            geometry.time_length(), geometry.frequency_length(),
        )),
        "pan state"
    );
}

pub(super) fn computed(input_id: u64, time: Option<View>, frequency: Option<frequency::View>) {
    tracing::trace!(target: "argand::pan_trace", input_id, ?time, ?frequency, "pan computed");
}

pub(super) fn accepted(
    input_id: u64,
    requested: (Option<View>, Option<frequency::View>),
    before: (Option<View>, frequency::View),
    after: (Option<View>, frequency::View),
) {
    tracing::trace!(target: "argand::pan_trace", input_id, ?requested, ?before, ?after, "pan accepted");
}
