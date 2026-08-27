//! Decoder-backed reader for containers that are not a flat sample array.
//!
//! FLAC lives here. So does any WAVE layout the native reader declines, such
//! as 24-bit, which costs nothing extra now that the decoder is linked in.

use std::fs::File;
use std::path::{Path, PathBuf};

use argand_core::{Domain, SampleFormat, SampleSource, SampleType, SignalMeta, SourceError};
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{CODEC_TYPE_NULL, Decoder, DecoderOptions};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::{FormatOptions, FormatReader, SeekMode, SeekTo};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::core::units::TimeBase;

use crate::normalize::{AUTO_HEADROOM, Normalize, gain_factor};

pub struct DecodedSource {
    reader: Box<dyn FormatReader>,
    decoder: Box<dyn Decoder>,
    track_id: u32,
    time_base: TimeBase,
    meta: SignalMeta,
    /// Interleaved values already decoded but not yet handed to the caller.
    pending: Vec<f32>,
    pending_pos: usize,
    /// Values to drop after a seek that landed early.
    skip: usize,
    scale: f32,
    divisor: f32,
    exhausted: bool,
}

impl DecodedSource {
    pub fn open(
        path: &Path,
        container: &'static str,
        center_freq: f64,
        sample_rate_override: Option<f64>,
        sample_type_override: Option<SampleType>,
        normalize: Normalize,
        gain_db: f32,
    ) -> Result<Self, SourceError> {
        let mut source = Self::open_plain(path, container, center_freq, sample_rate_override)?;
        if let Some(forced) = sample_type_override {
            source.meta.sample_type = forced;
        }

        // Symphonia hands back values already on the unit scale, so the only
        // divisor left is a peak measurement, and that needs a decode pass.
        let divisor = match normalize {
            Normalize::None => 1.0,
            Normalize::Factor(v) => v,
            Normalize::Auto => {
                let peak = Self::open_plain(path, container, center_freq, sample_rate_override)?
                    .measure_peak()?
                    * AUTO_HEADROOM;
                if peak > 1e-9 { peak } else { 1.0 }
            }
        };
        source.divisor = divisor;
        source.meta.divisor = divisor;
        source.scale = gain_factor(gain_db) / divisor;
        Ok(source)
    }

    fn open_plain(
        path: &Path,
        container: &'static str,
        center_freq: f64,
        sample_rate_override: Option<f64>,
    ) -> Result<Self, SourceError> {
        let file = File::open(path)?;
        let mss = MediaSourceStream::new(Box::new(file), Default::default());

        let mut hint = Hint::new();
        if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
            hint.with_extension(ext);
        }

        let probed = symphonia::default::get_probe()
            .format(
                &hint,
                mss,
                &FormatOptions::default(),
                &MetadataOptions::default(),
            )
            .map_err(decode_err)?;
        let reader = probed.format;

        let track = reader
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
            .ok_or_else(|| SourceError::Decode("no decodable track".into()))?;
        let track_id = track.id;
        let params = track.codec_params.clone();

        let channels = params
            .channels
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
        let time_base = params
            .time_base
            .unwrap_or_else(|| TimeBase::new(1, sample_rate as u32));

        let decoder = symphonia::default::get_codecs()
            .make(&params, &DecoderOptions::default())
            .map_err(decode_err)?;

        let format = match params.bits_per_sample {
            Some(8) => SampleFormat::U8,
            Some(16) => SampleFormat::I16,
            Some(32) if is_float(&params) => SampleFormat::F32,
            Some(_) => SampleFormat::I32,
            None if is_float(&params) => SampleFormat::F32,
            None => SampleFormat::I32,
        };

        let mut source = Self {
            reader,
            decoder,
            track_id,
            time_base,
            meta: SignalMeta {
                sample_rate: sample_rate_override.unwrap_or(sample_rate),
                center_freq,
                sample_type: SampleType::new(domain, format),
                len_samples: params.n_frames.unwrap_or(0),
                container,
                divisor: 1.0,
                source: PathBuf::from(path),
            },
            pending: Vec::new(),
            pending_pos: 0,
            skip: 0,
            scale: 1.0,
            divisor: 1.0,
            exhausted: false,
        };

        // STREAMINFO usually carries the length; when it does not, the only
        // honest answer is to decode once and count. sgvr silently reported
        // zero here, which then truncated the whole analysis.
        if source.meta.len_samples == 0 {
            tracing::debug!("stream reports no frame count, counting by decoding");
            let mut counter = Self::open_plain(path, container, center_freq, sample_rate_override)?;
            source.meta.len_samples = counter.count_samples()?;
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
    fn measure_peak(&mut self) -> Result<f32, SourceError> {
        let mut peak = 0.0f32;
        let mut buf = vec![0.0f32; 65536];
        loop {
            let n = self.read_unscaled(&mut buf)?;
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
                Ok(p) => p,
                Err(SymphoniaError::IoError(e))
                    if e.kind() == std::io::ErrorKind::UnexpectedEof =>
                {
                    self.exhausted = true;
                    return Ok(false);
                }
                Err(SymphoniaError::ResetRequired) => {
                    self.exhausted = true;
                    return Ok(false);
                }
                Err(e) => return Err(decode_err(e)),
            };
            if packet.track_id() != self.track_id {
                continue;
            }

            let decoded = match self.decoder.decode(&packet) {
                Ok(d) => d,
                // A damaged packet is worth skipping, not dying on.
                Err(SymphoniaError::DecodeError(msg)) => {
                    tracing::warn!("skipping undecodable packet: {msg}");
                    continue;
                }
                Err(e) => return Err(decode_err(e)),
            };
            if decoded.frames() == 0 {
                continue;
            }

            let mut sample_buf = SampleBuffer::<f32>::new(decoded.frames() as u64, *decoded.spec());
            sample_buf.copy_interleaved_ref(decoded);
            self.pending.clear();
            self.pending.extend_from_slice(sample_buf.samples());
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
}

fn is_float(params: &symphonia::core::codecs::CodecParameters) -> bool {
    matches!(
        params.sample_format,
        Some(symphonia::core::sample::SampleFormat::F32)
            | Some(symphonia::core::sample::SampleFormat::F64)
    )
}

fn decode_err(e: impl std::fmt::Display) -> SourceError {
    SourceError::Decode(e.to_string())
}

impl SampleSource for DecodedSource {
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

        let result = self
            .reader
            .seek(
                SeekMode::Accurate,
                SeekTo::Time {
                    time: self.time_base.calc_time(sample),
                    track_id: Some(self.track_id),
                },
            )
            .map_err(decode_err)?;

        // Seeks land on a packet boundary at or before the target, so the
        // remainder is dropped on the next read.
        self.skip =
            (result.required_ts.saturating_sub(result.actual_ts) as usize) * self.meta.channels();
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
