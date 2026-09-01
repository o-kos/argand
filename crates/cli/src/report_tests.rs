use super::*;
use argand_core::{Colormap, Domain, SampleFormat, SampleRange, SampleType, SpectrogramImage};
use argand_core::{Psd, SignalMeta};
use argand_dsp::{
    Analysis, AnalysisRequest, DynamicRange, DynamicRangeResult, Reduce, StftConfig, Window,
};
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
    let recommended_db = (((peak_db - floor_db) * 1.5 / 10.0).ceil() * 10.0).clamp(20.0, 120.0);
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
        dynamic_range: DynamicRangeResult {
            requested: DynamicRange::Default,
            effective_db: 110.0,
            recommended_db,
        },
    }
}

/// The whole fixture capture: 43.2 M samples at 24 kHz, so 1800 seconds.
const FULL_SPAN: u64 = 43_200_000;

fn request(dynamic_range: DynamicRange, analysed_samples: u64) -> AnalysisRequest {
    AnalysisRequest {
        cfg: StftConfig::new(2048, Window::Hann),
        range: SampleRange::new(0, analysed_samples),
        width: 4,
        height: 4,
        reduce: Reduce::Max,
        colormap: Colormap::Oceanic,
        dynamic_range,
        waveform_columns: None,
    }
}

fn unscaled() -> Scaling {
    Scaling {
        normalize: Normalize::None,
        gain_db: 0.0,
    }
}

fn report(m: &SignalMeta, a: &Analysis) -> Report {
    Report::new(
        m,
        a,
        &request(a.dynamic_range.requested, FULL_SPAN),
        unscaled(),
    )
}

fn block(r: &Report, detail: Detail) -> String {
    let mut out = Vec::new();
    r.write_block(&mut out, detail).unwrap();
    String::from_utf8(out).unwrap()
}

fn human(r: &Report) -> String {
    block(r, Detail::Default)
}

fn verbose(r: &Report) -> String {
    block(r, Detail::Verbose)
}

/// The same report with the render written beside its input, which is where a
/// run without `-o` puts it.
fn rendered(r: Report) -> Report {
    r.with_output(
        std::path::Path::new("/data/12.579000_capture.iqw.png"),
        2048,
        512,
        253_952,
        "waveform".to_string(),
    )
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
    let beside = rendered(report(&m, &a));

    let line = compact(&beside, 2, 7);
    assert!(
        line.starts_with("[2/7] 12.579000_capture.iqw: wav iq_i16, 24 kHz, 30m"),
        "{line}"
    );
    assert!(
        line.contains("→ *.png, 248 KiB"),
        "the render is named by what it adds to the input:\n{line}"
    );
    assert!(!line.contains("/data/12.579000_capture.iqw.png"), "{line}");
    assert_eq!(line.lines().count(), 1, "one file, one line:\n{line}");

    // A render that is not the input's own name is spelled out in full.
    let elsewhere = report(&m, &a).with_output(
        std::path::Path::new("/tmp/spec.png"),
        2048,
        512,
        253_952,
        "waveform".to_string(),
    );
    assert!(compact(&elsewhere, 1, 1).contains("→ /tmp/spec.png"));
}

#[test]
fn a_compact_line_names_the_same_fields_as_the_block() {
    let m = meta(SampleFormat::I16, 32768.0);
    let a = analysis(-11.4, -87.2, 0.2692, 0.0);
    let r = rendered(report(&m, &a));

    // `peak` is the sample peak and `bin` the spectral one, in the block and
    // on the line alike, so the two shapes cannot name the same level twice.
    let line = compact(&r, 1, 2);
    assert!(line.contains("peak -11.4, bin -11.4 dBFS"), "{line}");
    assert!(human(&r).contains("peak -11.4, bin -11.4 @ +2.404 kHz"), "{r:?}");

    // What a listing has no room for: the bin's frequency and the floor.
    assert!(!line.contains(" @ "), "{line}");
    assert!(!line.contains("floor"), "{line}");
}

#[test]
fn the_default_block_is_two_named_sections_of_two_facts() {
    let m = meta(SampleFormat::I16, 32768.0);
    let a = analysis(-11.4, -87.2, 0.2692, 0.0);
    let text = human(&rendered(report(&m, &a)));

    assert_eq!(
        text,
        "12.579000_capture.iqw:\n  \
         wav iq_i16, 24 kHz, 30m\n  \
         peak -11.4, bin -11.4 @ +2.404 kHz, floor -87.2 dBFS\n\
         12.579000_capture.iqw.png:\n  \
         fft 2048, hann, hop 512, 84372 frames, range 110 dB\n  \
         2048×512, 248 KiB, 0ms\n"
    );
    assert_eq!(text.lines().count(), 6, "{text}");
    assert!(!text.contains("\n\n"), "no blank line inside the report:\n{text}");
}

