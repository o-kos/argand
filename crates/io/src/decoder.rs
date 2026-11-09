//! Decoder-backed reader for containers that are not a flat sample array.
//!
//! FLAC lives here. So does any WAVE layout the native reader declines, such
//! as 24-bit, which costs nothing extra now that the decoder is linked in.

use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use argand_core::{Domain, SampleFormat, SampleSource, SampleType, SignalMeta, SourceError};
use symphonia::core::codecs::audio::well_known::{CODEC_ID_PCM_U8, CODEC_ID_PCM_U8_PLANAR};
use symphonia::core::codecs::audio::{
    AudioCodecParameters, AudioDecoder, AudioDecoderOptions, CODEC_ID_NULL_AUDIO,
};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, FormatReader, SeekMode, SeekTo, Track};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::units::{Duration, Timestamp};

use crate::normalize::{AUTO_HEADROOM, Normalize, gain_factor};
use crate::shared_file::SharedFile;

pub struct DecodedSource {
    reader: Box<dyn FormatReader + 'static>,
    decoder: Box<dyn AudioDecoder>,
    track_id: u32,
    meta: SignalMeta,
    /// Interleaved values already decoded but not yet handed to the caller.
    pending: Vec<f32>,
    pending_pos: usize,
    /// Values to drop after a seek that landed early.
    skip: usize,
    scale: f32,
    original_units: Option<(f64, f64)>,
    bits_per_sample: Option<u32>,
    divisor: f32,
    exhausted: bool,
    /// Fail on a damaged packet instead of skipping it, as exact copies need.
    strict: bool,
    /// End of the last decoded packet, or the actual landing point after a seek.
    decoded_end: Timestamp,
    /// The decoded file as it was opened.
    stamp: Option<crate::write::SourceStamp>,
    /// The handle the file was opened through, which every fresh parser of it reads.
    file: Arc<File>,
}

/// Level policy for decoder-backed opening; an absent budget preserves full scans.
pub struct DecodeLevels {
    pub normalize: Normalize,
    pub gain_db: f32,
    pub scan_bytes: Option<usize>,
}

impl DecodedSource {
    pub fn open(
        path: &Path,
        container: &'static str,
        center_freq: Option<f64>,
        sample_rate_override: Option<f64>,
        sample_type_override: Option<SampleType>,
        normalize: Normalize,
        gain_db: f32,
    ) -> Result<Self, SourceError> {
        Self::with_levels(
            path,
            container,
            center_freq,
            sample_rate_override,
            sample_type_override,
            DecodeLevels {
                normalize,
                gain_db,
                scan_bytes: None,
            },
        )
    }

    pub fn with_levels(
        path: &Path,
        container: &'static str,
        center_freq: Option<f64>,
        sample_rate_override: Option<f64>,
        sample_type_override: Option<SampleType>,
        levels: DecodeLevels,
    ) -> Result<Self, SourceError> {
        let mut source =
            Self::open_plain(path, container, center_freq, sample_rate_override, None)?;
        if let Some(forced) = sample_type_override {
            source.meta.sample_type = forced;
        }
        source.apply_levels(levels)
    }

    pub(crate) fn apply_levels(mut self, levels: DecodeLevels) -> Result<Self, SourceError> {
        let DecodeLevels {
            normalize,
            gain_db,
            scan_bytes,
        } = levels;
        // Symphonia hands back values already on the unit scale, so the only
        // divisor left is a peak measurement, and that needs a decode pass.
        let divisor = match normalize {
            Normalize::None => 1.0,
            Normalize::Factor(v) => v,
            Normalize::Auto => {
                let peak = self.fresh()?.measure_peak(scan_bytes)? * AUTO_HEADROOM;
                if peak > 1e-9 { peak } else { 1.0 }
            }
        };
        self.divisor = divisor;
        self.meta.divisor = divisor;
        self.scale = gain_factor(gain_db) / divisor;
        Ok(self)
    }

