//! PNG composition: axes, panels, colour bar and labels around the RGBA
//! buffer the core produced.
//!
//! The layout is worked out before the transform runs, so the spectrogram is
//! computed at exactly the pixel size it will occupy and gets blitted one to
//! one. Nothing is resampled, which is what keeps a single-frame carrier one
//! pixel wide instead of a smear.

use argand_core::{Colormap, Psd, SpectrogramImage, WaveformEnvelope};
use argand_dsp::{Analysis, DbReference};
use image::{Rgb, RgbImage};

use crate::text::{Anchor, TextRenderer, TextStyle};
use crate::ticks::{self, Axis, AxisKind, LabelMetrics, LabelRun, Tick};

const EMPTY_PANELS_NAME: &str = "none";

macro_rules! cli_enum {
    (
        $(#[$enum_meta:meta])*
        $visibility:vis enum $name:ident {
            $(
                $(#[$variant_meta:meta])*
                $variant:ident => $canonical:literal
            ),+ $(,)?
        }
    ) => {
        $(#[$enum_meta])*
        $visibility enum $name {
            $($(#[$variant_meta])* $variant),+
        }

        impl $name {
            const ALL: &'static [Self] = &[$(Self::$variant),+];

            pub const fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $canonical),+
                }
            }
        }
    };
}

cli_enum! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Panel {
        Waveform => "waveform",
        Psd => "psd",
        Db => "db",
    }
}

impl Panel {
    const fn aliases(self) -> &'static [&'static str] {
        match self {
            Self::Waveform => &["wave"],
            Self::Psd => &["spectrum"],
            Self::Db => &["colorbar"],
        }
    }

    fn from_name(name: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|panel| panel.as_str() == name || panel.aliases().contains(&name))
    }

    const fn is_enabled(self, panels: Panels) -> bool {
        match self {
            Self::Waveform => panels.waveform,
            Self::Psd => panels.psd,
            Self::Db => panels.db,
        }
    }

    fn enable(self, panels: &mut Panels) {
        match self {
            Self::Waveform => panels.waveform = true,
            Self::Psd => panels.psd = true,
            Self::Db => panels.db = true,
        }
    }
}

/// Panels drawn beside the spectrogram.
///
/// The spectrogram itself is not selectable. It is the point of the tool, so
/// `--panels` names only what joins it, and `none` renders it on its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Panels {
    pub waveform: bool,
    pub psd: bool,
    pub db: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ParsePanelsError {
    #[error("unknown panel `{name}`, expected one of: {options}")]
    Unknown { name: String, options: String },
    #[error("no panels given; use `{empty}` for the spectrogram on its own")]
    Empty { empty: &'static str },
    #[error("`{empty}` cannot be combined with other panels")]
    NoneWithOthers { empty: &'static str },
}

impl Panels {
    pub const ALL: Self = Self {
        waveform: true,
        psd: true,
        db: true,
    };

    pub const NONE: Self = Self {
        waveform: false,
        psd: false,
        db: false,
    };

    pub const WAVEFORM: Self = Self {
        waveform: true,
        psd: false,
        db: false,
    };

    pub fn names(self) -> Vec<&'static str> {
        Panel::ALL
            .iter()
            .copied()
            .filter(|panel| panel.is_enabled(self))
            .map(Panel::as_str)
            .collect()
    }

    fn options() -> String {
        Panel::ALL
            .iter()
            .copied()
            .map(Panel::as_str)
            .chain(std::iter::once(EMPTY_PANELS_NAME))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

pub(crate) fn panels_help() -> String {
    format!("Panels beside the spectrogram [{}]", Panels::options())
}

pub(crate) fn panels_overview() -> String {
    let panels = Panel::ALL
        .iter()
        .copied()
        .map(|panel| {
            if panel == Panel::Db {
                format!("{} (the colour bar)", panel.as_str())
            } else {
                panel.as_str().to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!("{panels}, or {} for the spectrogram alone.", Panels::NONE)
}

impl std::str::FromStr for Panels {
    type Err = ParsePanelsError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut panels = Panels::NONE;
        let mut explicit_none = false;
        let mut count = 0;

        for token in s.split(',') {
            let token = token.trim().to_ascii_lowercase();
            if token.is_empty() {
                continue;
            }
            count += 1;
            if token == EMPTY_PANELS_NAME {
                explicit_none = true;
                continue;
            }

            let Some(panel) = Panel::from_name(&token) else {
                return Err(ParsePanelsError::Unknown {
                    name: token,
                    options: Panels::options(),
                });
            };
            panel.enable(&mut panels);
        }

        match (count, explicit_none) {
            (0, _) => Err(ParsePanelsError::Empty {
                empty: EMPTY_PANELS_NAME,
            }),
            (1, true) => Ok(Panels::NONE),
            (_, true) => Err(ParsePanelsError::NoneWithOthers {
                empty: EMPTY_PANELS_NAME,
            }),
            _ => Ok(panels),
        }
    }
}

impl std::fmt::Display for Panels {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let names = self.names();
        if names.is_empty() {
            return f.write_str(EMPTY_PANELS_NAME);
        }
        f.write_str(&names.join(","))
    }
}

cli_enum! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Orientation {
        /// Time along the horizontal axis, as the editor's linked views will use.
        Horizontal => "horizontal",
        /// Time downwards: the familiar SDR waterfall.
        Vertical => "vertical",
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("unknown {what} `{name}`, expected one of: {options}")]
pub struct ParseError {
    pub what: &'static str,
    pub name: String,
    pub options: String,
}

impl Orientation {
    const fn short_alias(self) -> &'static str {
        match self {
            Self::Horizontal => "h",
            Self::Vertical => "v",
        }
    }

    const fn help_description(self) -> &'static str {
        match self {
            Self::Horizontal => "across",
            Self::Vertical => "waterfall",
        }
    }

    fn from_name(name: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|orientation| orientation.as_str() == name || orientation.short_alias() == name)
    }

    fn options() -> String {
        Self::ALL
            .iter()
            .copied()
            .map(Self::as_str)
            .collect::<Vec<_>>()
            .join(", ")
    }
}

pub(crate) fn orientation_help() -> String {
    let orientations = Orientation::ALL
        .iter()
        .map(|orientation| format!("{orientation} ({})", orientation.help_description()))
        .collect::<Vec<_>>()
        .join(" or ");
    format!("Time axis direction: {orientations}")
}

pub(crate) fn vertical_orientation_alias() -> &'static str {
    Orientation::Vertical.short_alias()
}

