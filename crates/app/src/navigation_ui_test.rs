use super::*;

#[test]
fn zoom_symbols_use_physical_shift_without_leaking_into_time_zoom() {
    let control = gpui_kit::Modifiers {
        control: true,
        ..Default::default()
    };
    let shifted = gpui_kit::Modifiers {
        shift: true,
        ..control
    };
    for key in ["+", "=", "add"] {
        assert!(
            plot_shortcut(key, control, false)
                .unwrap()
                .as_any()
                .is::<ZoomIn>()
        );
        assert!(
            plot_shortcut(key, control, true)
                .unwrap()
                .as_any()
                .is::<FrequencyZoomIn>()
        );
        assert!(
            plot_shortcut(key, shifted, false)
                .unwrap()
                .as_any()
                .is::<FrequencyZoomIn>()
        );
    }
    for key in ["-", "_", "subtract"] {
        assert!(
            plot_shortcut(key, control, false)
                .unwrap()
                .as_any()
                .is::<ZoomOut>()
        );
        assert!(
            plot_shortcut(key, control, true)
                .unwrap()
                .as_any()
                .is::<FrequencyZoomOut>()
        );
        assert!(
            plot_shortcut(key, shifted, false)
                .unwrap()
                .as_any()
                .is::<FrequencyZoomOut>()
        );
    }
    for key in ["0", ")"] {
        assert!(
            plot_shortcut(key, control, true)
                .unwrap()
                .as_any()
                .is::<FitFrequency>()
        );
        assert!(
            plot_shortcut(key, shifted, false)
                .unwrap()
                .as_any()
                .is::<FitFrequency>()
        );
    }
    assert!(plot_shortcut("0", control, false).is_none());
    for modifiers in [
        gpui_kit::Modifiers::default(),
        gpui_kit::Modifiers {
            alt: true,
            ..control
        },
        gpui_kit::Modifiers {
            platform: true,
            ..control
        },
    ] {
        assert!(plot_shortcut("+", modifiers, true).is_none());
        assert!(plot_shortcut(")", modifiers, true).is_none());
    }
    for key in ["up", "down", "home", "a"] {
        assert!(plot_shortcut(key, control, true).is_none());
    }
}

fn geometry() -> PlotGeometry {
    PlotGeometry {
        orientation: crate::orientation::Mode::Horizontal,
        scale: 1.,
        time_ruler: Bounds::new(point(px(10.), px(150.)), size(px(100.), px(20.))),
        unit_hints: [None; 2],
        zoom_zones: [None, None],
        time_scheme: None,
        frequency_scheme: None,
        minimap_columns: 100,
        frequency_ruler: Bounds::new(point(px(110.), px(50.)), size(px(30.), px(100.))),
        spectrum: Bounds::new(point(px(10.), px(50.)), size(px(100.), px(100.))),
        navigation: Bounds::new(point(px(10.), px(10.)), size(px(100.), px(160.))),
        minimap: Bounds::new(point(px(10.), px(10.)), size(px(100.), px(30.))),
    }
}

#[test]
fn space_shows_an_open_hand_on_the_spectrum_and_selecting_keeps_the_crosshair() {
    use gpui_kit::CursorStyle::{Crosshair, OpenHand};
    let geometry = geometry();
    let spectrum = Some(point(px(60.), px(100.)));
    let ruler = Some(point(px(60.), px(160.)));
    assert_eq!(
        geometry.selection_cursor(spectrum, false, None, true),
        Some(OpenHand)
    );
    assert_eq!(geometry.selection_cursor(ruler, false, None, true), None);
    assert_eq!(
        geometry.selection_cursor(spectrum, false, None, false),
        None
    );
    assert_eq!(
        geometry.selection_cursor(ruler, false, Some(gpui_kit::CursorStyle::Crosshair), false),
        Some(Crosshair)
    );
    assert_eq!(
        geometry.selection_cursor(spectrum, true, None, true),
        None,
        "a pan in progress keeps its closed hand"
    );
}

#[test]
fn scale_button_zones_take_the_pointer() {
    let mut geometry = geometry();
    geometry.zoom_zones = [
        Some(axes::Rect {
            x: 10.,
            y: 125.,
            width: 45.,
            height: 22.,
        }),
        None,
    ];
    assert!(geometry.over_scale_buttons(Some(point(px(30.), px(135.)))));
    assert!(!geometry.over_scale_buttons(Some(point(px(60.), px(135.)))));
    assert!(!geometry.over_scale_buttons(None));
}