    pub(crate) fn reopen(meta: &SignalMeta, gain_db: f32) -> Result<Self, SourceError> {
        let mut source = Self::open_plain(
            &meta.source,
            meta.container,
            Some(meta.center_freq),
            Some(meta.sample_rate),
            Some(meta.len_samples),
        )?;
        source.meta.sample_type = meta.sample_type;
        source.meta.divisor = meta.divisor;
        source.divisor = meta.divisor;
        source.scale = gain_factor(gain_db) / meta.divisor;
        Ok(source)
    }

    fn open_plain(
        path: &Path,
        container: &'static str,
        center_freq: Option<f64>,
        sample_rate_override: Option<f64>,
        known_len: Option<u64>,
    ) -> Result<Self, SourceError> {
        let file = File::open(path)?;
        let stamp = crate::write::SourceStamp::of_file(&file).ok();
        Self::open_shared(
            Arc::new(file),
            stamp,
            path,
            container,
            center_freq,
            sample_rate_override,
            known_len,
        )
    }

    /// A parser of the same handle from its start, keeping this one's levels and strictness.
    ///
    /// The handle is stamped again first, so a file overwritten in place is not decoded as a mix.
    fn fresh(&self) -> Result<Self, SourceError> {
        let now = crate::write::SourceStamp::of_file(&self.file).ok();
        if self.stamp.is_none() || now != self.stamp {
            return Err(SourceError::Changed);
        }
        let mut fresh = Self::open_shared(
            self.file.clone(),
            self.stamp,
            &self.meta.source,
            self.meta.container,
            Some(self.meta.center_freq),
            Some(self.meta.sample_rate),
            Some(self.meta.len_samples),
        )?;
        fresh.meta.sample_type = self.meta.sample_type;
        fresh.meta.divisor = self.divisor;
        fresh.divisor = self.divisor;
        fresh.scale = self.scale;
        Ok(if self.strict { fresh.strict() } else { fresh })
    }

    /// Decode `file`, whose stamp at opening is `stamp`.
    fn open_shared(
        file: Arc<File>,
        stamp: Option<crate::write::SourceStamp>,
        path: &Path,
        container: &'static str,
        center_freq: Option<f64>,
        sample_rate_override: Option<f64>,
        known_len: Option<u64>,
    ) -> Result<Self, SourceError> {
        let mss =
            MediaSourceStream::new(Box::new(SharedFile::new(file.clone())?), Default::default());

        let mut hint = Hint::new();
        if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
            hint.with_extension(ext);
        }

        let mut reader = symphonia::default::get_probe()
            .probe(
                &hint,
                mss,
                FormatOptions::default(),
                MetadataOptions::default(),
            )
            .map_err(decode_err)?;
        let tagged_freq = tagged(&mut *reader, REFERENCE_TAG);
        let tagged_rate = tagged(&mut *reader, RATE_TAG);
        let center_freq = center_freq.or(tagged_freq);

        let (track, params) = audio_track(reader.tracks())?;
        let track_id = track.id;
        let frames = track.num_frames;
        let params = params.clone();

        let channels = params
            .channels
            .as_ref()
            .ok_or_else(|| SourceError::Decode("channel count missing".into()))?
            .count();
        let domain = match channels {
            1 => Domain::Real,
            2 => Domain::Iq,
            n => {
                return Err(SourceError::Decode(format!(
                    "unsupported channel count: {n}, expected 1 (real) or 2 (i/q)"
                )));
            }
        };

        let sample_rate = params
            .sample_rate
            .ok_or_else(|| SourceError::Decode("sample rate missing".into()))?
            as f64;

        let decoder = symphonia::default::get_codecs()
            .make_audio_decoder(&params, &AudioDecoderOptions::default())
            .map_err(decode_err)?;

        let format = match params.bits_per_sample {
            Some(8) => SampleFormat::U8,
            Some(16) => SampleFormat::I16,
            Some(32) if is_float(&params) => SampleFormat::F32,
            Some(_) => SampleFormat::I32,
            None if is_float(&params) => SampleFormat::F32,
            None => SampleFormat::I32,
        };

