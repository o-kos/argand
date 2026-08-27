use super::*;
use argand_core::{Domain, SampleFormat, SampleType, SpectrogramImage};
use argand_core::{Psd, SignalMeta};
use argand_dsp::{Analysis, Window};
use std::path::PathBuf;

fn meta(format: SampleFormat, divisor: f32) -> SignalMeta {
    SignalMeta {
        sample_rate: 24_000.0,
        center_freq: 0.0,
        sample_type: SampleType::new(Domain::Iq, format),
        len_samples: 43_200_000,
        container: "wav",
        divisor,
        source: PathBuf::from("/data/12.579000_capture.iqw"),
    }
}

fn analysis(peak_db: f32, floor_db: f32, time_peak: f32, db_max: f32) -> Analysis {
    analysis_at(0.0, peak_db, floor_db, time_peak, db_max)
}

/// The dsp emits bin frequencies already offset by the centre frequency, so
/// fixtures have to as well or the reported offset comes out nonsensical.
fn analysis_at(
    center: f64,
    peak_db: f32,
    floor_db: f32,
    time_peak: f32,
    db_max: f32,
) -> Analysis {
    let mut image = SpectrogramImage::new(4, 4);
    image.db_max = db_max;
    image.db_min = db_max - 110.0;
    Analysis {
        spectrogram: image,
        psd: Psd {
            freqs_hz: vec![center - 2404.0, center, center + 2404.0],
            db: vec![floor_db, floor_db, peak_db],
            segments: 10,
        },
        waveform: None,
        time_peak,
        frames: 84_372,
        enbw_hz: 17.578125,
    }
}

fn report(m: &SignalMeta, a: &Analysis, reference: DbReference) -> Report {
    Report::new(
        m,
        a,
        &StftConfig::new(2048, Window::Hann),
        Reduce::Max,
        reference,
        110.0,
        Normalize::None,
        0.0,
        1800.0,
    )
}

fn human(r: &Report) -> String {
    let mut out = Vec::new();
    r.write_human(&mut out).unwrap();
    String::from_utf8(out).unwrap()
}

fn compact(r: &Report, index: usize, total: usize) -> String {
    let mut out = Vec::new();
    r.write_compact(&mut out, index, total).unwrap();
    String::from_utf8(out).unwrap()
}

#[test]
fn the_compact_line_folds_the_input_path_into_a_star() {
    let m = meta(SampleFormat::I16, 32768.0);
    let a = analysis(-11.4, -87.2, 0.2692, 0.0);
    let beside = report(&m, &a, DbReference::FullScale).with_output(
        std::path::Path::new("/data/12.579000_capture.iqw.png"),
        2048,
        512,
        253_952,
        "waveform".to_string(),
    );

    let line = compact(&beside, 2, 7);
    assert!(line.starts_with("[2/7] 12.579000_capture.iqw  iq_i16 · 24 kHz · 30m"), "{line}");
    assert!(line.contains("peak -11.4 dBFS"), "{line}");
    assert!(
        line.contains("→  *.png  248 KiB"),
        "the render is named by what it adds to the input:\n{line}"
    );
    assert!(!line.contains("/data/12.579000_capture.iqw.png"), "{line}");
    assert_eq!(line.lines().count(), 1, "one file, one line:\n{line}");

    // A render that is not the input's own name is spelled out in full.
    let elsewhere = report(&m, &a, DbReference::FullScale).with_output(
        std::path::Path::new("/tmp/spec.png"),
        2048,
        512,
        253_952,
        "waveform".to_string(),
    );
    assert!(compact(&elsewhere, 1, 1).contains("→  /tmp/spec.png"));
}

#[test]
fn levels_are_given_in_decibels_and_in_file_units() {
    let m = meta(SampleFormat::I16, 32768.0);
    let a = analysis(-11.4, -87.2, 0.2692, 0.0);
    let text = human(&report(&m, &a, DbReference::FullScale));

    assert!(text.contains("-11.4 dBFS"), "{text}");
    // -11.4 dBFS is 0.2692 of full scale, or 8820 counts of 32768.
    assert!(text.contains("8820/32768"), "{text}");
    assert!(text.contains("peak spl"), "{text}");
}

