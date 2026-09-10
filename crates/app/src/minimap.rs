//! A bounded full-capture envelope whose lifetime does not depend on FFT requests.

use std::sync::Arc;
use std::time::Instant;

use anyhow::{Result, ensure};
use argand_core::{AccessPattern, SampleSource, WaveformEnvelope};
use argand_dsp::EnvelopeBuilder;

use crate::document::Origin;
use crate::navigation::View;

const COLUMNS: usize = 65536;
const BLOCK_VALUES: usize = 65536;
const PREVIEW_BLOCKS: u64 = 128;
const PREVIEW_SAMPLES: u64 = 256;

pub struct Snapshot {
    pub envelope: WaveformEnvelope,
    pub full_scale: f32,
    pub complete: bool,
}

pub fn start(
    origin: Origin,
    meta: argand_core::SignalMeta,
) -> async_channel::Receiver<Result<Arc<Snapshot>>> {
    let (sender, receiver) = async_channel::bounded(2);
    let outgoing = sender.clone();
    let spawned = std::thread::Builder::new()
        .name("argand-minimap".into())
        .spawn(move || {
            if sender.is_closed() {
                return;
            }
            let started = Instant::now();
            let result = argand_io::reopen(&meta, &origin.hints)
                .map_err(anyhow::Error::from)
                .and_then(|mut source| {
                    build(source.as_mut(), &|| !sender.is_closed(), &mut |snapshot| {
                        sender.try_send(Ok(Arc::new(snapshot))).is_ok()
                    })
                });
            if let Err(error) = result {
                let _ = sender.try_send(Err(error));
            }
            tracing::debug!(elapsed = ?started.elapsed(), "minimap task finished");
        });
    if let Err(error) = spawned {
        let _ = outgoing.try_send(Err(error.into()));
    }
    receiver
}

fn build(
    source: &mut dyn SampleSource,
    active: &dyn Fn() -> bool,
    publish: &mut dyn FnMut(Snapshot) -> bool,
) -> Result<()> {
    let total = source.meta().len_samples;
    let channels = source.meta().channels();
    let duration = total as f64 / source.meta().sample_rate;
    let mut envelope = EnvelopeBuilder::new(total.min(COLUMNS as u64) as usize, channels, total);
    let mut buffer = vec![0.; BLOCK_VALUES];
    if total > (BLOCK_VALUES / channels) as u64 {
        source.access_pattern(AccessPattern::Sparse);
        for index in 0..PREVIEW_BLOCKS {
            if !active() {
                return Ok(());
            }
            let first = ((u128::from(total - PREVIEW_SAMPLES) * u128::from(index))
                / u128::from(PREVIEW_BLOCKS - 1)) as u64;
            source.seek(first)?;
            fold(
                source,
                &mut envelope,
                &mut buffer[..PREVIEW_SAMPLES as usize * channels],
                first,
            )?;
        }
        if !publish(snapshot(envelope.clone(), duration, false)) {
            return Ok(());
        }
    }
    source.access_pattern(AccessPattern::Sequential);
    source.seek(0)?;
    let mut first = 0;
    while first < total {
        if !active() {
            return Ok(());
        }
        let count = (total - first).min((BLOCK_VALUES / channels) as u64) as usize * channels;
        fold(source, &mut envelope, &mut buffer[..count], first)?;
        first += (count / channels) as u64;
    }
    if active() {
        publish(snapshot(envelope, duration, true));
    }
    Ok(())
}

fn fold(
    source: &mut dyn SampleSource,
    envelope: &mut EnvelopeBuilder,
    buffer: &mut [f32],
    first: u64,
) -> Result<()> {
    let read = source.read(buffer)?;
    ensure!(
        read == buffer.len(),
        "capture ended before the waveform was complete"
    );
    envelope.fold(buffer, first);
    Ok(())
}

fn snapshot(builder: EnvelopeBuilder, duration: f64, complete: bool) -> Snapshot {
    let envelope = builder.finish(0., duration);
    let full_scale = envelope
        .min
        .iter()
        .chain(&envelope.max)
        .filter(|value| value.is_finite())
        .map(|value| value.abs())
        .fold(1e-6, f32::max);
    Snapshot {
        envelope,
        full_scale,
        complete,
    }
}

/// Conservative min/max reduction: a narrow event survives every display width.
pub fn rebin(source: &WaveformEnvelope, columns: usize) -> WaveformEnvelope {
    let mut result = WaveformEnvelope::new(columns, source.channels);
    result.t0 = source.t0;
    result.t1 = source.t1;
    if source.columns == 0 || columns == 0 {
        return result;
    }
    for column in 0..columns {
        let first = column * source.columns / columns;
        let end = ((column + 1) * source.columns).div_ceil(columns);
        for channel in 0..source.channels {
            let mut low = f32::INFINITY;
            let mut high = f32::NEG_INFINITY;
            for cell in first..end {
                if let Some((min, max)) = source.column(cell, channel) {
                    low = low.min(min);
                    high = high.max(max);
                }
            }
            result.min[column * source.channels + channel] = low;
            result.max[column * source.channels + channel] = high;
        }
    }
    result
}

/// Fractions of the full capture, with a visible minimum width at deep zoom.
pub fn viewport(view: View, total: u64, columns: usize) -> (f64, f64) {
    if total == 0 || columns == 0 {
        return (0., 1.);
    }
    let first = (u128::from(view.start.min(total)) * columns as u128 / u128::from(total)) as usize;
    let end = (u128::from(view.start.saturating_add(view.len).min(total)) * columns as u128)
        .div_ceil(u128::from(total)) as usize;
    let first = first.min(columns - 1);
    let end = end.max(first + 1).min(columns);
    (first as f64 / columns as f64, end as f64 / columns as f64)
}

#[derive(Debug, PartialEq)]
pub enum Click {
    Grab,
    Step(i64),
    Center,
}

pub fn click(fraction: f64, viewport: (f64, f64), control: bool, count: usize) -> Click {
    let direction = if fraction < viewport.0 {
        -1
    } else if fraction > viewport.1 {
        1
    } else {
        return Click::Grab;
    };
    if count == 2 {
        Click::Center
    } else {
        Click::Step(direction * if control { 5 } else { 1 })
    }
}

pub fn center(view: View, fraction: f64, total: u64) -> View {
    let center = (fraction.clamp(0., 1.) * total as f64).round() as u64;
    View {
        start: center.saturating_sub(view.len / 2).min(total - view.len),
        ..view
    }
}

#[cfg(test)]
mod tests {
    include!("minimap_tests.rs");
}
