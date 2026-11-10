use super::*;
use symphonia::core::codecs::{CodecParameters, video::VideoCodecParameters};

#[test]
fn selects_the_first_non_null_audio_track() {
    let unknown = Track::new(0);
    let mut video = Track::new(1);
    video.codec_params = Some(CodecParameters::Video(VideoCodecParameters::default()));
    let mut null_audio = Track::new(2);
    null_audio.codec_params = Some(CodecParameters::Audio(AudioCodecParameters::default()));
    let mut audio = Track::new(3);
    audio.codec_params = Some(CodecParameters::Audio(AudioCodecParameters {
        codec: CODEC_ID_PCM_U8,
        ..Default::default()
    }));
    let mut later = audio.clone();
    later.id = 4;
    let tracks = [unknown, video, null_audio, audio, later];
    assert_eq!(audio_track(&tracks).unwrap().0.id, 3);
    assert!(audio_track(&tracks[..3]).is_err());
}
