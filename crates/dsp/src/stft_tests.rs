use super::*;
use argand_core::{Domain, SampleFormat, SampleType};
use std::path::PathBuf;

const RATE: f64 = 24_000.0;
const FFT: usize = 1024;
/// Exactly on bin 103, so the peak has nowhere to smear.
const TONE_BIN: usize = 103;
const TONE_HZ: f64 = TONE_BIN as f64 * RATE / FFT as f64;

/// A signal held in memory, standing in for a file.
struct VecSource {
    meta: SignalMeta,
    data: Vec<f32>,
    pos: usize,
}

impl VecSource {
    fn new(domain: Domain, data: Vec<f32>, center_freq: f64) -> Self {
        let sample_type = SampleType::new(domain, SampleFormat::F32);
        let len_samples = (data.len() / sample_type.channels()) as u64;
        Self {
            meta: SignalMeta {
                sample_rate: RATE,
                center_freq,
                sample_type,
                len_samples,
                container: "test",
                divisor: 1.0,
                source: PathBuf::from("memory"),
            },
            data,
            pos: 0,
        }
    }
}

impl SampleSource for VecSource {
    fn meta(&self) -> &SignalMeta {
        &self.meta
    }

    fn seek(&mut self, sample: u64) -> Result<(), SourceError> {
        self.pos = sample as usize * self.meta.channels();
        Ok(())
    }

    fn read(&mut self, buf: &mut [f32]) -> Result<usize, SourceError> {
        let channels = self.meta.channels();
        let usable = buf.len() - buf.len() % channels;
        let n = usable.min(self.data.len().saturating_sub(self.pos));
        buf[..n].copy_from_slice(&self.data[self.pos..self.pos + n]);
        self.pos += n;
        Ok(n)
    }
}

fn iq_tone(len: usize, freq_hz: f64, amplitude: f32) -> Vec<f32> {
    let mut out = Vec::with_capacity(len * 2);
    for n in 0..len {
        let phase = std::f64::consts::TAU * freq_hz * n as f64 / RATE;
        out.push(amplitude * phase.cos() as f32);
        out.push(amplitude * phase.sin() as f32);
    }
    out
}

fn real_tone(len: usize, freq_hz: f64, amplitude: f32) -> Vec<f32> {
    (0..len)
        .map(|n| {
            let phase = std::f64::consts::TAU * freq_hz * n as f64 / RATE;
            amplitude * phase.cos() as f32
        })
        .collect()
}

fn run(src: &mut dyn SampleSource, width: usize, height: usize) -> Analysis {
    run_with(src, width, height, Reduce::Max, DynamicRange::Default)
}

fn run_with(
    src: &mut dyn SampleSource,
    width: usize,
    height: usize,
    reduce: Reduce,
    dynamic_range: DynamicRange,
) -> Analysis {
    let range = SampleRange::new(0, src.meta().len_samples);
    analyze(
        src,
        &AnalysisRequest {
            reduce,
            dynamic_range,
            ..request(width, height, range)
        },
        &mut |_, _| {},
    )
    .expect("analysis should succeed")
}

/// The defaults every test starts from; each one overrides what it is about.
fn request(width: usize, height: usize, range: SampleRange) -> AnalysisRequest {
    AnalysisRequest {
        cfg: StftConfig::new(FFT, Window::Hann),
        range,
        width,
        height,
        reduce: Reduce::Max,
        colormap: Colormap::Grayscale,
        dynamic_range: DynamicRange::Default,
        waveform_columns: None,
    }
}

#[test]
fn a_complex_tone_lands_in_the_two_sided_spectrum() {
    let mut src = VecSource::new(Domain::Iq, iq_tone(8192, TONE_HZ, 1.0), 0.0);
    let analysis = run(&mut src, 64, 64);

    assert_eq!(analysis.psd.db.len(), FFT, "i/q spectrum is two-sided");

    let peak = analysis.psd.peak(0.0).unwrap();
    // Bin 0 sits at -Fs/2, so a positive tone lands above the midpoint.
    assert_eq!(peak.bin, FFT / 2 + TONE_BIN);
    assert!((peak.offset_hz - TONE_HZ).abs() < 1.0, "{}", peak.offset_hz);
}

