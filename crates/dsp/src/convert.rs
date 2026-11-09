//! Conversion between explicitly requested physical bands, in bounded streams.

use argand_core::SignalMeta;

use crate::{ExtractError, ExtractPlan, Extractor, resample};

/// A checked conversion, cheap to construct on the interface thread.
#[derive(Debug, Clone)]
pub struct ConversionPlan {
    input_rate: f64,
    reference_delta: f64,
    input_real: bool,
    local_band: (f64, f64),
    width: f64,
    output_rate: f64,
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
    retains_input_band: bool,
}

impl ConversionPlan {
    pub fn new(input: &SignalMeta, output: &SignalMeta) -> Result<Self, ExtractError> {
        let plan =
            Self::with_reference_delta(input, output, output.center_freq - input.center_freq)?;
        let a = input.frequency_span();
        let b = output.frequency_span();
        if a.0 >= a.1 || b.0 >= b.1 {
            return Err(ExtractError::OutsideCapture {
                low: plan.band.0,
                high: plan.band.1,
            });
        }
        Ok(plan)
    }

    /// Use an exact output-minus-input reference delta in input-local hertz.
    /// Carrier conversion must not recover this delta from rounded metadata origins.
    /// Output metadata is a caller-owned annotation and does not determine this shift.
    pub fn with_reference_delta(
        input: &SignalMeta,
        output: &SignalMeta,
        delta: f64,
    ) -> Result<Self, ExtractError> {
        for rate in [input.sample_rate, output.sample_rate] {
            if !rate.is_finite() || rate <= 0.0 {
                return Err(ExtractError::BadRate(rate));
            }
        }
        let a = input.frequency_span();
        let b = output.frequency_span();
        let native = relative_span(input);
        let target = relative_span(output);
        let local_band = (
            native.0.max(delta + target.0),
            native.1.min(delta + target.1),
        );
        let band = (
            input.center_freq + local_band.0,
            input.center_freq + local_band.1,
        );
        if ![
            input.center_freq,
            output.center_freq,
            delta,
            a.0,
            a.1,
            b.0,
            b.1,
        ]
        .into_iter()
        .all(f64::is_finite)
            || local_band.0 >= local_band.1
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
            && delta == 0.0
            && input.is_iq() == output.is_iq();
        let input_width = input.sample_rate / if input.is_iq() { 1.0 } else { 2.0 };
        let output_width = output.sample_rate / if output.is_iq() { 1.0 } else { 2.0 };
        // Subtracting large physical references must not enlarge a declared Nyquist span.
        let width = (local_band.1 - local_band.0)
            .min(input_width)
            .min(output_width);
        if !identity {
            ExtractPlan::check_conversion(
                input.sample_rate,
                local_band.0,
                local_band.1,
                !input.is_iq(),
            )?;
        }
        Ok(Self {
            input_rate: input.sample_rate,
            reference_delta: delta,
            input_real: !input.is_iq(),
            local_band,
            width,
            output_rate: output.sample_rate,
            output_real: !output.is_iq(),
            band,
            input_band: a,
            passband: if identity {
                band
            } else {
                let edge = width * 0.01;
                (
                    input.center_freq + (local_band.0 + edge),
                    input.center_freq + (local_band.1 - edge),
                )
            },
            samples: samples as u64,
            identity,
            retains_input_band: local_band == native,
        })
    }

    /// Retained input-array frequencies before finite filter transitions.
    pub fn local_input_band(&self) -> (f64, f64) {
        self.local_band
    }

    /// Whether the nominal input band is wholly retained, compared in local coordinates.
    pub fn retains_input_band(&self) -> bool {
        self.retains_input_band
    }

    /// No filtering, translation or reconstruction is needed.
    pub fn is_identity(&self) -> bool {
        self.identity
    }

    /// Build filters on the worker thread, never while rendering the dialog.
    pub fn stream(&self, input_samples: u64) -> Result<ConversionStream, ExtractError> {
        let (low, high) = self.local_band;
        let extract = ExtractPlan::for_conversion(self.input_rate, low, high, self.input_real)?;
        let reconstruction =
            resample::Plan::bandlimited(extract.output_rate(), self.output_rate, self.width)?;
        let centre = extract.centre();
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
            step: (centre - self.reference_delta) / self.output_rate,
            real: self.output_real,
            given: 0,
        })
    }
}

fn relative_span(meta: &SignalMeta) -> (f64, f64) {
    let half = meta.sample_rate / 2.0;
    (if meta.is_iq() { -half } else { 0.0 }, half)
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
