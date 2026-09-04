use super::*;

/// A two-pixel picture whose colours cannot be confused for each other under
/// any permutation of the channels.
fn two_pixels() -> SpectrogramImage {
    let mut image = SpectrogramImage::new(2, 1);
    image.put(0, 0, [10, 20, 30]);
    image.put(1, 0, [200, 100, 50]);
    image
}

#[test]
fn the_upload_reads_as_bgra_because_that_is_what_the_shader_reads() {
    // Not a round trip: the point is that the bytes leave in the order gpui
    // reads them, which is the opposite of the order they arrive in.
    assert_eq!(
        bgra(&two_pixels()),
        vec![30, 20, 10, 255, 50, 100, 200, 255]
    );
}

#[test]
fn every_pixel_keeps_the_alpha_the_shading_step_gave_it() {
    let bytes = bgra(&two_pixels());
    assert!(
        bytes.chunks_exact(CHANNELS).all(|pixel| pixel[3] == 255),
        "a spectrogram is opaque, and gpui expects the alpha premultiplied"
    );
}

#[test]
fn a_picture_with_no_pixels_is_nothing_to_upload() {
    assert!(texture(&SpectrogramImage::new(0, 0)).is_none());
}