impl std::str::FromStr for Orientation {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let name = s.trim().to_ascii_lowercase();
        Orientation::from_name(&name).ok_or_else(|| ParseError {
            what: "orientation",
            name: s.to_string(),
            options: Orientation::options(),
        })
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
const CBAR_W: i64 = 14;
const GAP: i64 = 14;
/// Clear space between a label and the plot edge it is aligned against.
const LABEL_PAD: i64 = 6;
/// The waveform is a mini-map rather than a panel that grows with the image.
const WAVEFORM_SPAN: i64 = 64;
const FONT_SIZE: f32 = 13.0;
const TITLE_SIZE: f32 = 14.0;

/// The lowest decibel the transform can produce.
///
/// It clamps silence rather than letting it reach `-inf`, and `argand-dsp`
/// publishes where: a test there keeps the two clamps that produce this honest,
/// so it cannot drift away from what is drawn.
///
/// It is a floor on the data, not on every axis drawn from it. The spectrum
/// panel puts air around its trace, so its own scale reaches a little below
/// this; three digits and a sign still cover that, which is what the gutter is
/// reserved for and what a test here holds to.
pub const DB_FLOOR: f64 = argand_dsp::DB_FLOOR as f64;

/// The plot's heading.
const TITLE: TextStyle = TextStyle {
    size: TITLE_SIZE,
    color: Theme::TEXT,
};
/// Every axis tick, footer and legend label.
const LABEL: TextStyle = TextStyle {
    size: FONT_SIZE,
    color: Theme::MUTED,
};

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

/// Room the stacked axis labels need beside the plot.
///
/// Measured before the transform runs, because the transform is asked for
/// exactly the pixels the spectrogram will occupy and how many that is depends
/// on what the labels take. The fixed gutters this replaced predated the
/// frequency formatter: a capture centred on 12.579 MHz prints
/// `12.579887 MHz`, half again wider than the 78 pixels reserved for it, so
/// the label ran off the left of the canvas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Gutters {
    /// Frequency labels, left of a horizontal plot.
    frequency: i64,
    /// Time labels, left of a vertical plot.
    time: i64,
    /// The spectrum panel's decibels, left of a vertical plot.
    decibels: i64,
    /// The colour bar's decibels, right of the bar.
    colorbar: i64,
}

