//! Whole-capture conversion, independent of the interface toolkit.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use argand_core::{SampleSource, SignalMeta, SourceError};
use argand_dsp::convert::ConversionPlan;
use argand_edit::{Capture, EditedSource};
use argand_io::write::{EncodedWave, FloatWaveRequest, Protected, Saved, SourceFile, WriteError};

const BLOCK: usize = 4096;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Preserve,
    Normalize,
}

#[derive(Clone)]
pub struct Request {
    pub capture: Capture,
    pub sources: Vec<Option<SourceFile>>,
    pub input: SignalMeta,
    pub output: SignalMeta,
    pub container: argand_io::write::Container,
    pub target: PathBuf,
    pub protected: Vec<Protected>,
    pub level: Level,
    /// The document's stored full scale, used by computed pieces' encodings.
    pub full_scale: f64,
    /// Existing writer request, eligible for a byte-preserving WAVE identity conversion.
    pub copy: Option<argand_io::write::SaveRequest>,
}

pub struct Converted {
    pub saved: Saved,
    pub clipped: u64,
}

impl Request {
    pub fn plan(&self) -> Result<ConversionPlan, WriteError> {
        let mut input = self.input.clone();
        input.len_samples = self.capture.len();
        let plan = ConversionPlan::new(&input, &self.output)
            .map_err(|error| self.refused(error.to_string()))?;
        EncodedWave::check(&self.output, plan.samples)?;
        self.container.check(&self.output, plan.samples)?;
        if !self.full_scale.is_finite() || self.full_scale <= 0.0 {
            return Err(self.refused("the stored sample scale is unknown".into()));
        }
        Ok(plan)
    }

    fn refused(&self, reason: String) -> WriteError {
        WriteError::Unsupported {
            path: self.target.clone(),
            reason,
        }
    }
}

pub fn run(
    request: &Request,
    progress: &mut dyn FnMut(u64, u64),
    cancel: &AtomicBool,
) -> Result<Converted, WriteError> {
    let plan = request.plan()?;
    if let Some(copy) = identity_copy(request, &plan) {
        render(request, &plan, progress, cancel, &mut |_| Ok(()))?;
        let saved = argand_io::write::save(&copy, progress, cancel)?;
        return Ok(Converted { saved, clipped: 0 });
    }
    let total = request.capture.len()
        * if request.level == Level::Normalize {
            2
        } else {
            1
        };
    let mut peak = 0.0f64;
    let gain = if request.level == Level::Normalize {
        render(
            request,
            &plan,
            &mut |done, _| progress(done, total),
            cancel,
            &mut |samples| {
                for &[i, q] in samples {
                    peak = peak.max(f64::from(i).hypot(f64::from(q)));
                }
                Ok(())
            },
        )?;
        crate::levelling::gain(peak, 1.0).unwrap_or(1.0)
    } else {
        1.0
    };
    let mut file = EncodedWave::create(FloatWaveRequest {
        meta: request.output.clone(),
        samples: plan.samples,
        target: request.target.clone(),
        sources: request.sources.iter().flatten().cloned().collect(),
        protected: request.protected.clone(),
    })?;
    let offset = total - request.capture.len();
    render(
        request,
        &plan,
        &mut |done, _| progress(offset + done, total),
        cancel,
        &mut |samples| {
            let scaled: Vec<[f32; 2]> = samples
                .iter()
                .map(|sample| sample.map(|v| (f64::from(v) * gain) as f32))
                .collect();
            file.write(&scaled)
        },
    )?;
    let clipped = file.clipped();
    crate::extraction::unchanged(&request.sources)?;
    let saved = file.finish(cancel)?;
    Ok(Converted { saved, clipped })
}

fn identity_copy(
    request: &Request,
    plan: &ConversionPlan,
) -> Option<argand_io::write::SaveRequest> {
    let mut copy = request.copy.clone()?;
    let eligible = plan.is_identity()
        && request.level == Level::Preserve
        && request.input.sample_type == request.output.sample_type
        && matches!(request.input.container, "wav" | "rf64" | "bw64")
        && request.capture.adjustment().is_identity()
        && request
            .sources
            .iter()
            .flatten()
            .all(|source| source.encoding.is_none());
    if !eligible {
        return None;
    }
    let storage = match &copy.format {
        Some(format) => format.storage(),
        None => argand_io::write::storage(copy.sources.first()?).ok()?,
    };
    let argand_io::write::Storage::Linear { bits, .. } = storage else {
        return None;
    };
    if usize::from(bits) != request.output.sample_type.format.bytes() * 8 {
        return None;
    }
    copy.target = request.target.clone();
    Some(copy)
}

