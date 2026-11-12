include!("plot_view_context_menu_test.rs");

use super::*;
use gpui_kit::test::TestWindowExt;
use gpui_kit::{Entity, TestAppContext, WindowHandle};

/// Stands in for the shell: owns the plot, counts what reaches it, and
/// offers a second focus target beside the plot.
struct Harness {
    plot: Option<Entity<PlotView>>,
    other: FocusHandle,
    intents: Vec<PlotIntent>,
    grid: usize,
    viewport: Option<gpui_kit::Size<Pixels>>,
    /// Stands in for the session's scale-controls choice.
    scale_ui: bool,
    /// Stands in for a hint or a menu lying on the plot.
    cover: Option<Bounds<Pixels>>,
    /// Takes the selections the plot reports into its snapshot, as the shell does.
    follow_selection: bool,
    _intents: Option<Subscription>,
}

impl Render for Harness {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .when_some(self.viewport, |view, viewport| {
                view.w(viewport.width).h(viewport.height)
            })
            .relative()
            .flex()
            .flex_col()
            .key_context("Shell")
            .on_action(cx.listener(|harness, _: &ToggleGrid, _, _| harness.grid += 1))
            .on_action(
                cx.listener(|harness, _: &ToggleScaleUi, _, _| {
                    harness.scale_ui = !harness.scale_ui
                }),
            )
            .children(self.plot.clone())
            .child(div().id("other").h(px(40.)).track_focus(&self.other))
            .when_some(self.cover, |harness, cover| {
                harness.child(
                    div()
                        .absolute()
                        .left(cover.origin.x)
                        .top(cover.origin.y)
                        .w(cover.size.width)
                        .h(cover.size.height)
                        .occlude(),
                )
            })
    }
}

/// Take a reported selection into the plot's snapshot, as the shell's next frame would.
fn follow(plot: &mut PlotView, selection: argand_core::Selection) {
    if let Some(snapshot) = plot.snapshot.as_mut() {
        snapshot.selected = selection;
    }
}

fn snapshot() -> PlotSnapshot {
    let view = crate::navigation::View {
        start: 250_000,
        len: 500_000,
    };
    let ink = gpui_kit::rgb(0x808080);
    PlotSnapshot {
        extents: axes::Extents {
            frequency_offset: 0.,
            orientation: crate::navigation::orientation::Mode::Horizontal,
            time: crate::navigation::time_ruler::Ruler {
                mode: crate::navigation::time_ruler::Mode::Seconds,
                view,
                total: 1_000_000,
            },
            seconds: (0.25, 0.75),
            hertz: (-500_000., 500_000.),
        },
        texture: None,
        deep: None,
        backdrop: None,
        held: None,
        first_picture: None,
        minimap: waveform::Panel {
            waveform: None,
            viewport: Some((view, 1_000_000)),
            selection: None,
            selection_fill: gpui_kit::black(),
            separator: gpui_kit::black(),
            paper: gpui_kit::black(),
            ink: waveform::Ink {
                active: ink,
                muted: ink,
            },
        },
        minimap_size: crate::panels::MinimapSize::default(),
        time_scheme: None,
        frequency_scheme: None,
        frequency: crate::navigation::frequency::View::default(),
        show_grid: true,
        tooltips_allowed: true,
        show_scale_ui: false,
        pointer_in_window: true,
        selection: None,
        selected: argand_core::Selection::default(),
        capture_band: None,
        badges: None,
        entry: None,
        notice: None,
        edit: Default::default(),
    }
}

fn open(cx: &mut TestAppContext) -> WindowHandle<Harness> {
    cx.update(|cx| {
        gpui_kit::init(cx);
        crate::shell::plot::navigation_ui::init(cx);
    });
    let handle = cx.add_window(|window, cx| {
        let plot = cx.new(|cx| {
            let mut plot = PlotView::new(window, cx);
            plot.snapshot = Some(snapshot());
            plot
        });
        let intents = cx.subscribe(&plot, |harness: &mut Harness, plot, intent, cx| {
            harness.intents.push(*intent);
            let PlotIntent::Select(selection) = intent else {
                return;
            };
            let selection = selection.unwrap_or_default();
            if harness.follow_selection {
                plot.update(cx, |plot, _| follow(plot, selection));
            }
        });
        let focus = plot.read(cx).focus.clone();
        window.focus(&focus, cx);
        Harness {
            plot: Some(plot),
            other: cx.focus_handle(),
            intents: Vec::new(),
            grid: 0,
            viewport: None,
            scale_ui: false,
            cover: None,
            follow_selection: false,
            _intents: Some(intents),
        }
    });
    frame(cx, handle);
    frame(cx, handle);
    handle
}

fn frame(cx: &mut TestAppContext, handle: WindowHandle<Harness>) {
    cx.update_window(handle.into(), |_, window, cx| window.render_frame(cx))
        .unwrap();
    cx.run_until_parked();
}

fn press(cx: &mut TestAppContext, handle: WindowHandle<Harness>, key: &str) {
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
        window.press(key, cx);
    })
    .unwrap();
    cx.run_until_parked();
}

/// The navigation intents received since the last call.
fn navigation(cx: &mut TestAppContext, handle: WindowHandle<Harness>) -> Vec<PlotIntent> {
    handle
        .update(cx, |harness, _, _| {
            std::mem::take(&mut harness.intents)
                .into_iter()
                .filter(|intent| matches!(intent, PlotIntent::Time(_) | PlotIntent::Frequency(_)))
                .collect()
        })
        .unwrap()
}

#[gpui_kit::test]
fn every_navigation_key_emits_exactly_one_intent(cx: &mut TestAppContext) {
    let handle = open(cx);
    navigation(cx, handle);
    let zoom_in = TimeIntent::Zoom {
        factor: 0.5,
        anchor: 0.5,
    };
    let frequency_in = FrequencyIntent::Zoom {
        factor: 0.5,
        anchor: 0.5,
    };
    for (key, expected) in [
        ("ctrl-=", PlotIntent::Time(zoom_in)),
        ("ctrl-+", PlotIntent::Time(zoom_in)),
        ("ctrl-add", PlotIntent::Time(zoom_in)),
        (
            "ctrl--",
            PlotIntent::Time(TimeIntent::Zoom {
                factor: 2.,
                anchor: 0.5,
            }),
        ),
        ("ctrl-0", PlotIntent::Time(TimeIntent::Fit)),
        ("left", PlotIntent::Time(TimeIntent::Ticks(-1))),
        ("right", PlotIntent::Time(TimeIntent::Ticks(1))),
        ("ctrl-left", PlotIntent::Time(TimeIntent::Ticks(-5))),
        ("ctrl-right", PlotIntent::Time(TimeIntent::Ticks(5))),
        ("home", PlotIntent::Time(TimeIntent::Pan(-1e20))),
        ("end", PlotIntent::Time(TimeIntent::Pan(1e20))),
        ("ctrl-shift-=", PlotIntent::Frequency(frequency_in)),
        ("ctrl-shift-add", PlotIntent::Frequency(frequency_in)),
        (
            "ctrl-shift--",
            PlotIntent::Frequency(FrequencyIntent::Zoom {
                factor: 2.,
                anchor: 0.5,
            }),
        ),
        ("ctrl-shift-0", PlotIntent::Frequency(FrequencyIntent::Fit)),
        ("up", PlotIntent::Frequency(FrequencyIntent::Ticks(1.))),
        ("down", PlotIntent::Frequency(FrequencyIntent::Ticks(-1.))),
        ("ctrl-up", PlotIntent::Frequency(FrequencyIntent::Ticks(5.))),
        (
            "ctrl-down",
            PlotIntent::Frequency(FrequencyIntent::Ticks(-5.)),
        ),
    ] {
        press(cx, handle, key);
        assert_eq!(navigation(cx, handle), vec![expected], "{key}");
    }
}

#[gpui_kit::test]
fn zooming_out_the_right_axis_releases_the_held_gutter(cx: &mut TestAppContext) {
    let handle = open(cx);
    let floor = |cx: &mut TestAppContext| {
        handle
            .update(cx, |harness, _, cx| {
                harness.plot.as_ref().unwrap().read(cx).gutter_floor
            })
            .unwrap()
    };
    let hold = |cx: &mut TestAppContext| {
        handle
            .update(cx, |harness, _, cx| {
                harness
                    .plot
                    .as_ref()
                    .unwrap()
                    .update(cx, |plot, _| plot.gutter_floor = 500.)
            })
            .unwrap()
    };
    hold(cx);
    for key in ["ctrl-0", "ctrl--", "ctrl-=", "ctrl-shift-=", "home", "up"] {
        press(cx, handle, key);
        assert_eq!(
            floor(cx),
            500.,
            "{key} keeps the frequency ruler from narrowing"
        );
    }
    press(cx, handle, "ctrl-shift--");
    assert!(
        floor(cx) < 500.,
        "zooming frequency out lets its ruler refit"
    );
    hold(cx);
    press(cx, handle, "ctrl-shift-0");
    assert!(floor(cx) < 500., "fitting frequency lets its ruler refit");
    hold(cx);
    handle
        .update(cx, |harness, _, cx| {
            harness
                .plot
                .as_ref()
                .unwrap()
                .update(cx, |plot, cx| plot.reorient(cx))
        })
        .unwrap();
    assert!(floor(cx) < 500., "a new orientation starts from its labels");
}

