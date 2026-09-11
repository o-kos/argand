//! The marks around the spectrogram, and the font they are measured with.
//!
//! Where a tick goes is [`argand_core::axis`]'s decision and not this module's:
//! the same ladder of round steps, the same measure-then-accept policy and the
//! same formatter that `aspec` places its own marks with. Only two things are a
//! front end's, and both are here. One is a font, which the policy asks for
//! through [`LabelMeasure`] -- this is the second implementation of that trait,
//! and the reason it exists. The other is the drawing.
//!
//! The layout half decides everything through `argand_core::axis` and a
//! [`LabelMeasure`] it is handed, so where the picture ends up inside a panel
//! is settled and tested with the fixture font and no window in sight. It does
//! name one toolkit type, the panel's [`Size`]; what it does not need is a
//! running one. [`Labels`] and [`paint`] need both.

use argand_core::axis::{self, Axis, AxisKind, LabelMeasure, LabelMetrics, LabelRun, Tick};
use gpui::{App, Bounds, Font, FontId, Hsla, Pixels, Point, Size, Window, fill, point, px, size};

#[path = "cursor_guides.rs"]
mod cursor_guides;
pub use cursor_guides::{BadgeMetrics, CursorGuides};

/// Room between a label and whatever it labels.
const LABEL_PAD: f32 = 6.0;
/// Space between the complete axis layout and adjacent panels or window edges.
const OUTER_PAD: f32 = 4.0;
/// How far a tick's mark reaches out of the plot.
const TICK_LEN: f32 = 3.0;
/// The size the labels are drawn at.
///
/// A shade under the window's smallest text: an axis is read by glancing at
/// it, and the marks should not compete with the picture they surround.
const LABEL_SIZE: f32 = 11.0;

/// What the axes span: the requested time view and the full frequency range.
///
/// Taken from the file's own description rather than from a finished analysis,
/// so the room the labels need is known before the first transform runs -- and
/// it is that room which decides how many columns the transform is asked for.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Extents {
    pub time: crate::time_ruler::Ruler,
    pub seconds: (f64, f64),
    pub hertz: (f64, f64),
}

impl Extents {
    pub fn picture(self) -> crate::navigation::PictureView {
        crate::navigation::PictureView {
            time: self.seconds,
            frequency: self.hertz,
        }
    }
}

/// A rectangle inside the panel, in the panel's own logical pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    fn right(self) -> f32 {
        self.x + self.width
    }

    fn bottom(self) -> f32 {
        self.y + self.height
    }
}

/// A unit caption's measured hover area and expanded meaning.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UnitHint {
    pub bounds: Rect,
    pub text: &'static str,
    pub units: &'static str,
    pub per_pixel: f64,
}

impl UnitHint {
    pub fn resolution(self) -> String {
        let value = self.per_pixel;
        let number = if value > 0. && !(1e-9..1e9).contains(&value) {
            format!("{value:.3e}")
        } else {
            let decimals = (3. - value.log10().floor()).clamp(0., 12.) as usize;
            format!("{value:.decimals$}")
        };
        crate::numbers::text(&format!("Resolution: {number} {}/px", self.units))
    }
}

/// Where the picture goes inside a panel, and what is drawn around it.
pub struct Frame {
    /// The spectrogram's own rectangle, which is what a transform is sized to.
    pub plot: Rect,
    pub time: Vec<Tick>,
    pub time_scheme: Option<axis::TickScheme>,
    pub frequency: Vec<Tick>,
    pub frequency_scheme: Option<axis::TickScheme>,
    /// The unit the frequency labels are in, named once above them instead of
    /// on every tick.
    pub caption: Option<&'static str>,
    /// Where the ink of a time label is centred, under the plot.
    time_row: f32,
    time_caption: &'static str,
    /// Where the ink of the caption is centred, over the gutter.
    caption_row: f32,
    per_pixel: (f64, f64),
}

