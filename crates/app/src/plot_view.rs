//! The plot's own focus, pointer and gestures, painting what the shell supplies.
//!
//! The shell keeps the document, the analysis, the view ranges and every GPU
//! texture. This entity owns what only the plot needs to know -- where the
//! pointer is, which drag is under way, where the rulers were laid out -- and
//! turns input into [`PlotIntent`]s for the shell to act on.

use super::navigation_ui::*;
use super::*;
use gpui_kit::EventEmitter;

/// Everything the plot paints, rebuilt by the shell on every frame.
///
/// Only shared references: the shell alone uploads and retires the images, and
/// clears this before it lets go of one.
#[derive(Clone)]
pub(super) struct PlotSnapshot {
    pub extents: axes::Extents,
    pub texture: Option<Arc<RenderImage>>,
    pub deep: Option<Arc<plot_ui::DeepPreview>>,
    pub backdrop: Option<backdrop::Backdrop>,
    pub held: Option<crate::navigation::PictureView>,
    pub first_picture: Option<(Instant, Arc<AtomicBool>)>,
    pub minimap: waveform::Panel,
    pub fraction: Option<f32>,
    pub time_scheme: Option<argand_core::axis::TickScheme>,
    pub frequency_scheme: Option<argand_core::axis::TickScheme>,
    pub frequency: crate::frequency::View,
    pub show_grid: bool,
    pub show_scale_ui: bool,
    /// The mouse position counts only while this holds, since it goes stale once the pointer leaves.
    pub pointer_in_window: bool,
}