#[gpui_kit::test]
fn session_commands_bubble_from_the_focused_plot(cx: &mut TestAppContext) {
    let handle = open(cx);
    press(cx, handle, "ctrl-g");
    assert_eq!(handle.update(cx, |harness, _, _| harness.grid).unwrap(), 1);
}

#[gpui_kit::test]
fn plot_keys_need_the_plot_itself_focused(cx: &mut TestAppContext) {
    let handle = open(cx);
    handle
        .update(cx, |harness, window, cx| window.focus(&harness.other, cx))
        .unwrap();
    navigation(cx, handle);
    for key in ["ctrl-=", "ctrl-shift-=", "ctrl-0", "left", "up", "ctrl-g"] {
        press(cx, handle, key);
    }
    assert!(navigation(cx, handle).is_empty());
    assert_eq!(handle.update(cx, |harness, _, _| harness.grid).unwrap(), 0);
}

#[gpui_kit::test]
fn another_window_never_reaches_the_plot(cx: &mut TestAppContext) {
    let handle = open(cx);
    navigation(cx, handle);
    let other = cx.add_window(|window, cx| {
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        Harness {
            plot: None,
            other: focus,
            intents: Vec::new(),
            grid: 0,
            viewport: None,
            scale_ui: false,
            cover: None,
            follow_selection: false,
            _intents: None,
        }
    });
    for key in ["ctrl-=", "ctrl-shift-=", "ctrl-shift-0"] {
        press(cx, other, key);
    }
    assert!(navigation(cx, handle).is_empty());
}

#[gpui_kit::test]
fn a_replaced_plot_stops_intercepting(cx: &mut TestAppContext) {
    let handle = open(cx);
    let old = handle
        .update(cx, |harness, _, _| {
            harness.plot.as_ref().unwrap().downgrade()
        })
        .unwrap();
    handle
        .update(cx, |harness, window, cx| {
            harness.plot = None;
            harness._intents = None;
            window.focus(&harness.other, cx);
        })
        .unwrap();
    frame(cx, handle);
    assert!(old.upgrade().is_none(), "the replaced plot is released");
    navigation(cx, handle);
    press(cx, handle, "ctrl-=");
    assert!(navigation(cx, handle).is_empty());
}

#[gpui_kit::test]
fn retirement_clears_the_snapshot_first(cx: &mut TestAppContext) {
    let handle = open(cx);
    let image = spectrogram::texture(
        &argand_core::SpectrogramImage::new(2, 2),
        crate::navigation::orientation::Mode::Horizontal,
    )
    .unwrap();
    let plot = handle
        .read_with(cx, |harness, _| harness.plot.clone().unwrap())
        .unwrap();
    cx.update_window(handle.into(), |_, window, cx| {
        plot.update(cx, |plot, _| {
            plot.snapshot.as_mut().unwrap().texture = Some(image.clone())
        });
        retire(Some(&plot), std::iter::empty(), window, cx);
        assert!(
            plot.read(cx).snapshot.is_some(),
            "nothing retired leaves it"
        );
        retire(Some(&plot), [image.clone()], window, cx);
        assert!(plot.read(cx).snapshot.is_none());
    })
    .unwrap();
}

/// Pans the spectrum with the middle button from its centre to `to` and returns the last view it asked for.
fn drag_time(
    cx: &mut TestAppContext,
    handle: WindowHandle<Harness>,
    spectrum: Bounds<Pixels>,
    to: gpui_kit::Point<Pixels>,
) -> crate::navigation::View {
    let from = spectrum.center();
    handle
        .update(cx, |harness, _, _| harness.intents.clear())
        .unwrap();
    let mut input = gpui_kit::VisualTestContext::from_window(handle.into(), cx);
    let none = gpui_kit::Modifiers::default();
    input.simulate_mouse_move(from, None, none);
    input.simulate_mouse_down(from, MouseButton::Middle, none);
    input.simulate_mouse_move(from + (to - from) / 2., MouseButton::Middle, none);
    input.simulate_mouse_move(to, MouseButton::Middle, none);
    input.simulate_mouse_up(to, MouseButton::Middle, none);
    cx.run_until_parked();
    let dragging = handle
        .update(cx, |harness, _, cx| {
            harness.plot.as_ref().unwrap().read(cx).dragging()
        })
        .unwrap();
    assert!(!dragging, "release ends the drag");
    handle
        .update(cx, |harness, _, _| {
            harness
                .intents
                .iter()
                .filter_map(|intent| match intent {
                    PlotIntent::Drag {
                        time: Some(view), ..
                    } => Some(*view),
                    _ => None,
                })
                .next_back()
        })
        .unwrap()
        .expect("the drag moved the view")
}

fn spectrum(cx: &mut TestAppContext, handle: WindowHandle<Harness>) -> Bounds<Pixels> {
    handle
        .update(cx, |harness, _, cx| {
            let geometry = harness.plot.as_ref().unwrap().read(cx).geometry;
            geometry.unwrap().spectrum
        })
        .unwrap()
}

#[gpui_kit::test]
fn a_drag_keeps_tracking_beyond_the_plot(cx: &mut TestAppContext) {
    let handle = open(cx);
    let spectrum = spectrum(cx, handle);
    let bottom = handle
        .update(cx, |_, window, _| window.viewport_size().height)
        .unwrap();
    let from = spectrum.center();
    let to = point(from.x - px(200.), bottom - px(10.));
    assert!(!spectrum.contains(&to));
    let width = f32::from(spectrum.size.width) as f64;
    let expected = snapshot().extents.time.view.pan(200. / width, 1_000_000);
    assert_eq!(
        drag_time(cx, handle, spectrum, to),
        expected,
        "the step outside the plot was followed"
    );
}

fn cover(cx: &mut TestAppContext, handle: WindowHandle<Harness>, bounds: Bounds<Pixels>) {
    handle
        .update(cx, |harness, _, cx| {
            harness.cover = Some(bounds);
            cx.notify();
        })
        .unwrap();
    frame(cx, handle);
}

fn pointer(
    cx: &mut TestAppContext,
    handle: WindowHandle<Harness>,
) -> Option<gpui_kit::Point<Pixels>> {
    handle
        .update(cx, |harness, _, cx| {
            harness.plot.as_ref().unwrap().read(cx).pointer
        })
        .unwrap()
}

#[gpui_kit::test]
fn an_overlay_on_the_plot_hides_its_pointer_until_the_first_move_back(cx: &mut TestAppContext) {
    let handle = open(cx);
    let spectrum = spectrum(cx, handle);
    let covered = spectrum.center();
    let free = point(covered.x - px(100.), covered.y);
    cover(
        cx,
        handle,
        Bounds::new(covered - point(px(20.), px(20.)), size(px(40.), px(40.))),
    );
    let mut input = gpui_kit::VisualTestContext::from_window(handle.into(), cx);
    let none = gpui_kit::Modifiers::default();
    input.simulate_mouse_move(free, None, none);
    assert_eq!(pointer(cx, handle), Some(free));
    input.simulate_mouse_move(covered, None, none);
    assert_eq!(
        pointer(cx, handle),
        None,
        "no readout or guides under a hint"
    );
    input.simulate_mouse_move(free, None, none);
    assert_eq!(
        pointer(cx, handle),
        Some(free),
        "the first move back restores it"
    );
}

#[gpui_kit::test]
fn an_overlay_on_the_plot_takes_its_wheel_clicks_and_drags(cx: &mut TestAppContext) {
    let handle = open(cx);
    let spectrum = spectrum(cx, handle);
    let covered = spectrum.center();
    cover(
        cx,
        handle,
        Bounds::new(covered - point(px(40.), px(40.)), size(px(80.), px(80.))),
    );
    handle
        .update(cx, |harness, _, _| harness.intents.clear())
        .unwrap();
    let mut input = gpui_kit::VisualTestContext::from_window(handle.into(), cx);
    let none = gpui_kit::Modifiers::default();
    input.simulate_mouse_move(covered, None, none);
    input.simulate_event(gpui_kit::ScrollEvent {
        position: covered,
        delta: gpui_kit::ScrollDelta::Pixels(point(px(0.), px(40.))),
        ..Default::default()
    });
    input.simulate_mouse_down(covered, MouseButton::Left, none);
    input.simulate_mouse_move(covered + point(px(20.), px(0.)), MouseButton::Left, none);
    input.simulate_mouse_up(covered + point(px(20.), px(0.)), MouseButton::Left, none);
    let intents = handle
        .update(cx, |harness, _, _| std::mem::take(&mut harness.intents))
        .unwrap();
    assert!(
        intents.is_empty(),
        "nothing under the overlay reaches the plot: {intents:?}"
    );
    assert_eq!(pointer(cx, handle), None);
}