        let original_units = original_units(&params);
        let mut source = Self {
            reader,
            decoder,
            track_id,
            meta: SignalMeta {
                sample_rate: sample_rate_override.unwrap_or_else(|| {
                    // The exact rate is trusted only while it still agrees with the stream's own.
                    tagged_rate
                        .filter(|exact| *exact > 0.0 && (exact - sample_rate).abs() < 1.0)
                        .unwrap_or(sample_rate)
                }),
                center_freq: center_freq.unwrap_or(0.0),
                sample_type: SampleType::new(domain, format),
                len_samples: frames.filter(|&count| count > 0).or(known_len).unwrap_or(0),
                container,
                divisor: 1.0,
                source: PathBuf::from(path),
            },
            pending: Vec::new(),
            pending_pos: 0,
            skip: 0,
            scale: 1.0,
            original_units,
            bits_per_sample: params.bits_per_sample,
            divisor: 1.0,
            exhausted: false,
            strict: false,
            decoded_end: Timestamp::ZERO,
            stamp,
            file: file.clone(),
        };

        // STREAMINFO usually carries the length; when it does not, the only
        // honest answer is to decode once and count. sgvr silently reported
        // zero here, which then truncated the whole analysis.
        if source.meta.len_samples == 0 && known_len.is_none() {
            tracing::debug!("stream reports no frame count, counting by decoding");
            source.meta.len_samples = source.fresh()?.count_samples()?;
        }

