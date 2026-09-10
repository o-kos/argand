use super::*;
use argand_dsp::AnalysisRequest;
use argand_dsp::Window;

/// A scratch directory removed when it goes out of scope.
struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new(label: &str) -> Self {
        use std::sync::atomic::{AtomicU32, Ordering};
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "argand-app-{label}-{}-{unique}",
            std::process::id()
        ));
        std::fs::create_dir_all(&path).expect("create scratch dir");
        Self { path }
    }

    fn write(&self, name: &str, text: &str) -> PathBuf {
        let path = self.path.join(name);
        std::fs::write(&path, text).expect("write fixture");
        path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

#[test]
fn distributed_template_explicitly_lists_every_configuration_key() {
    use std::collections::BTreeSet;

    let template: toml::Table =
        toml::from_str(include_str!("../assets/argand.toml")).expect("valid template");
    let sections: &[(&str, &[&str])] = &[
        (
            "",
            &[
                "theme", "number_format", "color_scheme", "dynamic_range", "aggregation",
                "stft", "analysis", "panels",
            ],
        ),
        ("stft", &["fft_size", "window"]),
        ("analysis", &["workers", "batch_frames", "affinity"]),
        ("panels", &["waveform_fraction"]),
    ];
    for &(section, keys) in sections {
        let table = if section.is_empty() {
            &template
        } else {
            template[section].as_table().expect("configuration section")
        };
        let actual: BTreeSet<_> = table.keys().map(String::as_str).collect();
        assert_eq!(actual, keys.iter().copied().collect(), "section {section:?}");
    }
}

#[test]
fn distributed_template_parses_without_repairs_and_matches_built_in_defaults() {
    let template = include_str!("../assets/argand.toml");
    let config: Config = toml::from_str(template).expect("valid distributed configuration");
    assert_eq!(config, Config::default());
    assert_eq!(config.clone().repaired(), config);
}

#[test]
fn number_format_defaults_to_system_and_accepts_explicit_locales() {
    let dir = TempDir::new("number-format");
    assert_eq!(Config::default().number_format, "system");
    for format in ["system", "ru-RU", "en-US", "ar-EG-u-nu-latn", "C", "POSIX"] {
        let path = dir.write("argand.toml", &format!("number_format = {format:?}\n"));
        assert_eq!(Config::load(&[path]).number_format, format);
    }
}

#[test]
fn invalid_number_format_preserves_other_configuration() {
    let dir = TempDir::new("invalid-number-format");
    for format in ["", "ru_RU.UTF-8", "not a locale"] {
        let path = dir.write(
            "argand.toml",
            &format!("theme = \"light\"\nnumber_format = {format:?}\n"),
        );
        let config = Config::load(&[path]);
        assert_eq!(config.number_format, "system");
        assert_eq!(config.theme, Theme::Light);
    }
}

#[test]
fn a_file_that_sets_one_value_is_a_complete_file() {
    let dir = TempDir::new("partial");
    let path = dir.write("argand.toml", "theme = \"light\"\n");

    let config = Config::load(&[path]);
    assert_eq!(config.theme, Theme::Light);
    // Everything it did not mention keeps the shipped value.
    assert_eq!(config.panels, Panels::default());
    assert_eq!(config.number_format, "system");
}

#[test]
fn nothing_in_the_file_can_stop_the_application_starting() {
    let dir = TempDir::new("broken");
    // Each of these is a different way for a person to get it wrong.
    for (name, text) in [
        ("truncated.toml", "theme = "),
        ("wrong-type.toml", "theme = 42"),
        ("unknown-key.toml", "colour_scheme = \"oceanic\""),
        ("not-toml.toml", "<html></html>"),
    ] {
        let path = dir.write(name, text);
        assert_eq!(
            Config::load(&[path]),
            Config::default(),
            "{name} did not fall back to defaults"
        );
    }
}

#[test]
fn a_missing_file_is_not_a_failure_and_does_not_stop_the_search() {
    let dir = TempDir::new("search");
    let missing = dir.path.join("absent.toml");
    let present = dir.write("argand.toml", "theme = \"light\"\n");

    // The first candidate is simply not there, so the second one answers.
    assert_eq!(
        Config::load(&[missing.clone(), present]).theme,
        Theme::Light
    );
    // And no candidate at all is still a working configuration.
    assert_eq!(Config::load(&[missing]), Config::default());
}

#[test]
fn the_copy_beside_the_binary_wins_over_the_one_on_the_host() {
    let dir = TempDir::new("order");
    let beside = dir.write("beside.toml", "theme = \"light\"\n");
    let host = dir.write("host.toml", "theme = \"dark\"\n");

    // Whichever comes first in the search path decides.
    assert_eq!(Config::load(&[beside, host]).theme, Theme::Light);

    // And what `search_path` puts first is the executable's own directory,
    // which is what makes a copy carried beside the binary win.
    let ordered = Config::search_path();
    let first = ordered.first().expect("nowhere to read a configuration from");
    let exe = std::env::current_exe().expect("this test is running from somewhere");
    assert_eq!(
        first.parent(),
        exe.parent(),
        "the first candidate is not the one beside the binary"
    );
    assert_eq!(first.file_name(), Some(FILE_NAME.as_ref()));
}

#[test]
fn the_application_never_writes_the_file_a_person_owns() {
    let dir = TempDir::new("readonly");
    let text = "# a comment someone wrote\ntheme = \"light\"\n";
    let path = dir.write("argand.toml", text);

    let _ = Config::load(std::slice::from_ref(&path));
    assert_eq!(
        std::fs::read_to_string(&path).expect("the file is still there"),
        text,
        "loading the configuration rewrote it"
    );
}

#[test]
fn every_setting_the_file_offers_is_read_by_the_name_the_cli_uses() {
    let dir = TempDir::new("full");
    let path = dir.write(
        "argand.toml",
        r#"
theme = "light"
color_scheme = "viridis"
dynamic_range = "auto"

[stft]
fft_size = 4096
window = "blackman-harris"

[panels]
waveform_fraction = 0.35
"#,
    );

    let config = Config::load(std::slice::from_ref(&path));
    assert_eq!(config.theme, Theme::Light);
    assert_eq!(config.color_scheme, Colormap::Viridis);
    assert_eq!(config.dynamic_range, DynamicRange::Auto);
    assert_eq!(config.stft.fft_size, 4096);
    assert_eq!(config.stft.window, Window::BlackmanHarris);
    assert!((config.panels.waveform_fraction - 0.35).abs() < 1e-6);
}

#[test]
fn the_colour_range_takes_the_word_the_report_prints() {
    let dir = TempDir::new("range");
    // `default` is what the report shows and what the command line spells by
    // leaving `-d` out, so a file may say it.
    let path = dir.write("argand.toml", "dynamic_range = \"default\"\n");
    assert_eq!(Config::load(&[path]).dynamic_range, DynamicRange::Default);

    let path = dir.write("numeric.toml", "dynamic_range = \"60\"\n");
    assert_eq!(
        Config::load(&[path]).dynamic_range,
        DynamicRange::Fixed(60.0)
    );
}

#[test]
fn a_value_that_parses_but_cannot_be_used_costs_only_itself() {
    let dir = TempDir::new("repair");
    // A transform size the FFT would refuse, beside a setting that is fine.
    let path = dir.write(
        "argand.toml",
        "theme = \"light\"\n\n[stft]\nfft_size = 1000\n",
    );

    let config = Config::load(&[path]);
    assert_eq!(config.stft.fft_size, Config::default().stft.fft_size);
    // The rest of the file survived: one bad value is not a bad file.
    assert_eq!(config.theme, Theme::Light);
}

#[test]
fn a_waveform_share_that_leaves_no_plot_is_refused() {
    let dir = TempDir::new("fraction");
    let default = Config::default().panels.waveform_fraction;
    for value in ["0.0", "1.0", "-0.5", "2.0", "nan", "inf"] {
        let path = dir.write(
            "argand.toml",
            &format!("[panels]\nwaveform_fraction = {value}\n"),
        );
        let got = Config::load(&[path]).panels.waveform_fraction;
        assert!(
            (got - default).abs() < 1e-6,
            "{value} was accepted as {got}"
        );
    }
}

#[test]
fn a_name_the_cli_would_reject_is_rejected_here_too() {
    let dir = TempDir::new("names");
    for text in [
        "color_scheme = \"chartreuse\"",
        "dynamic_range = \"loud\"",
        "[stft]\nwindow = \"triangular\"",
    ] {
        let path = dir.write("argand.toml", text);
        assert_eq!(
            Config::load(&[path]),
            Config::default(),
            "{text} was accepted"
        );
    }
}

#[test]
fn the_transform_a_picture_is_drawn_from_is_the_one_the_file_asked_for() {
    let text = "\
color_scheme = \"viridis\"
dynamic_range = \"60\"

[stft]
fft_size = 512
window = \"blackman-harris\"
";
    let config = Config::parse(text, Path::new("argand.toml"));
    let meta = argand_core::SignalMeta {
        sample_rate: 24_000.0,
        center_freq: 0.0,
        sample_type: argand_core::SampleType::new(
            argand_core::Domain::Iq,
            argand_core::SampleFormat::I16,
        ),
        len_samples: 96_000,
        container: "wav",
        divisor: 32_768.0,
        source: PathBuf::from("capture.wav"),
    };

    let request = crate::settings::Settings::from_config(&config).analysis_request(&meta, 1024, 480);

    assert_eq!(request.cfg.fft_size, 512);
    assert_eq!(request.cfg.window, argand_dsp::Window::BlackmanHarris);
    // Three quarters of overlap, which is what `aspec` uses without `--hop`.
    assert_eq!(request.cfg.hop, 128);
    assert_eq!(request.colormap, argand_core::Colormap::Viridis);
    assert_eq!(request.dynamic_range, DynamicRange::Fixed(60.0));
    assert_eq!(request.width, 1024);
    assert_eq!(request.height, 480);
    // The whole file, since nothing narrows it yet.
    assert_eq!(request.range, argand_core::SampleRange::new(0, 96_000));
    assert_eq!(request.waveform_columns, Some(request.width));
}

#[test]
fn a_transform_size_of_two_still_leaves_a_hop_of_at_least_one() {
    let config = Config {
        stft: Stft {
            fft_size: 2,
            ..Stft::default()
        },
        ..Config::default()
    };
    let meta = argand_core::SignalMeta {
        sample_rate: 8_000.0,
        center_freq: 0.0,
        sample_type: argand_core::SampleType::new(
            argand_core::Domain::Real,
            argand_core::SampleFormat::F32,
        ),
        len_samples: 8_000,
        container: "wav",
        divisor: 1.0,
        source: PathBuf::from("capture.wav"),
    };

    assert_eq!(crate::settings::Settings::from_config(&config).analysis_request(&meta, 64, 32).cfg.hop, 1);
}

#[test]
fn aggregation_defaults_and_switches_without_changing_the_transform_or_waveform() {
    let meta = argand_core::SignalMeta {
        sample_rate: 24_000.0,
        center_freq: 0.0,
        sample_type: argand_core::SampleType::new(argand_core::Domain::Real, argand_core::SampleFormat::F32),
        len_samples: 48_000,
        container: "test",
        divisor: 1.0,
        source: PathBuf::from("memory"),
    };
    let default = crate::settings::Settings::from_config(&Config::default()).analysis_request(&meta, 800, 400);
    assert_eq!(default.reduce, Reduce::Max);
    for aggregation in Aggregation::ALL {
        let name = aggregation.reduce().as_str();
        let config: Config = toml::from_str(&format!("aggregation = \"{name}\"\n")).unwrap();
        assert_eq!(config.aggregation, aggregation);
        let request = crate::settings::Settings::from_config(&config).analysis_request(&meta, 800, 400);
        assert_eq!(request.reduce, aggregation.reduce());
        assert_eq!(AnalysisRequest { reduce: default.reduce, ..request }, default);
    }
    assert!(toml::from_str::<Config>("aggregation = \"mean\"").is_err());
}