#[gpui_kit::test]
fn key_presses_keep_the_pointer_over_the_plot(cx: &mut TestAppContext) {
    let handle = open(cx);
    let resting = spectrum(cx, handle).center();
    let mut input = gpui_kit::VisualTestContext::from_window(handle.into(), cx);
    input.simulate_mouse_move(resting, None, gpui_kit::Modifiers::default());
    frame(cx, handle);
    for key in ["a", "left", "tab"] {
        press(cx, handle, key);
        frame(cx, handle);
        assert_eq!(pointer(cx, handle), Some(resting), "{key}");
    }
}

/// Rests the pointer on a covered plot, then takes the cover away without a move.
fn uncover_under(
    cx: &mut TestAppContext,
    handle: WindowHandle<Harness>,
    in_window: bool,
) -> gpui_kit::Point<Pixels> {
    let resting = spectrum(cx, handle).center();
    cover(
        cx,
        handle,
        Bounds::new(resting - point(px(40.), px(40.)), size(px(80.), px(80.))),
    );
    let mut input = gpui_kit::VisualTestContext::from_window(handle.into(), cx);
    input.simulate_mouse_move(resting, None, gpui_kit::Modifiers::default());
    frame(cx, handle);
    assert_eq!(pointer(cx, handle), None, "covered");
    handle
        .update(cx, |harness, _, cx| {
            harness.cover = None;
            let plot = harness.plot.clone().unwrap();
            plot.update(cx, |plot, _| {
                plot.snapshot.as_mut().unwrap().pointer_in_window = in_window
            });
            cx.notify();
        })
        .unwrap();
    frame(cx, handle);
    frame(cx, handle);
    resting
}

#[gpui_kit::test]
fn a_plot_uncovered_under_a_still_pointer_takes_it_up(cx: &mut TestAppContext) {
    let handle = open(cx);
    let resting = uncover_under(cx, handle, true);
    assert_eq!(pointer(cx, handle), Some(resting));
}

#[gpui_kit::test]
fn a_pointer_that_left_the_window_is_not_taken_up(cx: &mut TestAppContext) {
    let handle = open(cx);
    uncover_under(cx, handle, false);
    assert_eq!(pointer(cx, handle), None);
}

#[gpui_kit::test]
fn a_new_orientation_keeps_a_resting_pointer(cx: &mut TestAppContext) {
    let handle = open(cx);
    let resting = spectrum(cx, handle).center();
    let mut input = gpui_kit::VisualTestContext::from_window(handle.into(), cx);
    input.simulate_mouse_move(resting, None, gpui_kit::Modifiers::default());
    frame(cx, handle);
    handle
        .update(cx, |harness, _, cx| {
            let plot = harness.plot.clone().unwrap();
            plot.update(cx, |plot, cx| {
                plot.snapshot.as_mut().unwrap().extents.orientation =
                    crate::navigation::orientation::Mode::Vertical;
                plot.reorient(cx);
                assert!(plot.hover().is_some(), "no frame without a readout");
            });
        })
        .unwrap();
    frame(cx, handle);
    frame(cx, handle);
    assert_eq!(pointer(cx, handle), Some(resting));
}

/// Presses the middle button in the spectrum's centre and pans a little, returning the drag's input.
fn start_drag(
    cx: &mut TestAppContext,
    handle: WindowHandle<Harness>,
) -> (gpui_kit::VisualTestContext, gpui_kit::Point<Pixels>) {
    let from = spectrum(cx, handle).center();
    let mut input = gpui_kit::VisualTestContext::from_window(handle.into(), cx);
    let none = gpui_kit::Modifiers::default();
    input.simulate_mouse_move(from, None, none);
    input.simulate_mouse_down(from, MouseButton::Middle, none);
    input.simulate_mouse_move(from - point(px(10.), px(0.)), MouseButton::Middle, none);
    let dragging = handle
        .update(cx, |harness, _, cx| {
            harness.plot.as_ref().unwrap().read(cx).dragging()
        })
        .unwrap();
    assert!(dragging, "the press began a drag");
    (input, from)
}

fn drag_steps(cx: &mut TestAppContext, handle: WindowHandle<Harness>) -> usize {
    handle
        .update(cx, |harness, _, _| {
            std::mem::take(&mut harness.intents)
                .into_iter()
                .filter(|intent| matches!(intent, PlotIntent::Drag { .. }))
                .count()
        })
        .unwrap()
}

#[gpui_kit::test]
fn an_interrupted_drag_does_not_resume(cx: &mut TestAppContext) {
    let handle = open(cx);
    let (mut input, from) = start_drag(cx, handle);
    handle
        .update(cx, |harness, _, cx| {
            harness
                .plot
                .as_ref()
                .unwrap()
                .update(cx, |plot, cx| plot.interrupt(cx))
        })
        .unwrap();
    assert_eq!(pointer(cx, handle), None, "the readout goes with the drag");
    drag_steps(cx, handle);
    let none = gpui_kit::Modifiers::default();
    input.simulate_mouse_move(from - point(px(60.), px(0.)), MouseButton::Middle, none);
    input.simulate_mouse_up(from - point(px(60.), px(0.)), MouseButton::Middle, none);
    assert_eq!(drag_steps(cx, handle), 0, "later moves do not resume it");
}

#[gpui_kit::test]
fn a_drag_keeps_tracking_over_a_hint_on_the_plot(cx: &mut TestAppContext) {
    let handle = open(cx);
    let spectrum = spectrum(cx, handle);
    let from = spectrum.center();
    let to = point(from.x - px(100.), from.y);
    cover(
        cx,
        handle,
        Bounds::new(to - point(px(20.), px(20.)), size(px(40.), px(40.))),
    );
    let width = f32::from(spectrum.size.width) as f64;
    let expected = snapshot().extents.time.view.pan(100. / width, 1_000_000);
    assert_eq!(
        drag_time(cx, handle, spectrum, to),
        expected,
        "the step over the hint was followed"
    );
}

/// The selections reported since the last call.
fn selections(
    cx: &mut TestAppContext,
    handle: WindowHandle<Harness>,
) -> Vec<Option<argand_core::Selection>> {
    handle
        .update(cx, |harness, _, _| {
            std::mem::take(&mut harness.intents)
                .into_iter()
                .filter_map(|intent| match intent {
                    PlotIntent::Select(selection) => Some(selection),
                    _ => None,
                })
                .collect()
        })
        .unwrap()
}

/// Drags with the left button through a midpoint, as a hand does.
fn left_drag(
    cx: &mut TestAppContext,
    handle: WindowHandle<Harness>,
    from: gpui_kit::Point<Pixels>,
    to: gpui_kit::Point<Pixels>,
    modifiers: gpui_kit::Modifiers,
) {
    handle
        .update(cx, |harness, _, _| harness.intents.clear())
        .unwrap();
    let mut input = gpui_kit::VisualTestContext::from_window(handle.into(), cx);
    input.simulate_mouse_move(from, None, modifiers);
    input.simulate_mouse_down(from, MouseButton::Left, modifiers);
    input.simulate_mouse_move(from + (to - from) / 2., MouseButton::Left, modifiers);
    input.simulate_mouse_move(to, MouseButton::Left, modifiers);
    input.simulate_mouse_up(to, MouseButton::Left, modifiers);
    cx.run_until_parked();
}

/// The sample boundary under `x` in the test snapshot's horizontal view.
fn boundary_at(spectrum: Bounds<Pixels>, x: Pixels) -> u64 {
    let fraction = f32::from(x - spectrum.left()) as f64 / f32::from(spectrum.size.width) as f64;
    snapshot().extents.time.view.boundary(fraction)
}

fn span(a: u64, b: u64) -> Option<argand_core::Selection> {
    argand_core::SampleSpan::between(a, b).map(argand_core::Selection::time)
}

/// The frequency at a fraction down from the top of the visible band.
fn hertz_at(fraction: f64) -> f64 {
    let (low, high) = snapshot().extents.hertz;
    high - fraction * (high - low)
}

