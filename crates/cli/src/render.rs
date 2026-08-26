//! PNG composition: axes, panels, colour bar and labels around the RGBA
//! buffer the core produced.
//!
//! The layout is worked out before the transform runs, so the spectrogram is
//! computed at exactly the pixel size it will occupy and gets blitted one to
//! one. Nothing is resampled, which is what keeps a single-frame carrier one
//! pixel wide instead of a smear.

use argand_core::{Colormap, SpectrogramImage, format_duration, format_hz};
use argand_dsp::Analysis;
use image::{Rgb, RgbImage};

use crate::text::{Align, TextRenderer};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Spectrogram,
    Psd,
    Both,
}

pub const MODE_NAMES: [&str; 3] = ["spectrogram", "psd", "both"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orientation {
    /// Time along the horizontal axis, as the editor's linked views will use.
    Horizontal,
    /// Time downwards: the familiar SDR waterfall.
    Vertical,
}

pub const ORIENTATION_NAMES: [&str; 2] = ["horizontal", "vertical"];

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("unknown {what} `{name}`, expected one of: {options}")]
pub struct ParseError {
    pub what: &'static str,
    pub name: String,
    pub options: String,
}

impl Mode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Mode::Spectrogram => "spectrogram",
            Mode::Psd => "psd",
            Mode::Both => "both",
        }
    }
}

impl std::str::FromStr for Mode {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "spectrogram" | "spec" => Ok(Mode::Spectrogram),
            "psd" | "spectrum" => Ok(Mode::Psd),
            "both" => Ok(Mode::Both),
            _ => Err(ParseError {
                what: "mode",
                name: s.to_string(),
                options: MODE_NAMES.join(", "),
            }),
        }
    }
}

impl std::fmt::Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Orientation {
    pub const fn as_str(self) -> &'static str {
        match self {
            Orientation::Horizontal => "horizontal",
            Orientation::Vertical => "vertical",
        }
    }
}

impl std::str::FromStr for Orientation {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "horizontal" | "h" => Ok(Orientation::Horizontal),
            "vertical" | "v" => Ok(Orientation::Vertical),
            _ => Err(ParseError {
                what: "orientation",
                name: s.to_string(),
                options: ORIENTATION_NAMES.join(", "),
            }),
        }
    }
}

impl std::fmt::Display for Orientation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

struct Theme;

impl Theme {
    const BACKGROUND: Rgb<u8> = Rgb([18, 20, 26]);
    const PANEL: Rgb<u8> = Rgb([12, 13, 17]);
    const TEXT: Rgb<u8> = Rgb([200, 204, 212]);
    const MUTED: Rgb<u8> = Rgb([132, 140, 156]);
    const AXIS: Rgb<u8> = Rgb([72, 79, 94]);
    const GRID: Rgb<u8> = Rgb([38, 42, 52]);
    const TRACE: Rgb<u8> = Rgb([120, 200, 255]);
}

const PAD: i64 = 12;
const HEADER_H: i64 = 24;
const FOOTER_H: i64 = 20;
const TICK_LABEL_H: i64 = 20;
const FREQ_LABEL_W: i64 = 78;
const DB_LABEL_W: i64 = 44;
const CBAR_W: i64 = 14;
const CBAR_LABEL_W: i64 = 60;
const GAP: i64 = 14;
const FONT_SIZE: f32 = 13.0;
const TITLE_SIZE: f32 = 14.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i64,
    pub y: i64,
    pub w: i64,
    pub h: i64,
}

impl Rect {
    fn right(&self) -> i64 {
        self.x + self.w
    }
    fn bottom(&self) -> i64 {
        self.y + self.h
    }
    fn is_valid(&self) -> bool {
        self.w > 0 && self.h > 0
    }
}

/// Where each piece of the plot goes.
#[derive(Debug, Clone)]
pub struct Layout {
    pub width: u32,
    pub height: u32,
    pub mode: Mode,
    pub orientation: Orientation,
    pub spectrogram: Option<Rect>,
    pub psd: Option<Rect>,
    pub colorbar: Option<Rect>,
}

impl Layout {
    /// A standalone spectrum is always drawn the conventional way round --
    /// frequency across, level up -- so orientation only applies when a
    /// spectrogram is present.
    fn psd_is_standalone(mode: Mode) -> bool {
        matches!(mode, Mode::Psd)
    }