/// What the plot asks of the shell.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum PlotIntent {
    /// The plot was laid out at this size.
    Layout {
        plot: PlotSize,
        time_length_changed: bool,
        frequency_length_changed: bool,
    },
    /// The pointer, and with it the readout, changed.
    Pointer,
    /// A drag began on these axes, whose tick schemes are held from here.
    GestureStarted {
        time: bool,
        frequency: bool,
    },
    Time(TimeIntent),
    Frequency(FrequencyIntent),
    /// A drag step moving one or both axes, answered by one analysis request.
    Drag {
        time: Option<crate::navigation::View>,
        frequency: Option<crate::frequency::View>,
    },
    WaveformFraction(f32),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum TimeIntent {
    Zoom { factor: f64, anchor: f64 },
    Pan(f64),
    Ticks(i64),
    Fit,
    Show(crate::navigation::View),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum FrequencyIntent {
    Zoom { factor: f64, anchor: f64 },
    Pan(f64),
    Ticks(f64),
    Fit,
}

pub(super) struct PlotView {
    pub(super) focus: FocusHandle,
    pub(super) snapshot: Option<PlotSnapshot>,
    pub(super) pointer: Option<gpui_kit::Point<Pixels>>,
    pub(super) pan: Option<Pan>,
    pub(super) frequency_pan: Option<(gpui_kit::Point<Pixels>, crate::frequency::View)>,
    pub(super) splitter_dragging: bool,
    pub(super) geometry: Option<PlotGeometry>,
    pub(super) panel_bounds: Option<Bounds<Pixels>>,
    pub(super) measured: Option<PlotSize>,
    /// The right ruler only widens while zooming, until an explicit reset.
    pub(super) gutter_floor: f32,
    pub(super) badge_metrics: axes::BadgeMetrics,
    pub(super) open_menu: Option<WeakEntity<PopupMenu>>,
    pub(super) menu_dismiss: Option<Subscription>,
    /// Kept because dropping it stops the symbol shortcuts.
    _symbols: Subscription,
}

impl EventEmitter<PlotIntent> for PlotView {}

impl PlotView {
    pub(super) fn new(window: &Window, cx: &mut Context<Self>) -> Self {
        let focus = cx.focus_handle().tab_stop(true);
        let symbols = symbol_shortcuts(focus.clone(), window.window_handle().window_id(), cx);
        Self {
            focus,
            snapshot: None,
            pointer: None,
            pan: None,
            frequency_pan: None,
            splitter_dragging: false,
            geometry: None,
            panel_bounds: None,
            measured: None,
            gutter_floor: 0.,
            badge_metrics: axes::BadgeMetrics::default(),
            open_menu: None,
            menu_dismiss: None,
            _symbols: symbols,
        }
    }

    pub(super) fn dragging(&self) -> bool {
        self.pan.is_some() || self.frequency_pan.is_some() || self.splitter_dragging
    }

    /// The pointer over the plot and the layout it was measured against.
    pub(super) fn hover(&self) -> Option<(gpui_kit::Point<Pixels>, PlotGeometry)> {
        self.pointer.zip(self.geometry)
    }

    pub(super) fn cancel_drags(&mut self, cx: &mut Context<Self>) {
        self.pan = None;
        self.frequency_pan = None;
        cx.notify();
    }

    pub(super) fn clear_pointer(&mut self, cx: &mut Context<Self>) {
        if self.pointer.take().is_some() {
            cx.emit(PlotIntent::Pointer);
            cx.notify();
        }
    }

    pub(super) fn dismiss_menu(&mut self, cx: &mut Context<Self>) {
        if let Some(menu) = self.open_menu.take() {
            let _ = menu.update(cx, |_, cx| cx.emit(gpui_kit::DismissEvent));
        }
    }

    /// End the drags, which only the pointer that began them may finish.
    pub(super) fn end_gestures(&mut self, cx: &mut Context<Self>) {
        if self.dragging() {
            self.pan = None;
            self.frequency_pan = None;
            self.splitter_dragging = false;
            cx.notify();
        }
    }

    /// Leave nothing behind for an overlay that is taking the input.
    pub(super) fn interrupt(&mut self, cx: &mut Context<Self>) {
        self.end_gestures(cx);
        self.dismiss_menu(cx);
        self.clear_pointer(cx);
    }

    /// Let the right ruler fit its current labels again.
    pub(super) fn reset_gutter(&mut self, cx: &mut Context<Self>) {
        self.gutter_floor = 0.;
        cx.notify();
    }

    /// Send a view change, letting the right ruler refit when its own axis zooms out.
    ///
    /// Zooming in keeps the ruler from narrowing, and so do panning and the
    /// other axis. Zooming out or fitting the ruler's own axis lets it fit its
    /// new labels, which can still widen it when a format or unit grows.
    pub(super) fn view_intent(&mut self, intent: PlotIntent, cx: &mut Context<Self>) {
        let vertical = self
            .snapshot
            .as_ref()
            .is_some_and(|snapshot| snapshot.extents.orientation.vertical());
        let refit = match intent {
            PlotIntent::Time(TimeIntent::Zoom { factor, .. }) => vertical && factor > 1.,
            PlotIntent::Time(TimeIntent::Fit) => vertical,
            PlotIntent::Frequency(FrequencyIntent::Zoom { factor, .. }) => !vertical && factor > 1.,
            PlotIntent::Frequency(FrequencyIntent::Fit) => !vertical,
            _ => false,
        };
        if refit {
            self.gutter_floor = 0.;
        }
        cx.emit(intent);
    }

    /// Forget the gestures the other orientation cannot reuse.
    pub(super) fn reorient(&mut self, cx: &mut Context<Self>) {
        self.gutter_floor = 0.;
        self.pan = None;
        self.frequency_pan = None;
        self.splitter_dragging = false;
        // The old layout serves until the next frame measures the new one, so the cursor and readout do not blink.
        cx.notify();
    }

    fn time(&mut self, intent: TimeIntent, cx: &mut Context<Self>) {
        self.view_intent(PlotIntent::Time(intent), cx);
    }

    fn frequency(&mut self, intent: FrequencyIntent, cx: &mut Context<Self>) {
        self.view_intent(PlotIntent::Frequency(intent), cx);
    }

    /// The navigation keys, handled only while the plot itself has focus.
    fn surface(&self, snapshot: &PlotSnapshot, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("plot-surface")
            .size_full()
            .track_focus(&self.focus)
            .key_context(if snapshot.extents.orientation.vertical() {
                "Plot Vertical"
            } else {
                "Plot Horizontal"
            })
            .on_action(cx.listener(|plot, _: &ZoomIn, _, cx| {
                plot.time(
                    TimeIntent::Zoom {
                        factor: 0.5,
                        anchor: 0.5,
                    },
                    cx,
                )
            }))
            .on_action(cx.listener(|plot, _: &ZoomOut, _, cx| {
                plot.time(
                    TimeIntent::Zoom {
                        factor: 2.,
                        anchor: 0.5,
                    },
                    cx,
                )
            }))
            .on_action(cx.listener(|plot, _: &FitCapture, _, cx| plot.time(TimeIntent::Fit, cx)))
            .on_action(cx.listener(|plot, _: &PanLeft, _, cx| plot.time(TimeIntent::Ticks(-1), cx)))
            .on_action(cx.listener(|plot, _: &PanRight, _, cx| plot.time(TimeIntent::Ticks(1), cx)))
            .on_action(
                cx.listener(|plot, _: &PanFarLeft, _, cx| plot.time(TimeIntent::Ticks(-5), cx)),
            )
            .on_action(
                cx.listener(|plot, _: &PanFarRight, _, cx| plot.time(TimeIntent::Ticks(5), cx)),
            )
            .on_action(
                cx.listener(|plot, _: &GoStart, _, cx| plot.time(TimeIntent::Pan(-1e20), cx)),
            )
            .on_action(cx.listener(|plot, _: &GoEnd, _, cx| plot.time(TimeIntent::Pan(1e20), cx)))
            .on_action(cx.listener(|plot, _: &FrequencyZoomIn, _, cx| {
                plot.frequency(
                    FrequencyIntent::Zoom {
                        factor: 0.5,
                        anchor: 0.5,
                    },
                    cx,
                )
            }))
            .on_action(cx.listener(|plot, _: &FrequencyZoomOut, _, cx| {
                plot.frequency(
                    FrequencyIntent::Zoom {
                        factor: 2.,
                        anchor: 0.5,
                    },
                    cx,
                )
            }))
            .on_action(
                cx.listener(|plot, _: &FitFrequency, _, cx| {
                    plot.frequency(FrequencyIntent::Fit, cx)
                }),
            )
            .on_action(
                cx.listener(|plot, _: &PanUp, _, cx| {
                    plot.frequency(FrequencyIntent::Ticks(1.), cx)
                }),
            )
            .on_action(cx.listener(|plot, _: &PanDown, _, cx| {
                plot.frequency(FrequencyIntent::Ticks(-1.), cx)
            }))
            .on_action(cx.listener(|plot, _: &PanFarUp, _, cx| {
                plot.frequency(FrequencyIntent::Ticks(5.), cx)
            }))
            .on_action(cx.listener(|plot, _: &PanFarDown, _, cx| {
                plot.frequency(FrequencyIntent::Ticks(-5.), cx)
            }))
            .child(self.spectrogram(snapshot.clone(), cx))
    }
}

impl Render for PlotView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(snapshot) = self.snapshot.clone() else {
            return div().flex_1().min_h_0().into_any_element();
        };
        let cursor = self
            .geometry
            .map_or(gpui_kit::CursorStyle::Arrow, |geometry| {
                geometry.cursor(
                    self.pointer,
                    self.pan.is_some() || self.frequency_pan.is_some(),
                    Some((snapshot.extents.time.view, snapshot.extents.time.total)),
                    snapshot.frequency.span < 1.,
                )
            });
        // Focusable overlays sit beside the surface, so they never inherit Plot bindings.
        div()
            .id("time-plot")
            .cursor(cursor)
            .on_scroll_wheel(cx.listener(Self::wheel))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::begin_pan))
            .on_mouse_move(
                cx.listener(|plot, event, window, cx| plot.pointer_moved(event, window, cx)),
            )
            .on_mouse_up(MouseButton::Left, cx.listener(Self::finish_drags))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::finish_drags))
            // Keys must not end the hover, and a plot uncovered under a still pointer takes it up.
            .hover_listener_mode(gpui_kit::HoverListenerMode::InputModalityIndependent)
            .on_hover(cx.listener(|plot, hovered, window, cx| {
                if *hovered {
                    plot.pick_up_pointer(window, cx);
                } else {
                    plot.clear_pointer(cx);
                }
            }))
            .flex_1()
            .min_h_0()
            .relative()
            .child(self.surface(&snapshot, cx))
            .child(self.splitter(&snapshot, window, cx))
            .children(self.time_context_menu(&snapshot, cx))
            .children(
                self.panel_bounds
                    .and_then(|bounds| self.unit_hint(1, bounds.origin, cx)),
            )
            .children(self.ruler_zoom_buttons(&snapshot, cx))
            .when(self.dragging(), |plot| {
                plot.child(drag_tracker(cx.entity().downgrade()))
            })
            .into_any_element()
    }
}