#[test]
fn cursor_identifies_spectrum_minimap_viewport_and_both_rulers() {
    let geometry = geometry();
    let viewport = Some((
        View {
            start: 200,
            len: 300,
        },
        1000,
    ));
    for (x, y, expected) in [
        (50., 20., gpui_kit::CursorStyle::OpenHand),
        (20., 20., gpui_kit::CursorStyle::Arrow),
        (50., 160., gpui_kit::CursorStyle::OpenHand),
        (120., 80., gpui_kit::CursorStyle::Arrow),
        (50., 80., gpui_kit::CursorStyle::Crosshair),
    ] {
        assert_eq!(
            geometry.cursor(Some(point(px(x), px(y))), false, viewport, false),
            expected
        );
    }
    assert_eq!(
        geometry.cursor(None, true, viewport, false),
        gpui_kit::CursorStyle::ClosedHand
    );
    assert_eq!(
        geometry.cursor(None, false, viewport, false),
        gpui_kit::CursorStyle::Arrow
    );
    assert_eq!(
        geometry.cursor(Some(point(px(50.), px(20.))), false, None, false),
        gpui_kit::CursorStyle::Arrow
    );
}

#[test]
fn hidpi_minimap_hand_and_click_match_the_last_visible_device_pixel() {
    let geometry = PlotGeometry {
        minimap_columns: 200,
        ..geometry()
    };
    let view = View {
        start: 999999,
        len: 1,
    };
    let viewport = Some((view, 1000000));
    let interval = crate::minimap::viewport(view, 1000000, geometry.minimap_columns);
    assert_eq!(interval, (0.995, 1.));
    for (x, cursor, click) in [
        (
            109.25,
            gpui_kit::CursorStyle::Arrow,
            crate::minimap::Click::Step(-1),
        ),
        (
            109.75,
            gpui_kit::CursorStyle::OpenHand,
            crate::minimap::Click::Grab,
        ),
    ] {
        let position = point(px(x), px(20.));
        assert_eq!(
            geometry.cursor(Some(position), false, viewport, false),
            cursor
        );
        assert_eq!(
            crate::minimap::click(geometry.minimap_fraction(position), interval, false, 1),
            click
        );
    }
}

#[test]
fn full_capture_and_non_draggable_frequency_ruler_use_an_arrow() {
    let geometry = geometry();
    for viewport in [None, Some((View::full(1000), 1000))] {
        for (x, y) in [(50., 20.), (50., 160.), (120., 80.)] {
            assert_eq!(
                geometry.cursor(Some(point(px(x), px(y))), false, viewport, false),
                gpui_kit::CursorStyle::Arrow,
            );
        }
    }
}

#[test]
fn wheel_bindings_are_identical_on_spectrum_and_ruler() {
    let geometry = geometry();
    for position in [point(px(60.), px(80.)), point(px(60.), px(160.))] {
        let delta = point(px(0.), px(-160.));
        assert!(
            matches!(geometry.scroll(position, delta, gpui_kit::Modifiers::default()), Scroll::Pan(f) if (f - 1.6).abs() < 1e-10)
        );
        let control = gpui_kit::Modifiers {
            control: true,
            ..Default::default()
        };
        assert!(matches!(
            geometry.scroll(position, delta, control),
            Scroll::Zoom {
                factor: 2.0,
                anchor: 0.5
            }
        ));
        let shift = gpui_kit::Modifiers {
            shift: true,
            ..Default::default()
        };
        assert!(matches!(
            geometry.scroll(position, delta, shift),
            Scroll::FrequencyPan(_)
        ));
    }
}
#[test]
fn frequency_ruler_and_shift_gestures_use_the_frequency_axis() {
    let geometry = geometry();
    let ruler = point(px(120.), px(75.));
    assert_eq!(
        geometry.cursor(Some(ruler), false, None, true),
        gpui_kit::CursorStyle::OpenHand
    );
    let delta = point(px(0.), px(40.));
    assert!(
        matches!(geometry.scroll(ruler, delta, gpui_kit::Modifiers::default()), Scroll::FrequencyPan(f) if (f - 0.4).abs() < 1e-10)
    );
    let control = gpui_kit::Modifiers {
        control: true,
        ..Default::default()
    };
    assert!(matches!(
        geometry.scroll(ruler, delta, control),
        Scroll::FrequencyZoom { anchor: 0.75, .. }
    ));
    let shift_control = gpui_kit::Modifiers {
        shift: true,
        control: true,
        ..Default::default()
    };
    assert!(matches!(
        geometry.scroll(point(px(60.), px(100.)), delta, shift_control),
        Scroll::FrequencyZoom { anchor: 0.5, .. }
    ));
}
#[test]
fn shifted_horizontal_wheel_delta_matches_vertical_delivery() {
    for orientation in [
        crate::orientation::Mode::Horizontal,
        crate::orientation::Mode::Vertical,
    ] {
        let geometry = PlotGeometry {
            orientation,
            ..geometry()
        };
        for position in [
            point(px(60.), px(80.)),
            point(px(60.), px(160.)),
            point(px(120.), px(75.)),
        ] {
            for control in [false, true] {
                let modifiers = gpui_kit::Modifiers {
                    shift: true,
                    control,
                    ..Default::default()
                };
                let vertical = geometry.scroll(position, point(px(0.), px(-160.)), modifiers);
                let horizontal = geometry.scroll(position, point(px(-160.), px(0.)), modifiers);
                assert_eq!(horizontal, vertical);
            }
        }
    }
}