#[test]
fn a_negative_frequency_is_distinguishable_from_a_positive_one() {
    // The whole point of a complex capture: -f and +f are different signals.
    // Treating the interleaved stream as real would collapse them together.
    let mut low = VecSource::new(Domain::Iq, iq_tone(8192, -TONE_HZ, 1.0), 0.0);
    let mut high = VecSource::new(Domain::Iq, iq_tone(8192, TONE_HZ, 1.0), 0.0);

    let low_peak = run(&mut low, 64, 64).psd.peak(0.0).unwrap();
    let high_peak = run(&mut high, 64, 64).psd.peak(0.0).unwrap();

    assert_eq!(low_peak.bin, FFT / 2 - TONE_BIN);
    assert_eq!(high_peak.bin, FFT / 2 + TONE_BIN);
    assert!(low_peak.offset_hz < 0.0 && high_peak.offset_hz > 0.0);
}

#[test]
fn a_real_tone_lands_in_the_one_sided_spectrum() {
    let mut src = VecSource::new(Domain::Real, real_tone(8192, TONE_HZ, 1.0), 0.0);
    let analysis = run(&mut src, 64, 64);

    assert_eq!(analysis.psd.db.len(), FFT / 2 + 1, "real spectrum is one-sided");
    let peak = analysis.psd.peak(0.0).unwrap();
    assert_eq!(peak.bin, TONE_BIN);
    assert!((peak.freq_hz - TONE_HZ).abs() < 1.0, "{}", peak.freq_hz);
}

#[test]
fn a_full_scale_tone_reads_zero_dbfs() {
    // Complex and real full-scale tones must both calibrate to 0 dBFS, which
    // is what makes the colour scale mean the same thing for either.
    let mut iq = VecSource::new(Domain::Iq, iq_tone(8192, TONE_HZ, 1.0), 0.0);
    let iq_db = run(&mut iq, 64, 64).psd.peak(0.0).unwrap().db;
    assert!(iq_db.abs() < 0.1, "i/q peak was {iq_db} dBFS");

    let mut real = VecSource::new(Domain::Real, real_tone(8192, TONE_HZ, 1.0), 0.0);
    let real_db = run(&mut real, 64, 64).psd.peak(0.0).unwrap().db;
    assert!(real_db.abs() < 0.1, "real peak was {real_db} dBFS");
}

#[test]
fn a_tone_between_bins_loses_the_window_s_scalloping_and_no_more() {
    // 0 dBFS is the reading for a tone on a bin centre. One that falls between
    // bins reads under it, and the shortfall is the window's scalloping loss,
    // not anything to do with the bin's width -- which is why the plot calls
    // its scale dBFS rather than a per-hertz density.
    let bin = RATE / FFT as f64;
    let losses: Vec<f32> = [0.0, 0.25, 0.5]
        .into_iter()
        .map(|offset| {
            let hz = (TONE_BIN as f64 + offset) * bin;
            let mut src = VecSource::new(Domain::Iq, iq_tone(8192, hz, 1.0), 0.0);
            run(&mut src, 64, 64).psd.peak(0.0).unwrap().db
        })
        .collect();

    assert!(losses[0].abs() < 0.01, "on the bin centre: {losses:?}");
    // Hann's worst case is half a bin off centre, at about 1.42 dB.
    assert!(
        (losses[2] + 1.42).abs() < 0.05,
        "half a bin off centre: {losses:?}"
    );
    assert!(
        losses[0] > losses[1] && losses[1] > losses[2],
        "the loss has to grow with the offset: {losses:?}"
    );
}

#[test]
fn halving_the_amplitude_costs_six_decibels() {
    let mut loud = VecSource::new(Domain::Iq, iq_tone(8192, TONE_HZ, 1.0), 0.0);
    let mut quiet = VecSource::new(Domain::Iq, iq_tone(8192, TONE_HZ, 0.5), 0.0);

    let loud_db = run(&mut loud, 64, 64).psd.peak(0.0).unwrap().db;
    let quiet_db = run(&mut quiet, 64, 64).psd.peak(0.0).unwrap().db;
    assert!((loud_db - quiet_db - 6.02).abs() < 0.1, "{loud_db} vs {quiet_db}");
}