impl Gutters {
    /// Measure the widest label each axis could print, in the font and at the
    /// size the plot will draw with.
    ///
    /// `decibels` is the widest window the colour bar can be asked to show,
    /// which `--dynamic-range` and `--ref` decide between them. The spectrum
    /// panel's own scale follows its trace and answers to neither, so it keeps
    /// [`DB_FLOOR`] whatever the colour bar was told to span.
    pub fn measure(time: (f64, f64), frequency: (f64, f64), decibels: (f64, f64)) -> Self {
        let text = TextRenderer::new();
        // Every candidate is measured, because which of two strings needs more
        // room is a question about glyphs, not about characters.
        // The caption counts too: on a narrow range `MHz` is wider than the
        // digits it heads.
        let width = |kind: AxisKind, (min, max): (f64, f64)| -> i64 {
            ticks::widest_labels(kind, min, max)
                .iter()
                .map(String::as_str)
                .chain(ticks::caption(kind, min, max))
                .map(|label| text.width(label, FONT_SIZE).ceil() as i64)
                .max()
                .unwrap_or(0)
        };
        Self {
            frequency: width(AxisKind::Frequency, frequency),
            time: width(AxisKind::Time, time),
            decibels: width(AxisKind::Decibels, (DB_FLOOR, 0.0)),
            colorbar: width(AxisKind::Decibels, decibels),
        }
    }

    /// What the frequency labels take out of the left of a horizontal plot.
    const fn across(self) -> i64 {
        self.frequency + LABEL_PAD
    }

    /// What the labels take out of the left of a vertical plot.
    ///
    /// The time labels are always there; the spectrum panel's decibels share
    /// the same gutter, but only when the panel is up. Reserving for a panel
    /// nobody asked for narrows the waterfall for nothing.
    fn down(self, panels: Panels) -> i64 {
        let stacked = if panels.psd {
            self.time.max(self.decibels)
        } else {
            self.time
        };
        stacked + LABEL_PAD
    }

    /// What the colour bar and its labels take out of the right of either.
    const fn colorbar(self, panels: Panels) -> i64 {
        if panels.db {
            CBAR_W + LABEL_PAD + self.colorbar + GAP
        } else {
            0
        }
    }
}

/// Where each piece of the plot goes.
///
/// The spectrogram is unconditional; `panels` only decides what joins it. The
/// waveform strip shares the spectrogram's time axis -- its width when time
/// runs across, its height when time runs down -- so a burst can be traced
/// from one panel into the other.
#[derive(Debug, Clone)]
pub struct Layout {
    pub width: u32,
    pub height: u32,
    pub orientation: Orientation,
    pub spectrogram: Option<Rect>,
    pub waveform: Option<Rect>,
    pub psd: Option<Rect>,
    pub colorbar: Option<Rect>,
}

impl Layout {
    /// Time across the image: the strip sits above the spectrogram.
    fn place_time_across(&mut self, content: Rect, panels: Panels, gutters: Gutters, cbar_x: i64) {
        let plot = Rect {
            x: content.x + gutters.across(),
            y: content.y,
            w: content.w - gutters.across() - gutters.colorbar(panels),
            h: content.h - TICK_LABEL_H,
        };
        if !plot.is_valid() {
            return;
        }

        // The spectrum takes its column first: it is the only panel
        // whose width carries meaning.
        let psd_w = if panels.psd {
            ((plot.w as f64 * 0.16) as i64)
                .clamp(110, 300)
                .min(plot.w / 2)
        } else {
            0
        };
        let time_w = plot.w - if psd_w > 0 { psd_w + GAP } else { 0 };

        let strip_h = if panels.waveform {
            WAVEFORM_SPAN.min(plot.h / 3)
        } else {
            0
        };
        let spec_y = plot.y + if strip_h > 0 { strip_h + GAP } else { 0 };
        let spec_h = plot.bottom() - spec_y;

        if strip_h > 0 {
            self.waveform = Some(Rect {
                w: time_w,
                h: strip_h,
                ..plot
            })
            .filter(Rect::is_valid);
        }
        self.spectrogram = Some(Rect {
            y: spec_y,
            w: time_w,
            h: spec_h,
            ..plot
        })
        .filter(Rect::is_valid);
        if psd_w > 0 {
            // Aligned to the spectrogram's frequency axis, not to the
            // strip above it: a bin has to sit on its own row.
            self.psd = Some(Rect {
                x: plot.right() - psd_w,
                y: spec_y,
                w: psd_w,
                h: spec_h,
            })
            .filter(Rect::is_valid);
        }
        if panels.db {
            self.colorbar = Some(Rect {
                x: cbar_x,
                y: spec_y,
                w: CBAR_W,
                h: spec_h,
            })
            .filter(Rect::is_valid);
        }
    }

