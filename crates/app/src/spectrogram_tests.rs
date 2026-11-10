use super::*;

/// A two-pixel picture whose colours cannot be confused for each other under
/// any permutation of the channels.
fn two_pixels() -> SpectrogramImage {
    let mut image = SpectrogramImage::new(2, 1);
    image.put(0, 0, [10, 20, 30]).expect("a pixel inside the image");
    image.put(1, 0, [200, 100, 50]).expect("a pixel inside the image");
    image
}

#[test]
fn the_upload_reads_as_bgra_because_that_is_what_the_shader_reads() {
    // Not a round trip: the point is that the bytes leave in the order gpui
    // reads them, which is the opposite of the order they arrive in.
    // Both alpha bytes also assert that the uploaded picture stays opaque.
    assert_eq!(
        bgra(&two_pixels(), crate::orientation::Mode::Horizontal),
        vec![30, 20, 10, 255, 50, 100, 200, 255]
    );
}

#[test]
fn a_picture_with_no_pixels_is_nothing_to_upload() {
    assert!(texture(&SpectrogramImage::new(0, 0), crate::orientation::Mode::Horizontal).is_none());
}

#[test]
fn every_pixel_of_a_multirow_image_keeps_its_position_and_channels() {
    let mut image = SpectrogramImage::new(3, 2);
    image.rgba = vec![
        10, 20, 30, 255, 40, 50, 60, 255, 70, 80, 90, 255,
        100, 110, 120, 255, 130, 140, 150, 255, 160, 170, 180, 255,
    ];
    let uploaded = bgra(&image, crate::orientation::Mode::Horizontal);
    assert_eq!(uploaded.len(), 3 * 2 * CHANNELS);
    assert_eq!(
        uploaded,
        vec![
            30, 20, 10, 255, 60, 50, 40, 255, 90, 80, 70, 255,
            120, 110, 100, 255, 150, 140, 130, 255, 180, 170, 160, 255,
        ]
    );
}

#[test]
fn deep_zoom_column_keeps_every_frequency_row_and_channel_order() {
    let mut source = SpectrogramImage::new(2, 2);
    source.rgba = vec![1, 2, 3, 255, 4, 5, 6, 255, 7, 8, 9, 255, 10, 11, 12, 255];
    let strip = column_texture(&source, 1, crate::orientation::Mode::Horizontal).unwrap();
    let top = [6, 5, 4, 255].repeat(3);
    let bottom = [12, 11, 10, 255].repeat(3);
    assert_eq!(strip.as_bytes(0).unwrap(), [top.clone(), top, bottom.clone(), bottom].concat());
    assert!(column_texture(&source, 2, crate::orientation::Mode::Horizontal).is_none());
}

#[test]
fn deep_zoom_column_refuses_an_image_whose_buffer_is_short_of_its_shape() {
    // A buffer short of the declared width is refused rather than read past.
    let mut source = SpectrogramImage::new(2, 2);
    source.rgba = vec![1, 2, 3, 255, 4, 5, 6, 255, 7, 8, 9, 255, 10, 11, 12, 255];
    source.rgba.truncate(8);
    assert_eq!(source.shape(), None, "the buffer is short of the shape");
    assert!(
        column_texture(&source, 0, crate::orientation::Mode::Horizontal).is_none(),
        "a column is refused when the image does not hold the shape it declares"
    );
    assert!(column_texture(&source, 2, crate::orientation::Mode::Horizontal).is_none());
}

#[test]
fn deep_zoom_column_refuses_a_width_whose_offset_would_overflow() {
    // The fields are a caller's to set, and the strip built beside the read is
    // sized by the height, so a width past any buffer still reaches the loop.
    // The hand-written product the accessor replaced overflowed on the second
    // row, which panicked on a checked build and wrapped onto a foreign pixel
    // on a release one.
    let source = SpectrogramImage {
        width: usize::MAX,
        height: 2,
        rgba: vec![1, 2, 3, 255, 4, 5, 6, 255],
        t0: 0.0,
        t1: 1.0,
        f0: -1.0,
        f1: 1.0,
        db_min: -110.0,
        db_max: 0.0,
    };
    assert_eq!(source.shape(), None, "no buffer covers that width");
    assert!(
        column_texture(&source, 1, crate::orientation::Mode::Horizontal).is_none(),
        "an overflowing offset must refuse rather than wrap onto a pixel"
    );
}

#[test]
fn texture_padding_replicates_edges_without_changing_the_interior() {
    let texture = texture(&two_pixels(), crate::orientation::Mode::Horizontal).unwrap();
    assert_eq!(u32::from(texture.size(0).width), 4);
    assert_eq!(u32::from(texture.size(0).height), 3);
    let row = [30,20,10,255, 30,20,10,255, 50,100,200,255, 50,100,200,255];
    assert_eq!(texture.as_bytes(0).unwrap(), row.repeat(3));
}

#[test]
fn vertical_upload_rotates_frequency_right_and_time_down_including_strip_padding() {
    use crate::orientation::Mode;
    let mut image = SpectrogramImage::new(2, 3);
    image.rgba = (1..=6).flat_map(|n| [n, 0, 0, 255]).collect();
    let expected: Vec<_> = [5, 3, 1, 6, 4, 2].into_iter().flat_map(|n| [0, 0, n, 255]).collect();
    assert_eq!(bgra(&image, Mode::Vertical), expected);
    let texture = texture(&image, Mode::Vertical).unwrap();
    assert_eq!((u32::from(texture.size(0).width), u32::from(texture.size(0).height)), (5, 4));
    let strip = column_texture(&image, 1, Mode::Vertical).unwrap();
    let row: Vec<_> = [6, 6, 4, 2, 2].into_iter().flat_map(|n| [0, 0, n, 255]).collect();
    assert_eq!(strip.as_bytes(0).unwrap(), row.repeat(3));
}