#[test]
fn a_weak_bin_keeps_its_decimals_instead_of_rounding_to_zero() {
    let m = meta(SampleFormat::I16, 32768.0);
    let a = analysis(-99.8, -121.7, 0.0025, -77.8);
    let text = human(&report(&m, &a, DbReference::Peak));

    assert!(!text.contains("0/32768"), "a real level printed as zero:\n{text}");
    assert!(text.contains("0.335/32768"), "{text}");
}

#[test]
fn float_formats_print_the_value_rather_than_a_count() {
    let m = meta(SampleFormat::F32, 1.0);
    let a = analysis(-6.0, -60.0, 0.5, 0.0);
    let text = human(&report(&m, &a, DbReference::FullScale));
    assert!(!text.contains('/'), "float levels have no denominator:\n{text}");
    assert!(text.contains("0.500"), "{text}");
}

#[test]
fn the_offset_frequency_carries_its_sign() {
    let m = meta(SampleFormat::I16, 32768.0);
    let a = analysis(-11.4, -87.2, 0.5, 0.0);
    assert!(human(&report(&m, &a, DbReference::FullScale)).contains("+2.404 kHz"));

    let mut negative = analysis(-11.4, -87.2, 0.5, 0.0);
    negative.psd.db = vec![-11.4, -87.2, -87.2];
    assert_eq!(negative.psd.freqs_hz[0], -2404.0);
    assert!(human(&report(&m, &negative, DbReference::FullScale)).contains("-2.404 kHz"));
}

#[test]
fn an_absolute_frequency_appears_only_when_a_centre_was_given() {
    let a = analysis(-11.4, -87.2, 0.5, 0.0);

    let baseband = human(&report(&meta(SampleFormat::I16, 32768.0), &a, DbReference::FullScale));
    assert!(!baseband.contains("MHz"), "{baseband}");

    let mut tuned = meta(SampleFormat::I16, 32768.0);
    tuned.center_freq = 12_579_000.0;
    let tuned_analysis = analysis_at(12_579_000.0, -11.4, -87.2, 0.5, 0.0);
    let text = human(&report(&tuned, &tuned_analysis, DbReference::FullScale));
    assert!(text.contains("+2.404 kHz"), "offset stays relative: {text}");
    assert!(text.contains("12.581404 MHz"), "{text}");
}

#[test]
fn a_dark_render_says_so_and_suggests_a_fix() {
    let m = meta(SampleFormat::I16, 32768.0);
    // Peak 100 dB below full scale: almost the whole ramp goes unused.
    let dark = report(&m, &analysis(-99.8, -121.7, 0.0025, 0.0), DbReference::FullScale);
    let hint = dark.contrast_hint().expect("should warn");
    assert!(hint.contains("--ref peak"), "{hint}");
    assert!(hint.contains("-d 40"), "range should fit peak to floor: {hint}");
    assert!(human(&dark).contains("hint"), "the hint should be printed");

    // A healthy capture gets no lecture.
    let bright = report(&m, &analysis(-11.4, -87.2, 0.5, 0.0), DbReference::FullScale);
    assert!(bright.contrast_hint().is_none());
    assert!(!human(&bright).contains("hint"));

    // Neither does one that already asked for the peak reference.
    let peaked = report(&m, &analysis(-99.8, -121.7, 0.0025, -77.8), DbReference::Peak);
    assert!(peaked.contrast_hint().is_none());
}