    /// Time down the image: the strip sits to the spectrogram's right.
    fn place_time_down(&mut self, content: Rect, panels: Panels, gutters: Gutters, cbar_x: i64) {
        let plot = Rect {
            x: content.x + gutters.down(panels),
            y: content.y,
            w: content.w - gutters.down(panels) - gutters.colorbar(panels),
            h: content.h - TICK_LABEL_H,
        };
        if !plot.is_valid() {
            return;
        }

        let psd_h = if panels.psd {
            ((plot.h as f64 * 0.22) as i64)
                .clamp(60, 200)
                .min(plot.h / 2)
        } else {
            0
        };
        let time_y = plot.y + if psd_h > 0 { psd_h + GAP } else { 0 };
        let time_h = plot.bottom() - time_y;

        let strip_w = if panels.waveform {
            WAVEFORM_SPAN.min(plot.w / 3)
        } else {
            0
        };
        let spec_w = plot.w - if strip_w > 0 { strip_w + GAP } else { 0 };

        self.spectrogram = Some(Rect {
            y: time_y,
            w: spec_w,
            h: time_h,
            ..plot
        })
        .filter(Rect::is_valid);
        if strip_w > 0 {
            self.waveform = Some(Rect {
                x: plot.x + spec_w + GAP,
                y: time_y,
                w: strip_w,
                h: time_h,
            })
            .filter(Rect::is_valid);
        }
        if psd_h > 0 {
            self.psd = Some(Rect {
                w: spec_w,
                h: psd_h,
                ..plot
            })
            .filter(Rect::is_valid);
        }
        if panels.db {
            self.colorbar = Some(Rect {
                x: cbar_x,
                y: time_y,
                w: CBAR_W,
                h: time_h,
            })
            .filter(Rect::is_valid);
        }
    }

    pub fn compute(
        width: u32,
        height: u32,
        panels: Panels,
        orientation: Orientation,
        gutters: Gutters,
    ) -> Self {
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
            orientation,
            spectrogram: None,
            waveform: None,
            psd: None,
            colorbar: None,
        };
        if !content.is_valid() {
            return layout;
        }

        let cbar_x = w - PAD - LABEL_PAD - gutters.colorbar - CBAR_W;

        match orientation {
            Orientation::Horizontal => layout.place_time_across(content, panels, gutters, cbar_x),
            Orientation::Vertical => layout.place_time_down(content, panels, gutters, cbar_x),
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
            None => (0, 0),
        }
    }

    /// Envelope columns the strip needs, along whichever axis carries time.
    pub fn waveform_columns(&self) -> Option<usize> {
        self.waveform.map(|r| match self.orientation {
            Orientation::Horizontal => r.w as usize,
            Orientation::Vertical => r.h as usize,
        })
    }
}

pub struct PlotInput<'a> {
    pub analysis: &'a Analysis,
    pub title: &'a str,
    pub footer: &'a str,
    pub colormap: Colormap,
    /// Sample value the edge of the waveform strip stands for.
    pub waveform_full_scale: f32,
}

/// The level the edge of the strip stands for, following `--ref`.
///
/// The reference is read in the time domain: the loudest *sample* rather than
/// the loudest bin, because a sample is what the strip actually draws.
///
/// The scale is linear. A decibel strip was tried first, so that `-d` would
/// size the strip and the colour bar alike, but a min/max span in decibels
/// pins almost anything above the noise to the edges: a capture at -6 dBFS
/// fills 90% of the half-height, and the shape the strip exists to show
/// disappears into a solid band.
pub fn waveform_full_scale(time_peak: f32, reference: DbReference) -> f32 {
    match reference {
        DbReference::FullScale => 1.0,
        DbReference::Peak => time_peak.max(1e-6),
    }
}

/// Whether an axis draws its labels, or only the grid lines that go with them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Labelled {
    Yes,
    No,
}

/// What every panel draws against.
///
/// The time and frequency ticks are chosen once, from the spectrogram's
/// geometry, and handed to each panel that shares those axes. Choosing them
/// twice and trusting the two to agree is how grid lines drift apart.
struct Scene<'a> {
    text: &'a TextRenderer,
    input: &'a PlotInput<'a>,
    layout: &'a Layout,
    /// Half a digit's ink: what centres a stacked label on its tick, and what
    /// the outermost one borrows past the end of its axis.
    rise: i64,
    /// Baseline drop from a plot's lower edge to the labels underneath it.
    drop: i64,
    time: Vec<Tick>,
    frequency: Vec<Tick>,
}

impl<'a> Scene<'a> {
    fn new(layout: &'a Layout, input: &'a PlotInput<'a>, text: &'a TextRenderer) -> Self {
        let ink = f64::from(text.digit_height(FONT_SIZE));
        let rise = (ink / 2.0).round() as i64;
        let img = &input.analysis.spectrogram;
        Self {
            text,
            input,
            layout,
            rise,
            drop: ink.ceil() as i64 + LABEL_PAD,
            time: time_ticks(layout, img, text, rise),
            frequency: frequency_ticks(layout, img, text, rise),
        }
    }

