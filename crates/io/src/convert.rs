//! Raw bytes to normalised `f32`.
//!
//! Every reader funnels through here so that the five sample formats behave
//! identically no matter which container they arrived in.

use argand_core::SampleFormat;

/// Decode `bytes` into `out`, multiplying by `scale`.
///
/// `scale` folds together the normalization factor and the gain, so integer
/// formats pass 1.0 / full_scale and an unnormalised float pass
/// gain / peak. Stops at whichever of the two buffers runs out first and
/// returns the number of values written.
pub fn convert(format: SampleFormat, bytes: &[u8], out: &mut [f32], scale: f32) -> usize {
    let width = format.bytes();
    let count = (bytes.len() / width).min(out.len());
    let bytes = &bytes[..count * width];
    let out = &mut out[..count];

    match format {
        SampleFormat::U8 => {
            // Offset binary: silence sits at 128.
            for (dst, src) in out.iter_mut().zip(bytes.iter()) {
                *dst = (*src as f32 - 128.0) * scale;
            }
        }
        SampleFormat::I16 => {
            for (dst, src) in out.iter_mut().zip(bytes.chunks_exact(2)) {
                *dst = i16::from_le_bytes([src[0], src[1]]) as f32 * scale;
            }
        }
        SampleFormat::I32 => {
            for (dst, src) in out.iter_mut().zip(bytes.chunks_exact(4)) {
                *dst = i32::from_le_bytes([src[0], src[1], src[2], src[3]]) as f32 * scale;
            }
        }
        SampleFormat::F32 | SampleFormat::F16x8 => {
            for (dst, src) in out.iter_mut().zip(bytes.chunks_exact(4)) {
                *dst = f32::from_le_bytes([src[0], src[1], src[2], src[3]]) * scale;
            }
        }
    }
    count
}

/// Largest absolute value in `bytes`, in the format's own units.
///
/// Used by the normalization pre-scan, so it deliberately does not apply the
/// full scale: the caller wants the raw peak to divide by.
pub fn peak_abs(format: SampleFormat, bytes: &[u8]) -> f32 {
    match format {
        SampleFormat::U8 => bytes
            .iter()
            .map(|b| (*b as f32 - 128.0).abs())
            .fold(0.0f32, f32::max),
        SampleFormat::I16 => bytes
            .chunks_exact(2)
            .map(|c| (i16::from_le_bytes([c[0], c[1]]) as f32).abs())
            .fold(0.0f32, f32::max),
        SampleFormat::I32 => bytes
            .chunks_exact(4)
            .map(|c| (i32::from_le_bytes([c[0], c[1], c[2], c[3]]) as f32).abs())
            .fold(0.0f32, f32::max),
        SampleFormat::F32 | SampleFormat::F16x8 => bytes
            .chunks_exact(4)
            .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]).abs())
            .fold(0.0f32, f32::max),
    }
}

#[cfg(test)]
mod tests {
    include!("convert_tests.rs");
}