    pub fn compute(width: u32, height: u32, mode: Mode, orientation: Orientation) -> Self {
        let (w, h) = (width as i64, height as i64);
        let content = Rect {
            x: PAD,
            y: PAD + HEADER_H,
            w: w - 2 * PAD,
            h: h - 2 * PAD - HEADER_H - FOOTER_H,
        };

        let mut layout = Self {
            width,
            height,
            mode,
            orientation,
            spectrogram: None,
            psd: None,
            colorbar: None,
        };
        if !content.is_valid() {
            return layout;
        }

        if Self::psd_is_standalone(mode) {
            layout.psd = Some(Rect {
                x: content.x + DB_LABEL_W,
                y: content.y,
                w: content.w - DB_LABEL_W,
                h: content.h - TICK_LABEL_H,
            })
            .filter(Rect::is_valid);
            return layout;
        }

        let cbar_reserve = CBAR_W + CBAR_LABEL_W + GAP;

        match orientation {
            Orientation::Horizontal => {
                let plot = Rect {
                    x: content.x + FREQ_LABEL_W,
                    y: content.y,
                    w: content.w - FREQ_LABEL_W - cbar_reserve,
                    h: content.h - TICK_LABEL_H,
                };
                if !plot.is_valid() {
                    return layout;
                }

                let psd_w = if matches!(mode, Mode::Both) {
                    ((plot.w as f64 * 0.16) as i64)
                        .clamp(110, 300)
                        .min(plot.w / 2)
                } else {
                    0
                };
                let spec_w = plot.w - if psd_w > 0 { psd_w + GAP } else { 0 };

                layout.spectrogram = Some(Rect { w: spec_w, ..plot }).filter(Rect::is_valid);
                if psd_w > 0 {
                    layout.psd = Some(Rect {
                        x: plot.right() - psd_w,
                        w: psd_w,
                        ..plot
                    })
                    .filter(Rect::is_valid);
                }
                layout.colorbar = Some(Rect {
                    x: w - PAD - CBAR_LABEL_W - CBAR_W,
                    y: plot.y,
                    w: CBAR_W,
                    h: plot.h,
                })
                .filter(Rect::is_valid);
            }
            Orientation::Vertical => {
                let plot = Rect {
                    x: content.x + DB_LABEL_W.max(FREQ_LABEL_W / 2),
                    y: content.y,
                    w: content.w - DB_LABEL_W.max(FREQ_LABEL_W / 2) - cbar_reserve,
                    h: content.h - TICK_LABEL_H,
                };
                if !plot.is_valid() {
                    return layout;
                }

                let psd_h = if matches!(mode, Mode::Both) {
                    ((plot.h as f64 * 0.22) as i64)
                        .clamp(60, 200)
                        .min(plot.h / 2)
                } else {
                    0
                };
                let spec_y = plot.y + if psd_h > 0 { psd_h + GAP } else { 0 };

                if psd_h > 0 {
                    layout.psd = Some(Rect { h: psd_h, ..plot }).filter(Rect::is_valid);
                }
                layout.spectrogram = Some(Rect {
                    y: spec_y,
                    h: plot.bottom() - spec_y,
                    ..plot
                })
                .filter(Rect::is_valid);
                layout.colorbar = Some(Rect {
                    x: w - PAD - CBAR_LABEL_W - CBAR_W,
                    y: spec_y,
                    w: CBAR_W,
                    h: plot.bottom() - spec_y,
                })
                .filter(Rect::is_valid);
            }
        }

        layout
    }

    /// Pixel size the transform should produce, so the blit is one to one.
    ///
    /// The spectrogram's own axes are time and frequency; which of those is
    /// the image's width depends on the orientation.
    pub fn transform_size(&self) -> (usize, usize) {
        match self.spectrogram {
            Some(r) => match self.orientation {
                Orientation::Horizontal => (r.w as usize, r.h as usize),
                Orientation::Vertical => (r.h as usize, r.w as usize),
            },
            // No spectrogram panel, but the transform still drives the PSD.
            None => (64, 64),
        }
    }
}