    /// Labels side by side along their axis.
    fn along(&self) -> LabelMetrics<'_> {
        LabelMetrics::new(self.text, FONT_SIZE, LabelRun::Across)
    }

    /// Labels stacked across it.
    fn stacked(&self) -> LabelMetrics<'_> {
        LabelMetrics::new(self.text, FONT_SIZE, LabelRun::Down)
    }

    /// Baseline for a label centred on `y` beside a plot.
    fn beside(&self, y: i64) -> f32 {
        (y + self.rise) as f32
    }

    /// Baseline for a label under a plot whose lower edge is `bottom`.
    fn under(&self, bottom: i64) -> f32 {
        (bottom + self.drop) as f32
    }

    /// Name the unit once at the head of a stacked axis's label column.
    ///
    /// The axis is told to keep its labels clear of `caption_rows` at that end,
    /// so this cannot land on one. An axis that produced no labels is left
    /// alone: a unit with nothing under it explains nothing.
    fn caption_above(&self, canvas: &mut RgbImage, at: Anchor, marks: &[Tick], caption: &str) {
        if marks.is_empty() {
            return;
        }
        self.text.draw(canvas, caption, at, LABEL);
    }

    /// Name it once past the last label of an axis its labels run along.
    ///
    /// The room past that label is what the axis reserved, so an axis with no
    /// labels has nowhere the caption is known to fit, and gets none.
    fn caption_after(&self, canvas: &mut RgbImage, rect: Rect, marks: &[Tick], caption: &str) {
        let Some(last) = marks.last() else {
            return;
        };
        let end = rect.x + last.offset + self.text.width(&last.label, FONT_SIZE).ceil() as i64 / 2;
        self.text.draw(
            canvas,
            caption,
            Anchor::left((end + LABEL_PAD) as f32, self.under(rect.bottom())),
            LABEL,
        );
    }

    /// Ticks running left to right: a grid line each, and labels underneath.
    fn across(&self, canvas: &mut RgbImage, rect: Rect, ticks: &[Tick], labelled: Labelled) {
        for tick in ticks {
            let x = rect.x + tick.offset;
            vline(canvas, x, rect.y, rect.bottom(), Theme::GRID);
            if labelled == Labelled::No {
                continue;
            }
            vline(canvas, x, rect.bottom(), rect.bottom() + 3, Theme::AXIS);
            self.text.draw(
                canvas,
                &tick.label,
                Anchor::center(x as f32, self.under(rect.bottom())),
                LABEL,
            );
        }
    }

    /// Ticks running top to bottom, which is how time falls on a waterfall.
    fn down(&self, canvas: &mut RgbImage, rect: Rect, ticks: &[Tick], labelled: Labelled) {
        for tick in ticks {
            self.row(canvas, rect, rect.y + tick.offset, tick, labelled);
        }
    }

    /// Ticks running bottom to top, which is how frequency and decibels rise.
    fn up(&self, canvas: &mut RgbImage, rect: Rect, ticks: &[Tick], labelled: Labelled) {
        for tick in ticks {
            self.row(
                canvas,
                rect,
                rect.bottom() - 1 - tick.offset,
                tick,
                labelled,
            );
        }
    }

    /// One horizontal grid line, with its label right-aligned in the gutter.
    fn row(&self, canvas: &mut RgbImage, rect: Rect, y: i64, tick: &Tick, labelled: Labelled) {
        hline(canvas, rect.x, rect.right(), y, Theme::GRID);
        if labelled == Labelled::No {
            return;
        }
        hline(canvas, rect.x - 3, rect.x, y, Theme::AXIS);
        self.text.draw(
            canvas,
            &tick.label,
            Anchor::right((rect.x - LABEL_PAD) as f32, self.beside(y)),
            LABEL,
        );
    }
}

/// Rows a caption takes at the head of a stacked axis's label column.
///
/// The unit is named once there instead of on every tick, so the axis is told
/// to keep its labels clear of that much of its own far end.
const fn caption_rows(rise: i64) -> i64 {
    2 * rise + LABEL_PAD
}

/// Pixels a caption takes past the end of an axis its labels run along.
fn caption_run(text: &TextRenderer, caption: &str) -> i64 {
    if caption.is_empty() {
        return 0;
    }
    text.width(caption, FONT_SIZE).ceil() as i64 + LABEL_PAD
}

/// Pixels a label may take to the right of `edge`.
///
/// Two panels that label the same row meet halfway across the gap between
/// them, so neither has to know how wide the other's labels are. With nothing
/// beside it, a label runs to the edge of the canvas.
fn room_right(edge: i64, neighbour: Option<i64>, canvas: u32) -> i64 {
    neighbour.map_or(i64::from(canvas) - edge, |left| (left - edge) / 2)
}

