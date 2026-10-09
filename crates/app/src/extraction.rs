//! Saving a frequency band of a capture, or a rectangle of it, as a new complex capture.
//!
//! The band is computed rather than copied: the edited capture is read through
//! an `EditedSource`, `argand_dsp::extract` moves the band to baseband and
//! decimates it, and `argand_io::write::FloatIq` writes it as I/Q `f32`.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use argand_core::{FrequencyBand, SampleSource, SampleSpan, SignalMeta};
use argand_dsp::{ExtractPlan, Extractor};
use argand_edit::{Capture, EditedSource};
use argand_io::write::{FloatIq, FloatIqRequest, Protected, Saved, SourceFile, WriteError};

/// Input samples read at a time, which bounds what one block holds.
const BLOCK_SAMPLES: usize = 1 << 20;

/// A band of a capture, over a time span of it, to save as a file.
#[derive(Debug, Clone)]
pub struct ExtractRequest {
    /// The version of the capture the band is taken from.
    pub capture: Capture,
    /// The files it reads, by source id, with none for those it does not read.
    pub sources: Vec<Option<SourceFile>>,
    /// The capture as the window describes it: its rate, reference frequency and kind.
    pub meta: SignalMeta,
    pub span: SampleSpan,
    /// The band in physical hertz.
    pub band: FrequencyBand,
    pub target: PathBuf,
    pub protected: Vec<Protected>,
}

impl ExtractRequest {
    /// The plan the band is extracted by, refused with the reason it cannot be saved.
    pub fn plan(&self) -> Result<ExtractPlan, WriteError> {
        let reference = self.meta.center_freq;
        ExtractPlan::new(
            self.meta.sample_rate,
            self.band.low() - reference,
            self.band.high() - reference,
            !self.meta.is_iq(),
        )
        .map_err(|error| WriteError::Unsupported {
            path: self.target.clone(),
            reason: error.to_string(),
        })
    }
}

/// Extract and write the band, reporting `(input samples done, total)` and stopping when `cancel` is set.
pub fn run(
    request: &ExtractRequest,
    progress: &mut dyn FnMut(u64, u64),
    cancel: &AtomicBool,
) -> Result<Saved, WriteError> {
    let plan = request.plan()?;
    let mut source = open(request)?;
    let margin = plan.margin();
    let span = request.span;
    let len = request.capture.len();
    let from = span.start().saturating_sub(margin);
    let to = span.end().saturating_add(margin).min(len);
    let mut output_meta = request.meta.clone();
    output_meta.sample_rate = plan.output_rate();
    output_meta.center_freq = request.meta.center_freq + plan.centre();
    let mut file = FloatIq::create(FloatIqRequest {
        meta: output_meta,
        samples: plan.output_len(span.count()),
        target: request.target.clone(),
        sources: request.sources.iter().flatten().cloned().collect(),
        protected: request.protected.clone(),
    })?;
    let mut extractor = Extractor::new(plan, span.count());
    let mut out = Vec::new();
    extractor.push_zeros(margin - (span.start() - from), &mut out);
    let channels = request.meta.channels();
    let path = &request.meta.source;
    let read_error = |error| WriteError::Source {
        path: path.clone(),
        source: error,
    };
    source.seek(from).map_err(read_error)?;
    let mut buffer = vec![0.0f32; BLOCK_SAMPLES * channels];
    let total = to - from;
    let mut done = 0;
    progress(0, total);
    while done < total {
        if cancel.load(Ordering::Relaxed) {
            return Err(WriteError::Cancelled);
        }
        let want = ((total - done) as usize).min(BLOCK_SAMPLES) * channels;
        let read = source.read(&mut buffer[..want]).map_err(read_error)?;
        if read == 0 {
            return Err(WriteError::SourceChanged { path: path.clone() });
        }
        extractor.push(&buffer[..read], channels, &mut out);
        file.write(&out)?;
        out.clear();
        done += (read / channels) as u64;
        progress(done, total);
    }
    extractor.push_zeros(extractor.wanted(), &mut out);
    file.write(&out)?;
    if cancel.load(Ordering::Relaxed) {
        return Err(WriteError::Cancelled);
    }
    // A mapped file changed while it was read gives other samples without an error.
    for source in request.sources.iter().flatten() {
        let path = &source.meta.source;
        if argand_io::write::SourceStamp::of(path).ok() != source.stamp {
            return Err(WriteError::SourceChanged { path: path.clone() });
        }
    }
    file.finish()
}

/// The capture read through its files, each checked to be the file it was when opened.
fn open(request: &ExtractRequest) -> Result<EditedSource, WriteError> {
    let mut opened: Vec<Option<Box<dyn SampleSource>>> = Vec::new();
    for source in &request.sources {
        let Some(source) = source else {
            opened.push(None);
            continue;
        };
        let path = &source.meta.source;
        let (reader, stamp) = argand_io::open_stamped(path, &source.hints).map_err(|error| {
            WriteError::Unsupported {
                path: path.clone(),
                reason: error.to_string(),
            }
        })?;
        if stamp.is_none() || stamp != source.stamp {
            return Err(WriteError::SourceChanged { path: path.clone() });
        }
        opened.push(Some(reader));
    }
    let mut meta = request.meta.clone();
    meta.len_samples = request.capture.len();
    EditedSource::new(request.capture.clone(), opened, meta).map_err(|error| WriteError::Source {
        path: request.meta.source.clone(),
        source: error,
    })
}

#[cfg(test)]
mod tests {
    include!("extraction_tests.rs");
}