pub struct PlotInput<'a> {
    pub analysis: &'a Analysis,
    pub title: &'a str,
    pub footer: &'a str,
    pub colormap: Colormap,
}

pub fn render(layout: &Layout, input: &PlotInput<'_>) -> RgbImage {
    let mut canvas = RgbImage::from_pixel(layout.width, layout.height, Theme::BACKGROUND);
    let text = TextRenderer::new();

    text.draw(
        &mut canvas,
        input.title,
        PAD as f32,
        (PAD + 15) as f32,
        TITLE_SIZE,
        Theme::TEXT,
        Align::Left,
    );
    text.draw(
        &mut canvas,
        input.footer,
        PAD as f32,
        (layout.height as i64 - PAD + 2) as f32,
        FONT_SIZE,
        Theme::MUTED,
        Align::Left,
    );

    if let Some(rect) = layout.spectrogram {
        draw_spectrogram(&mut canvas, &text, rect, layout.orientation, input);
    }
    if let Some(rect) = layout.psd {
        draw_psd(&mut canvas, &text, rect, layout, input);
    }
    if let Some(rect) = layout.colorbar {
        draw_colorbar(&mut canvas, &text, rect, input);
    }

    canvas
}

fn draw_spectrogram(
    canvas: &mut RgbImage,
    text: &TextRenderer,
    rect: Rect,
    orientation: Orientation,
    input: &PlotInput<'_>,
) {
    let img = &input.analysis.spectrogram;
    blit(canvas, rect, img, orientation);
    frame(canvas, rect);

    match orientation {
        Orientation::Horizontal => {
            time_axis_horizontal(canvas, text, rect, img);
            freq_axis_vertical(canvas, text, rect, img, true);
        }
        Orientation::Vertical => {
            time_axis_vertical(canvas, text, rect, img);
            freq_axis_horizontal(canvas, text, rect, img);
        }
    }
}

/// Copy the spectrogram into place, rotating it for a vertical waterfall.
fn blit(canvas: &mut RgbImage, rect: Rect, img: &SpectrogramImage, orientation: Orientation) {
    for py in 0..rect.h {
        for px in 0..rect.w {
            // The transform's own axes are (time, frequency). Horizontal maps
            // them straight through; vertical turns time downwards and
            // frequency across, with low frequency on the left.
            let (sx, sy) = match orientation {
                Orientation::Horizontal => (px, py),
                Orientation::Vertical => (py, rect.w - 1 - px),
            };
            if sx < 0 || sy < 0 || sx as usize >= img.width || sy as usize >= img.height {
                continue;
            }
            let [r, g, b, _] = img.get(sx as usize, sy as usize);
            put(canvas, rect.x + px, rect.y + py, Rgb([r, g, b]));
        }
    }
}

fn draw_psd(
    canvas: &mut RgbImage,
    text: &TextRenderer,
    rect: Rect,
    layout: &Layout,
    input: &PlotInput<'_>,
) {
    fill(canvas, rect, Theme::PANEL);
    frame(canvas, rect);

    let psd = &input.analysis.psd;
    let img = &input.analysis.spectrogram;
    if psd.db.is_empty() {
        return;
    }

    // The averaged spectrum sits well below the spectrogram's per-frame
    // maxima, so sharing the colour bar's dB range would squash the whole
    // trace against one edge. The panel gets its own scale, labelled.
    let (db_min, db_max) = psd_range(&psd.db);
    let span = (db_max - db_min).max(1e-6);

    // Frequency runs along whichever axis the spectrogram is not using for
    // time, so the two panels line up bin for bin.
    let freq_vertical = matches!(layout.orientation, Orientation::Horizontal)
        && !Layout::psd_is_standalone(layout.mode);

    let level_at = |i: usize| ((psd.db[i] - db_min) / span).clamp(0.0, 1.0);

    if freq_vertical {
        // Consecutive rows are joined so a fast-moving trace reads as a line
        // rather than a dotted scatter.
        let mut previous: Option<i64> = None;
        for py in 0..rect.h {
            // Row 0 is the top of the panel: highest frequency.
            let t = (rect.h - 1 - py) as f64 / (rect.h.max(2) - 1) as f64;
            let i = ((t * (psd.db.len() - 1) as f64).round() as usize).min(psd.db.len() - 1);
            let x = rect.x + (level_at(i) * (rect.w - 1) as f32).round() as i64;
            match previous {
                Some(prev) if (prev - x).abs() > 1 => hline(
                    canvas,
                    prev.min(x),
                    prev.max(x) + 1,
                    rect.y + py,
                    Theme::TRACE,
                ),
                _ => put(canvas, x, rect.y + py, Theme::TRACE),
            }
            previous = Some(x);
        }
        db_axis_horizontal(canvas, text, rect, db_min, db_max);
    } else {
        let mut previous: Option<i64> = None;
        for px in 0..rect.w {
            let t = px as f64 / (rect.w.max(2) - 1) as f64;
            let i = ((t * (psd.db.len() - 1) as f64).round() as usize).min(psd.db.len() - 1);
            let y = rect.bottom() - 1 - (level_at(i) * (rect.h - 1) as f32).round() as i64;
            match previous {
                Some(prev) if (prev - y).abs() > 1 => vline(
                    canvas,
                    rect.x + px,
                    prev.min(y),
                    prev.max(y) + 1,
                    Theme::TRACE,
                ),
                _ => put(canvas, rect.x + px, y, Theme::TRACE),
            }
            previous = Some(y);
        }
        db_axis_vertical(canvas, text, rect, db_min, db_max);
        if Layout::psd_is_standalone(layout.mode) {
            freq_axis_horizontal(canvas, text, rect, img);
        }
    }
}