#[test]
fn the_frequency_axis_is_offset_by_the_centre_frequency() {
    let center = 12_579_000.0;
    let mut src = VecSource::new(Domain::Iq, iq_tone(8192, TONE_HZ, 1.0), center);
    let analysis = run(&mut src, 64, 64);

    let peak = analysis.psd.peak(center).unwrap();
    assert!((peak.freq_hz - (center + TONE_HZ)).abs() < 1.0, "{}", peak.freq_hz);
    assert!((peak.offset_hz - TONE_HZ).abs() < 1.0);

    assert!((analysis.spectrogram.f0 - (center - RATE / 2.0)).abs() < 1e-6);
    assert!((analysis.spectrogram.f1 - (center + RATE / 2.0)).abs() < 1e-6);
}

#[test]
fn the_image_carries_the_extents_it_was_drawn_for() {
    let mut src = VecSource::new(Domain::Iq, iq_tone(4800, TONE_HZ, 1.0), 0.0);
    let analysis = run(&mut src, 37, 23);
    let img = &analysis.spectrogram;

    assert_eq!((img.width, img.height), (37, 23));
    assert_eq!(img.rgba.len(), 37 * 23 * 4);
    assert_eq!(img.t0, 0.0);
    assert!((img.t1 - 0.2).abs() < 1e-9, "4800 samples at 24 kHz is 200 ms");
    assert_eq!(img.db_max, 0.0, "full-scale reference puts 0 dB at the top");
    assert_eq!(img.db_min, -110.0);
    assert!(img.rgba.chunks_exact(4).all(|p| p[3] == 255), "no gaps");
}

#[test]
fn the_tone_is_the_brightest_row_of_the_image() {
    let mut src = VecSource::new(Domain::Iq, iq_tone(8192, TONE_HZ, 1.0), 0.0);
    let analysis = run(&mut src, 16, FFT);
    let img = &analysis.spectrogram;

    // Row 0 is the top of the image, which is +Fs/2.
    let expected_row = FFT - 1 - (FFT / 2 + TONE_BIN);
    let brightest = (0..img.height)
        .max_by_key(|&y| img.get(4, y)[0])
        .expect("non-empty image");
    assert_eq!(brightest, expected_row, "tone should sit above the midline");
    assert!(brightest < img.height / 2);
}

#[test]
fn a_signal_spanning_several_blocks_matches_a_short_one() {
    // BLOCK_SAMPLES is deliberately tiny under test, so this crosses the
    // carry-over path several times.
    let short = iq_tone(4096, TONE_HZ, 1.0);
    let long = iq_tone(4096 * 5, TONE_HZ, 1.0);

    let mut a = VecSource::new(Domain::Iq, short, 0.0);
    let mut b = VecSource::new(Domain::Iq, long, 0.0);
    let short_peak = run(&mut a, 8, 8).psd.peak(0.0).unwrap();
    let long_analysis = run(&mut b, 8, 8);
    let long_peak = long_analysis.psd.peak(0.0).unwrap();

    assert_eq!(short_peak.bin, long_peak.bin);
    assert!((short_peak.db - long_peak.db).abs() < 0.1);
    assert!(long_analysis.frames > 4096 / (FFT / 4) as u64);
}

#[test]
fn every_column_gets_covered_when_frames_outnumber_them() {
    let mut src = VecSource::new(Domain::Iq, iq_tone(40_000, TONE_HZ, 1.0), 0.0);
    let analysis = run(&mut src, 100, 16);
    let img = &analysis.spectrogram;

    let expected_row = 16 - 1 - (16 / 2 + 103 * 16 / FFT);
    for x in 0..img.width {
        let brightest = (0..img.height).max_by_key(|&y| img.get(x, y)[0]).unwrap();
        assert_eq!(brightest, expected_row, "column {x} is blank or wrong");
    }
}

