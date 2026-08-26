use super::*;

fn decode(format: SampleFormat, bytes: &[u8]) -> Vec<f32> {
    let mut out = vec![0.0; bytes.len() / format.bytes()];
    let n = convert(format, bytes, &mut out, 1.0 / format.full_scale());
    assert_eq!(n, out.len());
    out
}

#[test]
fn u8_is_offset_binary_centred_on_128() {
    assert_eq!(decode(SampleFormat::U8, &[128, 255, 0, 192]), vec![
        0.0, 127.0 / 128.0, -1.0, 0.5
    ]);
}

#[test]
fn i16_spans_minus_one_to_just_under_one() {
    let bytes = [0x00, 0x00, 0x00, 0x80, 0xFF, 0x7F];
    let got = decode(SampleFormat::I16, &bytes);
    assert_eq!(got[0], 0.0);
    assert_eq!(got[1], -1.0);
    assert!((got[2] - 1.0).abs() < 1e-4);
}

#[test]
fn i32_spans_minus_one_to_just_under_one() {
    let bytes = [
        0x00, 0x00, 0x00, 0x80, // i32::MIN
        0xFF, 0xFF, 0xFF, 0x7F, // i32::MAX
    ];
    let got = decode(SampleFormat::I32, &bytes);
    assert_eq!(got[0], -1.0);
    assert!((got[1] - 1.0).abs() < 1e-6);
}

#[test]
fn float_formats_pass_through_unscaled() {
    let bytes: Vec<u8> = [0.25f32, -3.5, 1e6]
        .iter()
        .flat_map(|v| v.to_le_bytes())
        .collect();
    assert_eq!(decode(SampleFormat::F32, &bytes), vec![0.25, -3.5, 1e6]);
    assert_eq!(decode(SampleFormat::F16x8, &bytes), vec![0.25, -3.5, 1e6]);
}

#[test]
fn scale_is_applied_to_every_format() {
    let mut out = [0.0f32; 2];
    let n = convert(SampleFormat::I16, &[0xFF, 0x7F, 0x00, 0x80], &mut out, 2.0);
    assert_eq!(n, 2);
    assert_eq!(out, [32767.0 * 2.0, -32768.0 * 2.0]);
}

#[test]
fn stops_at_whichever_buffer_ends_first() {
    let mut out = [0.0f32; 1];
    assert_eq!(convert(SampleFormat::I16, &[1, 2, 3, 4], &mut out, 1.0), 1);

    let mut out = [0.0f32; 4];
    // A trailing partial sample is ignored, not decoded as garbage.
    assert_eq!(convert(SampleFormat::I16, &[1, 2, 3], &mut out, 1.0), 1);
}

#[test]
fn peak_scan_takes_the_largest_absolute_value() {
    let bytes: Vec<u8> = [0.25f32, -9.5, 3.0]
        .iter()
        .flat_map(|v| v.to_le_bytes())
        .collect();
    assert_eq!(peak_abs(SampleFormat::F16x8, &bytes), 9.5);
    assert_eq!(peak_abs(SampleFormat::F16x8, &[]), 0.0);
}

#[test]
fn peak_scan_reports_raw_units_for_every_format() {
    // Not scaled to full scale: the caller divides by this value.
    assert_eq!(peak_abs(SampleFormat::U8, &[128, 200, 60]), 72.0);
    assert_eq!(peak_abs(SampleFormat::I16, &[0x00, 0x40, 0x00, 0x80]), 32768.0);
    assert_eq!(
        peak_abs(SampleFormat::I32, &[0, 0, 0, 0x40, 0, 0, 0, 0x80]),
        2147483648.0
    );
}
