use super::*;
use argand_core::{Domain, SampleFormat, SampleType};
use argand_io::OpenHints;
use argand_io::testutil::{TempDir, iq_tone, write_wav};

const RATE: f64 = 48_000.0;

/// A complex tone at `frequency` on disk, as a source of the request.
fn capture(dir: &TempDir, frequency: f64) -> SourceFile {
    let values = iq_tone(48_000, RATE, frequency, 0.5);
    let path = write_wav(
        &dir.join("capture.wav"),
        SampleType::new(Domain::Iq, SampleFormat::F32),
        RATE as u32,
        &values,
        1.0,
    );
    let (source, stamp) = argand_io::open_stamped(&path, &OpenHints::default()).unwrap();
    SourceFile {
        meta: source.meta().clone(),
        hints: OpenHints::default(),
        stamp,
        encoding: None,
    }
}

fn request(
    source: SourceFile,
    span: SampleSpan,
    band: FrequencyBand,
    target: PathBuf,
) -> ExtractRequest {
    let mut meta = source.meta.clone();
    meta.center_freq = 10_000_000.0;
    ExtractRequest {
        capture: Capture::whole(argand_edit::SourceId(0), source.meta.len_samples),
        sources: vec![Some(source)],
        meta,
        span,
        band,
        target,
        protected: Vec::new(),
    }
}

fn read_all(path: &std::path::Path) -> (SignalMeta, Vec<f32>) {
    let mut source = argand_io::open(path, &OpenHints::default()).unwrap();
    let meta = source.meta().clone();
    let mut values = vec![0.0; meta.len_samples as usize * meta.channels()];
    let mut filled = 0;
    while filled < values.len() {
        filled += source.read(&mut values[filled..]).unwrap();
    }
    (meta, values)
}

#[test]
fn a_band_saves_as_a_complex_capture_centred_on_it() {
    let dir = TempDir::new("extract-band");
    let source = capture(&dir, 6_500.0);
    let target = dir.join("band.wav");
    let whole = SampleSpan::between(0, 48_000).unwrap();
    let band = FrequencyBand::between(10_005_000.0, 10_007_000.0).unwrap();
    let request = request(source, whole, band, target.clone());
    let saved = run(&request, &mut |_, _| {}, &AtomicBool::new(false)).unwrap();
    let (meta, values) = read_all(&target);
    assert_eq!(meta.sample_rate, 3_000.0);
    assert_eq!(meta.center_freq, 0.0);
    assert_eq!(meta.len_samples, 3_000);
    assert_eq!(saved.samples, meta.len_samples);
    let magnitudes: Vec<f32> = values.chunks(2).map(|iq| iq[0].hypot(iq[1])).collect();
    let middle = &magnitudes[magnitudes.len() / 4..magnitudes.len() * 3 / 4];
    assert!(middle.iter().all(|m| (m - 0.5).abs() < 0.01), "{middle:?}");
}

#[test]
fn a_rectangle_saves_its_span_only() {
    let dir = TempDir::new("extract-rectangle");
    let source = capture(&dir, 6_500.0);
    let target = dir.join("rectangle.wav");
    let span = SampleSpan::between(10_000, 20_000).unwrap();
    let band = FrequencyBand::between(10_005_000.0, 10_007_000.0).unwrap();
    run(
        &request(source, span, band, target.clone()),
        &mut |_, _| {},
        &AtomicBool::new(false),
    )
    .unwrap();
    // The capture's own samples beyond the span feed the filter, so even the edges keep the tone.
    let (meta, values) = read_all(&target);
    assert_eq!(meta.len_samples, 625);
    let first = values[0].hypot(values[1]);
    assert!((first - 0.5).abs() < 0.01, "{first}");
}

#[test]
fn a_source_changed_since_it_was_opened_is_refused() {
    let dir = TempDir::new("extract-changed");
    let mut source = capture(&dir, 6_500.0);
    // The stamp of another file stands for the file as it was before it changed.
    std::fs::write(dir.join("other.bin"), [0u8; 3]).unwrap();
    source.stamp = argand_io::write::SourceStamp::of(&dir.join("other.bin")).ok();
    let target = dir.join("band.wav");
    let whole = SampleSpan::between(0, 48_000).unwrap();
    let band = FrequencyBand::between(10_005_000.0, 10_007_000.0).unwrap();
    let result = run(
        &request(source, whole, band, target.clone()),
        &mut |_, _| {},
        &AtomicBool::new(false),
    );
    assert!(
        matches!(result, Err(WriteError::SourceChanged { .. })),
        "{result:?}"
    );
    assert!(!target.exists());
}