/// Pixels a label may take to the left of `edge`, by the same rule.
fn room_left(edge: i64, neighbour: Option<i64>) -> i64 {
    neighbour.map_or(edge, |right| (edge - right) / 2)
}

/// The time ticks the spectrogram and the waveform strip both draw.
fn time_ticks(
    layout: &Layout,
    img: &SpectrogramImage,
    text: &TextRenderer,
    rise: i64,
) -> Vec<Tick> {
    let Some(rect) = layout.spectrogram else {
        return Vec::new();
    };
    let (axis, run) = match layout.orientation {
        // Labels sit in a row under the plot. The canvas bounds them on the
        // left; on the right they meet the spectrum panel's decibel labels,
        // which share that row.
        Orientation::Horizontal => (
            Axis {
                length: rect.w,
                min: img.t0,
                max: img.t1,
                lead: rect.x,
                trail: room_right(rect.right(), layout.psd.map(|p| p.x), layout.width),
            },
            LabelRun::Across,
        ),
        // Labels stack in the left gutter, which was measured to hold them.
        Orientation::Vertical => (
            Axis {
                length: rect.h,
                min: img.t0,
                max: img.t1,
                lead: rise,
                trail: rise,
            },
            LabelRun::Down,
        ),
    };
    ticks::ticks(
        AxisKind::Time,
        axis,
        &LabelMetrics::new(text, FONT_SIZE, run),
    )
}

/// The frequency ticks the spectrogram and the spectrum panel both draw.
fn frequency_ticks(
    layout: &Layout,
    img: &SpectrogramImage,
    text: &TextRenderer,
    rise: i64,
) -> Vec<Tick> {
    let Some(rect) = layout.spectrogram else {
        return Vec::new();
    };
    // The unit is named once at the far end of the axis, so the labels are kept
    // clear of the room it takes rather than drawn over it.
    let caption = ticks::caption(AxisKind::Frequency, img.f0, img.f1).unwrap_or_default();
    let (axis, run) = match layout.orientation {
        // Upwards beside the plot: offset 0 is the lowest frequency, at the
        // bottom, and the caption heads the column at the top.
        Orientation::Horizontal => (
            Axis {
                length: rect.h,
                min: img.f0,
                max: img.f1,
                lead: rise,
                trail: -caption_rows(rise),
            },
            LabelRun::Down,
        ),
        // Across, under the plot, with the caption after the last label.
        Orientation::Vertical => (
            Axis {
                length: rect.w,
                min: img.f0,
                max: img.f1,
                lead: rect.x,
                trail: i64::from(layout.width) - rect.right() - caption_run(text, caption),
            },
            LabelRun::Across,
        ),
    };
    ticks::ticks(
        AxisKind::Frequency,
        axis,
        &LabelMetrics::new(text, FONT_SIZE, run),
    )
}

pub fn render(layout: &Layout, input: &PlotInput<'_>) -> RgbImage {
    let mut canvas = RgbImage::from_pixel(layout.width, layout.height, Theme::BACKGROUND);
    let text = TextRenderer::new();
    let scene = Scene::new(layout, input, &text);

    text.draw(
        &mut canvas,
        input.title,
        Anchor::left(PAD as f32, (PAD + 15) as f32),
        TITLE,
    );
    text.draw(
        &mut canvas,
        input.footer,
        Anchor::left(PAD as f32, (i64::from(layout.height) - PAD + 2) as f32),
        LABEL,
    );

    if let Some(rect) = layout.spectrogram {
        draw_spectrogram(&mut canvas, &scene, rect);
    }
    if let Some((rect, waveform)) = layout.waveform.zip(input.analysis.waveform.as_ref()) {
        draw_waveform(&mut canvas, &scene, rect, waveform);
    }
    if let Some(rect) = layout.psd {
        draw_psd(&mut canvas, &scene, rect);
    }
    if let Some(rect) = layout.colorbar {
        draw_colorbar(&mut canvas, &scene, rect);
    }

    canvas
}

