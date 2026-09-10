//! Time navigation gestures, actions and physical cursor readout.

use super::*;
use crate::navigation::{self, View};

actions!(
    navigation,
    [
        ZoomIn, ZoomOut, FitCapture, PanLeft, PanRight, GoStart, GoEnd
    ]
);

pub(super) fn init(cx: &mut gpui::App) {
    cx.bind_keys([
        KeyBinding::new("+", ZoomIn, Some("Plot")),
        KeyBinding::new("=", ZoomIn, Some("Plot")),
        KeyBinding::new("-", ZoomOut, Some("Plot")),
        KeyBinding::new("0", FitCapture, Some("Plot")),
        KeyBinding::new("left", PanLeft, Some("Plot")),
        KeyBinding::new("right", PanRight, Some("Plot")),
        KeyBinding::new("home", GoStart, Some("Plot")),
        KeyBinding::new("end", GoEnd, Some("Plot")),
    ]);
}

#[derive(Clone, Copy, PartialEq)]
pub(super) struct PlotGeometry {
    pub spectrum: Bounds<Pixels>,
    pub navigation: Bounds<Pixels>,
}

enum Scroll {
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
        shift: bool,
    ) -> Scroll {
        let horizontal = f32::from(delta.x).abs() > f32::from(delta.y).abs();
        let ruler = position.y >= self.spectrum.bottom();
        let width = f32::from(self.navigation.size.width) as f64;
        if ruler || shift || horizontal {
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
    pub(super) fn restore_view(&mut self) {
        let Some(file) = &self.file else { return };
        let Some(meta) = file.document.meta() else {
            return;
        };
        let path = std::path::absolute(&file.document.origin().path).ok();
        let saved = self
            .session
            .recent
            .iter()
            .find(|entry| Some(&entry.path) == path.as_ref())
            .and_then(|entry| entry.view);
        if let Some(settings) = self.settings_backup {
            self.settings_view_backup =
                Some(saved.unwrap_or(View::full(meta.len_samples)).bounded(
                    meta.len_samples,
                    settings.fft_size,
                    self.view_columns(),
                ));
        }
        self.view = Some(saved.unwrap_or(View::full(meta.len_samples)).bounded(
            meta.len_samples,
            self.settings.fft_size,
            self.view_columns(),
        ));
    }

    pub(super) fn bound_view(&mut self) {
        self.pan = None;
        if let Some(total) = self.sample_count()
            && let Some(view) = self.view
        {
            self.view = Some(view.bounded(total, self.settings.fft_size, self.view_columns()));
            self.remember_view();
        }
    }

    fn view_columns(&self) -> usize {
        self.plot.map_or(1, |plot| plot.width)
    }

    fn sample_count(&self) -> Option<u64> {
        Some(self.file.as_ref()?.document.meta()?.len_samples)
    }

    pub(super) fn remember_view(&mut self) {
        let Some(file) = &self.file else { return };
        let path = std::path::absolute(&file.document.origin().path).ok();
        if let Some(entry) = self
            .session
            .recent
            .iter_mut()
            .find(|entry| Some(&entry.path) == path.as_ref())
        {
            entry.view = self.view;
        }
    }

    fn navigate(&mut self, view: View, cx: &mut Context<Self>) {
        if self.view == Some(view) {
            return;
        }
        self.view = Some(view);
        if self.settings_backup.is_some() {
            self.settings_view_backup = Some(view);
        }
        self.remember_view();
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

    fn pan_by(&mut self, fraction: f64, cx: &mut Context<Self>) {
        if let Some(view) = self.view
            && let Some(total) = self.sample_count()
        {
            self.navigate(view.pan(fraction, total), cx);
        }
    }

    pub(super) fn wheel(
        &mut self,
        event: &gpui::ScrollWheelEvent,
        _: &mut Window,
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
        match geometry.scroll(event.position, delta, event.modifiers.shift) {
            Scroll::Pan(fraction) => self.pan_by(fraction, cx),
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
            .on_action(cx.listener(|shell, _: &PanLeft, _, cx| shell.pan_by(-0.1, cx)))
            .on_action(cx.listener(|shell, _: &PanRight, _, cx| shell.pan_by(0.1, cx)))
            .on_action(cx.listener(|shell, _: &GoStart, _, cx| shell.pan_by(-1e20, cx)))
            .on_action(cx.listener(|shell, _: &GoEnd, _, cx| shell.pan_by(1e20, cx)))
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
    fn ruler_accepts_drag_and_wheel_pan_without_zoom() {
        let geometry = geometry();
        let ruler = point(px(60.), px(160.));
        assert!(geometry.navigation.contains(&ruler));
        assert!(!geometry.navigation.contains(&point(px(120.), px(160.))));
        for delta in [point(px(0.), px(-40.)), point(px(-40.), px(0.))] {
            assert!(
                matches!(geometry.scroll(ruler, delta, false), Scroll::Pan(f) if (f - 0.4).abs() < 1e-10)
            );
        }
        assert!(matches!(
            geometry.scroll(point(px(60.), px(80.)), point(px(0.), px(-160.)), false),
            Scroll::Zoom {
                factor: 2.0,
                anchor: 0.5
            }
        ));
    }
}