fn band(a: f64, b: f64) -> Option<argand_core::FrequencyBand> {
    argand_core::FrequencyBand::between(a, b)
}

fn space(cx: &mut TestAppContext, handle: WindowHandle<Harness>, down: bool) {
    let keystroke = gpui_kit::Keystroke::parse("space").unwrap();
    let mut input = gpui_kit::VisualTestContext::from_window(handle.into(), cx);
    if down {
        input.simulate_event(gpui_kit::KeyDownEvent {
            keystroke,
            is_held: false,
            prefer_character_input: false,
        });
    } else {
        input.simulate_event(gpui_kit::KeyUpEvent { keystroke });
    }
    cx.run_until_parked();
}

fn space_held(cx: &mut TestAppContext, handle: WindowHandle<Harness>) -> bool {
    handle
        .update(cx, |harness, _, cx| {
            harness.plot.as_ref().unwrap().read(cx).space
        })
        .unwrap()
}

#[gpui_kit::test]
fn a_left_drag_on_the_spectrum_selects_time_and_does_not_pan(cx: &mut TestAppContext) {
    let handle = open(cx);
    let spectrum = spectrum(cx, handle);
    let from = spectrum.center();
    let to = from - point(px(100.), px(0.));
    left_drag(cx, handle, from, to, gpui_kit::Modifiers::default());
    let intents = handle
        .update(cx, |harness, _, _| harness.intents.clone())
        .unwrap();
    assert!(
        !intents
            .iter()
            .any(|intent| matches!(intent, PlotIntent::Drag { .. })),
        "a selection is not a pan: {intents:?}"
    );
    let reported = selections(cx, handle);
    assert_eq!(
        reported.last().copied().flatten(),
        span(boundary_at(spectrum, to.x), boundary_at(spectrum, from.x)),
        "the span runs from the press to the pointer, whichever way it went"
    );
}

#[gpui_kit::test]
fn a_selection_dragged_beyond_the_plot_stops_at_the_visible_edge(cx: &mut TestAppContext) {
    let handle = open(cx);
    let spectrum = spectrum(cx, handle);
    let from = spectrum.center();
    let beyond = point(spectrum.left() - px(60.), from.y + px(400.));
    left_drag(cx, handle, from, beyond, gpui_kit::Modifiers::default());
    let view = snapshot().extents.time.view;
    assert_eq!(
        selections(cx, handle).last().copied().flatten(),
        span(view.start, boundary_at(spectrum, from.x))
    );
}

#[gpui_kit::test]
fn a_click_and_escape_clear_the_selection(cx: &mut TestAppContext) {
    let handle = open(cx);
    let at = spectrum(cx, handle).center();
    left_drag(
        cx,
        handle,
        at,
        at + point(px(1.), px(1.)),
        gpui_kit::Modifiers::default(),
    );
    assert_eq!(
        selections(cx, handle),
        [None],
        "a press that barely moved is a click"
    );
    press(cx, handle, "escape");
    assert_eq!(selections(cx, handle), [None]);
}

#[gpui_kit::test]
fn the_release_settles_the_far_end_of_a_selection(cx: &mut TestAppContext) {
    let handle = open(cx);
    let spectrum = spectrum(cx, handle);
    let from = spectrum.center();
    let release = from + point(px(120.), px(0.));
    let mut input = gpui_kit::VisualTestContext::from_window(handle.into(), cx);
    let none = gpui_kit::Modifiers::default();
    input.simulate_mouse_move(from, None, none);
    input.simulate_mouse_down(from, MouseButton::Left, none);
    input.simulate_mouse_move(from + point(px(1.), px(0.)), MouseButton::Left, none);
    input.simulate_mouse_up(release, MouseButton::Left, none);
    cx.run_until_parked();
    assert_eq!(
        selections(cx, handle).last().copied().flatten(),
        span(
            boundary_at(spectrum, from.x),
            boundary_at(spectrum, release.x)
        ),
        "a release far from the last move is a drag to where it happened"
    );
}

#[gpui_kit::test]
fn escape_during_a_drag_ends_the_selection_gesture(cx: &mut TestAppContext) {
    let handle = open(cx);
    let from = spectrum(cx, handle).center();
    let mut input = gpui_kit::VisualTestContext::from_window(handle.into(), cx);
    let none = gpui_kit::Modifiers::default();
    input.simulate_mouse_move(from, None, none);
    input.simulate_mouse_down(from, MouseButton::Left, none);
    input.simulate_mouse_move(from + point(px(40.), px(0.)), MouseButton::Left, none);
    press(cx, handle, "escape");
    selections(cx, handle);
    let mut input = gpui_kit::VisualTestContext::from_window(handle.into(), cx);
    input.simulate_mouse_move(from + point(px(80.), px(0.)), MouseButton::Left, none);
    input.simulate_mouse_up(from + point(px(80.), px(0.)), MouseButton::Left, none);
    cx.run_until_parked();
    assert!(
        selections(cx, handle).is_empty(),
        "the moves after Escape do not bring it back"
    );
}

#[gpui_kit::test]
fn ctrl_drag_on_the_spectrum_selects_a_rectangle(cx: &mut TestAppContext) {
    let handle = open(cx);
    let spectrum = spectrum(cx, handle);
    let from = spectrum.center();
    let to = from - point(px(80.), px(30.));
    let ctrl = gpui_kit::Modifiers {
        control: true,
        ..Default::default()
    };
    left_drag(cx, handle, from, to, ctrl);
    let down =
        |y: Pixels| f32::from(y - spectrum.top()) as f64 / f32::from(spectrum.size.height) as f64;
    assert_eq!(
        selections(cx, handle).last().copied().flatten(),
        Some(argand_core::Selection {
            time: argand_core::SampleSpan::between(
                boundary_at(spectrum, to.x),
                boundary_at(spectrum, from.x)
            ),
            band: band(hertz_at(down(from.y)), hertz_at(down(to.y))),
        })
    );
    assert_eq!(drag_steps(cx, handle), 0, "a rectangle is not a pan");
}

#[gpui_kit::test]
fn a_rectangle_without_height_selects_nothing(cx: &mut TestAppContext) {
    let handle = open(cx);
    let from = spectrum(cx, handle).center();
    let ctrl = gpui_kit::Modifiers {
        control: true,
        ..Default::default()
    };
    left_drag(cx, handle, from, from - point(px(80.), px(0.)), ctrl);
    assert_eq!(selections(cx, handle).last().copied().flatten(), None);
}

/// The frequency ruler as the last frame laid it out.
fn frequency_ruler(cx: &mut TestAppContext, handle: WindowHandle<Harness>) -> Bounds<Pixels> {
    handle
        .update(cx, |harness, _, cx| {
            let geometry = harness.plot.as_ref().unwrap().read(cx).geometry;
            geometry.unwrap().frequency_ruler
        })
        .unwrap()
}

#[gpui_kit::test]
fn a_left_drag_on_the_frequency_ruler_selects_a_band(cx: &mut TestAppContext) {
    let handle = open(cx);
    let spectrum = spectrum(cx, handle);
    let ruler = frequency_ruler(cx, handle);
    let from = ruler.center();
    let to = from + point(px(0.), px(40.));
    left_drag(cx, handle, from, to, gpui_kit::Modifiers::default());
    let down =
        |y: Pixels| f32::from(y - spectrum.top()) as f64 / f32::from(spectrum.size.height) as f64;
    assert_eq!(
        selections(cx, handle).last().copied().flatten(),
        Some(argand_core::Selection {
            time: None,
            band: band(hertz_at(down(from.y)), hertz_at(down(to.y))),
        })
    );
    assert_eq!(
        drag_steps(cx, handle),
        0,
        "the ruler selects rather than pans"
    );
}

#[gpui_kit::test]
fn a_middle_drag_on_the_frequency_ruler_still_pans(cx: &mut TestAppContext) {
    let handle = open(cx);
    handle
        .update(cx, |harness, _, cx| {
            let plot = harness.plot.clone().unwrap();
            plot.update(cx, |plot, _| {
                plot.snapshot.as_mut().unwrap().frequency = crate::navigation::frequency::View {
                    start: 0.25,
                    span: 0.5,
                };
            });
        })
        .unwrap();
    frame(cx, handle);
    let from = frequency_ruler(cx, handle).center();
    let mut input = gpui_kit::VisualTestContext::from_window(handle.into(), cx);
    let none = gpui_kit::Modifiers::default();
    input.simulate_mouse_move(from, None, none);
    input.simulate_mouse_down(from, MouseButton::Middle, none);
    input.simulate_mouse_move(from + point(px(0.), px(30.)), MouseButton::Middle, none);
    input.simulate_mouse_up(from + point(px(0.), px(30.)), MouseButton::Middle, none);
    cx.run_until_parked();
    assert!(drag_steps(cx, handle) > 0);
    assert!(selections(cx, handle).is_empty());
}