#[test]
fn unit_captions_keep_arrow_cursor_and_do_not_start_dragging() {
    let mut geometry = geometry();
    for bounds in [geometry.time_ruler, geometry.frequency_ruler] {
        geometry.unit_hints[0] = Some(axes::UnitHint {
            bounds: axes::Rect {
                x: f32::from(bounds.left()),
                y: f32::from(bounds.top()),
                width: 20.,
                height: 12.,
            },
            text: "Time in seconds",
            units: "s",
            per_pixel: 1.,
        });
        let position = bounds.origin + point(px(2.), px(2.));
        let viewport = Some((
            View {
                start: 200,
                len: 300,
            },
            1000,
        ));
        assert_eq!(
            geometry.cursor(Some(position), false, viewport, true),
            gpui_kit::CursorStyle::Arrow
        );
        assert_eq!(geometry.drag_axes(position), (false, false));
    }
}

#[test]
fn ruler_zoom_buttons_keep_the_arrow_cursor_and_do_not_start_dragging() {
    let mut geometry = geometry();
    geometry.zoom_zones = [
        Some(axes::Rect {
            x: 10.,
            y: 158.,
            width: 44.,
            height: 20.,
        }),
        Some(axes::Rect {
            x: 119.,
            y: 106.,
            width: 20.,
            height: 44.,
        }),
    ];
    let viewport = Some((
        View {
            start: 200,
            len: 300,
        },
        1000,
    ));
    for position in [point(px(20.), px(166.)), point(px(128.), px(130.))] {
        assert_eq!(
            geometry.cursor(Some(position), false, viewport, true),
            gpui_kit::CursorStyle::Arrow
        );
        assert_eq!(geometry.drag_axes(position), (false, false));
    }
    assert_eq!(
        geometry.cursor(Some(point(px(60.), px(160.))), false, viewport, true),
        gpui_kit::CursorStyle::OpenHand
    );
}

#[test]
fn vertical_zoom_pairs_keep_the_arrow_cursor_and_do_not_start_dragging() {
    // Time pair along the right ruler, frequency pair along the bottom one.
    let geometry = PlotGeometry {
        orientation: crate::orientation::Mode::Vertical,
        spectrum: Bounds::new(point(px(50.), px(10.)), size(px(100.), px(200.))),
        minimap: Bounds::new(point(px(10.), px(10.)), size(px(30.), px(200.))),
        time_ruler: Bounds::new(point(px(150.), px(10.)), size(px(30.), px(200.))),
        frequency_ruler: Bounds::new(point(px(50.), px(210.)), size(px(100.), px(20.))),
        navigation: Bounds::new(point(px(10.), px(10.)), size(px(170.), px(200.))),
        zoom_zones: [
            Some(axes::Rect {
                x: 120.,
                y: 18.,
                width: 22.,
                height: 45.,
            }),
            Some(axes::Rect {
                x: 58.,
                y: 180.,
                width: 45.,
                height: 22.,
            }),
        ],
        ..geometry()
    };
    let viewport = Some((
        View {
            start: 200,
            len: 300,
        },
        1000,
    ));
    for position in [point(px(130.), px(55.)), point(px(95.), px(190.))] {
        assert!(geometry.over_scale_buttons(Some(position)));
        assert_eq!(
            geometry.cursor(Some(position), false, viewport, true),
            gpui_kit::CursorStyle::Arrow
        );
        assert_eq!(geometry.drag_axes(position), (false, false));
    }
    for position in [point(px(130.), px(80.)), point(px(110.), px(190.))] {
        assert!(!geometry.over_scale_buttons(Some(position)));
        assert_eq!(
            geometry.cursor(Some(position), false, viewport, true),
            gpui_kit::CursorStyle::Crosshair
        );
        assert_eq!(geometry.drag_axes(position), (true, true));
    }
}

