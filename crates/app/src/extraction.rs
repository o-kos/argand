//! Saving a frequency band of a capture, or a rectangle of it, as a new complex capture.
//!
//! The band is computed rather than copied. The edited capture is read through
//! an `EditedSource`, `argand_dsp::extract` moves the band to baseband and
//! decimates it, and `argand_io::write::FloatWave` writes the preserved domain as `f32`.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use argand_core::{FrequencyBand, SampleSource, SampleSpan, SignalMeta};
use argand_dsp::{ExtractPlan, Extractor};
use argand_edit::{Capture, EditedSource};
use argand_io::write::{FloatWave, FloatWaveRequest, Protected, Saved, SourceFile, WriteError};

/// Input samples read at a time, which bounds what one block holds.
const BLOCK_SAMPLES: usize = 1 << 18;

/// A band of a capture, over a time span of it, to save as a file.
#[derive(Debug, Clone)]
pub struct ExtractRequest {
    /// The version of the capture the band is taken from.
    pub capture: Capture,
    /// The files it reads, by source id, with none for those it does not read.
    pub sources: Vec<Option<SourceFile>>,
    /// The capture as the window describes it, with its rate, reference frequency and kind.
    pub meta: SignalMeta,
    pub span: SampleSpan,
    /// The band in physical hertz.
    pub band: FrequencyBand,
    pub target: PathBuf,
    pub protected: Vec<Protected>,
}

impl ExtractRequest {
    /// The band's edges relative to the capture's baseband 0 Hz, held to the capture's band.
    ///
    /// The band was clamped in physical hertz, and taking the reference away again
    /// can carry an edge a rounding error past the capture's.
    fn edges(&self) -> (f64, f64) {
        let nyquist = self.meta.sample_rate / 2.0;
        let floor = if self.meta.is_iq() { -nyquist } else { 0.0 };
        let reference = self.meta.center_freq;
        (
            (self.band.low() - reference).max(floor),
            (self.band.high() - reference).min(nyquist),
        )
    }

    fn refused(&self, error: &argand_dsp::ExtractError) -> WriteError {
        WriteError::Unsupported {
            path: self.target.clone(),
            reason: error.to_string(),
        }
    }

    /// Whether the band can be saved, quickly enough for the window's thread.
    pub fn check(&self) -> Result<(), WriteError> {
        let (low, high) = self.edges();
        ExtractPlan::check(self.meta.sample_rate, low, high, !self.meta.is_iq())
            .map_err(|error| self.refused(&error))
    }

    /// The plan the band is extracted by, its filter built.
    pub fn plan(&self) -> Result<ExtractPlan, WriteError> {
        let (low, high) = self.edges();
        ExtractPlan::for_export(self.meta.sample_rate, low, high, !self.meta.is_iq())
            .map_err(|error| self.refused(&error))
    }
}

/// Extract and write the band, reporting `(input samples done, total)` and stopping when `cancel` is set.
pub fn run(
    request: &ExtractRequest,
    progress: &mut dyn FnMut(u64, u64),
    cancel: &AtomicBool,
) -> Result<Saved, WriteError> {
    let len = request.capture.len();
    if request.span.end() > len {
        return Err(WriteError::OutOfRange);
    }
    let plan = request.plan()?;
    let mut source = open(request)?;
    let resampling = output_plan(request, &plan)?;
    let padding = resampling.margin().saturating_mul(plan.decimation());
    let leading = padding.saturating_sub(request.span.start());
    let trailing = padding.saturating_sub(len - request.span.end());
    let span = SampleSpan::between(
        request.span.start().saturating_sub(padding),
        request.span.end().saturating_add(padding).min(len),
    )
    .ok_or(WriteError::OutOfRange)?;
    let margin = plan.margin();
    let from = span.start().saturating_sub(margin);
    let to = span.end().saturating_add(margin).min(len);
    let extended = span
        .count()
        .checked_add(leading)
        .and_then(|count| count.checked_add(trailing))
        .ok_or(WriteError::OutOfRange)?;
    let mut output = Output::new(request, &plan, resampling, span, leading)?;
    let mut extractor = Extractor::new(plan, extended).map_err(|error| request.refused(&error))?;
    let mut out = Vec::new();
    extractor.push_zeros(margin + leading - (span.start() - from), &mut out);
    let channels = request.meta.channels();
    let path = &request.meta.source;
    let read_error = |error| WriteError::Source {
        path: path.clone(),
        source: error,
    };
    source.seek(from).map_err(read_error)?;
    let batch = ((BLOCK_SAMPLES as f64 * request.meta.sample_rate / output_rate(request)).floor()
        as usize)
        .clamp(1, BLOCK_SAMPLES);
    let mut buffer = vec![0.0f32; batch * channels];
    let total = to - from;
    let mut done = 0;
    progress(0, total);
    while done < total {
        if cancel.load(Ordering::Relaxed) {
            return Err(WriteError::Cancelled);
        }
        let want = ((total - done) as usize).min(batch) * channels;
        let read = source.read(&mut buffer[..want]).map_err(read_error)?;
        if read == 0 {
            return Err(WriteError::SourceChanged { path: path.clone() });
        }
        extractor.push(&buffer[..read], channels, &mut out);
        output.write(&out, cancel)?;
        out.clear();
        done += (read / channels) as u64;
        progress(done, total);
    }
    extractor.push_zeros(extractor.wanted(), &mut out);
    output.write(&out, cancel)?;
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
    output.finish(cancel)
}

