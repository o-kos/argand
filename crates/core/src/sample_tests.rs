use super::*;
use std::str::FromStr;

#[test]
fn parses_every_documented_token() {
    for token in SAMPLE_TYPE_TOKENS {
        let parsed = SampleType::from_str(token).expect(token);
        assert_eq!(parsed.to_string(), token, "round trip for {token}");
    }
}

#[test]
fn byte_sizes_match_the_wav_layouts() {
    // Mirrors the sampleSizes table the C++ writer uses.
    let cases = [
        ("rl_u8", 1),
        ("rl_i16", 2),
        ("rl_i24", 3),
        ("rl_i32", 4),
        ("rl_f32", 4),
        ("rl_f16x8", 4),
        ("iq_u8", 2),
        ("iq_i16", 4),
        ("iq_i24", 6),
        ("iq_i32", 8),
        ("iq_f32", 8),
        ("iq_f16x8", 8),
    ];
    for (token, bytes) in cases {
        assert_eq!(
            SampleType::from_str(token).unwrap().bytes_per_sample(),
            bytes,
            "{token}"
        );
    }
}
