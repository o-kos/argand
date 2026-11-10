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
fn packed_i24_sign_extension_byte_order_tails_and_raw_peak() {
    let bytes = [0, 0, 128, 255, 255, 255, 0, 0, 0, 255, 255, 127, 1, 2, 3];
    let counts = [-8388608.0, -1.0, 0.0, 8388607.0, 197121.0];
    let mut out = [f32::NAN; 7];
    assert_eq!(convert(SampleFormat::I24, &bytes, &mut out, 1.0), 5);
    assert_eq!(&out[..5], &counts);
    assert!(out[5].is_nan());
    assert_eq!(peak_abs(SampleFormat::I24, &bytes), 8388608.0);
    assert_eq!(decode(SampleFormat::I24, &bytes)[..4], [-1.0, -1.0 / 8388608.0, 0.0, 8388607.0 / 8388608.0]);
    for tail in [1, 2] {
        assert_eq!(convert(SampleFormat::I24, &bytes[..tail], &mut out, 1.0), 0);
        assert_eq!(peak_abs(SampleFormat::I24, &bytes[..tail]), 0.0);
    }
    assert_eq!(convert(SampleFormat::I24, &bytes, &mut out[..2], 2.0), 2);
    assert_eq!(&out[..2], &[-16777216.0, -2.0]);
}

#[test]
fn incomplete_scalar_tails_preserve_decoding_peak_and_output_bounds() {
    let cases = [
        (SampleFormat::I16, vec![0, 128, 0, 64], [-32768.0, 16384.0]),
        (SampleFormat::I24, vec![0, 0, 128, 0, 0, 64], [-8388608.0, 4194304.0]),
        (SampleFormat::I32, vec![0, 0, 0, 128, 0, 0, 0, 64], [-2147483648.0, 1073741824.0]),
        (SampleFormat::F32, vec![0, 0, 0, 192, 0, 0, 128, 63], [-2.0, 1.0]),
        (SampleFormat::F16x8, vec![0, 0, 0, 192, 0, 0, 128, 63], [-2.0, 1.0]),
    ];
    for (format, whole, expected) in cases {
        for tail in 0..format.bytes() {
            let mut bytes = whole.clone();
            bytes.extend(std::iter::repeat_n(255, tail));
            let mut out = [123.0; 3];
            assert_eq!(convert(format, &bytes, &mut out, 0.5), 2);
            assert_eq!(out, [expected[0] * 0.5, expected[1] * 0.5, 123.0]);
            assert_eq!(peak_abs(format, &bytes), -expected[0]);
            assert_eq!(convert(format, &bytes, &mut out[..1], 1.0), 1);
            assert_eq!(out[0], expected[0]);
            assert_eq!(out[1], expected[1] * 0.5);
            assert_eq!(convert(format, &bytes[..tail], &mut out, 1.0), 0);
            assert_eq!(peak_abs(format, &bytes[..tail]), 0.0);
        }
    }
}