#[gpui_kit::test]
fn a_vertical_plot_selects_a_band_along_its_frequency_axis(cx: &mut TestAppContext) {
    let handle = open(cx);
    handle
        .update(cx, |harness, _, cx| {
            let plot = harness.plot.clone().unwrap();
            plot.update(cx, |plot, cx| {
                plot.snapshot.as_mut().unwrap().extents.orientation =
                    crate::navigation::orientation::Mode::Vertical;
                plot.reorient(cx);
            });
        })
        .unwrap();
    frame(cx, handle);
    frame(cx, handle);
    let spectrum = spectrum(cx, handle);
    let from = frequency_ruler(cx, handle).center();
    let to = from + point(px(50.), px(0.));
    left_drag(cx, handle, from, to, gpui_kit::Modifiers::default());
    // Frequency runs to the right, so the right edge is the top of the band.
    let down = |x: Pixels| {
        1. - f32::from(x - spectrum.left()) as f64 / f32::from(spectrum.size.width) as f64
    };
    assert_eq!(
        selections(cx, handle).last().copied().flatten(),
        Some(argand_core::Selection {
            time: None,
            band: band(hertz_at(down(from.x)), hertz_at(down(to.x))),
        })
    );
}

#[gpui_kit::test]
fn space_and_a_left_drag_on_the_frequency_ruler_pan(cx: &mut TestAppContext) {
    let handle = open(cx);
    handle
        .update(cx, |harness, _, cx| {
            let plot = harness.plot.clone().unwrap();
            plot.update(cx, |plot, _| {
                plot.snapshot.as_mut().unwrap().frequency = crate::navigation::frequency::View {
                    start: 0.25,
                    span: 0.5,
                };
            });
        })
        .unwrap();
    frame(cx, handle);
    let from = frequency_ruler(cx, handle).center();
    space(cx, handle, true);
    left_drag(
        cx,
        handle,
        from,
        from + point(px(0.), px(30.)),
        gpui_kit::Modifiers::default(),
    );
    assert!(drag_steps(cx, handle) > 0);
    assert!(selections(cx, handle).is_empty());
}

#[gpui_kit::test]
fn a_vertical_plot_selects_a_rectangle_along_both_axes(cx: &mut TestAppContext) {
    let handle = open(cx);
    handle
        .update(cx, |harness, _, cx| {
            let plot = harness.plot.clone().unwrap();
            plot.update(cx, |plot, cx| {
                plot.snapshot.as_mut().unwrap().extents.orientation =
                    crate::navigation::orientation::Mode::Vertical;
                plot.reorient(cx);
            });
        })
        .unwrap();
    frame(cx, handle);
    frame(cx, handle);
    let spectrum = spectrum(cx, handle);
    let from = spectrum.center();
    let to = from + point(px(40.), px(80.));
    let ctrl = gpui_kit::Modifiers {
        control: true,
        ..Default::default()
    };
    left_drag(cx, handle, from, to, ctrl);
    let along = |y: Pixels| {
        let fraction =
            f32::from(y - spectrum.top()) as f64 / f32::from(spectrum.size.height) as f64;
        snapshot().extents.time.view.boundary(fraction)
    };
    let down = |x: Pixels| {
        1. - f32::from(x - spectrum.left()) as f64 / f32::from(spectrum.size.width) as f64
    };
    assert_eq!(
        selections(cx, handle).last().copied().flatten(),
        Some(argand_core::Selection {
            time: argand_core::SampleSpan::between(along(from.y), along(to.y)),
            band: band(hertz_at(down(from.x)), hertz_at(down(to.x))),
        })
    );
}

#[gpui_kit::test]
fn space_turns_a_left_drag_into_a_pan_until_it_is_released(cx: &mut TestAppContext) {
    let handle = open(cx);
    let spectrum = spectrum(cx, handle);
    let from = spectrum.center();
    space(cx, handle, true);
    assert!(space_held(cx, handle));
    left_drag(
        cx,
        handle,
        from,
        from - point(px(100.), px(0.)),
        gpui_kit::Modifiers::default(),
    );
    assert!(drag_steps(cx, handle) > 0, "Space and a left drag pan");
    space(cx, handle, false);
    assert!(!space_held(cx, handle));
    left_drag(
        cx,
        handle,
        from,
        from - point(px(100.), px(0.)),
        gpui_kit::Modifiers::default(),
    );
    assert_eq!(drag_steps(cx, handle), 0, "without Space it selects again");
}

#[gpui_kit::test]
fn space_released_with_a_modifier_down_is_released(cx: &mut TestAppContext) {
    let handle = open(cx);
    space(cx, handle, true);
    let mut input = gpui_kit::VisualTestContext::from_window(handle.into(), cx);
    input.simulate_event(gpui_kit::KeyUpEvent {
        keystroke: gpui_kit::Keystroke::parse("shift-space").unwrap(),
    });
    cx.run_until_parked();
    assert!(!space_held(cx, handle));
}

#[gpui_kit::test]
fn space_is_forgotten_when_the_plot_loses_focus(cx: &mut TestAppContext) {
    let handle = open(cx);
    // GPUI reports focus changes only within an active window.
    cx.update_window(handle.into(), |_, window, _| window.activate_window())
        .unwrap();
    frame(cx, handle);
    space(cx, handle, true);
    handle
        .update(cx, |harness, window, cx| window.focus(&harness.other, cx))
        .unwrap();
    frame(cx, handle);
    assert!(!space_held(cx, handle), "its release would go elsewhere");
}

#[gpui_kit::test]
fn a_left_drag_on_the_time_ruler_still_pans(cx: &mut TestAppContext) {
    let handle = open(cx);
    let ruler = handle
        .update(cx, |harness, _, cx| {
            harness
                .plot
                .as_ref()
                .unwrap()
                .read(cx)
                .geometry
                .unwrap()
                .time_ruler
        })
        .unwrap();
    let from = ruler.center();
    left_drag(
        cx,
        handle,
        from,
        from - point(px(60.), px(0.)),
        gpui_kit::Modifiers::default(),
    );
    assert!(drag_steps(cx, handle) > 0);
}

#[gpui_kit::test]
fn a_vertical_plot_selects_along_its_time_axis(cx: &mut TestAppContext) {
    let handle = open(cx);
    handle
        .update(cx, |harness, _, cx| {
            let plot = harness.plot.clone().unwrap();
            plot.update(cx, |plot, cx| {
                plot.snapshot.as_mut().unwrap().extents.orientation =
                    crate::navigation::orientation::Mode::Vertical;
                plot.reorient(cx);
            });
        })
        .unwrap();
    frame(cx, handle);
    frame(cx, handle);
    let spectrum = spectrum(cx, handle);
    let from = spectrum.center();
    let to = from + point(px(40.), px(80.));
    left_drag(cx, handle, from, to, gpui_kit::Modifiers::default());
    let along = |y: Pixels| {
        let fraction =
            f32::from(y - spectrum.top()) as f64 / f32::from(spectrum.size.height) as f64;
        snapshot().extents.time.view.boundary(fraction)
    };
    assert_eq!(
        selections(cx, handle).last().copied().flatten(),
        span(along(from.y), along(to.y)),
        "time runs down, so only the vertical movement counts"
    );
}

/// Whether the corner pairs are in the tree the last frame drew.
fn pairs_shown(cx: &mut TestAppContext, handle: WindowHandle<Harness>) -> bool {
    cx.update_window(handle.into(), |_, window, _| {
        window.try_find("Zoom in time").is_some()
    })
    .unwrap()
}

/// Flips the scale controls, as the shell does, and carries the choice over.
fn toggle_pairs(cx: &mut TestAppContext, handle: WindowHandle<Harness>) {
    press(cx, handle, "ctrl-u");
    handle
        .update(cx, |harness, _, cx| {
            let shown = harness.scale_ui;
            let plot = harness.plot.clone().unwrap();
            plot.update(cx, |plot, cx| {
                plot.snapshot.as_mut().unwrap().show_scale_ui = shown;
                cx.notify();
            });
        })
        .unwrap();
    frame(cx, handle);
    frame(cx, handle);
    let shown = handle.update(cx, |harness, _, _| harness.scale_ui).unwrap();
    assert_eq!(
        pairs_shown(cx, handle),
        shown,
        "the scale controls show and hide the pairs"
    );
}