#[test]
fn the_default_block_says_nothing_twice() {
    let m = meta(SampleFormat::I16, 32768.0);
    let dark = rendered(report(&m, &analysis(-99.8, -121.7, 0.0025, 0.0)));
    let text = human(&dark);

    // The input's name heads its own section and appears nowhere else; the
    // render's name heads its own and is not the full path.
    assert_eq!(text.matches("12.579000_capture.iqw:").count(), 1, "{text}");
    assert_eq!(text.matches("12.579000_capture.iqw.png").count(), 1, "{text}");
    assert!(!text.contains("/data/"), "{text}");
    // The divisor belongs to the file, not to each of its levels.
    assert!(!text.contains("32768"), "{text}");
    // The recommendation is made once, beside the range it argues with.
    assert_eq!(text.matches("-d 40").count(), 1, "{text}");
    // Defaults the caller did not ask for stay out of the way.
    assert!(!text.contains("normalize") && !text.contains("gain"), "{text}");
    // No label gutter: a field costs its own width and nothing more.
    assert!(!text.contains("  peak    "), "{text}");
}

#[test]
fn verbose_restores_the_detail_the_default_drops() {
    let m = meta(SampleFormat::I16, 32768.0);
    let a = analysis(-11.4, -87.2, 0.2692, 0.0);
    let text = verbose(&rendered(report(&m, &a)));

    // The divisor is named once, on the file it is a property of.
    assert!(text.contains("30m, 43.2 Mspl, full scale 32768"), "{text}");
    assert_eq!(text.matches("32768").count(), 1, "{text}");
    assert!(text.contains("normalize none, gain +0.0 dB"), "{text}");
    // The bin sits at -11.4 dBFS, which is 8820 of the 32768 counts the
    // format holds; the sample peak is the same level, rounded from 8820.8.
    assert!(text.contains("peak -11.4 (8821), bin -11.4 (8820)"), "{text}");
    assert!(text.contains("reduce max") && text.contains("range 110 dB (default)"), "{text}");
    assert!(text.contains("/data/12.579000_capture.iqw.png:"), "{text}");
    assert!(!text.contains("\n\n"), "no blank line inside the report:\n{text}");
}

#[test]
fn a_weak_bin_keeps_its_decimals_instead_of_rounding_to_zero() {
    let m = meta(SampleFormat::I16, 32768.0);
    let a = analysis(-99.8, -121.7, 0.0025, -77.8);
    let text = verbose(&report(&m, &a));

    assert!(!text.contains("(0)"), "a real level printed as zero:\n{text}");
    assert!(text.contains("bin -99.8 (0.335)"), "{text}");
}

#[test]
fn float_formats_print_the_value_rather_than_a_count() {
    let m = meta(SampleFormat::F32, 1.0);
    let a = analysis(-6.0, -60.0, 0.5, 0.0);
    let text = verbose(&report(&m, &a));
    assert!(!text.contains("full scale"), "float levels have no denominator:\n{text}");
    assert!(text.contains("peak -6.0 (0.500)"), "{text}");
}

#[test]
fn the_offset_frequency_carries_its_sign() {
    let m = meta(SampleFormat::I16, 32768.0);
    let a = analysis(-11.4, -87.2, 0.5, 0.0);
    assert!(human(&report(&m, &a)).contains("+2.404 kHz"));

    let mut negative = analysis(-11.4, -87.2, 0.5, 0.0);
    negative.psd.db = vec![-11.4, -87.2, -87.2];
    assert_eq!(negative.psd.freqs_hz[0], -2404.0);
    assert!(human(&report(&m, &negative)).contains("-2.404 kHz"));
}

#[test]
fn an_absolute_frequency_appears_only_when_a_centre_was_given() {
    let a = analysis(-11.4, -87.2, 0.5, 0.0);

    let baseband = human(&report(&meta(SampleFormat::I16, 32768.0), &a));
    assert!(!baseband.contains("MHz"), "{baseband}");

    let mut tuned = meta(SampleFormat::I16, 32768.0);
    tuned.center_freq = 12_579_000.0;
    let tuned_analysis = analysis_at(12_579_000.0, -11.4, -87.2, 0.5, 0.0);
    let text = human(&report(&tuned, &tuned_analysis));
    assert!(text.contains("+2.404 kHz"), "offset stays relative: {text}");
    assert!(text.contains("12.581404 MHz"), "{text}");
}