#[test]
fn mean_and_max_differ_on_a_burst() {
    // A tone for the first eighth, silence after: max keeps it visible.
    let mut data = iq_tone(2048, TONE_HZ, 1.0);
    data.extend(std::iter::repeat_n(0.0, 2048 * 7 * 2));

    let mut max_src = VecSource::new(Domain::Iq, data.clone(), 0.0);
    let mut mean_src = VecSource::new(Domain::Iq, data, 0.0);

    let max = run_with(&mut max_src, 1, FFT, Reduce::Max, DynamicRange::Default);
    let mean = run_with(&mut mean_src, 1, FFT, Reduce::Mean, DynamicRange::Default);

    let brightest = |a: &Analysis| {
        (0..a.spectrogram.height)
            .map(|y| a.spectrogram.get(0, y)[0])
            .max()
            .unwrap()
    };
    assert!(
        brightest(&max) > brightest(&mean),
        "max {} should beat mean {}",
        brightest(&max),
        brightest(&mean)
    );
}

#[test]
fn a_fixed_range_stretches_a_quiet_signal_to_full_brightness() {
    let mut fs_src = VecSource::new(Domain::Iq, iq_tone(8192, TONE_HZ, 0.001), 0.0);
    let mut peak_src = VecSource::new(Domain::Iq, iq_tone(8192, TONE_HZ, 0.001), 0.0);

    let fs = run_with(&mut fs_src, 8, 64, Reduce::Max, DynamicRange::Default);
    let peak = run_with(
        &mut peak_src,
        8,
        64,
        Reduce::Max,
        DynamicRange::Fixed(110.0),
    );

    assert_eq!(fs.spectrogram.db_max, 0.0);
    assert!(peak.spectrogram.db_max < -50.0, "{}", peak.spectrogram.db_max);

    let brightest = |a: &Analysis| (0..a.spectrogram.height).map(|y| a.spectrogram.get(4, y)[0]).max().unwrap();
    assert!(brightest(&peak) > brightest(&fs));
    assert_eq!(brightest(&peak), 255, "peak-relative range should reach the top");
}

#[test]
fn dynamic_range_literals_accept_auto_and_positive_numbers_only() {
    assert_eq!("auto".parse(), Ok(DynamicRange::Auto));
    assert_eq!("AUTO".parse(), Ok(DynamicRange::Auto));
    assert_eq!("40".parse(), Ok(DynamicRange::Fixed(40.0)));
    for bad in ["default", "0", "-1", "NaN", "inf", "wide"] {
        assert!(bad.parse::<DynamicRange>().is_err(), "accepted {bad}");
    }
}

#[test]
fn the_recommendation_adds_headroom_rounds_up_and_clamps() {
    let range_for = |db: Vec<f32>| {
        recommended_dynamic_range(&Psd {
            freqs_hz: vec![0.0; db.len()],
            db,
            segments: 1,
        })
    };

    assert_eq!(range_for(vec![-34.0, -34.0, -10.0]), 40.0);
    assert_eq!(range_for(vec![-11.0, -11.0, -10.0]), 20.0);
    assert_eq!(range_for(vec![-210.0, -210.0, -10.0]), 120.0);
    assert_eq!(range_for(Vec::new()), 20.0);
}

#[test]
fn every_mode_reports_and_applies_one_resolved_range() {
    let psd = Psd {
        freqs_hz: vec![0.0; 3],
        db: vec![-100.0, -100.0, -80.0],
        segments: 1,
    };
    let grid = [-95.0, -80.0];

    let (default, min, max) = resolve_dynamic_range(DynamicRange::Default, &grid, &psd);
    assert_eq!(default.requested.mode(), "default");
    assert_eq!((default.effective_db, default.recommended_db), (110.0, 30.0));
    assert_eq!((min, max), (-110.0, 0.0));

    let (fixed, min, max) =
        resolve_dynamic_range(DynamicRange::Fixed(40.0), &grid, &psd);
    assert_eq!(fixed.requested.mode(), "fixed");
    assert_eq!((fixed.effective_db, fixed.recommended_db), (40.0, 30.0));
    assert_eq!((min, max), (-120.0, -80.0));

    let (auto, min, max) = resolve_dynamic_range(DynamicRange::Auto, &grid, &psd);
    assert_eq!(auto.requested.mode(), "auto");
    assert_eq!((auto.effective_db, auto.recommended_db), (30.0, 30.0));
    assert_eq!((min, max), (-110.0, -80.0));
}