impl Frame {
    pub fn unit_hints(&self, measure: &dyn LabelMeasure) -> [Option<UnitHint>; 2] {
        let height = LINE_HEIGHT.max(measure.digit_height(LABEL_SIZE)).ceil();
        let frequency = self.caption.filter(|_| !self.frequency.is_empty());
        [
            (Some(self.time_caption), self.time_row, self.per_pixel.0),
            (frequency, self.caption_row, self.per_pixel.1),
        ]
        .map(|(caption, row, per_pixel)| {
            caption.map(|caption| UnitHint {
                bounds: Rect {
                    x: self.plot.right() + LABEL_PAD,
                    y: row - height / 2.,
                    width: measure.width(caption, LABEL_SIZE).ceil(),
                    height,
                },
                text: match caption {
                    "hms" => "Time in hours, minutes and seconds",
                    "s" => "Time in seconds",
                    "#" => "Time in samples",
                    "Hz" => "Frequency in Hz",
                    "kHz" => "Frequency in kHz",
                    "MHz" => "Frequency in MHz",
                    "GHz" => "Frequency in GHz",
                    _ => caption,
                },
                units: match caption {
                    "hms" => "s",
                    "#" => "samples",
                    _ => caption,
                },
                per_pixel: per_pixel
                    / match caption {
                        "kHz" => 1e3,
                        "MHz" => 1e6,
                        "GHz" => 1e9,
                        _ => 1.,
                    },
            })
        })
    }

    /// Reserve room for the labels, then lay out the marks in what is left.
    ///
    /// `scale` is the display's, and the plot's edges are snapped to whole
    /// device pixels with it. That is what lets a transform asked for exactly
    /// this many columns be drawn one column to one pixel: a plot half a
    /// device pixel wide at one edge is resampled by the GPU, and a resampled
    /// spectrogram is a blurred one.
    ///
    /// `None` when the panel is too small to hold a plot at all, which is the
    /// answer for a window dragged down to nothing: there is no rectangle to
    /// draw into and nothing to ask a transform for.
    pub fn measure(
        panel: Size<Pixels>,
        scale: f32,
        extents: Extents,
        measure: &dyn LabelMeasure,
        held: Option<axis::TickScheme>,
    ) -> Option<Self> {
        Self::measure_view(panel, scale, extents, measure, held, None)
    }

    pub fn measure_view(
        panel: Size<Pixels>,
        scale: f32,
        extents: Extents,
        measure: &dyn LabelMeasure,
        held: Option<axis::TickScheme>,
        held_frequency: Option<axis::TickScheme>,
    ) -> Option<Self> {
        let (t0, t1) = extents.time.bounds(extents.seconds);
        let (f0, f1) = extents.hertz;
        let caption = axis::caption(AxisKind::Frequency, f0, f1);

        // Time labels get a row below the plot; the frequency unit sits beside the minimap.
        let row_height = LINE_HEIGHT.max(measure.digit_height(LABEL_SIZE)).ceil();
        let foot = OUTER_PAD + row_height + LABEL_PAD;
        // Every candidate is measured, because which of two strings needs more
        // room is a question about glyphs and not about characters. The
        // caption counts too: over a narrow span `MHz` is wider than the
        // digits it heads.
        let gutter = axis::widest_labels(AxisKind::Frequency, f0, f1)
            .iter()
            .map(String::as_str)
            .chain(caption)
            .map(|label| measure.width(&measure.localize(label, AxisKind::Frequency), LABEL_SIZE))
            .chain(["hms", "s", "#"].map(|label| measure.width(label, LABEL_SIZE)))
            .fold(0.0f32, f32::max)
            .ceil()
            + LABEL_PAD;

        // Both edges are snapped rather than the origin and the size, so that
        // the width left between them is itself a whole number of device
        // pixels.
        let scale = if scale > 0.0 { scale } else { 1.0 };
        let ceil = |value: f32| (value * scale).ceil() / scale;
        let floor = |value: f32| (value * scale).floor() / scale;
        // Snap inward to preserve both label space and the outside margins.
        let left = ceil(OUTER_PAD);
        let right = floor(f32::from(panel.width) - OUTER_PAD - gutter);
        let top = 0.;
        let plot = Rect {
            x: left,
            y: top,
            width: right - left,
            height: floor(f32::from(panel.height) - foot) - top,
        };
        if plot.width < 1.0 || plot.height < 1.0 {
            return None;
        }

        let time = axis::tick_layout(
            extents.time.mode.kind(),
            Axis {
                length: plot.width as i64,
                min: t0,
                max: t1,
                lead: 0,
                trail: 0,
            },
            &LabelMetrics::new(measure, LABEL_SIZE, LabelRun::Across).after_tick(LABEL_PAD),
            held,
        );
        let frequency = axis::tick_layout(
            AxisKind::Frequency,
            Axis {
                length: plot.height as i64,
                min: f0,
                max: f1,
                lead: -(LABEL_PAD as i64),
                trail: -(LABEL_PAD as i64),
            },
            &LabelMetrics::new(measure, LABEL_SIZE, LabelRun::Down),
            held_frequency,
        );

        Some(Self {
            plot,
            time: time.ticks,
            time_scheme: time.scheme,
            // Frequency ink stays within the plot height, clear of both the
            // unit above and the time-label row below.
            frequency: frequency.ticks,
            frequency_scheme: frequency.scheme,
            caption,
            time_caption: extents.time.mode.caption(),
            time_row: plot.bottom() + LABEL_PAD + row_height / 2.0,
            caption_row: plot.y - measure.digit_height(LABEL_SIZE) / 2.0,
            per_pixel: (
                if extents.time.mode == crate::time_ruler::Mode::Samples {
                    extents.time.view.len as f64
                } else {
                    extents.seconds.1 - extents.seconds.0
                } / f64::from((plot.width * scale).round().max(1.)),
                (f1 - f0) / f64::from((plot.height * scale).round().max(1.)),
            ),
        })
    }
}