#[test]
fn hidden_scale_controls_leave_the_spectrum_navigation_alone() {
    // Hidden pairs leave no zones behind, so the corner area they would
    // cover pans and drags like the rest of the picture.
    let geometry = geometry();
    let corner = point(px(25.), px(135.));
    assert!(geometry.spectrum.contains(&corner));
    let viewport = Some((
        View {
            start: 200,
            len: 300,
        },
        1000,
    ));
    assert_eq!(
        geometry.cursor(Some(corner), false, viewport, true),
        gpui_kit::CursorStyle::Crosshair
    );
    assert_eq!(geometry.drag_axes(corner), (true, true));
}

#[test]
fn spectrum_drags_both_axes_but_rulers_and_minimap_stay_constrained() {
    let geometry = geometry();
    assert_eq!(geometry.drag_axes(point(px(60.), px(80.))), (true, true));
    assert_eq!(geometry.drag_axes(point(px(60.), px(160.))), (true, false));
    assert_eq!(geometry.drag_axes(point(px(120.), px(75.))), (false, true));
    assert_eq!(geometry.drag_axes(point(px(60.), px(20.))), (true, false));
    assert_eq!(geometry.drag_axes(point(px(0.), px(0.))), (false, false));
}

#[test]
fn vertical_rulers_and_minimap_use_time_down_and_frequency_right() {
    let geometry = PlotGeometry {
        orientation: crate::orientation::Mode::Vertical,
        spectrum: Bounds::new(point(px(50.), px(10.)), size(px(100.), px(200.))),
        minimap: Bounds::new(point(px(10.), px(10.)), size(px(30.), px(200.))),
        time_ruler: Bounds::new(point(px(150.), px(10.)), size(px(30.), px(200.))),
        frequency_ruler: Bounds::new(point(px(50.), px(210.)), size(px(100.), px(20.))),
        navigation: Bounds::new(point(px(10.), px(10.)), size(px(170.), px(200.))),
        ..geometry()
    };
    assert_eq!(geometry.drag_axes(point(px(75.), px(60.))), (true, true));
    assert_eq!(geometry.drag_axes(point(px(175.), px(60.))), (true, false));
    assert_eq!(geometry.drag_axes(point(px(75.), px(220.))), (false, true));
    let control = gpui_kit::Modifiers {
        control: true,
        ..Default::default()
    };
    let delta = point(px(0.), px(-160.));
    assert!(matches!(
        geometry.scroll(point(px(175.), px(60.)), delta, control),
        Scroll::Zoom {
            anchor: 0.25,
            factor: 2.
        }
    ));
    assert!(matches!(
        geometry.scroll(point(px(75.), px(220.)), delta, control),
        Scroll::FrequencyZoom {
            anchor: 0.25,
            factor: 2.
        }
    ));
    assert_eq!(geometry.minimap_fraction(point(px(20.), px(60.))), 0.25);
    let viewport = Some((
        View {
            start: 200,
            len: 300,
        },
        1000,
    ));
    assert_eq!(
        geometry.cursor(Some(point(px(20.), px(60.))), false, viewport, true),
        gpui_kit::CursorStyle::OpenHand
    );
    assert_eq!(
        geometry.cursor(Some(point(px(175.), px(60.))), false, viewport, true),
        gpui_kit::CursorStyle::OpenHand
    );
    assert_eq!(
        geometry.cursor(Some(point(px(75.), px(220.))), false, viewport, true),
        gpui_kit::CursorStyle::OpenHand
    );
}