fn draw_spectrogram(canvas: &mut RgbImage, scene: &Scene<'_>, rect: Rect) {
    let orientation = scene.layout.orientation;
    blit(canvas, rect, &scene.input.analysis.spectrogram, orientation);
    frame(canvas, rect);

    let img = &scene.input.analysis.spectrogram;
    let caption = ticks::caption(AxisKind::Frequency, img.f0, img.f1).unwrap_or_default();
    match orientation {
        Orientation::Horizontal => {
            scene.across(canvas, rect, &scene.time, Labelled::Yes);
            scene.up(canvas, rect, &scene.frequency, Labelled::Yes);
            let at = Anchor::right(
                (rect.x - LABEL_PAD) as f32,
                scene.beside(rect.y + scene.rise),
            );
            scene.caption_above(canvas, at, &scene.frequency, caption);
        }
        Orientation::Vertical => {
            scene.down(canvas, rect, &scene.time, Labelled::Yes);
            scene.across(canvas, rect, &scene.frequency, Labelled::Yes);
            scene.caption_after(canvas, rect, &scene.frequency, caption);
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

fn draw_psd(canvas: &mut RgbImage, scene: &Scene<'_>, rect: Rect) {
    fill(canvas, rect, Theme::PANEL);

    let psd = &scene.input.analysis.psd;
    if psd.db.is_empty() {
        frame(canvas, rect);
        return;
    }
    let range = psd_range(&psd.db);
    let decibels = psd_db_ticks(scene, rect, range);

    // Frequency runs along whichever axis the spectrogram is not using for
    // time, so the two panels line up bin for bin. The grid goes down before
    // the trace: drawn after it, a grid line cuts the trace into dashes
    // exactly where it is being read.
    let freq_vertical = matches!(scene.layout.orientation, Orientation::Horizontal);
    if freq_vertical {
        scene.up(canvas, rect, &scene.frequency, Labelled::No);
        scene.across(canvas, rect, &decibels, Labelled::Yes);
    } else {
        scene.across(canvas, rect, &scene.frequency, Labelled::No);
        scene.up(canvas, rect, &decibels, Labelled::Yes);
    }

    draw_psd_trace(canvas, rect, psd, range, freq_vertical);
    frame(canvas, rect);
}

/// The spectrum panel's own decibel scale.
///
/// The averaged spectrum sits well below the spectrogram's per-frame maxima,
/// so sharing the colour bar's dB range would squash the whole trace against
/// one edge. The panel gets its own, labelled.
fn psd_db_ticks(scene: &Scene<'_>, rect: Rect, range: (f32, f32)) -> Vec<Tick> {
    let (min, max) = (f64::from(range.0), f64::from(range.1));
    match scene.layout.orientation {
        // Across the panel, labelled underneath, sharing that row with the
        // spectrogram's time labels on the other side of the gap.
        Orientation::Horizontal => {
            let axis = Axis {
                length: rect.w,
                min,
                max,
                lead: room_left(rect.x, scene.layout.spectrogram.map(|s| s.right())),
                trail: room_right(
                    rect.right(),
                    scene.layout.colorbar.map(|c| c.x),
                    scene.layout.width,
                ),
            };
            ticks::ticks(AxisKind::Decibels, axis, &scene.along())
        }
        // Stacked in the left gutter, which was measured to hold them.
        Orientation::Vertical => {
            let axis = Axis {
                length: rect.h,
                min,
                max,
                lead: scene.rise,
                trail: scene.rise,
            };
            ticks::ticks(AxisKind::Decibels, axis, &scene.stacked())
        }
    }
}

/// The averaged spectrum itself, one pixel per row or column.
///
/// Consecutive samples are joined so a fast-moving trace reads as a line
/// rather than a dotted scatter.
fn draw_psd_trace(
    canvas: &mut RgbImage,
    rect: Rect,
    psd: &Psd,
    range: (f32, f32),
    freq_vertical: bool,
) {
    let (db_min, db_max) = range;
    let span = (db_max - db_min).max(1e-6);
    let level_at = |i: usize| ((psd.db[i] - db_min) / span).clamp(0.0, 1.0);
    let bin_at = |t: f64| ((t * (psd.db.len() - 1) as f64).round() as usize).min(psd.db.len() - 1);

    let mut previous: Option<i64> = None;
    if freq_vertical {
        for py in 0..rect.h {
            // Row 0 is the top of the panel: highest frequency.
            let t = (rect.h - 1 - py) as f64 / (rect.h.max(2) - 1) as f64;
            let x = rect.x + (level_at(bin_at(t)) * (rect.w - 1) as f32).round() as i64;
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
        return;
    }

    for px in 0..rect.w {
        let t = px as f64 / (rect.w.max(2) - 1) as f64;
        let y = rect.bottom() - 1 - (level_at(bin_at(t)) * (rect.h - 1) as f32).round() as i64;
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
}

/// The time-domain strip: a min/max span per column, scaled to the reference.
fn draw_waveform(
    canvas: &mut RgbImage,
    scene: &Scene<'_>,
    rect: Rect,
    waveform: &WaveformEnvelope,
) {
    fill(canvas, rect, Theme::PANEL);

    let horizontal = matches!(scene.layout.orientation, Orientation::Horizontal);
    let (columns, span) = if horizontal {
        (rect.w, rect.h)
    } else {
        (rect.h, rect.w)
    };
    // Zero sits in the middle; each half carries the whole dB window.
    let middle = if horizontal {
        rect.y + span / 2
    } else {
        rect.x + span / 2
    };
    let half = ((span - 1) / 2).max(1);

    // The strip carries the spectrogram's time grid and none of its labels:
    // the two panels are read together, so the values are written once.
    if horizontal {
        hline(canvas, rect.x, rect.right(), middle, Theme::GRID);
        scene.across(canvas, rect, &scene.time, Labelled::No);
    } else {
        vline(canvas, middle, rect.y, rect.bottom(), Theme::GRID);
        scene.down(canvas, rect, &scene.time, Labelled::No);
    }

    let full_scale = scene.input.waveform_full_scale.max(1e-6);
    let offset = |value: f32| -> i64 {
        let level = (value.abs() / full_scale).clamp(0.0, 1.0);
        let distance = (level * half as f32).round() as i64;
        if value >= 0.0 {
            // Positive is up when time runs across, right when it runs down.
            if horizontal { -distance } else { distance }
        } else if horizontal {
            distance
        } else {
            -distance
        }
    };

    // I and Q are merged into one span rather than drawn as two traces. On a
    // real capture their envelopes very nearly coincide, so the second colour
    // ended up hidden under the first everywhere except at the extremes, and
    // paid for that with a legend and a blend nobody could read.
    let mut previous: Option<(i64, i64)> = None;
    for step in 0..columns {
        let column = (step as usize * waveform.columns) / columns.max(1) as usize;
        let Some((mut lo, mut hi)) = merged_span(waveform, column, &offset) else {
            continue;
        };
        // Join to the previous column so a trace moving faster than one column
        // per pixel reads as a line rather than a dotted scatter.
        if let Some((prev_lo, prev_hi)) = previous {
            lo = lo.min(prev_hi);
            hi = hi.max(prev_lo);
        }
        previous = Some((lo, hi));

        if horizontal {
            vline(
                canvas,
                rect.x + step,
                middle + lo,
                middle + hi + 1,
                Theme::TRACE,
            );
        } else {
            hline(
                canvas,
                middle + lo,
                middle + hi + 1,
                rect.y + step,
                Theme::TRACE,
            );
        }
    }

    frame(canvas, rect);
}

/// The column's extent across every channel, in pixels from the centre line.
///
/// A complex signal is one track: the strip answers "how big was the signal
/// here", and that is the wider of I and Q, not either on its own.
fn merged_span(
    waveform: &WaveformEnvelope,
    column: usize,
    offset: &impl Fn(f32) -> i64,
) -> Option<(i64, i64)> {
    let mut span: Option<(i64, i64)> = None;
    for channel in 0..waveform.channels {
        let (min, max) = waveform.column(column, channel)?;
        let (lo, hi) = (offset(max).min(offset(min)), offset(max).max(offset(min)));
        span = Some(match span {
            Some((s_lo, s_hi)) => (s_lo.min(lo), s_hi.max(hi)),
            None => (lo, hi),
        });
    }
    span
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

fn draw_colorbar(canvas: &mut RgbImage, scene: &Scene<'_>, rect: Rect) {
    let gradient = scene.input.colormap.gradient();
    let img = &scene.input.analysis.spectrogram;

    for py in 0..rect.h {
        // Strongest at the top.
        let t = (rect.h - 1 - py) as f32 / (rect.h.max(2) - 1) as f32;
        let color = gradient[argand_core::gradient_index(t)];
        for px in 0..rect.w {
            put(canvas, rect.x + px, rect.y + py, Rgb(color));
        }
    }
    frame(canvas, rect);

    let axis = Axis {
        length: rect.h,
        min: f64::from(img.db_min),
        max: f64::from(img.db_max),
        lead: scene.rise,
        trail: -caption_rows(scene.rise),
    };
    let at = Anchor::left(
        (rect.right() + LABEL_PAD) as f32,
        scene.beside(rect.y + scene.rise),
    );
    let marks = ticks::ticks(AxisKind::Decibels, axis, &scene.stacked());
    scene.caption_above(canvas, at, &marks, "dB");
    for tick in &marks {
        let y = rect.bottom() - 1 - tick.offset;
        hline(canvas, rect.right(), rect.right() + 3, y, Theme::AXIS);
        scene.text.draw(
            canvas,
            &tick.label,
            Anchor::left((rect.right() + LABEL_PAD) as f32, scene.beside(y)),
            LABEL,
        );
    }
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
