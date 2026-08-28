//! Text drawing over a plain RGB canvas.
//!
//! The font is embedded rather than looked up on the host: argand ships as one
//! binary, and a plot whose labels depend on the machine's font configuration
//! is not the same plot on Linux, Windows and macOS.

use ab_glyph::{Font, FontRef, PxScale, ScaleFont, point};
use image::{Rgb, RgbImage};

const FONT_DATA: &[u8] = include_bytes!("../assets/DejaVuSans.ttf");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Left,
    Center,
    Right,
}

/// Where a label sits, and which way it grows from that point.
#[derive(Debug, Clone, Copy)]
pub struct Anchor {
    pub x: f32,
    pub y: f32,
    pub align: Align,
}

impl Anchor {
    pub const fn left(x: f32, y: f32) -> Self {
        Self {
            x,
            y,
            align: Align::Left,
        }
    }

    pub const fn center(x: f32, y: f32) -> Self {
        Self {
            x,
            y,
            align: Align::Center,
        }
    }

    pub const fn right(x: f32, y: f32) -> Self {
        Self {
            x,
            y,
            align: Align::Right,
        }
    }
}

/// How a label looks. A plot uses two of these, so they are named once.
#[derive(Debug, Clone, Copy)]
pub struct TextStyle {
    pub size: f32,
    pub color: Rgb<u8>,
}

pub struct TextRenderer {
    font: FontRef<'static>,
}

impl TextRenderer {
    pub fn new() -> Self {
        Self {
            font: FontRef::try_from_slice(FONT_DATA).expect("embedded font is valid"),
        }
    }

    /// Width of `text` in pixels at `size`.
    pub fn width(&self, text: &str, size: f32) -> f32 {
        let scaled = self.font.as_scaled(PxScale::from(size));
        let mut width = 0.0;
        let mut previous = None;
        for c in text.chars() {
            let id = scaled.glyph_id(c);
            if let Some(prev) = previous {
                width += scaled.kern(prev, id);
            }
            width += scaled.h_advance(id);
            previous = Some(id);
        }
        width
    }

    /// Draw `text` with its baseline at `at.y`, positioned horizontally by
    /// `at.align` relative to `at.x`.
    pub fn draw(&self, canvas: &mut RgbImage, text: &str, at: Anchor, style: TextStyle) {
        let TextStyle { size, color } = style;
        let scaled = self.font.as_scaled(PxScale::from(size));
        let mut caret = match at.align {
            Align::Left => at.x,
            Align::Center => at.x - self.width(text, size) / 2.0,
            Align::Right => at.x - self.width(text, size),
        };

        let (width, height) = (canvas.width() as i64, canvas.height() as i64);
        let mut previous = None;
        for c in text.chars() {
            let id = scaled.glyph_id(c);
            if let Some(prev) = previous {
                caret += scaled.kern(prev, id);
            }
            let glyph = id.with_scale_and_position(PxScale::from(size), point(caret, at.y));
            caret += scaled.h_advance(id);
            previous = Some(id);

            let Some(outline) = self.font.outline_glyph(glyph) else {
                continue;
            };
            let bounds = outline.px_bounds();
            outline.draw(|gx, gy, coverage| {
                let px = bounds.min.x as i64 + gx as i64;
                let py = bounds.min.y as i64 + gy as i64;
                if px < 0 || py < 0 || px >= width || py >= height {
                    return;
                }
                let dst = canvas.get_pixel_mut(px as u32, py as u32);
                // Anti-aliasing: blend toward the text colour by coverage.
                for i in 0..3 {
                    let a = coverage.clamp(0.0, 1.0);
                    dst.0[i] = (dst.0[i] as f32 * (1.0 - a) + color.0[i] as f32 * a).round() as u8;
                }
            });
        }
    }
}

impl Default for TextRenderer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    include!("text_tests.rs");
}
