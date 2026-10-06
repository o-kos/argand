use super::*;
use std::path::PathBuf;

use argand_core::{Domain, SampleFormat, SampleType};

/// A source in memory whose sample `n` is `base + n` in every channel.
struct Ramp {
    meta: SignalMeta,
    base: f32,
    pos: u64,
}

impl Ramp {
    fn boxed(len: u64, base: f32, channels: usize) -> Box<dyn SampleSource> {
        let domain = if channels == 2 { Domain::Iq } else { Domain::Real };
        Box::new(Self {
            meta: SignalMeta {
                sample_rate: 1000.0,
                center_freq: 0.0,
                sample_type: SampleType::new(domain, SampleFormat::F32),
                len_samples: len,
                container: "raw",
                divisor: 1.0,
                source: PathBuf::from("ramp"),
            },
            base,
            pos: 0,
        })
    }
}

impl SampleSource for Ramp {
    fn meta(&self) -> &SignalMeta {
        &self.meta
    }

    fn seek(&mut self, sample: u64) -> Result<(), SourceError> {
        self.pos = sample;
        Ok(())
    }

    fn read(&mut self, buf: &mut [f32]) -> Result<usize, SourceError> {
        let channels = self.meta.channels();
        let count = ((buf.len() / channels) as u64).min(self.meta.len_samples - self.pos) as usize;
        for (index, frame) in buf[..count * channels].chunks_mut(channels).enumerate() {
            let value = self.base + (self.pos + index as u64) as f32;
            frame.fill(value);
        }
        self.pos += count as u64;
        Ok(count * channels)
    }
}

fn span(a: u64, b: u64) -> SampleSpan {
    SampleSpan::between(a, b).unwrap()
}

fn read_all(source: &mut EditedSource, chunk: usize) -> Vec<f32> {
    let mut out = Vec::new();
    let mut buf = vec![0.0; chunk];
    loop {
        let n = source.read(&mut buf).unwrap();
        if n == 0 {
            return out;
        }
        out.extend_from_slice(&buf[..n]);
    }
}

fn edited(capture: Capture, channels: usize) -> EditedSource {
    let meta = Ramp::boxed(0, 0.0, channels).meta().clone();
    EditedSource::new(
        capture,
        vec![Some(Ramp::boxed(100, 0.0, channels)), Some(Ramp::boxed(100, 1000.0, channels))],
        meta,
    )
    .unwrap()
}

#[test]
fn reads_follow_the_pieces_across_sources() {
    let clip = Capture::whole(SourceId(1), 100).copy(span(5, 8));
    let capture = Capture::whole(SourceId(0), 10).delete(span(2, 4)).insert(3, &clip);
    let mut source = edited(capture, 1);
    assert_eq!(source.meta().len_samples, 11);
    let expected = [0., 1., 4., 1005., 1006., 1007., 5., 6., 7., 8., 9.];
    for chunk in [1, 2, 3, 64] {
        source.seek(0).unwrap();
        assert_eq!(read_all(&mut source, chunk), expected, "chunk {chunk}");
    }
    source.seek(4).unwrap();
    assert_eq!(read_all(&mut source, 64)[..3], [1006., 1007., 5.]);
}

#[test]
fn i_and_q_stay_together_across_a_boundary() {
    let capture = Capture::whole(SourceId(0), 6).delete(span(2, 4));
    let mut source = edited(capture, 2);
    assert_eq!(read_all(&mut source, 3), [0., 0., 1., 1., 4., 4., 5., 5.]);
}

#[test]
fn a_new_version_and_a_missing_source() {
    let mut source = edited(Capture::whole(SourceId(0), 10), 1);
    source.set_capture(Capture::whole(SourceId(0), 10).delete(span(0, 9))).unwrap();
    assert_eq!(read_all(&mut source, 8), [9.]);
    assert!(source.set_capture(Capture::whole(SourceId(2), 3)).is_err());
    assert!(source.seek(2).is_err());
}
