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
use gpui::{Bounds, Corners, Pixels, RenderImage, Window, point, size};

/// Bytes per pixel in both orders.
const CHANNELS: usize = 4;

/// Upload one spectrogram as a texture.
///
/// `None` for a picture with no pixels, which is what a transform over an
/// empty range produces: there is nothing to draw and nothing to upload.
pub fn texture(
    image: &SpectrogramImage,
    orientation: crate::orientation::Mode,
) -> Option<Arc<RenderImage>> {
    // A buffer of no bytes still satisfies the image crate, and an empty
    // texture reaches the atlas as a zero-sized allocation. Neither is a
    // picture, so the emptiness is answered here rather than passed on.
    if image.width == 0 || image.height == 0 {
        return None;
    }
    let (width, height) = orientation.axes(image.width, image.height);
    let width = u32::try_from(width).ok()?.checked_add(2)?;
    let height = u32::try_from(height).ok()?.checked_add(2)?;
    let buffer = image::RgbaImage::from_raw(width, height, padded_bgra(image, orientation)?)?;
    Some(Arc::new(RenderImage::new(vec![image::Frame::new(buffer)])))
}

/// Clip away replicated edge texels so linear atlas sampling never reads a neighbour.
pub fn paint(texture: Arc<RenderImage>, bounds: Bounds<Pixels>, window: &mut Window) {
    let dimensions = texture.size(0);
    let width = u32::from(dimensions.width).saturating_sub(2).max(1) as f32;
    let height = u32::from(dimensions.height).saturating_sub(2).max(1) as f32;
    let padding = point(bounds.size.width / width, bounds.size.height / height);
    let padded = Bounds {
        origin: bounds.origin - padding,
        size: size(
            bounds.size.width + padding.x * 2.0,
            bounds.size.height + padding.y * 2.0,
        ),
    };
    window.with_content_mask(Some(gpui::ContentMask { bounds }), |window| {
        if let Err(error) = window.paint_image(padded, Corners::default(), texture, 0, false) {
            tracing::warn!(%error, "cannot draw the spectrogram");
        }
    });
}

fn padded_bgra(image: &SpectrogramImage, orientation: crate::orientation::Mode) -> Option<Vec<u8>> {
    let (width, height) = orientation.axes(image.width, image.height);
    let stride = width.checked_mul(CHANNELS)?;
    if image.rgba.len() != stride.checked_mul(height)? || stride == 0 || height == 0 {
        return None;
    }
    let padded_stride = stride.checked_add(2 * CHANNELS)?;
    let mut padded = vec![0; padded_stride.checked_mul(height.checked_add(2)?)?];
    let bytes = bgra(image, orientation);
    for (y, row) in bytes.chunks_exact(stride).enumerate() {
        let start = (y + 1) * padded_stride;
        padded[start..start + CHANNELS].copy_from_slice(&row[..CHANNELS]);
        padded[start + CHANNELS..start + CHANNELS + stride].copy_from_slice(row);
        padded[start + CHANNELS + stride..start + padded_stride]
            .copy_from_slice(&row[stride - CHANNELS..]);
    }
    padded.copy_within(padded_stride..2 * padded_stride, 0);
    padded.copy_within(
        height * padded_stride..(height + 1) * padded_stride,
        (height + 1) * padded_stride,
    );
    Some(padded)
}

/// A source column for a deeply zoomed placeholder. Copy only visible strips,
/// bounded by `ceil(image.width / 1024) + 1`, rather than a stretched image.
pub fn column_texture(
    image: &SpectrogramImage,
    column: usize,
    orientation: crate::orientation::Mode,
) -> Option<Arc<RenderImage>> {
    if column >= image.width {
        return None;
    }
    let mut strip = SpectrogramImage::new(1, image.height);
    for row in 0..image.height {
        let offset = (row * image.width + column) * 4;
        strip.rgba[row * 4..row * 4 + 4].copy_from_slice(image.rgba.get(offset..offset + 4)?);
    }
    texture(&strip, orientation)
}

/// The same pixels with red and blue exchanged.
fn bgra(image: &SpectrogramImage, orientation: crate::orientation::Mode) -> Vec<u8> {
    let mut bytes = vec![0; image.rgba.len()];
    for (y, row) in image
        .rgba
        .chunks_exact(image.width.max(1) * CHANNELS)
        .enumerate()
    {
        for (x, pixel) in row.chunks_exact(CHANNELS).enumerate() {
            let index = if orientation.vertical() {
                x * image.height + image.height - 1 - y
            } else {
                y * image.width + x
            } * CHANNELS;
            bytes[index..index + CHANNELS]
                .copy_from_slice(&[pixel[2], pixel[1], pixel[0], pixel[3]]);
        }
    }
    bytes
}

#[cfg(test)]
mod tests {
    include!("spectrogram_tests.rs");
}