#[test]
fn a_cancelled_extraction_leaves_nothing() {
    let dir = TempDir::new("extract-cancel");
    let source = capture(&dir, 6_500.0);
    let target = dir.join("band.wav");
    let whole = SampleSpan::between(0, 48_000).unwrap();
    let band = FrequencyBand::between(10_005_000.0, 10_007_000.0).unwrap();
    let result = run(
        &request(source, whole, band, target.clone()),
        &mut |_, _| {},
        &AtomicBool::new(true),
    );
    assert!(matches!(result, Err(WriteError::Cancelled)));
    assert_eq!(
        std::fs::read_dir(dir.path()).unwrap().count(),
        1,
        "only the capture is left"
    );
}

#[test]
fn a_band_at_the_capture_edge_survives_the_reference_being_taken_away() {
    let dir = TempDir::new("extract-edge");
    let source = capture(&dir, 6_500.0);
    let mut request = request(
        source,
        SampleSpan::between(0, 48_000).unwrap(),
        FrequencyBand::between(0.0, 1.0).unwrap(),
        dir.join("band.wav"),
    );
    request.meta.sample_rate = 24_000.0 / 9.0;
    request.meta.center_freq = 12_580_000.0;
    let nyquist = request.meta.sample_rate / 2.0;
    let reference = request.meta.center_freq;
    request.band =
        FrequencyBand::between(reference + nyquist - 1_000.0, reference + nyquist).unwrap();
    assert!(
        request.band.high() - reference > nyquist,
        "the rounding this guards against"
    );
    assert!(request.check().is_ok());
    assert!(request.plan().is_ok());
}

#[test]
fn a_band_is_read_at_the_levels_the_window_shows() {
    let dir = TempDir::new("extract-levels");
    let mut source = capture(&dir, 6_500.0);
    // A normalization the window resolved differs from what a fresh scan would.
    source.meta.divisor = 2.0;
    let target = dir.join("band.wav");
    let whole = SampleSpan::between(0, 48_000).unwrap();
    let band = FrequencyBand::between(10_005_000.0, 10_007_000.0).unwrap();
    run(
        &request(source, whole, band, target.clone()),
        &mut |_, _| {},
        &AtomicBool::new(false),
    )
    .unwrap();
    let (_, values) = read_all(&target);
    let magnitudes: Vec<f32> = values.chunks(2).map(|iq| iq[0].hypot(iq[1])).collect();
    let middle = &magnitudes[magnitudes.len() / 4..magnitudes.len() * 3 / 4];
    assert!(
        middle.iter().all(|m| (m - 0.25).abs() < 0.01),
        "{:?}",
        &middle[..4]
    );
}

#[test]
fn a_real_rectangle_stays_real_and_moves_its_lower_edge_to_zero() {
    let dir = TempDir::new("extract-real");
    let values: Vec<f32> = (0..48_000)
        .map(|n| (std::f64::consts::TAU * 6_500.0 * n as f64 / RATE).cos() as f32 * 0.5)
        .collect();
    let path = write_wav(
        &dir.join("real.wav"),
        SampleType::new(Domain::Real, SampleFormat::F32),
        RATE as u32,
        &values,
        1.0,
    );
    let (source, stamp) = argand_io::open_stamped(&path, &OpenHints::default()).unwrap();
    let source = SourceFile {
        meta: source.meta().clone(),
        hints: OpenHints::default(),
        stamp,
        encoding: None,
    };
    let target = dir.join("rectangle.wav");
    let span = SampleSpan::between(12_000, 36_000).unwrap();
    let band = FrequencyBand::between(10_005_000.0, 10_007_000.0).unwrap();
    run(
        &request(source, span, band, target.clone()),
        &mut |_, _| {},
        &AtomicBool::new(false),
    )
    .unwrap();
    let (meta, values) = read_all(&target);
    assert_eq!(meta.sample_rate, 5_000.0);
    assert_eq!(meta.center_freq, 0.0);
    assert_eq!(
        meta.sample_type,
        SampleType::new(Domain::Real, SampleFormat::F32)
    );
    assert_eq!(meta.len_samples, 2_500);
    for (n, &value) in values.iter().enumerate() {
        let expected = 0.5 * (std::f64::consts::TAU * 1_500.0 * n as f64 / 5_000.0).cos();
        assert!(
            (f64::from(value) - expected).abs() < 0.002,
            "{n}: {value}, expected {expected}"
        );
    }
}

