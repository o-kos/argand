//! A complex `f32` capture written as it is computed, as a band extraction produces it.

use super::*;

/// What a computed capture is and where it goes.
#[derive(Debug, Clone)]
pub struct FloatIqRequest {
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

/// An I/Q `f32` WAVE file being written, moved over its target only by [`FloatIq::finish`].
#[derive(Debug)]
pub struct FloatIq {
    partial: Partial,
    target: PathBuf,
    protected: Vec<Protected>,
    read: Vec<Option<(u64, u64)>>,
    samples: u64,
    written: u64,
    container: &'static str,
}

impl FloatIq {
    /// Start the file beside its target with its header, refusing a target that is a source or protected.
    pub fn create(request: FloatIqRequest) -> Result<Self, WriteError> {
        Self::create_with_limit(request, RIFF_LIMIT)
    }

    pub(crate) fn create_with_limit(
        request: FloatIqRequest,
        riff_limit: u64,
    ) -> Result<Self, WriteError> {
        let FloatIqRequest {
            mut meta,
            samples,
            target,
            sources,
            protected,
        } = request;
        let read_paths = sources.iter().map(|source| source.meta.source.clone());
        check_target(read_paths, &protected, &target)?;
        meta.sample_type = argand_core::SampleType::new(argand_core::Domain::Iq, SampleFormat::F32);
        let fmt = synthesized_fmt(&meta)?;
        let data_len = samples.checked_mul(8).ok_or(WriteError::OutOfRange)?;
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
            samples,
            written: 0,
            container,
        })
    }

    /// Append samples, refusing more than the header states.
    pub fn write(&mut self, samples: &[[f32; 2]]) -> Result<(), WriteError> {
        let count = samples.len() as u64;
        if self.written + count > self.samples {
            return Err(WriteError::OutOfRange);
        }
        let mut bytes = Vec::with_capacity(samples.len() * 8);
        for value in samples.iter().flatten() {
            bytes.extend_from_slice(&value.to_le_bytes());
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
        self.partial.sync()?;
        // Syncing can take long, and a save cancelled meanwhile must leave the target alone.
        if cancel.load(Ordering::Relaxed) {
            return Err(WriteError::Cancelled);
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

#[cfg(test)]
mod tests {
    include!("float_iq_tests.rs");
}
