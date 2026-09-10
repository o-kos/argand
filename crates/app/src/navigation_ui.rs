//! Time navigation gestures, actions and physical cursor readout.

use super::*;
use crate::navigation::{self, View};

actions!(
    navigation,
    [
        ZoomIn,
        ZoomOut,
        FitCapture,
        PanLeft,
        PanRight,
        PanFarLeft,
        PanFarRight,
        GoStart,
        GoEnd
    ]
);

pub(super) fn init(cx: &mut gpui::App) {
    cx.bind_keys([
        KeyBinding::new("ctrl-+", ZoomIn, Some("Plot")),
        KeyBinding::new("ctrl-=", ZoomIn, Some("Plot")),
        KeyBinding::new("ctrl--", ZoomOut, Some("Plot")),
        KeyBinding::new("ctrl-0", FitCapture, Some("Plot")),
        KeyBinding::new("left", PanLeft, Some("Plot")),
        KeyBinding::new("right", PanRight, Some("Plot")),
        KeyBinding::new("ctrl-left", PanFarLeft, Some("Plot")),
        KeyBinding::new("ctrl-right", PanFarRight, Some("Plot")),
        KeyBinding::new("home", GoStart, Some("Plot")),
        KeyBinding::new("end", GoEnd, Some("Plot")),
    ]);
}

#[derive(Clone, Copy, PartialEq)]
pub(super) struct PlotGeometry {
    pub time_scheme: Option<argand_core::axis::TickScheme>,
    pub spectrum: Bounds<Pixels>,
    pub navigation: Bounds<Pixels>,
}

enum Scroll {
    VerticalPending,
    Pan(f64),
    Zoom { factor: f64, anchor: f64 },
}

impl PlotGeometry {
    pub fn cursor(self, pointer: Option<gpui::Point<Pixels>>, dragging: bool) -> gpui::CursorStyle {
        if dragging {
            return gpui::CursorStyle::ClosedHand;
        }
        if pointer.is_some_and(|position| self.spectrum.contains(&position)) {
            gpui::CursorStyle::Crosshair
        } else {
            gpui::CursorStyle::Arrow
        }
    }

    fn scroll(
        self,
        position: gpui::Point<Pixels>,
        delta: gpui::Point<Pixels>,
        modifiers: gpui::Modifiers,
    ) -> Scroll {
        let horizontal = f32::from(delta.x).abs() > f32::from(delta.y).abs();
        let width = f32::from(self.navigation.size.width) as f64;
        if modifiers.shift {
            return Scroll::VerticalPending;
        }
        if !modifiers.control {
            let distance = if horizontal { delta.x } else { delta.y };
            return Scroll::Pan(-f32::from(distance) as f64 / width);
        }
        Scroll::Zoom {
            factor: 2.0_f64.powf(-f32::from(delta.y) as f64 / 160.0),
            anchor: f32::from(position.x - self.navigation.left()) as f64 / width,
        }
    }
}

#[derive(Clone, Copy)]
pub(super) struct Pan {
    position: gpui::Point<Pixels>,
    view: View,
    width: f32,
}

impl Shell {
    pub(super) fn reset_view(&mut self) {
        let Some(total) = self.sample_count() else {
            return;
        };
        self.view = Some(View::full(total));
        self.time_scheme = None;
        self.tick_pan = None;
        if self.settings_backup.is_some() {
            self.settings_view_backup = self.view;
        }
    }

    pub(super) fn bound_view(&mut self) {
        self.pan = None;
        if let Some(total) = self.sample_count()
            && let Some(view) = self.view
        {
            let bounded = view.bounded(total, self.settings.fft_size, self.view_columns());
            if bounded.len != view.len {
                self.time_scheme = None;
                self.tick_pan = None;
            }
            self.view = Some(bounded);
        }
    }

    fn view_columns(&self) -> usize {
        self.plot.map_or(1, |plot| plot.width)
    }

    fn sample_count(&self) -> Option<u64> {
        Some(self.file.as_ref()?.document.meta()?.len_samples)
    }

    fn navigate(&mut self, view: View, cx: &mut Context<Self>) {
        if self.view == Some(view) {
            return;
        }
        if self.view.is_none_or(|old| old.len != view.len) {
            self.time_scheme = None;
        }
        self.tick_pan = None;
        self.view = Some(view);
        if self.settings_backup.is_some() {
            self.settings_view_backup = Some(view);
        }
        self.ask_for_a_picture();
        tracing::debug!(start = view.start, len = view.len, "time view requested");
        cx.notify();
    }