#[test]
fn a_non_kilohertz_source_rate_changes_samples_as_well_as_output_metadata() {
    let rate = 11_999.0;
    let values = iq_tone(11_999, rate, 2_700.0, 0.5);
    let span = SampleSpan::between(0, 11_999).unwrap();
    let band = FrequencyBand::between(10_002_000.0, 10_003_000.0).unwrap();
    let (meta, values) = memory_export(values, rate, false, span, band);
    assert_eq!(meta.sample_rate, 2_000.0);
    assert_eq!(meta.len_samples, 2_000);
    for (n, iq) in values
        .as_chunks::<2>()
        .0
        .iter()
        .enumerate()
        .skip(500)
        .take(1_000)
    {
        let phase = std::f64::consts::TAU * 200.0 * n as f64 / meta.sample_rate;
        assert!((f64::from(iq[0]) - 0.5 * phase.cos()).abs() < 0.002);
        assert!((f64::from(iq[1]) - 0.5 * phase.sin()).abs() < 0.002);
    }
}

#[test]
fn a_real_band_touching_dc_or_nyquist_keeps_its_amplitude() {
    for (frequency, low, high, shifted) in [
        (0.0, 0.0, 2_000.0, 0.0),
        (24_000.0, 22_000.0, 24_000.0, 2_000.0),
    ] {
        let span = SampleSpan::between(12_000, 36_000).unwrap();
        let band = FrequencyBand::between(10_000_000.0 + low, 10_000_000.0 + high).unwrap();
        let input = (0..48_000)
            .map(|n| (std::f64::consts::TAU * frequency * n as f64 / RATE).cos() as f32 * 0.5)
            .collect();
        let (meta, values) = memory_export(input, RATE, true, span, band);
        assert!(!meta.is_iq());
        assert_eq!(meta.sample_rate, 5_000.0);
        let (mut cosine, mut sine) = (0.0, 0.0);
        for (n, &value) in values.iter().enumerate().skip(500).take(1_500) {
            let phase = std::f64::consts::TAU * shifted * n as f64 / meta.sample_rate;
            cosine += f64::from(value) * phase.cos();
            sine += f64::from(value) * phase.sin();
        }
        // A finite tone on a filter edge has quadrature tails; measure the component's gain.
        let scale = if shifted == 0.0 { 1.0 } else { 2.0 } / 1_500.0;
        assert!(
            (cosine * scale - 0.5).abs() < 0.002,
            "{frequency} Hz: {}",
            cosine * scale
        );
        assert!(
            (sine * scale).abs() < 0.002,
            "{frequency} Hz: {}",
            sine * scale
        );
    }
}

#[test]
fn a_full_band_rectangle_preserves_a_tone_next_to_native_nyquist() {
    let rate = 11_999.0;
    let frequency = rate * (0.5 - 1.0 / 131_072.0);
    let mut values = Vec::with_capacity(2_400_000 * 2);
    for n in 0..2_400_000 {
        let high = std::f64::consts::TAU * frequency * n as f64 / rate;
        let middle = std::f64::consts::TAU * -2_000.0 * n as f64 / rate;
        values.push(0.25 * high.cos() as f32 + 0.25 * middle.cos() as f32);
        values.push(0.25 * high.sin() as f32 + 0.25 * middle.sin() as f32);
    }
    let span = SampleSpan::between(1_200_000, 1_248_000).unwrap();
    let band =
        FrequencyBand::between(10_000_000.0 - rate / 2.0, 10_000_000.0 + rate / 2.0).unwrap();
    let (meta, values) = memory_export(values, rate, false, span, band);
    assert_eq!(meta.sample_rate, 15_000.0);
    assert_eq!(meta.center_freq, 0.0);
    assert_eq!(meta.len_samples, 60_006);
    for (n, iq) in values.as_chunks::<2>().0.iter().enumerate() {
        let time = span.start() as f64 / rate + n as f64 / meta.sample_rate;
        let high = std::f64::consts::TAU * frequency * time;
        let middle = std::f64::consts::TAU * -2_000.0 * time;
        let expected = [
            0.25 * (high.cos() + middle.cos()),
            0.25 * (high.sin() + middle.sin()),
        ];
        for channel in 0..2 {
            assert!(
                (f64::from(iq[channel]) - expected[channel]).abs() < 0.001,
                "{n}: {iq:?}, {expected:?}"
            );
        }
    }
}