        Ok(source)
    }

    /// Decode everything, returning the sample count.
    fn count_samples(&mut self) -> Result<u64, SourceError> {
        let mut total = 0u64;
        let channels = self.meta.channels();
        let mut buf = vec![0.0f32; 65536];
        loop {
            let n = self.read_unscaled(&mut buf)?;
            if n == 0 {
                return Ok(total);
            }
            total += (n / channels) as u64;
        }
    }

    /// Decode everything, returning the largest absolute value.
    fn measure_peak(&mut self, budget: Option<usize>) -> Result<f32, SourceError> {
        let mut peak = 0.0f32;
        let mut buf = vec![0.0f32; 65536];
        let mut remaining = budget.unwrap_or(usize::MAX) / self.meta.sample_type.format.bytes();
        let channels = self.meta.channels();
        loop {
            let want = remaining.min(buf.len()) / channels * channels;
            if want == 0 {
                return Ok(peak);
            }
            let n = self.read_unscaled(&mut buf[..want])?;
            remaining -= n;
            if n == 0 {
                return Ok(peak);
            }
            peak = buf[..n].iter().fold(peak, |m, v| m.max(v.abs()));
        }
    }

    /// Pull the next decoded packet into `pending`.
    fn fill(&mut self) -> Result<bool, SourceError> {
        loop {
            let packet = match self.reader.next_packet() {
                Ok(Some(packet)) => packet,
                Ok(None) => {
                    if u64::try_from(self.decoded_end.get()).unwrap_or(0) < self.meta.len_samples {
                        return Err(SourceError::Decode(format!(
                            "premature end of stream at sample {}, expected {}",
                            self.decoded_end, self.meta.len_samples
                        )));
                    }
                    self.exhausted = true;
                    return Ok(false);
                }
                Err(e) => return Err(decode_err(e)),
            };
            if packet.track_id != self.track_id {
                continue;
            }
            if self.strict && packet.pts != self.decoded_end {
                return Err(SourceError::Decode(format!(
                    "a damaged frame is missing at sample {}",
                    self.decoded_end
                )));
            }

            let decoded = match self.decoder.decode(&packet) {
                Ok(d) => d,
                // A damaged packet is worth skipping, not dying on.
                Err(SymphoniaError::DecodeError(msg)) if !self.strict => {
                    tracing::warn!("skipping undecodable packet: {msg}");
                    continue;
                }
                Err(e) => return Err(decode_err(e)),
            };
            self.decoded_end = packet
                .pts
                .checked_add(Duration::new(decoded.frames() as u64))
                .ok_or_else(|| SourceError::Decode("packet timestamp overflow".into()))?;
            if decoded.frames() == 0 {
                continue;
            }

            decoded.copy_to_vec_interleaved(&mut self.pending);
            self.pending_pos = 0;
            return Ok(true);
        }
    }

    /// Shared body of `read`, without the level scaling.
    fn read_unscaled(&mut self, buf: &mut [f32]) -> Result<usize, SourceError> {
        let channels = self.meta.channels();
        let usable = buf.len() - buf.len() % channels;
        let mut written = 0;

        while written < usable {
            if self.pending_pos >= self.pending.len() && (self.exhausted || !self.fill()?) {
                break;
            }

            if self.skip > 0 {
                let drop = self.skip.min(self.pending.len() - self.pending_pos);
                self.pending_pos += drop;
                self.skip -= drop;
                continue;
            }

            let available = self.pending.len() - self.pending_pos;
            let take = available.min(usable - written);
            buf[written..written + take]
                .copy_from_slice(&self.pending[self.pending_pos..self.pending_pos + take]);
            self.pending_pos += take;
            written += take;
        }

        Ok(written - written % channels)
    }

    pub fn divisor(&self) -> f32 {
        self.divisor
    }

    /// Bit depth the stream declares, for re-encoding it unchanged.
    pub(crate) fn bits_per_sample(&self) -> Option<u32> {
        self.bits_per_sample
    }

    /// Refuse damaged packets from now on.
    pub(crate) fn strict(mut self) -> Self {
        self.strict = true;
        self.decoded_end = Timestamp::ZERO;
        self
    }

    /// The decoded file as it was when it was opened, read from the decoder's own handle.
    pub fn stamp(&self) -> Option<crate::write::SourceStamp> {
        self.stamp
    }

    /// A strict decoder of the already checked `file` that resolves its own length rather than trusting a known one.
    pub(crate) fn open_exact(
        file: File,
        path: &Path,
        container: &'static str,
    ) -> Result<Self, SourceError> {
        let stamp = crate::write::SourceStamp::of_file(&file).ok();
        let mut source = Self::open_shared(
            Arc::new(file),
            stamp,
            path,
            container,
            Some(0.0),
            None,
            Some(0),
        )?
        .strict();
        if source.meta.len_samples == 0 {
            source.meta.len_samples = source.fresh()?.count_samples()?;
        }
        Ok(source)
    }

    /// Decoded values exactly as the codec delivers them, before any scaling.
    pub(crate) fn read_plain(&mut self, buf: &mut [f32]) -> Result<usize, SourceError> {
        self.read_unscaled(buf)
    }
}

/// Vorbis comment holding the physical frequency of baseband 0 Hz.
pub(crate) const REFERENCE_TAG: &str = "ARGAND_REFERENCE_FREQUENCY";
/// Vorbis comment holding the exact sample rate, which STREAMINFO rounds to hertz.
pub(crate) const RATE_TAG: &str = "ARGAND_SAMPLE_RATE";

fn audio_track(tracks: &[Track]) -> Result<(&Track, &AudioCodecParameters), SourceError> {
    tracks
        .iter()
        .find_map(|track| {
            let params = track.codec_params.as_ref()?.audio()?;
            (params.codec != CODEC_ID_NULL_AUDIO).then_some((track, params))
        })
        .ok_or_else(|| SourceError::Decode("no decodable audio track".into()))
}

/// A number a stream's own media tags carry under `key`.
fn tagged(reader: &mut dyn FormatReader, key: &str) -> Option<f64> {
    reader
        .metadata()
        .current()
        .and_then(|revision| number_tag(&revision.media.tags, key))
}

fn number_tag(tags: &[symphonia::core::meta::Tag], key: &str) -> Option<f64> {
    tags.iter()
        .find(|tag| tag.raw.key.eq_ignore_ascii_case(key))
        .and_then(|tag| tag.raw.value.to_string().trim().parse::<f64>().ok())
        .filter(|value| value.is_finite())
}