/// The corner a pair occupies, in window coordinates.
fn zone(cx: &mut TestAppContext, handle: WindowHandle<Harness>, index: usize) -> Bounds<Pixels> {
    handle
        .update(cx, |harness, _, cx| {
            let geometry = harness.plot.as_ref().unwrap().read(cx).geometry;
            let zone = geometry.expect("the plot is measured").zoom_zones[index]
                .expect("the pair is shown");
            Bounds::new(
                point(px(zone.x), px(zone.y)),
                size(px(zone.width), px(zone.height)),
            )
        })
        .unwrap()
}

/// The centre of a zoom half, which is where a click of its own lands.
fn half(
    cx: &mut TestAppContext,
    handle: WindowHandle<Harness>,
    index: usize,
) -> gpui_kit::Point<Pixels> {
    let zone = zone(cx, handle, index);
    zone.center()
}

/// The two halves of a pair, with the frame they share.
fn halves(
    cx: &mut TestAppContext,
    handle: WindowHandle<Harness>,
    index: usize,
) -> (Bounds<Pixels>, Bounds<Pixels>, Bounds<Pixels>) {
    let (zoom_in, zoom_out) = if index == 0 {
        ("Zoom in time", "Zoom out time")
    } else {
        ("Zoom in frequency", "Zoom out frequency")
    };
    let read = |cx: &mut TestAppContext, id: &'static str| {
        cx.update_window(handle.into(), |_, window, _| window.find(id).bounds())
            .unwrap()
    };
    (
        read(cx, zoom_in),
        read(cx, zoom_out),
        zone(cx, handle, index),
    )
}

#[gpui_kit::test]
fn zoom_buttons_follow_the_painted_plot_in_the_first_resize_frame(cx: &mut TestAppContext) {
    let handle = open(cx);
    for orientation in [
        crate::navigation::orientation::Mode::Horizontal,
        crate::navigation::orientation::Mode::Vertical,
    ] {
        handle
            .update(cx, |harness, _, cx| {
                harness.plot.as_ref().unwrap().update(cx, |plot, cx| {
                    let snapshot = plot.snapshot.as_mut().unwrap();
                    snapshot.extents.orientation = orientation;
                    snapshot.show_scale_ui = true;
                    snapshot.selection = Some(waveform::Selected {
                        time: (0., 1.),
                        frequency: (0., 1.),
                    });
                    plot.reorient(cx);
                });
            })
            .unwrap();
        frame(cx, handle);
        for (width, height) in [(900., 620.), (1200., 620.), (1200., 880.), (650., 480.)] {
            cx.update_window(handle.into(), |_, window, cx| {
                    window.root::<Harness>().unwrap().unwrap().update(cx, |harness, cx| {
                        harness.viewport = Some(size(px(width), px(height)));
                        cx.notify();
                    });
                    window.render_frame(cx);
                    let tint: gpui_kit::Background = cx.theme().blue_light.opacity(0.3).into();
                    let painted = window.painted_quads().into_iter()
                        .find(|quad| quad.background == tint).expect("full selection paints the actual plot");
                    let scale = window.scale_factor();
                    let plot = Bounds::new(
                        point(px(painted.bounds.origin.x.as_f32() / scale), px(painted.bounds.origin.y.as_f32() / scale)),
                        size(px(painted.bounds.size.width.as_f32() / scale), px(painted.bounds.size.height.as_f32() / scale)),
                    );
                    for (time, horizontal) in [(true, !orientation.vertical()), (false, orientation.vertical())] {
                        let (plus, minus) = if time { ("Zoom in time", "Zoom out time") } else { ("Zoom in frequency", "Zoom out frequency") };
                        let halves = window.find(plus).bounds().union(&window.find(minus).bounds());
                        let expected = if horizontal {
                            Bounds::new(point(plot.left() + px(9.), plot.bottom() - px(29.)), size(px(43.), px(20.)))
                        } else {
                            Bounds::new(point(plot.right() - px(29.), plot.top() + px(9.)), size(px(20.), px(43.)))
                        };
                        assert_eq!(halves, expected, "{orientation:?} {width}x{height}: first frame, before deferred layout convergence");
                    }
                }).unwrap();
        }
    }
}

#[gpui_kit::test]
fn zoom_buttons_keep_paint_and_click_ownership_over_the_paste_marker(cx: &mut TestAppContext) {
    let handle = open(cx);
    toggle_pairs(cx, handle);
    let button = halves(cx, handle, 0).0;
    let at = boundary_at(spectrum(cx, handle), button.center().x);
    let _menu = handle
        .update(cx, |harness, window, cx| {
            let menu = PopupMenu::build(window, cx, |menu, _, _| menu);
            harness.plot.as_ref().unwrap().update(cx, |plot, cx| {
                plot.open_menu = Some(menu.downgrade());
                plot.paste_point = Some(at);
                cx.notify();
            });
            menu
        })
        .unwrap();
    frame(cx, handle);
    cx.update_window(handle.into(), |_, window, cx| {
        let scale = window.scale_factor();
        let centre = button
            .center()
            .map(|value| gpui_kit::ScaledPixels(f32::from(value) * scale));
        let quads = window.painted_quads();
        let marker_ink: gpui_kit::Background = cx.theme().foreground.into();
        let paper: gpui_kit::Background = cx.theme().background.opacity(0.55).into();
        let marker = quads
            .iter()
            .find(|quad| quad.background == marker_ink && quad.bounds.contains(&centre))
            .expect("the paste marker crosses the zoom button");
        let control = quads
            .iter()
            .find(|quad| quad.background == paper && quad.bounds.contains(&centre))
            .expect("the standard pair paints over the marker");
        assert!(
            marker.order < control.order,
            "the marker stays below the buttons"
        );
        window.click("Zoom in time", cx);
    })
    .unwrap();
    assert_eq!(
        navigation(cx, handle),
        vec![PlotIntent::Time(TimeIntent::Zoom {
            factor: 0.5,
            anchor: 0.5
        })]
    );
}

/// Whether the plot holds the keyboard, which its controls hand back to it.
fn plot_focused(cx: &mut TestAppContext, handle: WindowHandle<Harness>) -> bool {
    handle
        .update(cx, |harness, window, cx| {
            harness
                .plot
                .as_ref()
                .unwrap()
                .read(cx)
                .focus
                .is_focused(window)
        })
        .unwrap()
}

#[gpui_kit::test]
fn a_zoom_half_zooms_once_and_leaves_the_keyboard_on_the_plot(cx: &mut TestAppContext) {
    let handle = open(cx);
    toggle_pairs(cx, handle);
    navigation(cx, handle);
    cx.update_window(handle.into(), |_, window, cx| {
        window.click("Zoom in time", cx)
    })
    .unwrap();
    assert_eq!(
        navigation(cx, handle),
        vec![PlotIntent::Time(TimeIntent::Zoom {
            factor: 0.5,
            anchor: 0.5,
        })],
        "one activation is one zoom"
    );
    assert!(
        plot_focused(cx, handle),
        "the click leaves focus on the plot"
    );
}

#[gpui_kit::test]
fn hiding_the_pairs_during_a_press_leaves_nothing_behind(cx: &mut TestAppContext) {
    let handle = open(cx);
    toggle_pairs(cx, handle);
    let inside = half(cx, handle, 1);
    let outside = inside - point(px(40.), px(0.));
    let none = gpui_kit::Modifiers::default();
    let mut input = gpui_kit::VisualTestContext::from_window(handle.into(), cx);
    input.simulate_mouse_move(inside, None, none);
    input.simulate_mouse_down(inside, MouseButton::Left, none);
    toggle_pairs(cx, handle);
    input.simulate_mouse_up(outside, MouseButton::Left, none);
    assert!(
        navigation(cx, handle).is_empty(),
        "a hidden half cannot be pressed"
    );
    toggle_pairs(cx, handle);
    cx.update_window(handle.into(), |_, window, cx| {
        window.click("Zoom in frequency", cx)
    })
    .unwrap();
    assert_eq!(
        navigation(cx, handle),
        vec![PlotIntent::Frequency(FrequencyIntent::Zoom {
            factor: 0.5,
            anchor: 0.5,
        })],
        "the pairs that return work"
    );
}

/// Show `selection` on the plot, as the shell does after a gesture.
fn show_selection(
    cx: &mut TestAppContext,
    handle: WindowHandle<Harness>,
    selection: argand_core::Selection,
) {
    handle
        .update(cx, |harness, _, cx| {
            harness.plot.as_ref().unwrap().update(cx, |plot, cx| {
                plot.snapshot.as_mut().unwrap().selected = selection;
                cx.notify();
            });
        })
        .unwrap();
    cx.run_until_parked();
}

