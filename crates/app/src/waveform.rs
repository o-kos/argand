//! Full-capture minimap geometry is cached separately from the viewport overlay.

use crate::{axes::Frame, minimap, navigation::View};
use gpui::{Bounds, Pixels, Point, Window, fill, point, px, rgb, size};
use std::sync::{Arc, Mutex};

pub struct Waveform {
    snapshot: Arc<minimap::Snapshot>,
    cache: Mutex<Option<Spans>>,
}

struct Spans {
    columns: usize,
    rows: i64,
    values: Vec<Option<(i64, i64)>>,
}

impl Waveform {
    pub fn new(snapshot: Arc<minimap::Snapshot>) -> Self {
        Self {
            snapshot,
            cache: Mutex::new(None),
        }
    }

    pub fn paint(
        &self,
        frame: &Frame,
        origin: Point<Pixels>,
        height: f32,
        viewport: Option<(View, u64)>,
        window: &mut Window,
    ) {
        let scale = window.scale_factor();
        let columns = (frame
            .orientation
            .axes(frame.plot.width, frame.plot.height)
            .0
            * scale)
            .round() as usize;
        let rows = ((height - 9.).max(0.) * scale).round() as i64;
        if rows == 0 {
            return;
        }
        let mut cache = self
            .cache
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if cache
            .as_ref()
            .is_none_or(|cache| cache.columns != columns || cache.rows != rows)
        {
            let envelope = minimap::rebin(&self.snapshot.envelope, columns);
            let half = ((rows - 1) / 2).max(1);
            *cache = Some(Spans {
                columns,
                rows,
                values: envelope
                    .pixel_spans(columns, half, self.snapshot.full_scale)
                    .collect(),
            });
        }
        let Some(cache) = cache.as_ref() else { return };
        let (left, right) = viewport.map_or((0., 1.), |(view, total)| {
            minimap::viewport(view, total, columns)
        });
        let first = (left * columns as f64).round() as usize;
        let end = (right * columns as f64).round() as usize;
        let middle = rows / 2;
        for (column, span) in cache.values.iter().enumerate() {
            let Some((lo, hi)) = span else { continue };
            let top = (middle - hi).max(0);
            let bottom = (middle - lo + 1).min(rows);
            let bounds = if frame.orientation.vertical() {
                Bounds::new(
                    origin
                        + point(
                            px(4. + (rows - bottom) as f32 / scale),
                            px(frame.plot.y + column as f32 / scale),
                        ),
                    size(px((bottom - top) as f32 / scale), px(1. / scale)),
                )
            } else {
                Bounds::new(
                    origin
                        + point(
                            px(frame.plot.x + column as f32 / scale),
                            px(4. + top as f32 / scale),
                        ),
                    size(px(1. / scale), px((bottom - top) as f32 / scale)),
                )
            };
            window.paint_quad(fill(
                bounds,
                rgb(if (first..end).contains(&column) {
                    0x78c8ff
                } else {
                    0x243c4d
                }),
            ));
        }
    }
}

pub struct Panel {
    pub waveform: Option<Arc<Waveform>>,
    pub viewport: Option<(View, u64)>,
    pub separator: gpui::Hsla,
}

impl Panel {
    pub fn paint(&self, frame: &Frame, origin: Point<Pixels>, height: f32, window: &mut Window) {
        if let Some(waveform) = &self.waveform {
            waveform.paint(frame, origin, height, self.viewport, window);
        }
        let bounds = if frame.orientation.vertical() {
            Bounds::new(
                origin + point(px(height - 1.), px(frame.plot.y)),
                size(px(1.), px(frame.plot.height)),
            )
        } else {
            Bounds::new(
                origin + point(px(frame.plot.x), px(height - 1.)),
                size(px(frame.plot.width), px(1.)),
            )
        };
        window.paint_quad(fill(bounds, self.separator));
    }
}
