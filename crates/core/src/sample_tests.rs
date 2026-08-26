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
fn parsing_is_case_and_space_insensitive() {
    assert_eq!(
        SampleType::from_str("  IQ_F16X8 ").unwrap(),
        SampleType::new(Domain::Iq, SampleFormat::F16x8)
    );
}

#[test]
fn rejects_unprefixed_and_unknown_tokens() {
    for bad in ["i16", "iq", "iq_", "xx_i16", "iq_f64", ""] {
        assert!(SampleType::from_str(bad).is_err(), "should reject {bad:?}");
    }
}

#[test]
fn error_message_lists_valid_tokens() {
    let msg = SampleType::from_str("iq_f64").unwrap_err().to_string();
    assert!(msg.contains("iq_f16x8"), "{msg}");
}

#[test]
fn byte_sizes_match_the_wav_layouts() {
    // Mirrors the sampleSizes table the C++ writer uses.
    let cases = [
        ("rl_u8", 1),
        ("rl_i16", 2),
        ("rl_i32", 4),
        ("rl_f32", 4),
        ("rl_f16x8", 4),
        ("iq_u8", 2),
        ("iq_i16", 4),
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

#[test]
fn full_scale_matches_the_integer_ranges() {
    assert_eq!(SampleFormat::U8.full_scale(), 128.0);
    assert_eq!(SampleFormat::I16.full_scale(), 32768.0);
    assert_eq!(SampleFormat::I32.full_scale(), 2147483648.0);
    assert_eq!(SampleFormat::F32.full_scale(), 1.0);
    assert_eq!(SampleFormat::F16x8.full_scale(), 1.0);
}