#[test]
fn an_excessive_non_auto_range_suggests_the_measured_one() {
    let m = meta(SampleFormat::I16, 32768.0);
    let dark = report(&m, &analysis(-99.8, -121.7, 0.0025, 0.0));
    assert_eq!(dark.range_suggestion().as_deref(), Some("sugg -d 40"));
    let text = human(&rendered(dark));
    assert!(
        text.contains("range 110 dB, try -d 40 to fit the drawn range\n"),
        "{text}"
    );

    // A range already close to the recommendation gets no lecture.
    let bright = rendered(report(&m, &analysis(-11.4, -87.2, 0.5, 0.0)));
    assert!(bright.range_suggestion().is_none());
    assert!(!human(&bright).contains("-d "), "{}", human(&bright));

    // Auto never suggests the value it already applied.
    let mut automatic = analysis(-99.8, -121.7, 0.0025, -77.8);
    automatic.dynamic_range.requested = DynamicRange::Auto;
    automatic.dynamic_range.effective_db = automatic.dynamic_range.recommended_db;
    let automatic = report(&m, &automatic);
    assert!(automatic.range_suggestion().is_none());
}

#[test]
fn a_compact_batch_keeps_the_range_suggestion() {
    let m = meta(SampleFormat::I16, 32768.0);
    let dark = report(&m, &analysis(-99.8, -121.7, 0.0025, 0.0));
    let line = compact(&dark, 1, 2);
    assert!(line.contains(", try -d 40"), "{line}");
    assert_eq!(line.lines().count(), 1);
}

#[test]
fn the_analysed_span_is_only_mentioned_when_it_differs() {
    let m = meta(SampleFormat::I16, 32768.0);
    let a = analysis(-11.4, -87.2, 0.5, 0.0);
    assert!(!human(&report(&m, &a)).contains("analysed"));

    // 30 seconds of the 1800-second capture.
    let partial = Report::new(
        &m,
        &a,
        &request(DynamicRange::Default, 30 * 24_000),
        unscaled(),
    );
    assert!(human(&partial).contains("30m, analysed 30s"), "{}", human(&partial));
}

#[test]
fn the_plot_title_and_footer_describe_the_run() {
    let m = meta(SampleFormat::I16, 32768.0);
    let a = analysis(-11.4, -87.2, 0.5, 0.0);
    let r = report(&m, &a);

    let title = r.plot_title();
    assert!(title.starts_with("12.579000_capture.iqw"), "{title}");
    assert!(title.contains("iq_i16") && title.contains("24 kHz") && title.contains("30m"));

    let footer = r.plot_footer();
    assert!(footer.contains("fft 2048") && footer.contains("hann"));
    assert!(footer.contains("75% overlap"), "{footer}");
    assert!(footer.contains("full scale"), "{footer}");

    let mut suggested = r.clone();
    suggested.stft.recommended_dynamic_range_db = 100.0;
    let suggested_footer = suggested.plot_footer();
    let suggested_scale_footer = suggested.plot_scale_footer();
    assert!(
        suggested_footer.contains("full scale (sugg -d 100) · dBFS"),
        "{suggested_footer}"
    );
    assert!(
        suggested_scale_footer.starts_with("110 dB below full scale (sugg -d 100)"),
        "{suggested_scale_footer}"
    );

    // The unit and the bandwidth behind it remain one field after the optional
    // recommendation.
    // Six decimals would print 17.578125 Hz here.
    assert!(footer.ends_with("· dBFS, ENBW 17.578 Hz"), "{footer}");
    // The unit divides by the window's coherent gain, not by a bandwidth, so
    // its levels are not a density per hertz and must not claim to be. One
    // value per bin is exactly what they are.
    assert!(!footer.contains("/bin"), "{footer}");
    // And the bandwidth is the window's, not the raw bin spacing: `Fs / N` is
    // 11.719 Hz here, so calling it `bin` sent a reader to check a different
    // number.
    assert!(!footer.contains("bin 17"), "{footer}");
}

