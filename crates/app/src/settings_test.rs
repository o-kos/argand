use super::*;

#[test]
fn minimap_colormap_waits_for_the_picture_and_falls_back_before_the_first() {
    let original = Settings {
        colormap: Colormap::Oceanic,
        ..Settings::from_config(&Config::default())
    };
    let preview = Settings {
        colormap: Colormap::Inferno,
        ..original
    };
    assert_eq!(preview.minimap_colormap(None), Colormap::Inferno);
    assert_eq!(preview.minimap_colormap(Some(original)), Colormap::Oceanic);
    assert_eq!(preview.minimap_colormap(Some(preview)), Colormap::Inferno);
    assert_eq!(original.minimap_colormap(Some(preview)), Colormap::Inferno);
    assert_eq!(original.minimap_colormap(Some(original)), Colormap::Oceanic);
}

#[test]
fn confirming_numeric_edits_validates_both_fields_without_losing_either() {
    let settings = Settings::from_config(&Config::default());
    let edited = settings.edited_numbers("50", Some("47.5")).unwrap();
    assert_eq!(edited.overlap, 50);
    assert_eq!(edited.dynamic_range, DynamicRange::Fixed(47.5));
    for overlap in ["", "nan", "75.5", "96"] {
        assert!(settings.edited_numbers(overlap, Some("47.5")).is_err());
    }
    for range in ["", "0", "-1", "nan", "inf"] {
        assert!(settings.edited_numbers("50", Some(range)).is_err());
    }
    assert_eq!(
        settings.edited_numbers("50.0", None).unwrap().dynamic_range,
        DynamicRange::Default
    );
}

#[test]
fn saved_choices_round_trip_and_invalid_values_fall_back() {
    let config = Config::default();
    let settings = Settings {
        fft_size: 1024,
        window: Window::BlackmanHarris,
        overlap: 50,
        aggregation: Aggregation::MeanPower,
        colormap: Colormap::Inferno,
        dynamic_range: DynamicRange::Fixed(47.5),
    };
    let text = toml::to_string(&settings).unwrap();
    let saved = toml::from_str(&text).unwrap();
    assert!(!text.contains("dynamic_range"));
    assert_eq!(
        Settings::restored(Some(saved), &config),
        Settings {
            dynamic_range: config.dynamic_range,
            ..settings
        }
    );
    let legacy = format!("{text}dynamic_range = '43'\n");
    let saved = toml::from_str(&legacy).unwrap();
    let mut configured = config.clone();
    configured.dynamic_range = DynamicRange::Auto;
    assert_eq!(
        Settings::restored(Some(saved), &configured).dynamic_range,
        DynamicRange::Auto
    );
    assert_eq!(
        Settings::restored(
            Some(Settings {
                fft_size: 3,
                ..settings
            }),
            &config
        ),
        Settings::from_config(&config)
    );
    assert!(settings.validate(Some(32)).is_err());
    assert!(
        Settings {
            overlap: 100,
            ..settings
        }
        .validate(None)
        .is_err()
    );
    assert!(
        Settings {
            dynamic_range: DynamicRange::Fixed(f32::NAN),
            ..settings
        }
        .validate(None)
        .is_err()
    );
}
#[test]
fn overlap_aliases_have_identical_requests_and_can_acknowledge_the_display() {
    let base = Settings {
        fft_size: 4,
        ..Settings::from_config(&Config::default())
    };
    for overlap in [75, 87, 90, 95] {
        let selected = Settings { overlap, ..base };
        assert!(selected.equivalent(base));
        assert_eq!(selected.hop(), 1);
    }
    assert!(!Settings { overlap: 0, ..base }.equivalent(base));
    assert!(
        !Settings {
            colormap: Colormap::Inferno,
            ..base
        }
        .equivalent(base)
    );
}

#[test]
fn configuration_defaults_obey_the_same_limits_as_saved_and_interactive_settings() {
    let mut config = Config::default();
    config.stft.fft_size = usize::MAX / 2 + 1;
    config.color_scheme = Colormap::Inferno;
    let settings = Settings::restored(None, &config);
    assert!(settings.validate(None).is_ok());
    assert_eq!(settings.fft_size, 2048);
    assert_eq!(settings.colormap, Colormap::Inferno);
}