#[test]
fn the_time_domain_peak_is_reported() {
    let mut src = VecSource::new(Domain::Iq, iq_tone(8192, TONE_HZ, 0.25), 0.0);
    let analysis = run(&mut src, 8, 8);
    assert!((analysis.time_peak - 0.25).abs() < 1e-3, "{}", analysis.time_peak);
}

#[test]
fn the_time_peak_includes_samples_after_the_last_full_frame() {
    let mut values = vec![0.1; BLOCK_SAMPLES + 1];
    *values.last_mut().unwrap() = 0.9;
    let mut src = VecSource::new(Domain::Real, values, 0.0);
    let range = SampleRange::new(0, src.meta().len_samples);
    let analysis = analyze(
        &mut src,
        &AnalysisRequest {
            waveform_columns: None,
            ..request(32, 32, range)
        },
        &mut |_, _| {},
    )
    .unwrap();

    assert!((analysis.time_peak - 0.9).abs() < 1e-6);
    assert!(analysis.waveform.is_none());
}

#[test]
fn the_window_bandwidth_is_reported_in_hertz() {
    let mut src = VecSource::new(Domain::Iq, iq_tone(4096, TONE_HZ, 1.0), 0.0);
    let analysis = run(&mut src, 8, 8);
    let want = 1.5 * RATE / FFT as f64; // hann spreads noise over 1.5 bins
    assert!((analysis.enbw_hz - want).abs() < 0.1, "{}", analysis.enbw_hz);
}

#[test]
fn a_range_selects_part_of_the_signal() {
    let mut src = VecSource::new(Domain::Iq, iq_tone(24_000, TONE_HZ, 1.0), 0.0);
    let analysis = analyze(
        &mut src,
        &request(16, 16, SampleRange::new(6_000, 12_000)),
        &mut |_, _| {},
    )
    .unwrap();

    assert!((analysis.spectrogram.t0 - 0.25).abs() < 1e-9);
    assert!((analysis.spectrogram.t1 - 0.75).abs() < 1e-9);
}

#[test]
fn progress_runs_from_zero_to_the_frame_count() {
    let mut src = VecSource::new(Domain::Iq, iq_tone(20_000, TONE_HZ, 1.0), 0.0);
    let mut seen: Vec<(u64, u64)> = Vec::new();
    let range = SampleRange::new(0, src.meta().len_samples);
    let analysis = analyze(&mut src, &request(16, 16, range), &mut |done, total| {
        seen.push((done, total))
    })
    .unwrap();

    assert_eq!(seen.first().unwrap().0, 0);
    assert_eq!(seen.last().unwrap().0, analysis.frames);
    assert!(seen.iter().all(|(_, total)| *total == seen[0].1));
    assert!(seen.windows(2).all(|w| w[0].0 <= w[1].0), "must not go back");
}

#[test]
fn rejects_configurations_it_cannot_honour() {
    let mut src = VecSource::new(Domain::Iq, iq_tone(4096, TONE_HZ, 1.0), 0.0);
    let range = SampleRange::new(0, src.meta().len_samples);
    let call = |src: &mut VecSource, cfg: StftConfig, w: usize, h: usize| {
        analyze(
            src,
            &AnalysisRequest {
                cfg,
                ..request(w, h, range)
            },
            &mut |_, _| {},
        )
    };

    assert!(matches!(
        call(&mut src, StftConfig::new(1000, Window::Hann), 8, 8),
        Err(DspError::BadFftSize(1000))
    ));
    assert!(matches!(
        call(
            &mut src,
            StftConfig {
                fft_size: 1024,
                hop: 0,
                window: Window::Hann
            },
            8,
            8
        ),
        Err(DspError::BadHop)
    ));
    assert!(matches!(
        call(&mut src, StftConfig::new(FFT, Window::Hann), 0, 8),
        Err(DspError::BadOutputSize { width: 0, .. })
    ));

    let mut tiny = VecSource::new(Domain::Iq, iq_tone(100, TONE_HZ, 1.0), 0.0);
    let err = analyze(
        &mut tiny,
        &request(8, 8, SampleRange::new(0, 100)),
        &mut |_, _| {},
    );
    assert!(matches!(err, Err(DspError::TooShort { samples: 100, .. })));
}

