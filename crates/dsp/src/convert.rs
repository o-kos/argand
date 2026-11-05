//! Conversion between explicitly requested physical bands, in bounded streams.

use argand_core::SignalMeta;

use crate::{ExtractError, ExtractPlan, Extractor, resample};

/// A checked conversion, cheap to construct on the interface thread.
#[derive(Debug, Clone)]
pub struct ConversionPlan {
    input_rate: f64,
    input_reference: f64,
    input_real: bool,
    output_rate: f64,
    output_reference: f64,
    output_real: bool,
    /// Retained physical edges, before finite filter transitions.
    pub band: (f64, f64),
    /// The physical input band, for reporting discarded edges.
    pub input_band: (f64, f64),
    /// Interior with unity passband gain; native real endpoints are handled separately.
    pub passband: (f64, f64),
    /// Number of output frames, including both components for IQ.
    pub samples: u64,
    identity: bool,
}

impl ConversionPlan {
    pub fn new(input: &SignalMeta, output: &SignalMeta) -> Result<Self, ExtractError> {
        for rate in [input.sample_rate, output.sample_rate] {
            if !rate.is_finite() || rate <= 0.0 {
                return Err(ExtractError::BadRate(rate));
            }
        }
        let a = input.frequency_span();
        let b = output.frequency_span();
        let band = (a.0.max(b.0), a.1.min(b.1));
        if ![input.center_freq, output.center_freq, a.0, a.1, b.0, b.1]
            .into_iter()
            .all(f64::is_finite)
            || band.0 >= band.1
        {
            return Err(ExtractError::OutsideCapture {
                low: band.0,
                high: band.1,
            });
        }
        let samples = (input.len_samples as f64 * output.sample_rate / input.sample_rate).ceil();
        if input.len_samples >= (1u64 << 52)
            || input.len_samples == 0
            || !samples.is_finite()
            || samples >= (1u64 << 52) as f64
        {
            return Err(ExtractError::TooLong(input.len_samples));
        }
        let identity = input.sample_rate == output.sample_rate
            && input.center_freq == output.center_freq
            && input.is_iq() == output.is_iq();
        if !identity {
            ExtractPlan::check_conversion(
                input.sample_rate,
                band.0 - input.center_freq,
                band.1 - input.center_freq,
                !input.is_iq(),
            )?;
        }
        Ok(Self {
            input_rate: input.sample_rate,
            input_reference: input.center_freq,
            input_real: !input.is_iq(),
            output_rate: output.sample_rate,
            output_reference: output.center_freq,
            output_real: !output.is_iq(),
            band,
            input_band: a,
            passband: if identity {
                band
            } else {
                let edge = (band.1 - band.0) * 0.01;
                (band.0 + edge, band.1 - edge)
            },
            samples: samples as u64,
            identity,
        })
    }

    /// No filtering, translation or reconstruction is needed.
    pub fn is_identity(&self) -> bool {
        self.identity
    }

    /// Build filters on the worker thread, never while rendering the dialog.
    pub fn stream(&self, input_samples: u64) -> Result<ConversionStream, ExtractError> {
        let extract = ExtractPlan::for_conversion(
            self.input_rate,
            self.band.0 - self.input_reference,
            self.band.1 - self.input_reference,
            self.input_real,
        )?;
        // A fully occupied target Nyquist band needs a finite guard for its reconstruction filter.
        let width = (self.band.1 - self.band.0).min(self.output_rate * 0.99);
        let reconstruction = resample::Plan::new(extract.output_rate(), self.output_rate, width)?;
        let centre = extract.centre() + self.input_reference;
        let margin = extract.margin();
        let mut extractor = Extractor::new(extract, input_samples)?;
        let mut intermediate = Vec::new();
        extractor.push_zeros(margin, &mut intermediate);
        let stream = resample::Stream::new(
            reconstruction,
            resample::Span {
                offset: 0.0,
                samples: self.samples,
            },
        )?;
        Ok(ConversionStream {
            extractor,
            reconstruction: stream,
            intermediate,
            step: (centre - self.output_reference) / self.output_rate,
            real: self.output_real,
            given: 0,
        })
    }
}

/// Bounded FFT extraction and fractional reconstruction, followed by physical translation.
pub struct ConversionStream {
    extractor: Extractor,
    reconstruction: resample::Stream,
    intermediate: Vec<[f32; 2]>,
    step: f64,
    real: bool,
    given: u64,
}

impl ConversionStream {
    /// Feed a bounded input block. The sink receives at most 4096 frames per call.
    pub fn push<E>(
        &mut self,
        input: &[f32],
        channels: usize,
        sink: &mut impl FnMut(&[[f32; 2]]) -> Result<(), E>,
    ) -> Result<(), E> {
        self.extractor.push(input, channels, &mut self.intermediate);
        self.drain(sink)
    }

    pub fn finish<E>(
        &mut self,
        sink: &mut impl FnMut(&[[f32; 2]]) -> Result<(), E>,
    ) -> Result<(), E> {
        while !self.extractor.is_done() {
            self.extractor
                .push_zeros(self.extractor.wanted().min(4096), &mut self.intermediate);
            self.drain(sink)?;
        }
        while !self.reconstruction.is_done() {
            let mut out = Vec::new();
            self.reconstruction.finish(&mut out);
            self.translate(&mut out);
            sink(&out)?;
        }
        Ok(())
    }

    fn drain<E>(&mut self, sink: &mut impl FnMut(&[[f32; 2]]) -> Result<(), E>) -> Result<(), E> {
        let mut at = 0;
        while at < self.intermediate.len() {
            let mut out = Vec::new();
            at += self.reconstruction.push(&self.intermediate[at..], &mut out);
            self.translate(&mut out);
            sink(&out)?;
        }
        self.intermediate.clear();
        Ok(())
    }

    fn translate(&mut self, out: &mut [[f32; 2]]) {
        for sample in out {
            let phase = std::f64::consts::TAU * (self.given as f64 * self.step).rem_euclid(1.0);
            let (sin, cos) = phase.sin_cos();
            let [i, q] = sample.map(f64::from);
            *sample = [
                (i * cos - q * sin) as f32,
                if self.real {
                    0.0
                } else {
                    (i * sin + q * cos) as f32
                },
            ];
            self.given += 1;
        }
    }
}

#[cfg(test)]
mod tests;
