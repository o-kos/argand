//! The plot's own focus, pointer and gestures, painting what the shell supplies.
//!
//! The shell keeps the document, the analysis, the view ranges and every GPU
//! texture. This entity owns what only the plot needs to know -- where the
//! pointer is, which drag is under way, where the rulers were laid out -- and
//! turns input into [`PlotIntent`]s for the shell to act on.

use super::navigation_ui::*;
use crate::shell::*;
use gpui_kit::EventEmitter;

/// Everything the plot paints, rebuilt by the shell on every frame.
///
/// Only shared references: the shell alone uploads and retires the images, and
/// clears this before it lets go of one.
#[derive(Clone)]
pub(in crate::shell) struct PlotSnapshot {
    pub extents: axes::Extents,
    pub texture: Option<Arc<RenderImage>>,
    pub deep: Option<Arc<crate::shell::plot::plot_ui::DeepPreview>>,
    pub backdrop: Option<backdrop::Backdrop>,
    pub held: Option<crate::navigation::PictureView>,
    pub first_picture: Option<(Instant, Arc<AtomicBool>)>,
    pub minimap: waveform::Panel,
    pub minimap_size: crate::panels::MinimapSize,
    pub time_scheme: Option<argand_core::axis::TickScheme>,
    pub frequency_scheme: Option<argand_core::axis::TickScheme>,
    pub frequency: crate::navigation::frequency::View,
    pub show_grid: bool,
    pub tooltips_allowed: bool,
    pub show_scale_ui: bool,
    /// The mouse position counts only while this holds, since it goes stale once the pointer leaves.
    pub pointer_in_window: bool,
    /// The selection as fractions of the visible picture, where any of it shows.
    pub selection: Option<waveform::Selected>,
    /// The selection in samples and hertz, which its gestures change.
    pub selected: argand_core::Selection,
    /// The capture's band in hertz, which a band's edges stop at.
    pub capture_band: Option<(f64, f64)>,
    /// A time selection's readouts on the plot.
    pub badges: Option<axes::SelectionBadges>,
    /// What a selection's typed values are read against.
    pub entry: Option<crate::navigation::selection_entry::Scale>,
    /// A finished save, said over the picture until the next input.
    pub notice: Option<gpui_kit::SharedString>,
    /// Which edit commands the spectrum's context menu can offer.
    pub edit: crate::shell::editing_ui::EditCommands,
}

/// What the plot asks of the shell.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::shell) enum PlotIntent {
    /// The plot was laid out at this size.
    Layout {
        plot: PlotSize,
        time_length_changed: bool,
        frequency_length_changed: bool,
    },
    /// The pointer, and with it the readout, changed.
    Pointer,
    /// The selection a drag or a click left, if any.
    Select(Option<argand_core::Selection>),
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
        frequency: Option<crate::navigation::frequency::View>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::shell) enum TimeIntent {
    Zoom { factor: f64, anchor: f64 },
    Pan(f64),
    Ticks(i64),
    Fit,
    Show(crate::navigation::View),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::shell) enum FrequencyIntent {
    Zoom { factor: f64, anchor: f64 },
    Pan(f64),
    Ticks(f64),
    Fit,
}

pub(in crate::shell) struct PlotView {
    pub(in crate::shell) focus: FocusHandle,
    pub(in crate::shell) snapshot: Option<PlotSnapshot>,
    pub(in crate::shell) pointer: Option<gpui_kit::Point<Pixels>>,
    pub(in crate::shell) pan: Option<Pan>,
    pub(in crate::shell) frequency_pan:
        Option<(gpui_kit::Point<Pixels>, crate::navigation::frequency::View)>,
    /// The button that drives the pan in progress.
    pub(in crate::shell) pan_button: MouseButton,
    pub(in crate::shell) selecting: Option<Selecting>,
    /// Space is held over the focused plot, so a left drag pans.
    pub(in crate::shell) space: bool,
    pub(in crate::shell) geometry: Option<PlotGeometry>,
    pub(in crate::shell) panel_bounds: Option<Bounds<Pixels>>,
    pub(in crate::shell) measured: Option<PlotSize>,
    /// The right ruler only widens while zooming, until an explicit reset.
    pub(in crate::shell) gutter_floor: f32,
    pub(in crate::shell) badge_metrics: axes::BadgeMetrics,
    pub(in crate::shell) open_menu: Option<WeakEntity<PopupMenu>>,
    /// The sample a paste from the spectrum's context menu goes before.
    pub(in crate::shell) paste_point: Option<u64>,
    pub(in crate::shell) menu_dismiss: Option<Subscription>,
    /// Where each selection badge was painted last, in window coordinates.
    pub(in crate::shell) badges: Vec<(crate::navigation::selection_entry::Field, Bounds<Pixels>)>,
    /// The input a badge was turned into.
    pub(in crate::shell) typing: Option<badge_entry::BadgeEntry>,
    /// Kept because dropping it stops the symbol shortcuts.
    _symbols: Subscription,
    /// Forgets Space when the plot loses focus, since its release goes elsewhere.
    _blur: Subscription,
}

impl EventEmitter<PlotIntent> for PlotView {}