/// dB bounds that fit the trace with a little air around it.
fn psd_range(db: &[f32]) -> (f32, f32) {
    let finite = || db.iter().copied().filter(|v| v.is_finite());
    let lo = finite().fold(f32::INFINITY, f32::min);
    let hi = finite().fold(f32::NEG_INFINITY, f32::max);
    if !lo.is_finite() || !hi.is_finite() {
        return (-120.0, 0.0);
    }
    let margin = ((hi - lo) * 0.08).max(1.0);
    let lo = hi - (hi - lo + margin).max(10.0);
    (lo, hi + margin)
}

fn draw_colorbar(canvas: &mut RgbImage, text: &TextRenderer, rect: Rect, input: &PlotInput<'_>) {
    let gradient = input.colormap.gradient();
    let img = &input.analysis.spectrogram;

    for py in 0..rect.h {
        // Strongest at the top.
        let t = (rect.h - 1 - py) as f32 / (rect.h.max(2) - 1) as f32;
        let color = gradient[argand_core::gradient_index(t)];
        for px in 0..rect.w {
            put(canvas, rect.x + px, rect.y + py, Rgb(color));
        }
    }
    frame(canvas, rect);

    for value in nice_ticks(img.db_min as f64, img.db_max as f64, 5) {
        let t = (value - img.db_min as f64) / (img.db_max - img.db_min).max(1e-6) as f64;
        let y = rect.bottom() - 1 - (t * (rect.h - 1) as f64).round() as i64;
        if y < rect.y || y >= rect.bottom() {
            continue;
        }
        hline(canvas, rect.right(), rect.right() + 3, y, Theme::AXIS);
        text.draw(
            canvas,
            &format!("{value:.0} dB"),
            (rect.right() + 6) as f32,
            (y + 4) as f32,
            FONT_SIZE,
            Theme::MUTED,
            Align::Left,
        );
    }
}

fn time_axis_horizontal(
    canvas: &mut RgbImage,
    text: &TextRenderer,
    rect: Rect,
    img: &SpectrogramImage,
) {
    for value in nice_time_ticks(img.t0, img.t1, 8) {
        let t = (value - img.t0) / (img.t1 - img.t0).max(1e-9);
        let x = rect.x + (t * (rect.w - 1) as f64).round() as i64;
        if x < rect.x || x >= rect.right() {
            continue;
        }
        vline(canvas, x, rect.y, rect.bottom(), Theme::GRID);
        vline(canvas, x, rect.bottom(), rect.bottom() + 3, Theme::AXIS);
        text.draw(
            canvas,
            &time_label(value),
            x as f32,
            (rect.bottom() + 16) as f32,
            FONT_SIZE,
            Theme::MUTED,
            Align::Center,
        );
    }
}