#[test]
fn resampling_keeps_filter_tails_beyond_short_capture_edges() {
    let span = SampleSpan::between(0, 1).unwrap();
    let band = FrequencyBand::between(10_000_000.0 - 500.0, 10_000_000.0 + 500.0).unwrap();
    let (meta, values) = memory_export(vec![0.5, 0.0], 1_000.0, false, span, band);
    assert_eq!(meta.sample_rate, 2_000.0);
    assert_eq!(meta.len_samples, 2);
    assert!((values[0] - 0.5).abs() < 0.001, "{values:?}");
    assert!(
        (values[2] - 1.0 / std::f32::consts::PI).abs() < 0.001,
        "{values:?}"
    );
    assert!(values[1].abs() < 0.001 && values[3].abs() < 0.001);
}

struct MemorySource {
    meta: SignalMeta,
    values: Vec<f32>,
    at: usize,
}

impl SampleSource for MemorySource {
    fn meta(&self) -> &SignalMeta {
        &self.meta
    }
    fn seek(&mut self, sample: u64) -> Result<(), argand_core::SourceError> {
        if sample > self.meta.len_samples {
            return Err(argand_core::SourceError::SeekOutOfRange {
                requested: sample,
                total: self.meta.len_samples,
            });
        }
        self.at = sample as usize * self.meta.channels();
        Ok(())
    }
    fn read(&mut self, buffer: &mut [f32]) -> Result<usize, argand_core::SourceError> {
        let count = buffer.len().min(self.values.len() - self.at);
        buffer[..count].copy_from_slice(&self.values[self.at..self.at + count]);
        self.at += count;
        Ok(count)
    }
}

fn memory_export(
    values: Vec<f32>,
    rate: f64,
    real: bool,
    span: SampleSpan,
    band: FrequencyBand,
) -> (SignalMeta, Vec<f32>) {
    let sample_type = SampleType::new(
        if real { Domain::Real } else { Domain::Iq },
        SampleFormat::F32,
    );
    let meta = SignalMeta {
        sample_rate: rate,
        center_freq: 0.0,
        sample_type,
        len_samples: (values.len() / sample_type.channels()) as u64,
        container: "memory",
        divisor: 1.0,
        source: PathBuf::from("memory"),
    };
    let request = request(
        SourceFile {
            meta: meta.clone(),
            hints: OpenHints::default(),
            stamp: None,
            encoding: None,
        },
        span,
        band,
        PathBuf::from("unused.wav"),
    );
    let output_meta = request.output_meta().unwrap();
    let mut source = MemorySource {
        meta,
        values,
        at: 0,
    };
    let mut values = Vec::new();
    compute(
        &request,
        &mut source,
        &mut |samples| {
            for sample in samples {
                values.extend_from_slice(&sample[..output_meta.channels()]);
            }
            Ok(())
        },
        &mut |_, _| {},
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(
        values.len(),
        output_meta.len_samples as usize * output_meta.channels()
    );
    (output_meta, values)
}

#[test]
fn a_source_changed_after_reading_is_not_published() {
    let dir = TempDir::new("extract-changed-after-read");
    let source = capture(&dir, 6500.0);
    let path = source.meta.source.clone();
    let target = dir.join("band.wav");
    let request = request(
        source,
        SampleSpan::between(0, 48_000).unwrap(),
        FrequencyBand::between(10_005_000.0, 10_007_000.0).unwrap(),
        target.clone(),
    );
    let result = run(
        &request,
        &mut |done, total| {
            if done == total && done != 0 {
                use std::io::Write;
                std::fs::OpenOptions::new()
                    .append(true)
                    .open(&path)
                    .unwrap()
                    .write_all(b"changed source")
                    .unwrap();
            }
        },
        &AtomicBool::new(false),
    );
    assert!(
        matches!(result, Err(WriteError::SourceChanged { .. })),
        "{result:?}"
    );
    assert!(!target.exists());
}

#[test]
fn cancellation_after_reading_preserves_the_existing_target() {
    let dir = TempDir::new("extract-cancel-after-read");
    let source = capture(&dir, 6500.0);
    let target = dir.join("band.wav");
    std::fs::write(&target, b"original target").unwrap();
    let request = request(
        source,
        SampleSpan::between(0, 48_000).unwrap(),
        FrequencyBand::between(10_005_000.0, 10_007_000.0).unwrap(),
        target.clone(),
    );
    let cancel = AtomicBool::new(false);
    let result = run(
        &request,
        &mut |done, _| {
            if done != 0 {
                cancel.store(true, Ordering::Relaxed);
            }
        },
        &cancel,
    );
    assert!(matches!(result, Err(WriteError::Cancelled)), "{result:?}");
    assert_eq!(std::fs::read(&target).unwrap(), b"original target");
}