fn render(
    request: &Request,
    plan: &ConversionPlan,
    progress: &mut dyn FnMut(u64, u64),
    cancel: &AtomicBool,
    sink: &mut impl FnMut(&[[f32; 2]]) -> Result<(), WriteError>,
) -> Result<(), WriteError> {
    let mut source = open(request)?;
    let mut stream = if plan.is_identity() {
        None
    } else {
        Some(
            plan.stream(request.capture.len())
                .map_err(|error| request.refused(error.to_string()))?,
        )
    };
    let mut checked = |samples: &[[f32; 2]]| {
        if cancel.load(Ordering::Relaxed) {
            return Err(WriteError::Cancelled);
        }
        if samples.iter().flatten().any(|v| !v.is_finite()) {
            return Err(request.refused("the converted signal contains a nonfinite sample".into()));
        }
        sink(samples)
    };
    let channels = request.input.channels();
    let mut buffer = vec![0.0; BLOCK * channels];
    let mut done = 0;
    progress(0, request.capture.len());
    while done < request.capture.len() {
        if cancel.load(Ordering::Relaxed) {
            return Err(WriteError::Cancelled);
        }
        let want = (request.capture.len() - done).min(BLOCK as u64) as usize * channels;
        let read = source
            .read(&mut buffer[..want])
            .map_err(|source| WriteError::Source {
                path: request.input.source.clone(),
                source,
            })?;
        if read == 0 {
            return Err(WriteError::SourceChanged {
                path: request.input.source.clone(),
            });
        }
        if buffer[..read].iter().any(|v| !v.is_finite()) {
            return Err(request.refused("the input signal contains a nonfinite sample".into()));
        }
        if let Some(stream) = &mut stream {
            stream.push(&buffer[..read], channels, &mut checked)?;
        } else {
            let pairs: Vec<[f32; 2]> = buffer[..read]
                .chunks_exact(channels)
                .map(|v| [v[0], v.get(1).copied().unwrap_or(0.0)])
                .collect();
            checked(&pairs)?;
        }
        done += (read / channels) as u64;
        progress(done, request.capture.len());
    }
    if let Some(stream) = &mut stream {
        stream.finish(&mut checked)?;
    }
    crate::extraction::unchanged(&request.sources)
}

fn open(request: &Request) -> Result<EditedSource, WriteError> {
    let mut sources: Vec<Option<Box<dyn SampleSource>>> = Vec::new();
    for file in &request.sources {
        let Some(file) = file else {
            sources.push(None);
            continue;
        };
        let source = crate::extraction::reopen(file)?;
        let factor = if let Some(encoding) = file.encoding {
            encoding.factor / request.full_scale
        } else {
            let scale = argand_io::write::storage(file)?.full_scale();
            source
                .original_sample_units()
                .map(|(factor, _)| factor / scale)
                .ok_or_else(|| request.refused("the source's sample scale is unknown".into()))?
        };
        sources.push(Some(Box::new(Nominal { source, factor })));
    }
    EditedSource::new(request.capture.clone(), sources, request.input.clone()).map_err(|source| {
        WriteError::Source {
            path: request.input.source.clone(),
            source,
        }
    })
}

struct Nominal {
    source: Box<dyn SampleSource>,
    factor: f64,
}
impl SampleSource for Nominal {
    fn meta(&self) -> &SignalMeta {
        self.source.meta()
    }
    fn seek(&mut self, sample: u64) -> Result<(), SourceError> {
        self.source.seek(sample)
    }
    fn read(&mut self, values: &mut [f32]) -> Result<usize, SourceError> {
        let read = self.source.read(values)?;
        for value in &mut values[..read] {
            *value = (f64::from(*value) * self.factor) as f32;
        }
        Ok(read)
    }
}

#[cfg(test)]
mod tests;
