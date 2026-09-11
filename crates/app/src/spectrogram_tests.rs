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
        bgra(&two_pixels(), crate::orientation::Mode::Horizontal),
        vec![30, 20, 10, 255, 50, 100, 200, 255]
    );
}

#[test]
fn every_pixel_keeps_the_alpha_the_shading_step_gave_it() {
    let bytes = bgra(&two_pixels(), crate::orientation::Mode::Horizontal);
    assert!(
        bytes.chunks_exact(CHANNELS).all(|pixel| pixel[3] == 255),
        "a spectrogram is opaque, and gpui expects the alpha premultiplied"
    );
}

#[test]
fn a_picture_with_no_pixels_is_nothing_to_upload() {
    assert!(texture(&SpectrogramImage::new(0, 0), crate::orientation::Mode::Horizontal).is_none());
}

#[test]
fn every_pixel_uploaded_is_one_the_transform_produced() {
    use argand_core::{Domain, SampleFormat, SampleType};
    use argand_io::testutil::{TempDir, iq_tone, write_wav};

    // The whole path a picture takes, over a capture on disk: the request the
    // configuration builds, the transform `aspec` calls with the same request,
    // and the bytes this module hands to the GPU. What is checked is that the
    // last of those is the second, byte for byte, with nothing but red and
    // blue exchanged -- nothing resampled, recoloured, cropped or reordered on
    // the way through.
    let dir = TempDir::new("upload");
    let path = write_wav(
        &dir.join("capture.wav"),
        SampleType::new(Domain::Iq, SampleFormat::F32),
        48_000,
        &iq_tone(8192, 48_000.0, 6_000.0, 0.5),
        1.0,
    );

    let mut source =
        argand_io::open(&path, &argand_io::OpenHints::default()).expect("the fixture opens");
    let config = crate::config::Config::default();
    let request = crate::settings::Settings::from_config(&config).analysis_request(&source.meta().clone(), 128, 64);
    let analysis =
        argand_dsp::analyze(source.as_mut(), &request, &mut |_, _| {}).expect("the transform runs");

    let uploaded = bgra(&analysis.spectrogram, crate::orientation::Mode::Horizontal);
    assert_eq!(uploaded.len(), 128 * 64 * CHANNELS);
    for (i, (upload, transform)) in uploaded
        .chunks_exact(CHANNELS)
        .zip(analysis.spectrogram.rgba.chunks_exact(CHANNELS))
        .enumerate()
    {
        assert_eq!(
            upload,
            [transform[2], transform[1], transform[0], transform[3]],
            "pixel {i} of the upload is not the pixel the transform produced"
        );
    }
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
