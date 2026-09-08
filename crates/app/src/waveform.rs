//! Paint the same merged linear envelope that `aspec` renders.

use argand_core::WaveformEnvelope;
use gpui::{Bounds, Pixels, Point, Window, fill, point, px, rgb, size};

use crate::axes::Frame;

pub struct Waveform {
    pub envelope: WaveformEnvelope,
    pub full_scale: f32,
}

impl Waveform {
    pub fn paint(&self, frame: &Frame, origin: Point<Pixels>, height: f32, window: &mut Window) {
        let scale = window.scale_factor();
        let columns = (frame.plot.width * scale).round() as usize;
        let rows = ((height - 9.0).max(0.0) * scale).round() as i64;
        if rows == 0 {
            return;
        }
        let middle = rows / 2;
        let half = ((rows - 1) / 2).max(1);
        for (column, span) in self
            .envelope
            .pixel_spans(columns, half, self.full_scale)
            .enumerate()
        {
            let Some((lo, hi)) = span else {
                continue;
            };
            let top = (middle - hi).max(0);
            let bottom = (middle - lo + 1).min(rows);
            window.paint_quad(fill(
                Bounds {
                    origin: origin
                        + point(
                            px(frame.plot.x + column as f32 / scale),
                            px(4.0 + top as f32 / scale),
                        ),
                    size: size(px(1.0 / scale), px((bottom - top) as f32 / scale)),
                },
                rgb(0x78c8ff),
            ));
        }
    }
}