/// The window's font, measured the way the tick policy needs it measured.
///
/// [`LabelMeasure::width`] has to answer the same for any digit standing in
/// any place, because the gutter is reserved from a row of zeros before a
/// single tick has been chosen. Most faces give that, and none is obliged to:
/// a font with proportional figures would make that reservation a guess, and a
/// label would then run into the plot beside it.
///
/// Two things are done about it, and neither is a proof. Tabular figures are
/// asked for through the `tnum` feature, which every face that has one uses to
/// give its digits a single advance and no kerning between them. And every
/// digit is then measured as the widest digit the face reports, so a face
/// without the feature is still measured on its widest rather than on whichever
/// digits the value happened to have.
///
/// What is left is contextual shaping between a pair of digits in a face that
/// has neither `tnum` nor uniform advances. Bounding that would mean measuring
/// every number the axis could print, and no measure can. The residue is a
/// label a pixel or two wider than the gutter allowed for, on a desktop font
/// chosen for a user interface, which is why it is named here rather than
/// solved.
pub struct Labels {
    text: std::sync::Arc<gpui::WindowTextSystem>,
    font: Font,
    font_id: FontId,
    /// The digit every other digit is measured as.
    widest: char,
}

impl Labels {
    /// Take the font the window is drawing with, asking it for tabular
    /// figures.
    pub fn new(window: &Window) -> Self {
        let text = window.text_system().clone();
        let mut font = window.text_style().font();
        font.features = tabular(&font.features);
        let font_id = text.resolve_font(&font);
        let widest = crate::numbers::current()
            .digits()
            .into_iter()
            .max_by(|a, b| advance(&text, font_id, *a).total_cmp(&advance(&text, font_id, *b)))
            .unwrap_or('0');
        Self {
            text,
            font,
            font_id,
            widest,
        }
    }

    /// One label, shaped and ready to paint.
    fn shape(&self, text: &str, color: Hsla) -> gpui::ShapedLine {
        let run = gpui::TextRun {
            len: text.len(),
            font: self.font.clone(),
            color,
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        self.text
            .shape_line(text.to_owned().into(), px(LABEL_SIZE), &[run], None)
    }

    /// Where the top of a line box goes for its ink to sit centred on `y`.
    ///
    /// A label is placed by the ink a digit puts on the canvas, which is
    /// neither the line box nor the em: a row of figures carries no descender
    /// and no ascender, so centring the box would sit every label low.
    fn line_top(&self, y: f32, line: &gpui::ShapedLine) -> f32 {
        let size = px(LABEL_SIZE);
        let cap = f32::from(self.text.cap_height(self.font_id, size));
        // Painting uses the shaped line's metrics, which can differ from the
        // font metrics (notably on Linux).
        let baseline = (LINE_HEIGHT + f32::from(line.ascent) - f32::from(line.descent)) / 2.0;
        y + cap / 2.0 - baseline
    }
}

/// The box a single label is laid out in.
const LINE_HEIGHT: f32 = LABEL_SIZE * 1.4;

/// The window's own font features with `tnum` added.
///
/// Whatever the theme asked for is kept: this adds one feature rather than
/// replacing the set, so a face configured with ligatures off stays that way.
fn tabular(features: &gpui::FontFeatures) -> gpui::FontFeatures {
    const TABULAR: &str = "tnum";
    let mut tags: Vec<(String, u32)> = features
        .tag_value_list()
        .iter()
        .filter(|(tag, _)| tag != TABULAR)
        .cloned()
        .collect();
    tags.push((TABULAR.to_owned(), 1));
    gpui::FontFeatures(std::sync::Arc::new(tags))
}

fn advance(text: &gpui::WindowTextSystem, font_id: FontId, digit: char) -> f32 {
    text.advance(font_id, px(LABEL_SIZE), digit)
        .map_or(0.0, |size| f32::from(size.width))
}

impl LabelMeasure for Labels {
    fn localize(&self, text: &str, kind: AxisKind) -> String {
        crate::numbers::current().axis_label(text, kind)
    }

