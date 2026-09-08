//! Paint a channel-separated linear envelope in the shared time rectangle.

use argand_core::WaveformEnvelope;
use gpui::{App, Bounds, Hsla, Pixels, Point, Window, fill, hsla, point, px, size};

use crate::axes::{Colors, Frame, Labels, Rect};

pub struct Waveform {
    pub envelope: WaveformEnvelope,
}

impl Waveform {
    pub fn paint(&self, frame: &Frame, origin: Point<Pixels>, height: f32, window: &mut Window) {
        let rect = Rect {
            x: frame.plot.x,
            y: 18.0,
            width: frame.plot.width,
            height: (height - 23.0).max(0.0),
        };
        let full_scale = self.envelope.peak().max(1.0);
        let pixel = 1.0 / window.scale_factor();
        let column_width = rect.width / self.envelope.columns.max(1) as f32;
        for column in 0..self.envelope.columns {
            let channel_span = |channel| {
                self.envelope
                    .column(column, channel)
                    .and_then(|(low, high)| span(low, high, full_scale, rect.height, pixel))
            };
            for band in bands(channel_span(0), channel_span(1))
                .into_iter()
                .flatten()
            {
                window.paint_quad(fill(
                    Bounds {
                        origin: origin
                            + point(
                                px(rect.x + column as f32 * column_width),
                                px(rect.y + band.top),
                            ),
                        size: size(px(column_width), px(band.bottom - band.top)),
                    },
                    trace_color(band.trace),
                ));
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Trace {
    I,
    Q,
    Overlap,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Band {
    top: f32,
    bottom: f32,
    trace: Trace,
}

fn bands(i: Option<(f32, f32)>, q: Option<(f32, f32)>) -> [Option<Band>; 3] {
    let edges_of = |span: Option<(f32, f32)>| span.unwrap_or((0.0, 0.0));
    let (i0, i1) = edges_of(i);
    let (q0, q1) = edges_of(q);
    let mut edges = [i0, i1, q0, q1];
    edges.sort_by(f32::total_cmp);
    std::array::from_fn(|index| {
        let (top, bottom) = (edges[index], edges[index + 1]);
        if top == bottom {
            return None;
        }
        let midpoint = top + (bottom - top) / 2.0;
        let contains =
            |span: Option<(f32, f32)>| span.is_some_and(|(lo, hi)| lo <= midpoint && midpoint < hi);
        let trace = match (contains(i), contains(q)) {
            (true, true) => Trace::Overlap,
            (true, false) => Trace::I,
            (false, true) => Trace::Q,
            (false, false) => return None,
        };
        Some(Band { top, bottom, trace })
    })
}

fn trace_color(trace: Trace) -> Hsla {
    match trace {
        Trace::I => hsla(0.56, 0.85, 0.48, 1.0),
        Trace::Q => hsla(0.08, 0.95, 0.55, 1.0),
        Trace::Overlap => hsla(0.76, 0.65, 0.62, 1.0),
    }
}

fn span(low: f32, high: f32, scale: f32, height: f32, pixel: f32) -> Option<(f32, f32)> {
    if !low.is_finite() || !high.is_finite() || height <= 0.0 {
        return None;
    }
    let y = |value: f32| (1.0 - (value / scale).clamp(-1.0, 1.0)) * height / 2.0;
    let top = y(high).min(y(low)).min((height - pixel).max(0.0));
    let bottom = y(low).max(y(high)).max(top + pixel).min(height);
    Some((top, bottom))
}

pub fn paint_axes(
    frame: &Frame,
    origin: Point<Pixels>,
    height: f32,
    colors: Colors,
    window: &mut Window,
) {
    for tick in &frame.time {
        window.paint_quad(fill(
            Bounds {
                origin: origin + point(px(frame.plot.x + tick.offset as f32), px(18.0)),
                size: size(px(1.0), px((height - 23.0).max(0.0))),
            },
            colors.grid,
        ));
    }
    window.paint_quad(fill(
        Bounds {
            origin: origin + point(px(frame.plot.x), px((height + 13.0) / 2.0)),
            size: size(px(frame.plot.width), px(1.0)),
        },
        colors.grid,
    ));
}

pub fn paint_legend(
    waveform: &Waveform,
    origin: Point<Pixels>,
    labels: &Labels,
    window: &mut Window,
    cx: &mut App,
) {
    let scale = waveform.envelope.peak().max(1.0);
    let names = if waveform.envelope.channels == 2 {
        "I"
    } else {
        "Real"
    };
    labels.paint_text(
        names,
        origin + point(px(8.0), px(0.0)),
        trace_color(Trace::I),
        window,
        cx,
    );
    if waveform.envelope.channels == 2 {
        labels.paint_text(
            "Q",
            origin + point(px(24.0), px(0.0)),
            trace_color(Trace::Q),
            window,
            cx,
        );
    }
    if waveform.envelope.channels == 2 {
        labels.paint_text(
            "Both",
            origin + point(px(40.0), px(0.0)),
            trace_color(Trace::Overlap),
            window,
            cx,
        );
    }
    let caption_x = if waveform.envelope.channels == 2 {
        76.0
    } else {
        48.0
    };
    labels.paint_text(
        &format!("±{scale:.3} linear"),
        origin + point(px(caption_x), px(0.0)),
        window.text_style().color,
        window,
        cx,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coincident_and_nested_channels_have_explicit_overlap_bands() {
        let visible = |i, q| bands(i, q).into_iter().flatten().collect::<Vec<_>>();
        assert_eq!(
            visible(Some((4.0, 20.0)), Some((4.0, 20.0))),
            vec![Band {
                top: 4.0,
                bottom: 20.0,
                trace: Trace::Overlap
            }]
        );
        assert_eq!(
            visible(Some((0.0, 30.0)), Some((10.0, 20.0))),
            vec![
                Band {
                    top: 0.0,
                    bottom: 10.0,
                    trace: Trace::I
                },
                Band {
                    top: 10.0,
                    bottom: 20.0,
                    trace: Trace::Overlap
                },
                Band {
                    top: 20.0,
                    bottom: 30.0,
                    trace: Trace::I
                },
            ]
        );
        assert_eq!(
            visible(Some((0.0, 10.0)), Some((20.0, 30.0))),
            vec![
                Band {
                    top: 0.0,
                    bottom: 10.0,
                    trace: Trace::I
                },
                Band {
                    top: 20.0,
                    bottom: 30.0,
                    trace: Trace::Q
                },
            ]
        );
        assert_eq!(
            visible(Some((4.0, 5.0)), None),
            vec![Band {
                top: 4.0,
                bottom: 5.0,
                trace: Trace::I
            }]
        );
    }

    #[test]
    fn linear_extrema_and_silent_samples_remain_visible() {
        assert_eq!(span(-1.0, 1.0, 1.0, 40.0, 1.0), Some((0.0, 40.0)));
        assert_eq!(span(0.0, 0.0, 1.0, 40.0, 1.0), Some((20.0, 21.0)));
        assert_eq!(span(0.5, 0.5, 1.0, 40.0, 1.0), Some((10.0, 11.0)));
        assert_eq!(span(f32::NAN, 1.0, 1.0, 40.0, 1.0), None);
    }
}