/// Show a time span with its badges and the scale they are typed in.
fn show_badges(cx: &mut TestAppContext, handle: WindowHandle<Harness>, start: u64, end: u64) {
    let selected = span(start, end).unwrap();
    let scale = crate::navigation::selection_entry::Scale {
        frequency_offset: 0.,
        mode: crate::navigation::time_ruler::Mode::Seconds,
        rate: 1e6,
        total: 1_000_000,
        seconds_per_pixel: 1e-3,
        unit: "kHz",
        divisor: 1e3,
        decimals: 3,
        band: (-500_000., 500_000.),
    };
    let view = snapshot().extents.time.view;
    let fraction = |sample: u64| (sample - view.start) as f64 / view.len as f64;
    handle
        .update(cx, |harness, _, cx| {
            harness.plot.as_ref().unwrap().update(cx, |plot, cx| {
                let snapshot = plot.snapshot.as_mut().unwrap();
                snapshot.selected = selected;
                snapshot.entry = Some(scale);
                snapshot.badges = Some(axes::SelectionBadges {
                    time: Some(axes::AxisBadges {
                        texts: scale.time_texts(selected.time.unwrap()),
                        fields: crate::navigation::selection_entry::Field::TIME,
                        first: Some(fraction(start)),
                        second: Some(fraction(end)),
                        visible: (fraction(start), fraction(end)),
                        around: (0., 1.),
                        alone: true,
                    }),
                    band: None,
                });
                cx.notify();
            });
        })
        .unwrap();
    frame(cx, handle);
    frame(cx, handle);
}

/// Where the badge of `field` was painted.
fn badge(
    cx: &mut TestAppContext,
    handle: WindowHandle<Harness>,
    field: crate::navigation::selection_entry::Field,
) -> Bounds<Pixels> {
    handle
        .update(cx, |harness, _, cx| {
            let plot = harness.plot.as_ref().unwrap().read(cx);
            plot.badges
                .iter()
                .find(|(shown, _)| *shown == field)
                .map(|(_, bounds)| *bounds)
        })
        .unwrap()
        .expect("the badge is painted")
}

fn click(
    cx: &mut TestAppContext,
    handle: WindowHandle<Harness>,
    at: gpui_kit::Point<Pixels>,
    count: usize,
) {
    handle
        .update(cx, |harness, _, _| harness.intents.clear())
        .unwrap();
    let mut input = gpui_kit::VisualTestContext::from_window(handle.into(), cx);
    input.simulate_mouse_move(at, None, gpui_kit::Modifiers::default());
    input.simulate_event(gpui_kit::MouseDownEvent {
        button: MouseButton::Left,
        position: at,
        modifiers: gpui_kit::Modifiers::default(),
        click_count: count,
        first_mouse: false,
    });
    input.simulate_event(gpui_kit::MouseUpEvent {
        button: MouseButton::Left,
        position: at,
        modifiers: gpui_kit::Modifiers::default(),
        click_count: count,
    });
    cx.run_until_parked();
}

fn typing(cx: &mut TestAppContext, handle: WindowHandle<Harness>) -> Option<(String, bool)> {
    handle
        .update(cx, |harness, _, cx| {
            let plot = harness.plot.as_ref().unwrap().read(cx);
            plot.typing
                .as_ref()
                .map(|entry| (entry.text(cx), entry.invalid(cx)))
        })
        .unwrap()
}

fn type_text(cx: &mut TestAppContext, handle: WindowHandle<Harness>, keys: &str) {
    let mut input = gpui_kit::VisualTestContext::from_window(handle.into(), cx);
    input.simulate_keystrokes(keys);
    cx.run_until_parked();
}

#[gpui_kit::test]
fn a_double_click_on_a_badge_types_its_value(cx: &mut TestAppContext) {
    use crate::navigation::selection_entry::Field;
    let handle = open(cx);
    cx.update(badge_entry::init);
    show_badges(cx, handle, 400_000, 600_000);
    let start = badge(cx, handle, Field::TimeStart);
    click(cx, handle, start.center(), 1);
    assert!(
        selections(cx, handle).is_empty(),
        "a click on a badge keeps the selection"
    );
    assert_eq!(typing(cx, handle), None);
    click(cx, handle, start.center(), 2);
    assert_eq!(
        typing(cx, handle),
        Some(("0.400 s".into(), false)),
        "the input holds what the badge showed"
    );
    type_text(cx, handle, "0 . 3 enter");
    assert_eq!(selections(cx, handle), [span(300_000, 600_000)]);
    assert_eq!(typing(cx, handle), None);
    assert!(plot_focused(cx, handle), "the plot takes the keyboard back");
}

#[gpui_kit::test]
fn a_badge_input_reads_its_text_in_the_units_it_was_written_in(cx: &mut TestAppContext) {
    use crate::navigation::selection_entry::Field;
    let handle = open(cx);
    cx.update(badge_entry::init);
    show_badges(cx, handle, 400_000, 600_000);
    let start = badge(cx, handle, Field::TimeStart);
    click(cx, handle, start.center(), 2);
    handle
        .update(cx, |harness, _, cx| {
            harness.plot.as_ref().unwrap().update(cx, |plot, _| {
                let entry = plot.snapshot.as_mut().unwrap().entry.as_mut().unwrap();
                entry.mode = crate::navigation::time_ruler::Mode::Samples;
            });
        })
        .unwrap();
    type_text(cx, handle, "0 . 3 enter");
    assert_eq!(selections(cx, handle), [span(300_000, 600_000)]);
}

#[gpui_kit::test]
fn leaving_the_window_closes_a_badge_input_and_the_plot_keeps_the_keyboard(
    cx: &mut TestAppContext,
) {
    use crate::navigation::selection_entry::Field;
    let handle = open(cx);
    cx.update(badge_entry::init);
    // An inactive test window reports no focus path, so the input would never blur.
    cx.update_window(handle.into(), |_, window, _| window.activate_window())
        .unwrap();
    show_badges(cx, handle, 400_000, 600_000);
    let start = badge(cx, handle, Field::TimeStart);
    click(cx, handle, start.center(), 2);
    frame(cx, handle);
    assert!(typing(cx, handle).is_some());
    let mut input = gpui_kit::VisualTestContext::from_window(handle.into(), cx);
    input.deactivate_window();
    frame(cx, handle);
    cx.update_window(handle.into(), |_, window, _| window.activate_window())
        .unwrap();
    frame(cx, handle);
    assert_eq!(typing(cx, handle), None);
    assert!(plot_focused(cx, handle), "the plot takes the keyboard back");
}

#[gpui_kit::test]
fn up_and_down_step_a_badge_input_and_keep_it_open(cx: &mut TestAppContext) {
    use crate::navigation::selection_entry::Field;
    let handle = open(cx);
    cx.update(badge_entry::init);
    handle
        .update(cx, |harness, _, _| harness.follow_selection = true)
        .unwrap();
    show_badges(cx, handle, 400_000, 600_000);
    let start = badge(cx, handle, Field::TimeStart);
    click(cx, handle, start.center(), 2);
    type_text(cx, handle, "up");
    assert_eq!(selections(cx, handle), [span(401_000, 600_000)]);
    assert_eq!(typing(cx, handle), Some(("0.401 s".into(), false)));
    type_text(cx, handle, "shift-down");
    assert_eq!(
        selections(cx, handle),
        [span(391_000, 600_000)],
        "ten with Shift"
    );
}

#[gpui_kit::test]
fn a_bare_width_draft_steps_in_its_opened_unit_before_reformatting(cx: &mut TestAppContext) {
    use crate::navigation::selection_entry::Field;
    let handle = open(cx);
    cx.update(badge_entry::init);
    show_badges(cx, handle, 400_000, 600_000);
    handle
        .update(cx, |harness, _, cx| {
            harness.follow_selection = true;
            harness.plot.as_ref().unwrap().update(cx, |plot, cx| {
                let snapshot = plot.snapshot.as_mut().unwrap();
                let band = argand_core::FrequencyBand::between(0., 2_000.).unwrap();
                snapshot.selected = argand_core::Selection {
                    time: None,
                    band: Some(band),
                };
                let scale = snapshot.entry.unwrap();
                snapshot.badges = Some(axes::SelectionBadges {
                    time: None,
                    band: Some(axes::AxisBadges {
                        texts: scale.band_texts(band),
                        fields: Field::BAND,
                        first: Some(0.5),
                        second: Some(0.502),
                        visible: (0.5, 0.502),
                        around: (0., 1.),
                        alone: true,
                    }),
                });
                cx.notify();
            });
        })
        .unwrap();
    frame(cx, handle);
    frame(cx, handle);
    let width = badge(cx, handle, Field::BandWidth);
    click(cx, handle, width.center(), 2);
    type_text(cx, handle, "0 . 5 up");
    let band = handle
        .update(cx, |harness, _, cx| {
            harness
                .plot
                .as_ref()
                .unwrap()
                .read(cx)
                .snapshot
                .as_ref()
                .unwrap()
                .selected
                .band
                .unwrap()
        })
        .unwrap();
    assert_eq!((band.low(), band.high()), (0., 600.));
    assert_eq!(typing(cx, handle), Some(("600 Hz".into(), false)));
}