#[test]
fn the_analysed_span_is_only_mentioned_when_it_differs() {
    let m = meta(SampleFormat::I16, 32768.0);
    let a = analysis(-11.4, -87.2, 0.5, 0.0);
    assert!(!human(&report(&m, &a, DbReference::FullScale)).contains("analysed"));

    let partial = Report::new(
        &m,
        &a,
        &StftConfig::new(2048, Window::Hann),
        Reduce::Max,
        DbReference::FullScale,
        110.0,
        Normalize::None,
        0.0,
        30.0,
    );
    assert!(human(&partial).contains("analysed  30s"), "{}", human(&partial));
}

#[test]
fn the_plot_title_and_footer_describe_the_run() {
    let m = meta(SampleFormat::I16, 32768.0);
    let a = analysis(-11.4, -87.2, 0.5, 0.0);
    let r = report(&m, &a, DbReference::FullScale);

    let title = r.plot_title();
    assert!(title.starts_with("12.579000_capture.iqw"), "{title}");
    assert!(title.contains("iq_i16") && title.contains("24 kHz") && title.contains("30m"));

    let footer = r.plot_footer();
    assert!(footer.contains("fft 2048") && footer.contains("hann"));
    assert!(footer.contains("75% overlap"), "{footer}");
    assert!(footer.contains("full scale"), "{footer}");
    // Six decimals would print 17.578125 Hz here.
    assert!(footer.contains("17.578 Hz"), "{footer}");
}

#[test]
fn json_is_valid_and_carries_the_numbers() {
    let m = meta(SampleFormat::I16, 32768.0);
    let a = analysis(-11.4, -87.2, 0.2692, 0.0);
    let r = report(&m, &a, DbReference::FullScale)
        .with_output(
            std::path::Path::new("spec.png"),
            2048,
            512,
            253_952,
            "waveform".to_string(),
        );

    let value: serde_json::Value = serde_json::from_str(&r.to_json()).expect("valid json");
    assert_eq!(value["sample_type"], "iq_i16");
    assert_eq!(value["container"], "wav");
    assert_eq!(value["samples"], 43_200_000u64);
    assert_eq!(value["stft"]["fft_size"], 2048);
    assert_eq!(value["stft"]["frames"], 84_372u64);
    assert_eq!(value["output"]["width"], 2048);
    assert_eq!(value["output"]["bytes"], 253_952u64);

    let peak = &value["peak_bin"];
    assert_eq!(peak["bin"], 2);
    assert!((peak["dbfs"].as_f64().unwrap() - -11.4).abs() < 1e-4);
    assert!((peak["absolute"].as_f64().unwrap() - 8820.0).abs() < 2.0);
    assert!((value["floor"]["dbfs"].as_f64().unwrap() - -87.2).abs() < 1e-4);
}

#[test]
fn an_empty_spectrum_leaves_the_optional_fields_out() {
    let m = meta(SampleFormat::I16, 32768.0);
    let mut a = analysis(-11.4, -87.2, 0.5, 0.0);
    a.psd.db.clear();
    a.psd.freqs_hz.clear();

    let r = report(&m, &a, DbReference::FullScale);
    assert!(r.peak_bin.is_none() && r.floor.is_none());
    assert!(r.contrast_hint().is_none());

    let value: serde_json::Value = serde_json::from_str(&r.to_json()).unwrap();
    assert!(value["peak_bin"].is_null() && value["floor"].is_null());
    human(&r); // must not panic
}

#[test]
fn the_normalization_mode_is_reported_verbatim() {
    let m = meta(SampleFormat::F16x8, 4200.0);
    let a = analysis(-11.4, -87.2, 0.5, 0.0);
    let build = |mode| {
        Report::new(
            &m,
            &a,
            &StftConfig::new(2048, Window::Hann),
            Reduce::Max,
            DbReference::FullScale,
            110.0,
            mode,
            -6.0,
            1800.0,
        )
    };
    assert!(human(&build(Normalize::Auto)).contains("normalize auto"));
    assert!(human(&build(Normalize::None)).contains("normalize none"));
    assert!(human(&build(Normalize::Factor(2.5))).contains("normalize 2.5"));
    assert!(human(&build(Normalize::Auto)).contains("gain -6.0 dB"));
}
