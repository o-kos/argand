//! Time navigation gestures, actions and physical cursor readout.

use super::*;
use crate::navigation::{self, View};

actions!(
    navigation,
    [
        ClockRuler,
        SecondsRuler,
        SamplesRuler,
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
    pub unit_hints: [Option<axes::UnitHint>; 2],
    pub time_scheme: Option<argand_core::axis::TickScheme>,
    pub spectrum: Bounds<Pixels>,
    pub minimap: Bounds<Pixels>,
    pub minimap_columns: usize,
    pub navigation: Bounds<Pixels>,
    pub frequency_ruler: Bounds<Pixels>,
}

enum Scroll {
    VerticalPending,
    Pan(f64),
    Zoom { factor: f64, anchor: f64 },
}

impl PlotGeometry {
    pub fn cursor(
        self,
        pointer: Option<gpui::Point<Pixels>>,
        dragging: bool,
        viewport: Option<(View, u64)>,
    ) -> gpui::CursorStyle {
        if dragging {
            return gpui::CursorStyle::ClosedHand;
        }
        let Some(position) = pointer else {
            return gpui::CursorStyle::Arrow;
        };
        if self.spectrum.contains(&position) {
            return gpui::CursorStyle::Crosshair;
        }
        let time_ruler = self.navigation.contains(&position) && position.y > self.spectrum.bottom();
        let selected = self.minimap.contains(&position)
            && viewport.is_some_and(|(view, total)| {
                let fraction = self.minimap_fraction(position);
                let (left, right) = crate::minimap::viewport(view, total, self.minimap_columns);
                (left..=right).contains(&fraction)
            });
        let can_pan = viewport.is_some_and(|(view, total)| view.len < total);
        if can_pan && (time_ruler || selected) {
            gpui::CursorStyle::OpenHand
        } else {
            gpui::CursorStyle::Arrow
        }
    }

    fn minimap_fraction(self, position: gpui::Point<Pixels>) -> f64 {
        f32::from(position.x - self.minimap.left()) as f64
            / f32::from(self.minimap.size.width) as f64
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
    minimap: bool,
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
        let step = self.session.time_ruler.sample_step(scheme.step, rate);
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
        if geometry.minimap.contains(&event.position) {
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
        if self
            .view
            .zip(self.sample_count())
            .is_none_or(|(view, total)| view.len >= total)
        {
            return;
        }
        self.hold_time_scheme(window);
        let minimap = geometry.minimap.contains(&event.position);
        if minimap && !self.minimap_press(event, geometry, window, cx) {
            self.pan = None;
            return;
        }
        self.pan = self.view.map(|view| Pan {
            position: event.position,
            view,
            width: f32::from(geometry.navigation.size.width),
            minimap,
        });
        cx.notify();
    }

    fn minimap_press(
        &mut self,
        event: &gpui::MouseDownEvent,
        geometry: PlotGeometry,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(view) = self.view else { return false };
        let Some(total) = self.sample_count() else {
            return false;
        };
        let fraction = geometry.minimap_fraction(event.position);
        let viewport = crate::minimap::viewport(view, total, geometry.minimap_columns);
        match crate::minimap::click(
            fraction,
            viewport,
            event.modifiers.control,
            event.click_count,
        ) {
            crate::minimap::Click::Grab => return true,
            crate::minimap::Click::Step(divisions) => self.pan_ticks(divisions, window, cx),
            crate::minimap::Click::Center => {
                self.navigate(crate::minimap::center(view, fraction, total), cx)
            }
        }
        false
    }

    fn plot_pointer(&self, position: gpui::Point<Pixels>) -> Option<gpui::Point<Pixels>> {
        self.plot_geometry
            .filter(|geometry| {
                geometry.navigation.contains(&position)
                    || geometry.frequency_ruler.contains(&position)
            })
            .map(|_| position)
    }

    pub(super) fn pointer_moved(
        &mut self,
        event: &gpui::MouseMoveEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let pointer = self.plot_pointer(event.position);
        if self.pan.is_none() && self.pointer == pointer {
            return;
        }
        self.pointer = pointer;
        if let Some(pan) = self.pan
            && let Some(total) = self.sample_count()
        {
            if event.dragging() {
                let fraction = f32::from(pan.position.x - event.position.x) / pan.width;
                let fraction = if pan.minimap {
                    -f64::from(fraction) * total as f64 / pan.view.len.max(1) as f64
                } else {
                    f64::from(fraction)
                };
                self.navigate(pan.view.pan(fraction, total), cx);
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
        let mut extents = self.extents()?;
        if geometry.minimap.contains(&pointer) {
            let meta = self.file.as_ref()?.document.meta()?;
            extents.seconds = (0., meta.duration_seconds());
        }
        let x = f32::from(pointer.x - geometry.navigation.left()) as f64
            / f32::from(geometry.navigation.size.width) as f64;
        let time = extents.seconds.0 + x * (extents.seconds.1 - extents.seconds.0);
        let decimals = navigation::time_precision(
            (extents.seconds.1 - extents.seconds.0) / self.view_columns() as f64,
        );
        if !geometry.spectrum.contains(&pointer) {
            return Some(crate::numbers::text(&format!("{time:.decimals$} s")));
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
            .map_or_else(
                || "—".into(),
                |db| crate::numbers::text(&format!("{db:.1} dBFS")),
            );
        Some(format!(
            "{} · {level}",
            crate::numbers::text(&format!("{time:.decimals$} s · {frequency:.1} Hz"))
        ))
    }

    pub(super) fn navigation_actions(
        &self,
        content: gpui::Stateful<gpui::Div>,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        content
            .on_action(cx.listener(|shell, _: &ClockRuler, _, cx| {
                shell.set_time_ruler(crate::time_ruler::Mode::Clock, cx)
            }))
            .on_action(cx.listener(|shell, _: &SecondsRuler, _, cx| {
                shell.set_time_ruler(crate::time_ruler::Mode::Seconds, cx)
            }))
            .on_action(cx.listener(|shell, _: &SamplesRuler, _, cx| {
                shell.set_time_ruler(crate::time_ruler::Mode::Samples, cx)
            }))
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

    fn set_time_ruler(&mut self, mode: crate::time_ruler::Mode, cx: &mut Context<Self>) {
        if self.session.time_ruler == mode {
            return;
        }
        self.session.time_ruler = mode;
        self.time_scheme = None;
        self.tick_pan = None;
        self.save();
        cx.notify();
    }

    fn track_time_menu(
        &mut self,
        menu: &gpui::Entity<PopupMenu>,
        window: &Window,
        cx: &mut Context<Self>,
    ) {
        self.open_menu = Some(menu.downgrade());
        self.menu_dismiss = Some(cx.subscribe_in(menu, window, Self::time_menu_dismissed));
    }

    fn time_menu_dismissed(
        &mut self,
        menu: &gpui::Entity<PopupMenu>,
        _: &gpui::DismissEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self
            .open_menu
            .as_ref()
            .is_some_and(|open| open.entity_id() == menu.entity_id())
        {
            self.open_menu = None;
            self.pointer = self.plot_pointer(window.mouse_position());
            cx.notify();
        }
    }

    pub(super) fn time_context_menu(&self, cx: &mut Context<Self>) -> Option<gpui::AnyElement> {
        use gpui_component::menu::ContextMenuExt;
        let geometry = self.plot_geometry?;
        let panel = self.panel_bounds?;
        let focus = self.focus.clone();
        let owner = cx.entity().downgrade();
        let mode = self.session.time_ruler;
        Some(
            div()
                .id("time-scale-context")
                .absolute()
                .left(geometry.navigation.left() - panel.left())
                .top(geometry.spectrum.bottom() - panel.top())
                .w(geometry.frequency_ruler.right() - geometry.navigation.left())
                .h(geometry.navigation.bottom() - geometry.spectrum.bottom())
                .children(self.unit_hint(
                    0,
                    point(geometry.navigation.left(), geometry.spectrum.bottom()),
                    cx,
                ))
                .context_menu(move |menu, window, cx| {
                    let popup = cx.entity();
                    let _ = owner.update(cx, |shell, cx| shell.track_time_menu(&popup, window, cx));
                    time_scale_items(menu.action_context(focus.clone()), mode)
                })
                .into_any_element(),
        )
    }

    pub(super) fn view_menu(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let focus = self.focus.clone();
        let owner = cx.entity().downgrade();
        let ruler = self.session.time_ruler;
        Button::new("view-menu")
            .ghost()
            .small()
            .label("View")
            .dropdown_menu(move |menu, window, cx| {
                let popup = cx.entity().downgrade();
                let _ = owner.update(cx, |shell, _| shell.open_menu = Some(popup));
                let submenu_focus = focus.clone();
                menu.action_context(focus.clone())
                    .submenu("Time scale format", window, cx, move |menu, _, _| {
                        time_scale_items(menu.action_context(submenu_focus.clone()), ruler)
                    })
                    .separator()
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

fn time_scale_items(
    menu: gpui_component::menu::PopupMenu,
    mode: crate::time_ruler::Mode,
) -> gpui_component::menu::PopupMenu {
    use crate::time_ruler::Mode;
    menu.item(
        PopupMenuItem::new("Hours, minutes, seconds (hms)")
            .checked(mode == Mode::Clock)
            .action(Box::new(ClockRuler)),
    )
    .item(
        PopupMenuItem::new("Seconds")
            .checked(mode == Mode::Seconds)
            .action(Box::new(SecondsRuler)),
    )
    .item(
        PopupMenuItem::new("Sample numbers")
            .checked(mode == Mode::Samples)
            .action(Box::new(SamplesRuler)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn geometry() -> PlotGeometry {
        PlotGeometry {
            unit_hints: [None; 2],
            time_scheme: None,
            minimap_columns: 100,
            frequency_ruler: Bounds::new(point(px(110.), px(50.)), size(px(30.), px(100.))),
            spectrum: Bounds::new(point(px(10.), px(50.)), size(px(100.), px(100.))),
            navigation: Bounds::new(point(px(10.), px(10.)), size(px(100.), px(160.))),
            minimap: Bounds::new(point(px(10.), px(10.)), size(px(100.), px(30.))),
        }
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
            (50., 20., gpui::CursorStyle::OpenHand),
            (20., 20., gpui::CursorStyle::Arrow),
            (50., 160., gpui::CursorStyle::OpenHand),
            (120., 80., gpui::CursorStyle::Arrow),
            (50., 80., gpui::CursorStyle::Crosshair),
        ] {
            assert_eq!(
                geometry.cursor(Some(point(px(x), px(y))), false, viewport),
                expected
            );
        }
        assert_eq!(
            geometry.cursor(None, true, viewport),
            gpui::CursorStyle::ClosedHand
        );
        assert_eq!(
            geometry.cursor(None, false, viewport),
            gpui::CursorStyle::Arrow
        );
        assert_eq!(
            geometry.cursor(Some(point(px(50.), px(20.))), false, None),
            gpui::CursorStyle::Arrow
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
                gpui::CursorStyle::Arrow,
                crate::minimap::Click::Step(-1),
            ),
            (
                109.75,
                gpui::CursorStyle::OpenHand,
                crate::minimap::Click::Grab,
            ),
        ] {
            let position = point(px(x), px(20.));
            assert_eq!(geometry.cursor(Some(position), false, viewport), cursor);
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
                    geometry.cursor(Some(point(px(x), px(y))), false, viewport),
                    gpui::CursorStyle::Arrow,
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