impl PlotView {
    pub(in crate::shell) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus = cx.focus_handle().tab_stop(true);
        let symbols = symbol_shortcuts(focus.clone(), window.window_handle().window_id(), cx);
        let blur = cx.on_blur(&focus, window, |plot, _, cx| plot.release_space(cx));
        Self {
            focus,
            snapshot: None,
            pointer: None,
            pan: None,
            frequency_pan: None,
            pan_button: MouseButton::Left,
            selecting: None,
            space: false,
            geometry: None,
            panel_bounds: None,
            measured: None,
            gutter_floor: 0.,
            badge_metrics: axes::BadgeMetrics::default(),
            open_menu: None,
            paste_point: None,
            menu_dismiss: None,
            badges: Vec::new(),
            typing: None,
            _symbols: symbols,
            _blur: blur,
        }
    }

    pub(in crate::shell) fn dragging(&self) -> bool {
        self.panning() || self.selecting.is_some()
    }

    pub(in crate::shell) fn panning(&self) -> bool {
        self.pan.is_some() || self.frequency_pan.is_some()
    }

    fn release_space(&mut self, cx: &mut Context<Self>) {
        if self.space {
            self.space = false;
            cx.notify();
        }
    }

    /// The pointer over the plot and the layout it was measured against.
    pub(in crate::shell) fn hover(&self) -> Option<(gpui_kit::Point<Pixels>, PlotGeometry)> {
        self.pointer.zip(self.geometry)
    }

    pub(in crate::shell) fn cancel_drags(&mut self, cx: &mut Context<Self>) {
        self.pan = None;
        self.frequency_pan = None;
        self.selecting = None;
        cx.notify();
    }

    pub(in crate::shell) fn clear_pointer(&mut self, cx: &mut Context<Self>) {
        if self.pointer.take().is_some() {
            cx.emit(PlotIntent::Pointer);
            cx.notify();
        }
    }

    pub(in crate::shell) fn dismiss_menu(&mut self, cx: &mut Context<Self>) {
        if let Some(menu) = self.open_menu.take() {
            let _ = menu.update(cx, |_, cx| cx.emit(gpui_kit::DismissEvent));
        }
    }

    /// End the drags, which only the pointer that began them may finish.
    pub(in crate::shell) fn end_gestures(&mut self, cx: &mut Context<Self>) {
        if self.dragging() || self.space {
            self.pan = None;
            self.frequency_pan = None;
            self.selecting = None;
            self.space = false;
            cx.notify();
        }
    }

    /// Leave nothing behind for an overlay that is taking the input.
    pub(in crate::shell) fn interrupt(&mut self, cx: &mut Context<Self>) {
        self.end_gestures(cx);
        self.dismiss_menu(cx);
        self.clear_pointer(cx);
    }

    /// Let the right ruler fit its current labels again.
    pub(in crate::shell) fn reset_gutter(&mut self, cx: &mut Context<Self>) {
        self.gutter_floor = 0.;
        cx.notify();
    }

    /// Send a view change, letting the right ruler refit when its own axis zooms out.
    ///
    /// Zooming in keeps the ruler from narrowing, and so do panning and the
    /// other axis. Zooming out or fitting the ruler's own axis lets it fit its
    /// new labels, which can still widen it when a format or unit grows.
    pub(in crate::shell) fn view_intent(&mut self, intent: PlotIntent, cx: &mut Context<Self>) {
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
    pub(in crate::shell) fn reorient(&mut self, cx: &mut Context<Self>) {
        self.gutter_floor = 0.;
        self.pan = None;
        self.frequency_pan = None;
        self.selecting = None;
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
            .on_key_down(cx.listener(|plot, event: &gpui_kit::KeyDownEvent, _, cx| {
                if is_space(&event.keystroke) && !plot.space {
                    plot.space = true;
                    cx.notify();
                }
            }))
            .on_key_up(cx.listener(|plot, event: &gpui_kit::KeyUpEvent, _, cx| {
                // Released with a modifier still down is still released
                if event.keystroke.key == "space" {
                    plot.release_space(cx);
                }
            }))
            .on_action(cx.listener(|plot, _: &ClearSelection, _, cx| {
                plot.selecting = None;
                cx.emit(PlotIntent::Select(None));
                cx.notify();
            }))
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
        let entry = self.badge_entry(window, cx);
        let cursor = self
            .geometry
            .map_or(gpui_kit::CursorStyle::Arrow, |geometry| {
                geometry
                    .selection_cursor(
                        self.pointer,
                        self.panning(),
                        self.selecting.map(|selecting| selecting.cursor),
                        self.space,
                    )
                    .or_else(|| self.hover_cursor())
                    .unwrap_or_else(|| {
                        geometry.cursor(
                            self.pointer,
                            self.panning(),
                            Some((snapshot.extents.time.view, snapshot.extents.time.total)),
                            snapshot.frequency.span < 1.,
                        )
                    })
            });
        // Focusable overlays sit beside the surface, so they never inherit Plot bindings.
        div()
            .id("time-plot")
            .cursor(cursor)
            .on_scroll(cx.listener(Self::wheel))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::press))
            .on_mouse_down(MouseButton::Middle, cx.listener(Self::press))
            .on_mouse_move(cx.listener(|plot, event, _, cx| plot.pointer_moved(event, cx)))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::finish_drags))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::finish_drags))
            .on_mouse_up(MouseButton::Middle, cx.listener(Self::finish_drags))
            .on_mouse_up_out(MouseButton::Middle, cx.listener(Self::finish_drags))
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
            .children(self.spectrum_context_menu(&snapshot, cx))
            .children(self.time_context_menu(&snapshot, cx))
            .children(
                self.panel_bounds
                    .and_then(|bounds| self.unit_hint(1, bounds.origin, cx)),
            )
            .children(self.saved_notice(&snapshot, cx))
            .children(entry)
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
                    let _ = plot.update(cx, |plot, cx| plot.pointer_moved(event, cx));
                }
            });
        },
    )
    .absolute()
    .inset_0()
}

/// Space alone, which turns a left drag on the spectrum into a pan.
fn is_space(keystroke: &gpui_kit::Keystroke) -> bool {
    keystroke.key == "space" && !keystroke.modifiers.modified()
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

#[cfg(test)]
#[path = "plot_view_test.rs"]
mod tests;