fn output_rate(request: &ExtractRequest) -> f64 {
    let width = request.band.high() - request.band.low();
    let factor = if request.meta.is_iq() { 1.25 } else { 2.5 };
    (factor * width / 1000.0).ceil() * 1000.0
}

fn output_plan(
    request: &ExtractRequest,
    plan: &ExtractPlan,
) -> Result<argand_dsp::resample::Plan, WriteError> {
    let (low, high) = request.edges();
    argand_dsp::resample::Plan::new(plan.output_rate(), output_rate(request), high - low)
        .map_err(|error| request.refused(&error))
}

struct Output {
    file: FloatWave,
    stream: argand_dsp::resample::Stream,
    converted: Vec<[f32; 2]>,
    phase: f64,
    step: f64,
    real: bool,
}

impl Output {
    fn new(
        request: &ExtractRequest,
        plan: &ExtractPlan,
        resampling: argand_dsp::resample::Plan,
        span: SampleSpan,
        leading: u64,
    ) -> Result<Self, WriteError> {
        let rate = output_rate(request);
        let samples = (request.span.count() as f64 * rate / request.meta.sample_rate).ceil();
        if !samples.is_finite() || samples >= (1u64 << 52) as f64 {
            return Err(WriteError::OutOfRange);
        }
        let samples = samples as u64;
        let offset = request.span.start() - span.start() + leading;
        let stream = argand_dsp::resample::Stream::new(
            resampling,
            argand_dsp::resample::Span {
                offset: plan.coordinate(offset),
                samples,
            },
        )
        .map_err(|error| request.refused(&error))?;
        let mut meta = request.meta.clone();
        meta.sample_rate = rate;
        meta.center_freq = 0.0;
        let file = FloatWave::create(FloatWaveRequest {
            meta,
            samples,
            target: request.target.clone(),
            sources: request.sources.iter().flatten().cloned().collect(),
            protected: request.protected.clone(),
        })?;
        let (low, high) = request.edges();
        let real = !request.meta.is_iq();
        let step = if real {
            (high - low) / (2.0 * rate)
        } else {
            0.0
        };
        let phase = (plan.centre() / request.meta.sample_rate * offset as f64).rem_euclid(1.0);
        Ok(Self {
            file,
            stream,
            converted: Vec::new(),
            phase,
            step,
            real,
        })
    }

    fn write(&mut self, samples: &[[f32; 2]], cancel: &AtomicBool) -> Result<(), WriteError> {
        let mut consumed = 0;
        while consumed < samples.len() {
            if cancel.load(Ordering::Relaxed) {
                return Err(WriteError::Cancelled);
            }
            consumed += self.stream.push(&samples[consumed..], &mut self.converted);
            self.flush()?;
        }
        Ok(())
    }

    fn flush(&mut self) -> Result<(), WriteError> {
        for sample in &mut self.converted {
            let (sin, cos) = (std::f64::consts::TAU * self.phase).sin_cos();
            self.phase = (self.phase + self.step).rem_euclid(1.0);
            let [i, q] = sample.map(f64::from);
            sample[0] = (i * cos - q * sin) as f32;
            sample[1] = if self.real {
                0.0
            } else {
                (i * sin + q * cos) as f32
            };
        }
        self.file.write(&self.converted)?;
        self.converted.clear();
        Ok(())
    }

    fn finish(mut self, cancel: &AtomicBool) -> Result<Saved, WriteError> {
        while !self.stream.is_done() {
            if cancel.load(Ordering::Relaxed) {
                return Err(WriteError::Cancelled);
            }
            self.stream.finish(&mut self.converted);
            self.flush()?;
        }
        self.file.finish(cancel)
    }
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
        // The levels the window shows, not a normalization resolved again with another budget.
        let (reader, stamp) =
            argand_io::reopen_stamped(&source.meta, &source.hints).map_err(|error| {
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
