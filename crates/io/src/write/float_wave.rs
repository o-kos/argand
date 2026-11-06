//! A real or complex `f32` capture written as it is computed, as a band extraction produces it.

use super::*;

/// What a computed capture is and where it goes.
#[derive(Debug, Clone)]
pub struct FloatWaveRequest {
    /// The rate and reference frequency the file states, whatever sample type it names.
    pub meta: SignalMeta,
    /// Samples the file will hold, which the header states before any is written.
    pub samples: u64,
    pub target: PathBuf,
    /// Files read to compute the samples, which the target must not be.
    pub sources: Vec<SourceFile>,
    /// Files that must not be written over beyond the sources, such as the open file.
    pub protected: Vec<Protected>,
}

/// A real or I/Q `f32` WAVE file being written, moved over its target only by [`FloatWave::finish`].
#[derive(Debug)]
pub struct FloatWave {
    partial: Partial,
    target: PathBuf,
    protected: Vec<Protected>,
    read: Vec<Option<(u64, u64)>>,
    sources: Vec<SourceFile>,
    samples: u64,
    channels: usize,
    written: u64,
    container: &'static str,
    format: SampleFormat,
    clipped: u64,
    pad: bool,
}

impl FloatWave {
    /// Start the file beside its target with its header, refusing a target that is a source or protected.
    pub fn create(request: FloatWaveRequest) -> Result<Self, WriteError> {
        Self::create_with_limit(request, RIFF_LIMIT)
    }

    pub(crate) fn create_with_limit(
        request: FloatWaveRequest,
        riff_limit: u64,
    ) -> Result<Self, WriteError> {
        Self::encoded(request, riff_limit, SampleFormat::F32)
    }

    fn encoded(
        request: FloatWaveRequest,
        riff_limit: u64,
        format: SampleFormat,
    ) -> Result<Self, WriteError> {
        let FloatWaveRequest {
            mut meta,
            samples,
            target,
            sources,
            protected,
        } = request;
        let read_paths = sources.iter().map(|source| source.meta.source.clone());
        check_target(read_paths, &protected, &target)?;
        let domain = if meta.is_iq() {
            argand_core::Domain::Iq
        } else {
            argand_core::Domain::Real
        };
        meta.sample_type = argand_core::SampleType::new(domain, format);
        let channels = meta.channels();
        let fmt = synthesized_fmt(&meta)?;
        let data_len = samples
            .checked_mul(channels as u64 * format.bytes() as u64)
            .ok_or(WriteError::OutOfRange)?;
        let mut partial = Partial::create(&target)?;
        let container =
            write_wave_header(&mut partial, &meta, &fmt, data_len, samples, riff_limit)?;
        Ok(Self {
            partial,
            target,
            protected,
            read: sources
                .iter()
                .map(|source| source.stamp.and_then(|stamp| stamp.identity))
                .collect(),
            sources,
            samples,
            channels,
            written: 0,
            container,
            format,
            clipped: 0,
            pad: data_len % 2 == 1,
        })
    }

    /// Append samples, refusing more than the header states.
    pub fn write(&mut self, samples: &[[f32; 2]]) -> Result<(), WriteError> {
        let count = samples.len() as u64;
        if self.written + count > self.samples {
            return Err(WriteError::OutOfRange);
        }
        let mut bytes = Vec::with_capacity(samples.len() * self.channels * 4);
        for value in samples.iter().flat_map(|sample| &sample[..self.channels]) {
            if !value.is_finite() {
                return Err(WriteError::Unsupported {
                    path: self.target.clone(),
                    reason: "the computed signal contains a nonfinite sample".into(),
                });
            }
            self.clipped += u64::from(encode(self.format, *value, &mut bytes));
        }
        self.partial.write(&bytes)?;
        self.written += count;
        Ok(())
    }

    /// Move the complete file over its target, checking it is still allowed to and not cancelled.
    pub fn finish(mut self, cancel: &AtomicBool) -> Result<Saved, WriteError> {
        if self.written != self.samples {
            return Err(WriteError::OutOfRange);
        }
        if self.pad {
            self.partial.write(&[0])?;
        }
        self.partial.sync()?;
        // Syncing can take long, and a save cancelled meanwhile must leave the target alone.
        if cancel.load(Ordering::Relaxed) {
            return Err(WriteError::Cancelled);
        }
        for source in &self.sources {
            if source.stamp.is_none() || SourceStamp::of(&source.meta.source).ok() != source.stamp {
                return Err(WriteError::SourceChanged {
                    path: source.meta.source.clone(),
                });
            }
        }
        let staged = Staged {
            partial: self.partial,
            target: self.target,
            replaces: None,
            protected: self.protected,
            read: self.read,
            samples: self.samples,
            container: self.container,
        };
        staged.commit().map_err(|refused| refused.error)
    }
}

/// A computed signal encoded directly into any supported scalar WAVE format.
pub struct EncodedWave(FloatWave);

impl EncodedWave {
    /// Validate header limits without creating any output file.
    pub fn check(meta: &SignalMeta, samples: u64) -> Result<(), WriteError> {
        synthesized_fmt(meta)?;
        samples
            .checked_mul(meta.sample_type.bytes_per_sample() as u64)
            .ok_or(WriteError::OutOfRange)?;
        Ok(())
    }

    pub fn create(request: FloatWaveRequest) -> Result<Self, WriteError> {
        let format = request.meta.sample_type.format;
        Self::check(&request.meta, request.samples)?;
        FloatWave::encoded(request, RIFF_LIMIT, format).map(Self)
    }

    pub fn write(&mut self, samples: &[[f32; 2]]) -> Result<(), WriteError> {
        self.0.write(samples)
    }

    /// Saturated scalar values, counting I and Q separately.
    pub fn clipped(&self) -> u64 {
        self.0.clipped
    }

    pub fn finish(self, cancel: &AtomicBool) -> Result<Saved, WriteError> {
        self.0.finish(cancel)
    }
}

fn encode(format: SampleFormat, value: f32, out: &mut Vec<u8>) -> bool {
    let (scale, low, high) = match format {
        SampleFormat::U8 => (128.0, -128.0, 127.0),
        SampleFormat::I16 => (32768.0, -32768.0, 32767.0),
        SampleFormat::I32 => (2147483648.0, -2147483648.0, 2147483647.0),
        SampleFormat::F32 | SampleFormat::F16x8 => {
            out.extend_from_slice(&value.to_le_bytes());
            return false;
        }
    };
    let rounded = (f64::from(value) * scale).round();
    let stored = rounded.clamp(low, high);
    match format {
        SampleFormat::U8 => out.push((stored + 128.0) as u8),
        SampleFormat::I16 => out.extend_from_slice(&(stored as i16).to_le_bytes()),
        SampleFormat::I32 => out.extend_from_slice(&(stored as i32).to_le_bytes()),
        SampleFormat::F32 | SampleFormat::F16x8 => unreachable!(),
    }
    rounded != stored
}

#[cfg(test)]
mod tests {
    include!("float_wave_tests.rs");
}