/// Follows a drag wherever the plot's own hitbox no longer sees the pointer.
///
/// That is beyond the plot and over a hint lying on it. The tracker's hitbox
/// matches the plot's, so each move goes to exactly one of them. Moves the
/// plot sees stay with its own listener, which also covers the ones that
/// arrive before the first frame drawn after the press.
fn drag_tracker(plot: WeakEntity<PlotView>) -> impl IntoElement {
    canvas(
        |bounds, window, _| window.insert_hitbox(bounds, gpui_kit::HitboxBehavior::Normal),
        move |_, hitbox, window, _| {
            let plot = plot.clone();
            window.on_mouse_event(move |event: &gpui_kit::MouseMoveEvent, phase, window, cx| {
                if phase == gpui_kit::DispatchPhase::Bubble && !hitbox.is_hovered(window) {
                    let _ = plot.update(cx, |plot, cx| plot.pointer_moved(event, window, cx));
                }
            });
        },
    )
    .absolute()
    .inset_0()
}

/// Symbol zoom keys whose Shift GPUI loses before binding match, for one focused plot.
///
/// Interceptors are the only hook ahead of binding match, and they fire for
/// every window, so both the window and the exact focus are checked first.
fn symbol_shortcuts(
    focus: FocusHandle,
    window_id: gpui_kit::WindowId,
    cx: &mut gpui_kit::App,
) -> Subscription {
    cx.intercept_keystrokes(move |event, window, cx| {
        if window.window_handle().window_id() != window_id || !focus.is_focused(window) {
            return;
        }
        if let Some(action) = plot_shortcut(
            &event.keystroke.key,
            event.keystroke.modifiers,
            window.modifiers().shift,
        ) {
            Shell::dismiss_window_ready_status(window, cx);
            window.dispatch_action(action, cx);
            cx.stop_propagation();
        }
    })
}