fn time_axis_vertical(
    canvas: &mut RgbImage,
    text: &TextRenderer,
    rect: Rect,
    img: &SpectrogramImage,
) {
    for value in nice_time_ticks(img.t0, img.t1, 6) {
        let t = (value - img.t0) / (img.t1 - img.t0).max(1e-9);
        let y = rect.y + (t * (rect.h - 1) as f64).round() as i64;
        if y < rect.y || y >= rect.bottom() {
            continue;
        }
        hline(canvas, rect.x, rect.right(), y, Theme::GRID);
        hline(canvas, rect.x - 3, rect.x, y, Theme::AXIS);
        text.draw(
            canvas,
            &time_label(value),
            (rect.x - 6) as f32,
            (y + 4) as f32,
            FONT_SIZE,
            Theme::MUTED,
            Align::Right,
        );
    }
}

fn freq_axis_vertical(
    canvas: &mut RgbImage,
    text: &TextRenderer,
    rect: Rect,
    img: &SpectrogramImage,
    labels: bool,
) {
    for value in nice_ticks(img.f0, img.f1, 6) {
        let t = (value - img.f0) / (img.f1 - img.f0).max(1e-9);
        // Highest frequency at the top.
        let y = rect.bottom() - 1 - (t * (rect.h - 1) as f64).round() as i64;
        if y < rect.y || y >= rect.bottom() {
            continue;
        }
        hline(canvas, rect.x, rect.right(), y, Theme::GRID);
        if labels {
            hline(canvas, rect.x - 3, rect.x, y, Theme::AXIS);
            text.draw(
                canvas,
                &format_hz(value),
                (rect.x - 6) as f32,
                (y + 4) as f32,
                FONT_SIZE,
                Theme::MUTED,
                Align::Right,
            );
        }
    }
}

fn freq_axis_horizontal(
    canvas: &mut RgbImage,
    text: &TextRenderer,
    rect: Rect,
    img: &SpectrogramImage,
) {
    for value in nice_ticks(img.f0, img.f1, 6) {
        let t = (value - img.f0) / (img.f1 - img.f0).max(1e-9);
        let x = rect.x + (t * (rect.w - 1) as f64).round() as i64;
        if x < rect.x || x >= rect.right() {
            continue;
        }
        vline(canvas, x, rect.y, rect.bottom(), Theme::GRID);
        vline(canvas, x, rect.bottom(), rect.bottom() + 3, Theme::AXIS);
        text.draw(
            canvas,
            &format_hz(value),
            x as f32,
            (rect.bottom() + 16) as f32,
            FONT_SIZE,
            Theme::MUTED,
            Align::Center,
        );
    }
}

fn db_axis_horizontal(
    canvas: &mut RgbImage,
    text: &TextRenderer,
    rect: Rect,
    db_min: f32,
    db_max: f32,
) {
    for value in nice_ticks(db_min as f64, db_max as f64, 3) {
        let t = (value - db_min as f64) / (db_max - db_min).max(1e-6) as f64;
        let x = rect.x + (t * (rect.w - 1) as f64).round() as i64;
        if x < rect.x || x >= rect.right() {
            continue;
        }
        vline(canvas, x, rect.y, rect.bottom(), Theme::GRID);
        text.draw(
            canvas,
            &format!("{value:.0}"),
            x as f32,
            (rect.bottom() + 16) as f32,
            FONT_SIZE,
            Theme::MUTED,
            Align::Center,
        );
    }
}

fn db_axis_vertical(
    canvas: &mut RgbImage,
    text: &TextRenderer,
    rect: Rect,
    db_min: f32,
    db_max: f32,
) {
    for value in nice_ticks(db_min as f64, db_max as f64, 4) {
        let t = (value - db_min as f64) / (db_max - db_min).max(1e-6) as f64;
        let y = rect.bottom() - 1 - (t * (rect.h - 1) as f64).round() as i64;
        if y < rect.y || y >= rect.bottom() {
            continue;
        }
        hline(canvas, rect.x, rect.right(), y, Theme::GRID);
        text.draw(
            canvas,
            &format!("{value:.0}"),
            (rect.x - 6) as f32,
            (y + 4) as f32,
            FONT_SIZE,
            Theme::MUTED,
            Align::Right,
        );
    }
}