#[test]
fn the_envelope_shares_the_spectrogram_time_axis() {
    let mut src = VecSource::new(Domain::Iq, iq_tone(24_000, TONE_HZ, 0.8), 0.0);
    let analysis = analyze(
        &mut src,
        &AnalysisRequest {
            waveform_columns: Some(64),
            ..request(64, 32, SampleRange::new(6_000, 12_000))
        },
        &mut |_, _| {},
    )
    .unwrap();

    let waveform = analysis.waveform.expect("a waveform was asked for");
    assert_eq!(waveform.columns, 64);
    assert_eq!(waveform.channels, 2, "i/q keeps both channels");
    assert_eq!(waveform.t0, analysis.spectrogram.t0);
    assert_eq!(waveform.t1, analysis.spectrogram.t1);
    // A unit-amplitude tone: both channels reach their amplitude everywhere.
    assert!((waveform.peak() - 0.8).abs() < 1e-3, "{}", waveform.peak());
}

#[test]
fn no_waveform_is_built_unless_one_is_asked_for() {
    let mut src = VecSource::new(Domain::Real, real_tone(4096, TONE_HZ, 0.5), 0.0);
    assert!(run(&mut src, 16, 16).waveform.is_none());
}

#[test]
fn the_envelope_covers_the_tail_past_the_last_whole_frame() {
    // 5000 samples with hop 256: the last frame ends at 4864, so the frame
    // loop never reads the final 136 samples. The strip still has to show them.
    let mut data = real_tone(5000, TONE_HZ, 0.1);
    data[4990] = 0.95;
    let mut src = VecSource::new(Domain::Real, data, 0.0);

    let analysis = analyze(
        &mut src,
        &AnalysisRequest {
            cfg: StftConfig::new(FFT, Window::Hann),
            waveform_columns: Some(25),
            ..request(25, 16, SampleRange::new(0, 5000))
        },
        &mut |_, _| {},
    )
    .unwrap();

    let waveform = analysis.waveform.expect("a waveform was asked for");
    let (_, max) = waveform.column(24, 0).expect("the last column");
    assert_eq!(max, 0.95, "the tail spike never reached the envelope");
}

#[test]
fn overlap_is_reported_from_the_hop() {
    assert_eq!(StftConfig::new(2048, Window::Hann).hop, 512);
    assert!((StftConfig::new(2048, Window::Hann).overlap_percent() - 75.0).abs() < 1e-9);
    let half = StftConfig {
        fft_size: 2048,
        hop: 1024,
        window: Window::Hann,
    };
    assert!((half.overlap_percent() - 50.0).abs() < 1e-9);
}

#[test]
fn the_published_decibel_floor_matches_the_floors_that_produce_it() {
    // Both clamps have to agree with the constant a renderer reserves from,
    // or a label can come out wider than the room set aside for it.
    assert_eq!(20.0 * MAG_FLOOR.log10(), DB_FLOOR);
    assert!((10.0 * POWER_FLOOR.log10() - f64::from(DB_FLOOR)).abs() < 1e-9);
}

#[test]
fn the_grid_holds_the_numbers_the_picture_was_shaded_from() {
    let mut src = VecSource::new(Domain::Iq, iq_tone(8192, TONE_HZ, 1.0), 12_579_000.0);
    let analysis = run(&mut src, 32, 16);
    let (grid, img) = (&analysis.db, &analysis.spectrogram);

    assert_eq!((grid.width, grid.height), (img.width, img.height));
    assert_eq!(grid.values.len(), grid.width * grid.height);
    // The extents describe one picture, so both views of it carry the same.
    assert_eq!((grid.t0, grid.t1), (img.t0, img.t1));
    assert_eq!((grid.f0, grid.f1), (img.f0, img.f1));

    // Row 0 of the image is the top, which is the highest frequency, so the
    // grid's last bin is what shaded it.
    let gradient = Colormap::Grayscale.gradient();
    let span = img.db_max - img.db_min;
    for x in 0..grid.width {
        for y in 0..grid.height {
            let value = grid.value(x, grid.height - 1 - y).expect("a bin inside the grid");
            let expected = gradient[gradient_index((value - img.db_min) / span)];
            assert_eq!(img.get(x, y), [expected[0], expected[1], expected[2], 255]);
        }
    }
    assert!(
        grid.values.iter().any(|v| *v > f32::from(-100i8)),
        "a full-scale tone left nothing above -100 dB"
    );
}