impl PlotView {
    fn splitter(
        &self,
        snapshot: &PlotSnapshot,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let orientation = snapshot.extents.orientation;
        let total = self.panel_bounds.map_or(0.0, |bounds| {
            f32::from(orientation.axes(bounds.size.width, bounds.size.height).1)
        });
        let height = panels::waveform_height(
            total,
            f32::from(cx.theme().font_size),
            snapshot.fraction,
            window.scale_factor(),
        );
        let divider = div().id("waveform-splitter").absolute();
        let divider = if orientation.vertical() {
            divider
                .top_0()
                .bottom_0()
                .left(px((height - 3.).max(0.)))
                .w(px(5.))
                .cursor(gpui_kit::CursorStyle::ResizeLeftRight)
        } else {
            divider
                .left_0()
                .right_0()
                .top(px((height - 3.).max(0.)))
                .h(px(5.))
                .cursor(gpui_kit::CursorStyle::ResizeUpDown)
        };
        divider.on_mouse_down(
            MouseButton::Left,
            cx.listener(|plot, _, _, cx| {
                plot.splitter_dragging = true;
                cx.stop_propagation();
                cx.notify();
            }),
        )
    }

    pub(super) fn drag_splitter(
        &mut self,
        event: &gpui_kit::MouseMoveEvent,
        window: &Window,
        cx: &mut Context<Self>,
    ) {
        if !self.splitter_dragging {
            return;
        }
        if !event.dragging() {
            self.splitter_dragging = false;
            cx.notify();
            return;
        }
        let (Some(bounds), Some(snapshot)) = (self.panel_bounds, &self.snapshot) else {
            return;
        };
        let orientation = snapshot.extents.orientation;
        let total = f32::from(orientation.axes(bounds.size.width, bounds.size.height).1);
        if total <= 0.0 {
            return;
        }
        let delta = event.position - bounds.origin;
        let requested = f32::from(orientation.axes(delta.x, delta.y).1) / total;
        let height = panels::waveform_height(
            total,
            f32::from(cx.theme().font_size),
            Some(requested.clamp(0.0, 1.0)),
            window.scale_factor(),
        );
        cx.emit(PlotIntent::WaveformFraction(height / total));
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
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
        /// Stands in for the session's scale-controls choice.
        scale_ui: bool,
        /// Stands in for a hint or a menu lying on the plot.
        cover: Option<Bounds<Pixels>>,
        _intents: Option<Subscription>,
    }