#[test]
fn json_is_valid_and_carries_the_numbers() {
    let m = meta(SampleFormat::I16, 32768.0);
    let a = analysis(-11.4, -87.2, 0.2692, 0.0);
    let r = report(&m, &a)
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
    assert_eq!(value["stft"]["dynamic_range_mode"], "default");
    assert_eq!(value["stft"]["dynamic_range_db"], 110.0);
    assert_eq!(value["stft"]["recommended_dynamic_range_db"], 120.0);
    assert_eq!(value["output"]["width"], 2048);
    assert_eq!(value["output"]["bytes"], 253_952u64);

    let peak = &value["peak_bin"];
    assert_eq!(peak["bin"], 2);
    assert!((peak["dbfs"].as_f64().unwrap() - -11.4).abs() < 1e-4);
    assert!((peak["absolute"].as_f64().unwrap() - 8820.0).abs() < 2.0);
    assert!((value["floor"]["dbfs"].as_f64().unwrap() - -87.2).abs() < 1e-4);
}

#[test]
fn human_report_names_requested_effective_and_recommended_ranges() {
    let m = meta(SampleFormat::I16, 32768.0);
    let mut a = analysis(-99.8, -121.7, 0.0025, -99.8);
    a.dynamic_range.requested = DynamicRange::Fixed(60.0);
    a.dynamic_range.effective_db = 60.0;

    let text = verbose(&rendered(report(&m, &a)));
    assert!(text.contains("range 60 dB (fixed), try -d 40"), "{text}");

    a.dynamic_range.requested = DynamicRange::Fixed(20.5);
    a.dynamic_range.effective_db = 20.5;
    let text = human(&rendered(report(&m, &a)));
    assert!(text.contains("range 20.5 dB\n"), "{text}");
}

#[test]
fn an_empty_spectrum_leaves_the_optional_fields_out() {
    let m = meta(SampleFormat::I16, 32768.0);
    let mut a = analysis(-11.4, -87.2, 0.5, 0.0);
    a.psd.db.clear();
    a.psd.freqs_hz.clear();

    let r = report(&m, &a);
    assert!(r.peak_bin.is_none() && r.floor.is_none());
    assert!(r.range_suggestion().is_none());

    let value: serde_json::Value = serde_json::from_str(&r.to_json()).unwrap();
    assert!(value["peak_bin"].is_null() && value["floor"].is_null());

    // Only the sample peak survives, and the levels keep their unit.
    assert!(human(&r).contains("\n  peak -6.0 dBFS\n"), "{}", human(&r));
    assert_eq!(human(&r).lines().count(), 3, "{}", human(&r));
    assert!(compact(&r, 1, 2).contains("peak -6.0 dBFS"));
}

#[test]
fn a_render_sent_elsewhere_is_headed_by_its_whole_path() {
    let m = meta(SampleFormat::I16, 32768.0);
    let a = analysis(-11.4, -87.2, 0.2692, 0.0);
    let elsewhere = report(&m, &a).with_output(
        std::path::Path::new("/tmp/spec.png"),
        2048,
        512,
        253_952,
        "waveform".to_string(),
    );
    assert!(
        human(&elsewhere).contains("\n/tmp/spec.png:\n"),
        "{}",
        human(&elsewhere)
    );

    // A relative -o resolves against the caller's directory, not against the
    // input's, so a bare name would point at the wrong place.
    let relative = report(&m, &a).with_output(
        std::path::Path::new("spec.png"),
        2048,
        512,
        253_952,
        "waveform".to_string(),
    );
    let header = human(&relative)
        .lines()
        .find(|line| line.ends_with("spec.png:"))
        .map(str::to_string)
        .expect("the render section is headed by its own file");
    assert!(
        std::path::Path::new(header.trim_end_matches(':')).is_absolute(),
        "{header}"
    );
}

#[test]
fn the_normalization_mode_is_reported_verbatim() {
    let m = meta(SampleFormat::F16x8, 4200.0);
    let a = analysis(-11.4, -87.2, 0.5, 0.0);
    let build = |mode| {
        Report::new(
            &m,
            &a,
            &request(DynamicRange::Default, FULL_SPAN),
            Scaling {
                normalize: mode,
                gain_db: -6.0,
            },
        )
    };
    assert!(verbose(&build(Normalize::Auto)).contains("normalize auto"));
    assert!(verbose(&build(Normalize::None)).contains("normalize none"));
    assert!(verbose(&build(Normalize::Factor(2.5))).contains("normalize 2.5"));
    assert!(verbose(&build(Normalize::Auto)).contains("gain -6.0 dB"));
}