#[test]
fn recolouring_a_grid_costs_no_second_pass_over_the_signal() {
    // What the separation is for: the same numbers under a different scheme
    // and a different window give exactly the picture the transform would
    // have produced, without the transform running again.
    let mut src = VecSource::new(Domain::Iq, iq_tone(8192, TONE_HZ, 1.0), 0.0);
    let analysis = run(&mut src, 24, 16);

    let mut again = VecSource::new(Domain::Iq, iq_tone(8192, TONE_HZ, 1.0), 0.0);
    let wanted = analyze(
        &mut again,
        &AnalysisRequest {
            colormap: Colormap::Oceanic,
            ..request(24, 16, SampleRange::new(0, 8192))
        },
        &mut |_, _| {},
    )
    .expect("analysis should succeed");

    let recoloured = shade(
        &analysis.db,
        Shading {
            colormap: Colormap::Oceanic,
            db_min: wanted.spectrogram.db_min,
            db_max: wanted.spectrogram.db_max,
        },
    );
    assert_eq!(recoloured.rgba, wanted.spectrogram.rgba);
    assert_eq!(recoloured.db_min, wanted.spectrogram.db_min);
    assert_eq!(recoloured.db_max, wanted.spectrogram.db_max);
    assert_eq!((recoloured.t0, recoloured.t1), (wanted.spectrogram.t0, wanted.spectrogram.t1));
    assert_eq!((recoloured.f0, recoloured.f1), (wanted.spectrogram.f0, wanted.spectrogram.f1));

    // And a different window over the same grid is a different picture. The
    // window has to reach past the floor to show it: a tone against silence
    // has both its levels pinned to the ends of any narrower one.
    let wider = shade(
        &analysis.db,
        Shading {
            colormap: Colormap::Oceanic,
            db_min: DB_FLOOR - 200.0,
            db_max: wanted.spectrogram.db_max,
        },
    );
    assert_ne!(
        wider.get(0, 0),
        recoloured.get(0, 0),
        "the floor kept its colour after the window moved past it"
    );
}

#[test]
fn shading_a_grid_that_does_not_cover_its_shape_draws_nothing() {
    // The shape is what sizes the buffer, so a width and height the values
    // cannot fill must be settled before the allocation rather than after it:
    // `2 * usize::MAX` is an overflow, not a picture.
    let broken = DbGrid {
        width: 2,
        height: usize::MAX,
        values: vec![-10.0],
        t0: 0.0,
        t1: 1.0,
        f0: -12_000.0,
        f1: 12_000.0,
    };
    let shading = Shading {
        colormap: Colormap::Grayscale,
        db_min: -110.0,
        db_max: 0.0,
    };
    let image = shade(&broken, shading);
    assert_eq!((image.width, image.height), (0, 0));
    assert!(image.rgba.is_empty());

    // And a shape that multiplies without overflowing but the values fall
    // short of, which used to draw the columns it had and leave the rest.
    let short = DbGrid {
        width: 4,
        height: 4,
        values: vec![-10.0; 8],
        ..broken
    };
    let image = shade(&short, shading);
    assert_eq!((image.width, image.height), (0, 0));

    // A grid that does cover its shape still draws all of it.
    let whole = DbGrid {
        width: 4,
        height: 4,
        values: vec![-10.0; 16],
        ..short
    };
    let image = shade(&whole, shading);
    assert_eq!((image.width, image.height), (4, 4));
    assert!(image.rgba.chunks_exact(4).all(|p| p[3] == 255));
}