    impl Render for Harness {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .size_full()
                .relative()
                .flex()
                .flex_col()
                .key_context("Shell")
                .on_action(cx.listener(|harness, _: &ToggleGrid, _, _| harness.grid += 1))
                .on_action(cx.listener(|harness, _: &ToggleScaleUi, _, _| {
                    harness.scale_ui = !harness.scale_ui
                }))
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

    fn snapshot() -> PlotSnapshot {
        let view = crate::navigation::View {
            start: 250_000,
            len: 500_000,
        };
        let ink = gpui_kit::rgb(0x808080);
        PlotSnapshot {
            extents: axes::Extents {
                orientation: crate::orientation::Mode::Horizontal,
                time: crate::time_ruler::Ruler {
                    mode: crate::time_ruler::Mode::Seconds,
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
                separator: gpui_kit::black(),
                ink: waveform::Ink {
                    active: ink,
                    muted: ink,
                },
            },
            fraction: None,
            time_scheme: None,
            frequency_scheme: None,
            frequency: crate::frequency::View::default(),
            show_grid: true,
            show_scale_ui: false,
            pointer_in_window: true,
        }
    }

    fn open(cx: &mut TestAppContext) -> WindowHandle<Harness> {
        cx.update(|cx| {
            gpui_kit::init(cx);
            navigation_ui::init(cx);
        });
        let handle = cx.add_window(|window, cx| {
            let plot = cx.new(|cx| {
                let mut plot = PlotView::new(window, cx);
                plot.snapshot = Some(snapshot());
                plot
            });
            let intents = cx.subscribe(&plot, |harness: &mut Harness, _, intent, _| {
                harness.intents.push(*intent)
            });
            let focus = plot.read(cx).focus.clone();
            window.focus(&focus, cx);
            Harness {
                plot: Some(plot),
                other: cx.focus_handle(),
                intents: Vec::new(),
                grid: 0,
                scale_ui: false,
                cover: None,
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
                    .filter(|intent| {
                        matches!(intent, PlotIntent::Time(_) | PlotIntent::Frequency(_))
                    })
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
                scale_ui: false,
                cover: None,
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
            crate::orientation::Mode::Horizontal,
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

    /// Drags the spectrum from its centre to `to` and returns the last view it asked for.
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
        cx.update_window(handle.into(), |_, window, cx| window.drag(from, to, cx))
            .unwrap();
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
        input.simulate_event(gpui_kit::ScrollWheelEvent {
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
                        crate::orientation::Mode::Vertical;
                    plot.reorient(cx);
                    assert!(plot.hover().is_some(), "no frame without a readout");
                });
            })
            .unwrap();
        frame(cx, handle);
        frame(cx, handle);
        assert_eq!(pointer(cx, handle), Some(resting));
    }

    /// Presses in the spectrum's centre and drags a little, returning the drag's input.
    fn start_drag(
        cx: &mut TestAppContext,
        handle: WindowHandle<Harness>,
    ) -> (gpui_kit::VisualTestContext, gpui_kit::Point<Pixels>) {
        let from = spectrum(cx, handle).center();
        let mut input = gpui_kit::VisualTestContext::from_window(handle.into(), cx);
        let none = gpui_kit::Modifiers::default();
        input.simulate_mouse_move(from, None, none);
        input.simulate_mouse_down(from, MouseButton::Left, none);
        input.simulate_mouse_move(from - point(px(10.), px(0.)), MouseButton::Left, none);
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
        input.simulate_mouse_move(from - point(px(60.), px(0.)), MouseButton::Left, none);
        input.simulate_mouse_up(from - point(px(60.), px(0.)), MouseButton::Left, none);
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

    /// Whether the corner pairs are in the tree the last frame drew.
    fn pairs_shown(cx: &mut TestAppContext, handle: WindowHandle<Harness>) -> bool {
        cx.update_window(handle.into(), |_, window, _| {
            window.try_find("Zoom in time").is_some()
        })
        .unwrap()
    }

    /// Flips the scale controls and repaints, the way the shell does when the
    /// toggle reaches it: the session choice travels on the next snapshot.
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
    fn zone(
        cx: &mut TestAppContext,
        handle: WindowHandle<Harness>,
        index: usize,
    ) -> Bounds<Pixels> {
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
    fn the_halves_split_their_pair_in_two(cx: &mut TestAppContext) {
        let handle = open(cx);
        toggle_pairs(cx, handle);
        let (first, second, pair) = halves(cx, handle, 0);
        assert_eq!(first.size, second.size, "the halves are equal");
        assert_eq!(
            first.size.width * 2. + px(1.) + px(2.),
            pair.size.width,
            "the frame holds both halves and the divider"
        );
        assert_eq!(first.size.height + px(2.), pair.size.height);
        let (first, second, pair) = halves(cx, handle, 1);
        assert_eq!(first.size, second.size, "the halves are equal");
        assert_eq!(
            first.size.height * 2. + px(1.) + px(2.),
            pair.size.height,
            "the frame holds both halves and the divider"
        );
        assert_eq!(first.size.width + px(2.), pair.size.width);
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
    fn a_release_outside_a_zoom_half_changes_nothing(cx: &mut TestAppContext) {
        let handle = open(cx);
        toggle_pairs(cx, handle);
        let inside = half(cx, handle, 0);
        let outside = inside - point(px(0.), px(40.));
        let none = gpui_kit::Modifiers::default();
        let mut input = gpui_kit::VisualTestContext::from_window(handle.into(), cx);
        input.simulate_mouse_move(inside, None, none);
        input.simulate_mouse_down(inside, MouseButton::Left, none);
        input.simulate_mouse_move(outside, Some(MouseButton::Left), none);
        input.simulate_mouse_up(outside, MouseButton::Left, none);
        assert!(
            navigation(cx, handle).is_empty(),
            "the release did not land on the half"
        );
        cx.update_window(handle.into(), |_, window, cx| {
            window.click("Zoom out time", cx)
        })
        .unwrap();
        assert_eq!(
            navigation(cx, handle),
            vec![PlotIntent::Time(TimeIntent::Zoom {
                factor: 2.,
                anchor: 0.5,
            })],
            "the half still works"
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
}