fn original_units(params: &AudioCodecParameters) -> Option<(f64, f64)> {
    if is_float(params) {
        return Some((1.0, 0.0));
    }
    let bits = params
        .bits_per_sample
        .filter(|bits| (1..=32).contains(bits))?;
    let offset = if matches!(params.codec, CODEC_ID_PCM_U8 | CODEC_ID_PCM_U8_PLANAR) {
        128.0
    } else {
        0.0
    };
    Some((2.0f64.powi(bits as i32 - 1), offset))
}

fn is_float(params: &AudioCodecParameters) -> bool {
    matches!(
        params.sample_format,
        Some(symphonia::core::audio::sample::SampleFormat::F32)
            | Some(symphonia::core::audio::sample::SampleFormat::F64)
    )
}

fn decode_err(e: impl std::fmt::Display) -> SourceError {
    SourceError::Decode(e.to_string())
}

impl SampleSource for DecodedSource {
    fn original_sample_units(&self) -> Option<(f64, f64)> {
        let (units, offset) = self.original_units?;
        let factor = units / f64::from(self.scale);
        (factor.is_finite() && factor > 0.0).then_some((factor, offset))
    }

    fn meta(&self) -> &SignalMeta {
        &self.meta
    }

    fn seek(&mut self, sample: u64) -> Result<(), SourceError> {
        if sample > self.meta.len_samples {
            return Err(SourceError::SeekOutOfRange {
                requested: sample,
                total: self.meta.len_samples,
            });
        }

        // Symphonia's FLAC seek can retain parsed packets at aligned frame offsets.
        // A fresh parser avoids stale data without recounting or renormalizing.
        // Strict decoding checks packet timestamps instead, so it keeps the handle it opened.
        if self.meta.container == "flac" && !self.strict {
            *self = self.fresh()?;
            if sample == 0 {
                return Ok(());
            }
        }

        // The endpoint is a logical EOF position, not a frame the demuxer can seek to.
        if sample == self.meta.len_samples {
            self.decoded_end = Timestamp::new(
                i64::try_from(sample)
                    .map_err(|_| SourceError::Decode("seek timestamp overflow".into()))?,
            );
            self.pending.clear();
            self.pending_pos = 0;
            self.skip = 0;
            self.exhausted = true;
            self.decoder.reset();
            return Ok(());
        }

        // FLAC and WAVE count timestamps in samples, so the sample is the timestamp.
        let result = self
            .reader
            .seek(
                SeekMode::Accurate,
                SeekTo::Timestamp {
                    ts: Timestamp::new(
                        i64::try_from(sample)
                            .map_err(|_| SourceError::Decode("seek timestamp overflow".into()))?,
                    ),
                    track_id: self.track_id,
                },
            )
            .map_err(decode_err)?;
        if self.strict && result.actual_ts > result.required_ts {
            return Err(SourceError::Decode(format!(
                "the frame holding sample {sample} is damaged"
            )));
        }
        self.decoded_end = result.actual_ts;

        // Seeks land on a packet boundary at or before the target, so the
        // remainder is dropped on the next read.
        self.skip = usize::try_from(
            result
                .required_ts
                .get()
                .saturating_sub(result.actual_ts.get()),
        )
        .ok()
        .and_then(|frames| frames.checked_mul(self.meta.channels()))
        .ok_or_else(|| SourceError::Decode("seek distance overflow".into()))?;
        self.decoder.reset();
        self.pending.clear();
        self.pending_pos = 0;
        self.exhausted = false;
        Ok(())
    }

    fn read(&mut self, buf: &mut [f32]) -> Result<usize, SourceError> {
        let written = self.read_unscaled(buf)?;
        if self.scale != 1.0 {
            for v in &mut buf[..written] {
                *v *= self.scale;
            }
        }
        Ok(written)
    }
}

#[cfg(test)]
mod tests {
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
}