#[gpui_kit::test]
fn a_badge_input_marks_a_bad_value_and_escape_leaves_the_selection(cx: &mut TestAppContext) {
    use crate::navigation::selection_entry::Field;
    let handle = open(cx);
    cx.update(badge_entry::init);
    show_badges(cx, handle, 400_000, 600_000);
    let length = badge(cx, handle, Field::TimeLength);
    click(cx, handle, length.center(), 2);
    type_text(cx, handle, "x enter");
    assert_eq!(
        typing(cx, handle),
        Some(("x".into(), true)),
        "kept and marked"
    );
    assert!(selections(cx, handle).is_empty());
    type_text(cx, handle, "backspace 1 enter");
    assert_eq!(
        selections(cx, handle),
        [span(400_000, 1_000_000)],
        "a length moves the end, held to the capture"
    );
    show_badges(cx, handle, 400_000, 600_000);
    let end = badge(cx, handle, Field::TimeEnd);
    click(cx, handle, end.center(), 2);
    type_text(cx, handle, "0 . 7 escape");
    assert_eq!(typing(cx, handle), None);
    assert!(selections(cx, handle).is_empty(), "Escape changes nothing");
    assert!(plot_focused(cx, handle));
}

/// Where `sample` lies across the spectrum.
fn x_of(spectrum: Bounds<Pixels>, sample: u64) -> Pixels {
    let view = snapshot().extents.time.view;
    let fraction = (sample - view.start) as f32 / view.len as f32;
    spectrum.left() + spectrum.size.width * fraction
}

#[gpui_kit::test]
fn dragging_an_edge_reshapes_the_selection_and_swaps_past_the_other(cx: &mut TestAppContext) {
    let handle = open(cx);
    let spectrum = spectrum(cx, handle);
    let centre = spectrum.center();
    let (start, end) = (
        boundary_at(spectrum, centre.x - px(50.)),
        boundary_at(spectrum, centre.x + px(50.)),
    );
    show_selection(cx, handle, span(start, end).unwrap());
    let edge = point(x_of(spectrum, end), centre.y);
    let past = point(centre.x - px(100.), centre.y);
    left_drag(cx, handle, edge, past, gpui_kit::Modifiers::default());
    assert_eq!(
        selections(cx, handle).last().copied().flatten(),
        span(boundary_at(spectrum, past.x), start),
        "the start stays and the end goes past it"
    );
    left_drag(cx, handle, edge, edge, gpui_kit::Modifiers::default());
    assert!(
        selections(cx, handle).is_empty(),
        "a click on an edge keeps the selection"
    );
}

#[gpui_kit::test]
fn a_drag_inside_moves_the_whole_selection(cx: &mut TestAppContext) {
    let handle = open(cx);
    let spectrum = spectrum(cx, handle);
    let centre = spectrum.center();
    let (start, end) = (
        boundary_at(spectrum, centre.x - px(50.)),
        boundary_at(spectrum, centre.x + px(50.)),
    );
    show_selection(cx, handle, span(start, end).unwrap());
    left_drag(
        cx,
        handle,
        centre,
        centre + point(px(80.), px(0.)),
        gpui_kit::Modifiers::default(),
    );
    let view = snapshot().extents.time.view;
    let shift = (80. / f64::from(f32::from(spectrum.size.width)) * view.len as f64).round() as u64;
    assert_eq!(
        selections(cx, handle).last().copied().flatten(),
        span(start + shift, end + shift)
    );
    left_drag(cx, handle, centre, centre, gpui_kit::Modifiers::default());
    assert_eq!(selections(cx, handle), [None], "a click inside clears it");
}

#[gpui_kit::test]
fn shift_click_moves_the_nearer_edge_to_the_pointer(cx: &mut TestAppContext) {
    let handle = open(cx);
    let spectrum = spectrum(cx, handle);
    let centre = spectrum.center();
    let (start, end) = (
        boundary_at(spectrum, centre.x - px(50.)),
        boundary_at(spectrum, centre.x + px(50.)),
    );
    show_selection(cx, handle, span(start, end).unwrap());
    let at = centre + point(px(120.), px(0.));
    let shift = gpui_kit::Modifiers {
        shift: true,
        ..Default::default()
    };
    left_drag(cx, handle, at, at, shift);
    assert_eq!(
        selections(cx, handle).last().copied().flatten(),
        span(start, boundary_at(spectrum, at.x))
    );
}

#[gpui_kit::test]
fn a_band_edge_drags_on_the_frequency_ruler_and_the_cursor_says_so(cx: &mut TestAppContext) {
    let handle = open(cx);
    let spectrum = spectrum(cx, handle);
    let ruler = frequency_ruler(cx, handle);
    let selected = argand_core::Selection {
        time: None,
        band: band(hertz_at(0.3), hertz_at(0.6)),
    };
    show_selection(cx, handle, selected);
    let y = |fraction: f32| spectrum.top() + spectrum.size.height * fraction;
    let edge = point(ruler.center().x, y(0.3));
    let mut input = gpui_kit::VisualTestContext::from_window(handle.into(), cx);
    input.simulate_mouse_move(edge, None, gpui_kit::Modifiers::default());
    cx.run_until_parked();
    let cursor = handle
        .update(cx, |harness, _, cx| {
            harness.plot.as_ref().unwrap().read(cx).hover_cursor()
        })
        .unwrap();
    assert_eq!(cursor, Some(gpui_kit::CursorStyle::ResizeUpDown));
    let to = point(edge.x, y(0.1));
    left_drag(cx, handle, edge, to, gpui_kit::Modifiers::default());
    let down =
        |y: Pixels| f32::from(y - spectrum.top()) as f64 / f32::from(spectrum.size.height) as f64;
    assert_eq!(
        selections(cx, handle).last().copied().flatten(),
        Some(argand_core::Selection {
            time: None,
            band: band(hertz_at(down(to.y)), hertz_at(0.6)),
        })
    );
}

#[gpui_kit::test]
fn a_rectangle_s_time_edge_drags_on_the_minimap(cx: &mut TestAppContext) {
    let handle = open(cx);
    let minimap = handle
        .update(cx, |harness, _, cx| {
            harness
                .plot
                .as_ref()
                .unwrap()
                .read(cx)
                .geometry
                .unwrap()
                .minimap
        })
        .unwrap();
    let total = snapshot().extents.time.total;
    let (start, end) = (300_000, 600_000);
    let rectangle = argand_core::Selection {
        time: argand_core::SampleSpan::between(start, end),
        band: band(hertz_at(0.25), hertz_at(0.75)),
    };
    show_selection(cx, handle, rectangle);
    let x = |sample: u64| minimap.left() + minimap.size.width * (sample as f32 / total as f32);
    let edge = point(x(end), minimap.center().y);
    let to = point(x(800_000), minimap.center().y);
    left_drag(cx, handle, edge, to, gpui_kit::Modifiers::default());
    let fraction = f32::from(to.x - minimap.left()) as f64 / f32::from(minimap.size.width) as f64;
    let boundary = crate::navigation::View::full(total).boundary(fraction);
    assert_eq!(
        selections(cx, handle).last().copied().flatten(),
        Some(argand_core::Selection {
            time: argand_core::SampleSpan::between(start, boundary),
            band: rectangle.band,
        })
    );
}

#[gpui_kit::test]
fn a_band_edge_stops_at_the_capture_s_band(cx: &mut TestAppContext) {
    let handle = open(cx);
    let spectrum = spectrum(cx, handle);
    let ruler = frequency_ruler(cx, handle);
    handle
        .update(cx, |harness, _, cx| {
            harness.plot.as_ref().unwrap().update(cx, |plot, _| {
                plot.snapshot.as_mut().unwrap().capture_band = Some((0., 500_000.));
            });
        })
        .unwrap();
    let selected = argand_core::Selection {
        time: None,
        band: band(hertz_at(0.3), hertz_at(0.4)),
    };
    show_selection(cx, handle, selected);
    let y = |fraction: f32| spectrum.top() + spectrum.size.height * fraction;
    let edge = point(ruler.center().x, y(0.3));
    // Far below the capture's lowest frequency, past the band's own low edge.
    left_drag(
        cx,
        handle,
        edge,
        point(edge.x, y(0.95)),
        gpui_kit::Modifiers::default(),
    );
    assert_eq!(
        selections(cx, handle).last().copied().flatten(),
        Some(argand_core::Selection {
            time: None,
            band: band(0., hertz_at(0.4)),
        })
    );
}