    fn width(&self, text: &str, size: f32) -> f32 {
        // Digits normalized, so that what a row of zeros measures bounds what
        // any number in their place will measure. Nothing else is touched: the
        // separators and the sign shape as they will be drawn.
        let uniform: String = text
            .chars()
            .map(|c| {
                if crate::numbers::current().digits().contains(&c) {
                    self.widest
                } else {
                    c
                }
            })
            .collect();
        let run = gpui::TextRun {
            len: uniform.len(),
            font: self.font.clone(),
            color: Hsla::default(),
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        f32::from(
            self.text
                .layout_line(&uniform, px(size), &[run], None)
                .width,
        )
    }

    fn digit_height(&self, size: f32) -> f32 {
        // Cap height rather than line height: lining figures reach exactly as
        // high as a capital and no lower than the baseline, and an axis spaced
        // by the line box would leave a third of itself empty for strokes no
        // label draws.
        f32::from(self.text.cap_height(self.font_id, px(size)))
    }
}

/// The colours the marks are drawn in, taken from the window's theme.
#[derive(Debug, Clone, Copy)]
pub struct Colors {
    /// Lines crossing the picture.
    pub grid: Hsla,
    /// The ticks outside it.
    pub tick: Hsla,
    pub label: Hsla,
}

/// Draw the grid, the ticks and the labels around a plot.
///
/// `origin` is where the panel sits in the window; everything in [`Frame`] is
/// relative to it.
pub fn paint(
    frame: &Frame,
    origin: Point<Pixels>,
    labels: &Labels,
    colors: Colors,
    window: &mut Window,
    cx: &mut App,
) {
    let plot = frame.plot;
    let at = |x: f32, y: f32| point(origin.x + px(x), origin.y + px(y));
    let line = |window: &mut Window, x: f32, y: f32, w: f32, h: f32, color: Hsla| {
        window.paint_quad(fill(
            Bounds {
                origin: at(x, y),
                size: size(px(w), px(h)),
            },
            color,
        ));
    };

    for tick in &frame.time {
        let x = plot.x + tick.offset as f32;
        line(window, x, plot.y, 1.0, plot.height, colors.grid);
        line(window, x, plot.bottom(), 1.0, TICK_LEN, colors.tick);

        let shaped = labels.shape(&tick.label, colors.label);
        let left = x + LABEL_PAD;
        let top = labels.line_top(frame.time_row, &shaped);
        let _ = shaped.paint(at(left, top), px(LINE_HEIGHT), window, cx);
    }

    for tick in &frame.frequency {
        // Offset 0 is the lowest frequency, which is the bottom of the plot.
        let y = plot.bottom() - tick.offset as f32;
        line(window, plot.x, y, plot.width, 1.0, colors.grid);
        line(window, plot.right(), y, TICK_LEN, 1.0, colors.tick);

        let shaped = labels.shape(&tick.label, colors.label);
        let left = plot.right() + LABEL_PAD;
        let _ = shaped.paint(
            at(left, labels.line_top(y + 0.5, &shaped)),
            px(LINE_HEIGHT),
            window,
            cx,
        );
    }

    line(window, plot.x, plot.bottom(), plot.width, 1.0, colors.tick);
    // Join the minimap divider and close the ruler corner below.
    line(
        window,
        plot.right(),
        -1.0,
        1.0,
        plot.bottom() + 2.0,
        colors.tick,
    );

    let shaped = labels.shape(frame.time_caption, colors.label);
    let _ = shaped.paint(
        at(
            plot.right() + LABEL_PAD,
            labels.line_top(frame.time_row, &shaped),
        ),
        px(LINE_HEIGHT),
        window,
        cx,
    );

    // The unit, once, above the labels it belongs to. An axis that placed no
    // label has nothing for it to head.
    if let Some(caption) = frame.caption.filter(|_| !frame.frequency.is_empty()) {
        let shaped = labels.shape(caption, colors.label);
        let left = plot.right() + LABEL_PAD;
        let top = labels.line_top(frame.caption_row, &shaped);
        let _ = shaped.paint(at(left, top), px(LINE_HEIGHT), window, cx);
    }
}

#[cfg(test)]
mod tests {
    include!("axes_tests.rs");
}