    fn zoom(&mut self, factor: f64, anchor: f64, cx: &mut Context<Self>) {
        if let Some(view) = self.view
            && let Some(total) = self.sample_count()
        {
            self.navigate(
                view.zoom(
                    factor,
                    anchor,
                    total,
                    self.settings.fft_size,
                    self.view_columns(),
                ),
                cx,
            );
        }
    }

    fn hold_time_scheme(&mut self, window: &Window) {
        if self.time_scheme.is_none() {
            self.time_scheme = self.measure_time_scheme(window);
        }
    }

    fn pan_ticks(&mut self, divisions: i64, window: &Window, cx: &mut Context<Self>) {
        let Some(view) = self.view else { return };
        let Some(meta) = self.file.as_ref().and_then(|file| file.document.meta()) else {
            return;
        };
        let total = meta.len_samples;
        let rate = meta.sample_rate;
        self.hold_time_scheme(window);
        let Some(scheme) = self.time_scheme else {
            return;
        };
        let step = scheme.step * rate;
        let mut pan = self
            .tick_pan
            .take()
            .filter(|pan| pan.view == view && pan.step == step)
            .unwrap_or_else(|| navigation::TickPan::new(view, step));
        pan.advance(divisions, total);
        self.navigate(pan.view, cx);
        self.tick_pan = Some(pan);
    }

    fn pan_by(&mut self, fraction: f64, window: &Window, cx: &mut Context<Self>) {
        self.hold_time_scheme(window);
        if let Some(view) = self.view
            && let Some(total) = self.sample_count()
        {
            self.navigate(view.pan(fraction, total), cx);
        }
    }

    pub(super) fn wheel(
        &mut self,
        event: &gpui::ScrollWheelEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(geometry) = self.plot_geometry else {
            return;
        };
        if !geometry.navigation.contains(&event.position) || self.splitter_dragging {
            return;
        }
        self.pan = None;
        let delta = event.delta.pixel_delta(px(40.));
        match geometry.scroll(event.position, delta, event.modifiers) {
            Scroll::VerticalPending => {}
            Scroll::Pan(fraction) => self.pan_by(fraction, window, cx),
            Scroll::Zoom { factor, anchor } => self.zoom(factor, anchor, cx),
        }
        cx.stop_propagation();
    }

    pub(super) fn begin_pan(
        &mut self,
        event: &gpui::MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(geometry) = self.plot_geometry else {
            return;
        };
        if !geometry.navigation.contains(&event.position) || self.splitter_dragging {
            return;
        }
        window.focus(&self.focus);
        self.hold_time_scheme(window);
        self.pan = self.view.map(|view| Pan {
            position: event.position,
            view,
            width: f32::from(geometry.navigation.size.width),
        });
        cx.notify();
    }

    pub(super) fn pointer_moved(
        &mut self,
        event: &gpui::MouseMoveEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let pointer = self
            .plot_geometry
            .filter(|geometry| geometry.navigation.contains(&event.position))
            .map(|_| event.position);
        if self.pan.is_none() && self.pointer == pointer {
            return;
        }
        self.pointer = pointer;
        if let Some(pan) = self.pan
            && let Some(total) = self.sample_count()
        {
            if event.dragging() {
                let fraction = f32::from(pan.position.x - event.position.x) / pan.width;
                self.navigate(pan.view.pan(fraction as f64, total), cx);
            } else {
                self.pan = None;
            }
        }
        cx.notify();
    }

    pub(super) fn finish_pan(
        &mut self,
        _: &gpui::MouseUpEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.pan = None;
        cx.notify();
    }

    pub(super) fn cursor_readout(&self) -> Option<String> {
        let geometry = self.plot_geometry?;
        let pointer = self.pointer?;
        if !geometry.navigation.contains(&pointer) {
            return None;
        }
        let extents = self.extents()?;
        let x = f32::from(pointer.x - geometry.navigation.left()) as f64
            / f32::from(geometry.navigation.size.width) as f64;
        let time = extents.seconds.0 + x * (extents.seconds.1 - extents.seconds.0);
        let decimals = navigation::time_precision(
            (extents.seconds.1 - extents.seconds.0) / self.view_columns() as f64,
        );
        if !geometry.spectrum.contains(&pointer) {
            return Some(format!("{time:.decimals$} s"));
        }
        let y = f32::from(pointer.y - geometry.spectrum.top()) as f64
            / f32::from(geometry.spectrum.size.height) as f64;
        let frequency = extents.hertz.1 - y * (extents.hertz.1 - extents.hertz.0);
        let level = self
            .file
            .as_ref()?
            .document
            .analysis()
            .and_then(|analysis| navigation::level_at(&analysis.db, extents.seconds, x, y))
            .or_else(|| {
                self.backdrop
                    .as_ref()
                    .and_then(|backdrop| backdrop.level_at(extents.seconds, x, y))
            })
            .map_or_else(|| "—".into(), |db| format!("{db:.1} dBFS"));
        Some(format!("{time:.decimals$} s · {frequency:.1} Hz · {level}"))
    }

