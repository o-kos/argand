//! A transform's RGBA buffer as a texture the window can draw.
//!
//! `argand-dsp` produces row-major RGBA with row 0 at the *highest* frequency,
//! which is already the order a picture is drawn in, so nothing here flips or
//! resamples anything. The one thing that does have to change is the channel
//! order: gpui uploads a [`RenderImage`] byte for byte and its shader reads
//! those bytes as BGRA. Handing it RGBA costs nothing at the API and shows a
//! spectrogram with its reds and blues exchanged, which is a plausible enough
//! picture that it is worth naming here rather than discovering on screen.
//!
//! Alpha is left alone. gpui expects it premultiplied, and every pixel the
//! shading step writes is opaque, which premultiplication leaves untouched.

use std::sync::Arc;

use argand_core::SpectrogramImage;
use gpui::RenderImage;

/// Bytes per pixel in both orders.
const CHANNELS: usize = 4;

/// Upload one spectrogram as a texture.
///
/// `None` for a picture with no pixels, which is what a transform over an
/// empty range produces: there is nothing to draw and nothing to upload.
pub fn texture(image: &SpectrogramImage) -> Option<Arc<RenderImage>> {
    // A buffer of no bytes still satisfies the image crate, and an empty
    // texture reaches the atlas as a zero-sized allocation. Neither is a
    // picture, so the emptiness is answered here rather than passed on.
    if image.width == 0 || image.height == 0 {
        return None;
    }
    let width = u32::try_from(image.width).ok()?;
    let height = u32::try_from(image.height).ok()?;
    let buffer = image::RgbaImage::from_raw(width, height, bgra(image))?;
    Some(Arc::new(RenderImage::new(vec![image::Frame::new(buffer)])))
}

/// A source column for a deeply zoomed placeholder. Copy only visible strips,
/// bounded by `ceil(image.width / 1024) + 1`, rather than a stretched image.
pub fn column_texture(image: &SpectrogramImage, column: usize) -> Option<Arc<RenderImage>> {
    if column >= image.width {
        return None;
    }
    let mut strip = SpectrogramImage::new(1, image.height);
    for row in 0..image.height {
        let offset = (row * image.width + column) * 4;
        strip.rgba[row * 4..row * 4 + 4].copy_from_slice(image.rgba.get(offset..offset + 4)?);
    }
    texture(&strip)
}

/// The same pixels with red and blue exchanged.
fn bgra(image: &SpectrogramImage) -> Vec<u8> {
    let mut bytes = image.rgba.clone();
    for pixel in bytes.chunks_exact_mut(CHANNELS) {
        pixel.swap(0, 2);
    }
    bytes
}

#[cfg(test)]
mod tests {
    include!("spectrogram_tests.rs");
}