/// Axis labels want a bare zero, not the report's "0ms".
fn time_label(seconds: f64) -> String {
    if seconds == 0.0 {
        "0".to_string()
    } else {
        format_duration(seconds)
    }
}

/// Tick positions for a time axis, in seconds.
///
/// Decimal steps put marks at 250-second intervals, which read as "8m20" and
/// tell nobody anything. Clocks step in 1, 5, 15, 30 and 60, so the axis does
/// too, falling back to decimal steps below a second.
pub fn nice_time_ticks(min: f64, max: f64, target: usize) -> Vec<f64> {
    const STEPS: [f64; 14] = [
        1.0, 2.0, 5.0, 10.0, 15.0, 30.0, // seconds
        60.0, 120.0, 300.0, 600.0, 900.0, 1800.0, // minutes
        3600.0, 7200.0, // hours
    ];
    if !min.is_finite() || !max.is_finite() || max <= min || target == 0 {
        return Vec::new();
    }

    let raw = (max - min) / target as f64;
    if raw < 1.0 {
        return nice_ticks(min, max, target);
    }
    let step = STEPS
        .iter()
        .copied()
        .find(|&s| s >= raw)
        // Past two hours, go back to round multiples of an hour.
        .unwrap_or_else(|| (raw / 3600.0).ceil() * 3600.0);

    ticks_from_step(min, max, step, target)
}

/// Round tick positions to values a reader can hold in their head.
pub fn nice_ticks(min: f64, max: f64, target: usize) -> Vec<f64> {
    if !min.is_finite() || !max.is_finite() || max <= min || target == 0 {
        return Vec::new();
    }
    let raw = (max - min) / target as f64;
    let magnitude = 10f64.powf(raw.log10().floor());
    let normalized = raw / magnitude;
    let step = magnitude
        * if normalized <= 1.0 {
            1.0
        } else if normalized <= 2.0 {
            2.0
        } else if normalized <= 5.0 {
            5.0
        } else {
            10.0
        };

    ticks_from_step(min, max, step, target)
}

fn ticks_from_step(min: f64, max: f64, step: f64, target: usize) -> Vec<f64> {
    if step <= 0.0 {
        return Vec::new();
    }
    let mut ticks = Vec::new();
    let mut value = (min / step).ceil() * step;
    // Guard against a step that rounding made useless.
    for _ in 0..(target * 4 + 4) {
        if value > max + step * 1e-9 {
            break;
        }
        // Snap away the float dust that ceil and multiply leave behind.
        ticks.push(if value.abs() < step * 1e-9 {
            0.0
        } else {
            value
        });
        value += step;
    }
    ticks
}

fn put(canvas: &mut RgbImage, x: i64, y: i64, color: Rgb<u8>) {
    if x >= 0 && y >= 0 && (x as u32) < canvas.width() && (y as u32) < canvas.height() {
        canvas.put_pixel(x as u32, y as u32, color);
    }
}

fn fill(canvas: &mut RgbImage, rect: Rect, color: Rgb<u8>) {
    for y in rect.y..rect.bottom() {
        for x in rect.x..rect.right() {
            put(canvas, x, y, color);
        }
    }
}

fn hline(canvas: &mut RgbImage, x0: i64, x1: i64, y: i64, color: Rgb<u8>) {
    for x in x0.min(x1)..x0.max(x1) {
        put(canvas, x, y, color);
    }
}

fn vline(canvas: &mut RgbImage, x: i64, y0: i64, y1: i64, color: Rgb<u8>) {
    for y in y0.min(y1)..y0.max(y1) {
        put(canvas, x, y, color);
    }
}

fn frame(canvas: &mut RgbImage, rect: Rect) {
    hline(
        canvas,
        rect.x - 1,
        rect.right() + 1,
        rect.y - 1,
        Theme::AXIS,
    );
    hline(
        canvas,
        rect.x - 1,
        rect.right() + 1,
        rect.bottom(),
        Theme::AXIS,
    );
    vline(
        canvas,
        rect.x - 1,
        rect.y - 1,
        rect.bottom() + 1,
        Theme::AXIS,
    );
    vline(
        canvas,
        rect.right(),
        rect.y - 1,
        rect.bottom() + 1,
        Theme::AXIS,
    );
}

#[cfg(test)]
mod tests {
    include!("render_tests.rs");
}