    pub(super) fn navigation_actions(
        &self,
        content: gpui::Stateful<gpui::Div>,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        content
            .on_action(cx.listener(|shell, _: &ZoomIn, _, cx| shell.zoom(0.5, 0.5, cx)))
            .on_action(cx.listener(|shell, _: &ZoomOut, _, cx| shell.zoom(2.0, 0.5, cx)))
            .on_action(
                cx.listener(|shell, _: &PanLeft, window, cx| shell.pan_ticks(-1, window, cx)),
            )
            .on_action(
                cx.listener(|shell, _: &PanRight, window, cx| shell.pan_ticks(1, window, cx)),
            )
            .on_action(
                cx.listener(|shell, _: &PanFarLeft, window, cx| shell.pan_ticks(-5, window, cx)),
            )
            .on_action(
                cx.listener(|shell, _: &PanFarRight, window, cx| shell.pan_ticks(5, window, cx)),
            )
            .on_action(
                cx.listener(|shell, _: &GoStart, window, cx| shell.pan_by(-1e20, window, cx)),
            )
            .on_action(cx.listener(|shell, _: &GoEnd, window, cx| shell.pan_by(1e20, window, cx)))
            .on_action(cx.listener(|shell, _: &FitCapture, _, cx| {
                if let Some(total) = shell.sample_count() {
                    shell.navigate(View::full(total), cx);
                }
            }))
    }

    pub(super) fn view_menu(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let focus = self.focus.clone();
        let owner = cx.entity().downgrade();
        Button::new("view-menu")
            .ghost()
            .small()
            .label("View")
            .dropdown_menu(move |menu, _, cx| {
                let popup = cx.entity().downgrade();
                let _ = owner.update(cx, |shell, _| shell.open_menu = Some(popup));
                menu.action_context(focus.clone())
                    .item(PopupMenuItem::new("Zoom in").action(Box::new(ZoomIn)))
                    .item(PopupMenuItem::new("Zoom out").action(Box::new(ZoomOut)))
                    .item(PopupMenuItem::new("Fit capture").action(Box::new(FitCapture)))
                    .separator()
                    .item(PopupMenuItem::new("Pan left").action(Box::new(PanLeft)))
                    .item(PopupMenuItem::new("Pan right").action(Box::new(PanRight)))
                    .item(
                        PopupMenuItem::new("Pan five divisions left").action(Box::new(PanFarLeft)),
                    )
                    .item(
                        PopupMenuItem::new("Pan five divisions right")
                            .action(Box::new(PanFarRight)),
                    )
                    .item(PopupMenuItem::new("Go to start").action(Box::new(GoStart)))
                    .item(PopupMenuItem::new("Go to end").action(Box::new(GoEnd)))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn geometry() -> PlotGeometry {
        PlotGeometry {
            time_scheme: None,
            spectrum: Bounds::new(point(px(10.), px(50.)), size(px(100.), px(100.))),
            navigation: Bounds::new(point(px(10.), px(10.)), size(px(100.), px(160.))),
        }
    }

    #[test]
    fn crosshair_is_limited_to_spectrum() {
        let geometry = geometry();
        for position in [
            point(px(50.), px(20.)),
            point(px(50.), px(160.)),
            point(px(120.), px(80.)),
        ] {
            assert_eq!(
                geometry.cursor(Some(position), false),
                gpui::CursorStyle::Arrow
            );
        }
        assert_eq!(
            geometry.cursor(Some(point(px(50.), px(80.))), false),
            gpui::CursorStyle::Crosshair
        );
        assert_eq!(geometry.cursor(None, true), gpui::CursorStyle::ClosedHand);
    }

    #[test]
    fn wheel_bindings_are_identical_on_spectrum_and_ruler() {
        let geometry = geometry();
        for position in [point(px(60.), px(80.)), point(px(60.), px(160.))] {
            let delta = point(px(0.), px(-160.));
            assert!(
                matches!(geometry.scroll(position, delta, gpui::Modifiers::default()), Scroll::Pan(f) if (f - 1.6).abs() < 1e-10)
            );
            let control = gpui::Modifiers {
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
            let shift = gpui::Modifiers {
                shift: true,
                ..Default::default()
            };
            assert!(matches!(
                geometry.scroll(position, delta, shift),
                Scroll::VerticalPending
            ));
        }
    }
}
