use super::*;

fn view(time: (f64, f64)) -> crate::navigation::PictureView {
    crate::navigation::PictureView {
        time,
        frequency: (0., 1000.),
    }
}

fn picture(complete: bool) -> Backdrop {
    let mut image = argand_core::SpectrogramImage::new(2, 1);
    image.t1 = 100.0;
    Backdrop {
        db: Arc::new(argand_core::DbGrid {
            width: 2,
            height: 1,
            values: vec![-10.0, -20.0],
            t0: 0.0,
            t1: 100.0,
            f0: 0.0,
            f1: 1000.0,
        }),
        texture: spectrogram::texture(&image, crate::navigation::orientation::Mode::Horizontal)
            .unwrap(),
        image: Arc::new(image),
        settings: Settings::from_config(&Config::default()),
        complete,
        deep: None,
    }
}

#[test]
fn an_interrupted_preview_is_only_a_picture_not_a_numeric_level_source() {
    assert_eq!(picture(false).level_at(view((0.0, 100.0)), 0.25, 0.5), None);
    assert_eq!(
        picture(true).level_at(view((0.0, 100.0)), 0.25, 0.5),
        Some(-10.0)
    );
    assert_eq!(
        picture(true).level_at(view((100.0, 200.0)), 0.25, 0.5),
        None
    );
}

#[test]
fn narrower_panned_results_keep_the_widest_picture_and_completion_upgrades_a_preview() {
    let held = picture(false);
    assert!(!held.should_replace(view((60.0, 110.0)), true, false));
    assert!(!held.should_replace(view((100.0, 150.0)), true, false));
    assert!(held.should_replace(view((0.0, 110.0)), false, false));
    assert!(held.should_replace(view((0.0, 100.0)), true, false));
    assert!(!picture(true).should_replace(view((0.0, 100.0)), true, false));
}
#[test]
fn pending_style_delivery_defers_wider_retention_and_preview_upgrade() {
    let held = picture(false);
    for incoming in [(0.0, 100.0), (0.0, 150.0)] {
        assert!(
            !held.should_replace(view(incoming), true, true),
            "a render must not evict the source of the parked foreground delivery"
        );
        assert!(
            held.should_replace(view(incoming), true, false),
            "once the delivery is applied, the next render may retain the completed picture"
        );
    }
}
