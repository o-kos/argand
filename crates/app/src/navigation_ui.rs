//! Time navigation gestures, actions and physical cursor readout.

use super::*;
use crate::navigation::{self, View};

actions!(
    navigation,
    [
        ToggleGrid,
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
        GoEnd,
        FrequencyZoomIn,
        FrequencyZoomOut,
        FitFrequency,
        PanUp,
        PanDown,
        PanFarUp,
        PanFarDown
    ]
);

pub(super) fn init(cx: &mut gpui::App) {
    cx.bind_keys([
        KeyBinding::new("ctrl-+", ZoomIn, Some("Plot")),
        KeyBinding::new("ctrl-=", ZoomIn, Some("Plot")),
        KeyBinding::new("ctrl--", ZoomOut, Some("Plot")),
        KeyBinding::new("ctrl-0", FitCapture, Some("Plot")),
        KeyBinding::new("left", PanLeft, Some("Plot && Horizontal")),
        KeyBinding::new("right", PanRight, Some("Plot && Horizontal")),
        KeyBinding::new("ctrl-left", PanFarLeft, Some("Plot && Horizontal")),
        KeyBinding::new("ctrl-right", PanFarRight, Some("Plot && Horizontal")),
        KeyBinding::new("up", PanLeft, Some("Plot && Vertical")),
        KeyBinding::new("down", PanRight, Some("Plot && Vertical")),
        KeyBinding::new("ctrl-up", PanFarLeft, Some("Plot && Vertical")),
        KeyBinding::new("ctrl-down", PanFarRight, Some("Plot && Vertical")),
        KeyBinding::new("left", PanDown, Some("Plot && Vertical")),
        KeyBinding::new("right", PanUp, Some("Plot && Vertical")),
        KeyBinding::new("ctrl-left", PanFarDown, Some("Plot && Vertical")),
        KeyBinding::new("ctrl-right", PanFarUp, Some("Plot && Vertical")),
        KeyBinding::new("home", GoStart, Some("Plot")),
        KeyBinding::new("end", GoEnd, Some("Plot")),
        KeyBinding::new("ctrl-shift-+", FrequencyZoomIn, Some("Plot")),
        KeyBinding::new("ctrl-shift--", FrequencyZoomOut, Some("Plot")),
        KeyBinding::new("ctrl-shift-home", FitFrequency, Some("Plot")),
        KeyBinding::new("up", PanUp, Some("Plot && Horizontal")),
        KeyBinding::new("down", PanDown, Some("Plot && Horizontal")),
        KeyBinding::new("ctrl-up", PanFarUp, Some("Plot && Horizontal")),
        KeyBinding::new("ctrl-down", PanFarDown, Some("Plot && Horizontal")),
    ]);
    cx.intercept_keystrokes(|event, window, cx| {
        if !event
            .context_stack
            .iter()
            .any(|context| context.contains("Plot"))
        {
            return;
        }
        // GPUI consumes Shift for symbols; the window retains its physical state.
        if let Some(action) = zoom_key(
            &event.keystroke.key,
            event.keystroke.modifiers,
            window.modifiers().shift,
        ) {
            // A popup can inherit Plot bindings; do not navigate behind it.
            if event
                .context_stack
                .last()
                .is_some_and(|context| context.contains("Plot"))
            {
                window.dispatch_action(action, cx);
            }
            cx.stop_propagation();
        }
    })
    .detach();
}

fn zoom_key(
    key: &str,
    modifiers: gpui::Modifiers,
    physical_shift: bool,
) -> Option<Box<dyn gpui::Action>> {
    if !modifiers.control || modifiers.alt || modifiers.platform {
        return None;
    }
    match (key, modifiers.shift || physical_shift) {
        ("+" | "=" | "add", false) => Some(Box::new(ZoomIn)),
        ("-" | "_" | "subtract", false) => Some(Box::new(ZoomOut)),
        ("+" | "=" | "add", true) => Some(Box::new(FrequencyZoomIn)),
        ("-" | "_" | "subtract", true) => Some(Box::new(FrequencyZoomOut)),
        _ => None,
    }
}

#[derive(Clone, Copy, PartialEq)]
pub(super) struct PlotGeometry {
    pub orientation: crate::orientation::Mode,
    pub time_ruler: Bounds<Pixels>,
    pub unit_hints: [Option<axes::UnitHint>; 2],
    pub time_scheme: Option<argand_core::axis::TickScheme>,
    pub frequency_scheme: Option<argand_core::axis::TickScheme>,
    pub spectrum: Bounds<Pixels>,
    pub minimap: Bounds<Pixels>,
    pub minimap_columns: usize,
    pub navigation: Bounds<Pixels>,
    pub frequency_ruler: Bounds<Pixels>,
}

#[derive(Debug, PartialEq)]
enum Scroll {
    FrequencyPan(f64),
    FrequencyZoom { factor: f64, anchor: f64 },
    Pan(f64),
    Zoom { factor: f64, anchor: f64 },
}

impl PlotGeometry {
    pub fn cursor(
        self,
        pointer: Option<gpui::Point<Pixels>>,
        dragging: bool,
        viewport: Option<(View, u64)>,
        frequency_zoomed: bool,
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
        let time_ruler = self.time_ruler.contains(&position);
        let selected = self.minimap.contains(&position)
            && viewport.is_some_and(|(view, total)| {
                let fraction = self.minimap_fraction(position);
                let (left, right) = crate::minimap::viewport(view, total, self.minimap_columns);
                (left..=right).contains(&fraction)
            });
        let can_pan = viewport.is_some_and(|(view, total)| view.len < total);
        if (can_pan && (time_ruler || selected))
            || (frequency_zoomed && self.frequency_ruler.contains(&position))
        {
            gpui::CursorStyle::OpenHand
        } else {
            gpui::CursorStyle::Arrow
        }
    }

    pub fn time_length(self) -> f32 {
        f32::from(
            self.orientation
                .axes(self.spectrum.size.width, self.spectrum.size.height)
                .0,
        )
    }

    pub fn frequency_length(self) -> f32 {
        f32::from(
            self.orientation
                .axes(self.spectrum.size.width, self.spectrum.size.height)
                .1,
        )
    }

    fn fractions(self, position: gpui::Point<Pixels>) -> (f64, f64) {
        self.orientation.fractions(
            f32::from(position.x - self.spectrum.left()) as f64
                / f32::from(self.spectrum.size.width) as f64,
            f32::from(position.y - self.spectrum.top()) as f64
                / f32::from(self.spectrum.size.height) as f64,
        )
    }

    fn minimap_fraction(self, position: gpui::Point<Pixels>) -> f64 {
        let delta = position - self.minimap.origin;
        f32::from(self.orientation.axes(delta.x, delta.y).0) as f64
            / f32::from(
                self.orientation
                    .axes(self.minimap.size.width, self.minimap.size.height)
                    .0,
            ) as f64
    }

    fn scroll(
        self,
        position: gpui::Point<Pixels>,
        delta: gpui::Point<Pixels>,
        modifiers: gpui::Modifiers,
    ) -> Scroll {
        // Linux backends turn Shift+wheel into a horizontal scroll delta.
        let distance = if f32::from(delta.x).abs() > f32::from(delta.y).abs() {
            delta.x
        } else {
            delta.y
        };
        let distance = f64::from(f32::from(distance));
        let width = self.time_length() as f64;
        let frequency_ruler = self.frequency_ruler.contains(&position);
        if modifiers.shift || frequency_ruler {
            let height = self.frequency_length() as f64;
            return if modifiers.control {
                Scroll::FrequencyZoom {
                    factor: 2.0_f64.powf(-distance / 160.0),
                    anchor: (1. - self.fractions(position).1).clamp(0., 1.),
                }
            } else {
                Scroll::FrequencyPan(distance / height)
            };
        }
        if !modifiers.control {
            return Scroll::Pan(-distance / width);
        }
        Scroll::Zoom {
            factor: 2.0_f64.powf(-distance / 160.0),
            anchor: self.fractions(position).0.clamp(0., 1.),
        }
    }

    fn drag_axes(self, position: gpui::Point<Pixels>) -> (bool, bool) {
        let frequency_ruler = self.frequency_ruler.contains(&position);
        (
            self.navigation.contains(&position) && !frequency_ruler,
            self.spectrum.contains(&position) || frequency_ruler,
        )
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
        self.frequency = crate::frequency::View::default();
        self.frequency_pan = None;
        self.frequency_scheme = None;
        self.time_scheme = None;
        self.tick_pan = None;
        if self.settings_backup.is_some() {
            self.settings_view_backup = self.view;
            self.settings_frequency_backup = Some(self.frequency);
        }
    }

    pub(super) fn bound_view(&mut self) {
        self.pan = None;
        self.frequency_pan = None;
        let bounded = self.frequency.zoom(1., 0.5, self.frequency_cells());
        if bounded != self.frequency {
            self.frequency = bounded;
            self.frequency_scheme = None;
        }
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
        if self.set_time_view(view) {
            self.ask_for_a_picture();
            cx.notify();
        }
    }

    fn set_time_view(&mut self, view: View) -> bool {
        if self.view == Some(view) {
            return false;
        }
        if self.view.is_none_or(|old| old.len != view.len) {
            self.time_scheme = None;
        }
        self.tick_pan = None;
        self.view = Some(view);
        if self.settings_backup.is_some() {
            self.settings_view_backup = Some(view);
        }
        tracing::debug!(start = view.start, len = view.len, "time view requested");
        true
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
        if !(geometry.navigation.contains(&event.position)
            || geometry.frequency_ruler.contains(&event.position))
            || self.splitter_dragging
        {
            return;
        }
        if geometry.minimap.contains(&event.position) {
            return;
        }
        self.pan = None;
        self.frequency_pan = None;
        let delta = event.delta.pixel_delta(px(40.));
        match geometry.scroll(event.position, delta, event.modifiers) {
            Scroll::FrequencyPan(fraction) => self.pan_frequency(fraction, cx),
            Scroll::FrequencyZoom { factor, anchor } => self.zoom_frequency(factor, anchor, cx),
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
        if !(geometry.navigation.contains(&event.position)
            || geometry.frequency_ruler.contains(&event.position))
            || self.splitter_dragging
        {
            return;
        }
        window.focus(&self.focus);
        self.pan = None;
        self.frequency_pan = None;
        let (time, frequency) = geometry.drag_axes(event.position);
        if frequency && self.frequency.span < 1. {
            self.hold_frequency_scheme();
            self.frequency_pan = Some((event.position, self.frequency));
            cx.notify();
        }
        if !time {
            return;
        }
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
            width: geometry.time_length(),
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
        if self.pan.is_none() && self.frequency_pan.is_none() && self.pointer == pointer {
            return;
        }
        self.pointer = pointer;
        let mut changed = false;
        if let Some((origin, view)) = self.frequency_pan {
            if event.dragging() {
                if let Some(geometry) = self.plot_geometry {
                    let fraction =
                        geometry.fractions(event.position).1 - geometry.fractions(origin).1;
                    changed |= self.set_frequency_view(view.pan(fraction));
                }
            } else {
                self.frequency_pan = None;
            }
        }
        if let Some(pan) = self.pan
            && let Some(total) = self.sample_count()
        {
            if event.dragging() {
                let delta = pan.position - event.position;
                let fraction =
                    f32::from(self.session.orientation.axes(delta.x, delta.y).0) / pan.width;
                let fraction = if pan.minimap {
                    -f64::from(fraction) * total as f64 / pan.view.len.max(1) as f64
                } else {
                    f64::from(fraction)
                };
                changed |= self.set_time_view(pan.view.pan(fraction, total));
            } else {
                self.pan = None;
            }
        }
        if changed {
            self.ask_for_a_picture();
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
        self.frequency_pan = None;
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
        let (x, y) = geometry.fractions(pointer);
        let time = extents.seconds.0 + x * (extents.seconds.1 - extents.seconds.0);
        let decimals = navigation::time_precision(
            (extents.seconds.1 - extents.seconds.0) / self.view_columns() as f64,
        );
        if !geometry.spectrum.contains(&pointer) {
            return Some(crate::numbers::text(&format!("{time:.decimals$} s")));
        }
        let frequency = extents.hertz.1 - y * (extents.hertz.1 - extents.hertz.0);
        let per_pixel = (extents.hertz.1 - extents.hertz.0) / self.plot?.height.max(1) as f64;
        let frequency_decimals = (-per_pixel.log10()).ceil().clamp(1., 9.) as usize;
        let level = self
            .file
            .as_ref()?
            .document
            .analysis()
            .and_then(|analysis| navigation::level_in_view(&analysis.db, extents.picture(), x, y))
            .or_else(|| {
                self.backdrop
                    .as_ref()
                    .and_then(|backdrop| backdrop.level_at(extents.picture(), x, y))
            })
            .map_or_else(
                || "—".into(),
                |db| crate::numbers::text(&format!("{db:.1} dBFS")),
            );
        Some(format!(
            "{} · {level}",
            crate::numbers::text(&format!(
                "{time:.decimals$} s · {frequency:.frequency_decimals$} Hz"
            ))
        ))
    }

    fn frequency_cells(&self) -> usize {
        let Some(meta) = self.file.as_ref().and_then(|file| file.document.meta()) else {
            return 1;
        };
        let bins = if meta.is_iq() {
            self.settings.fft_size
        } else {
            self.settings.fft_size / 2 + 1
        };
        crate::frequency::cells(meta.frequency_span(), bins)
    }

    fn navigate_frequency(&mut self, view: crate::frequency::View, cx: &mut Context<Self>) {
        if self.set_frequency_view(view) {
            self.ask_for_a_picture();
            cx.notify();
        }
    }

    fn set_frequency_view(&mut self, view: crate::frequency::View) -> bool {
        if self.frequency == view {
            return false;
        }
        if self.frequency.span != view.span {
            self.frequency_scheme = None;
        }
        self.frequency = view;
        if self.settings_backup.is_some() {
            self.settings_frequency_backup = Some(view);
        }
        tracing::debug!(
            start = view.start,
            span = view.span,
            "frequency view requested"
        );
        true
    }

    fn zoom_frequency(&mut self, factor: f64, anchor: f64, cx: &mut Context<Self>) {
        if self.extents().is_some() {
            self.navigate_frequency(
                self.frequency.zoom(factor, anchor, self.frequency_cells()),
                cx,
            );
        }
    }

    fn hold_frequency_scheme(&mut self) {
        if self.frequency_scheme.is_none() {
            self.frequency_scheme = self
                .plot_geometry
                .and_then(|geometry| geometry.frequency_scheme);
        }
    }

    fn pan_frequency(&mut self, fraction: f64, cx: &mut Context<Self>) {
        self.hold_frequency_scheme();
        self.navigate_frequency(self.frequency.pan(fraction), cx);
    }

    fn frequency_ticks(&mut self, divisions: f64, cx: &mut Context<Self>) {
        self.hold_frequency_scheme();
        if let Some(scheme) = self.frequency_scheme
            && let Some(extents) = self.extents()
        {
            self.pan_frequency(
                divisions * scheme.step / (extents.hertz.1 - extents.hertz.0),
                cx,
            );
        }
    }

    pub(super) fn navigation_actions(
        &self,
        content: gpui::Stateful<gpui::Div>,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        content
            .on_action(cx.listener(|shell, _: &ToggleGrid, _, cx| {
                shell.session.show_grid = !shell.session.show_grid;
                shell.save();
                cx.notify();
            }))
            .on_action(
                cx.listener(|shell, _: &FrequencyZoomIn, _, cx| shell.zoom_frequency(0.5, 0.5, cx)),
            )
            .on_action(
                cx.listener(|shell, _: &FrequencyZoomOut, _, cx| shell.zoom_frequency(2., 0.5, cx)),
            )
            .on_action(cx.listener(|shell, _: &FitFrequency, _, cx| {
                shell.navigate_frequency(crate::frequency::View::default(), cx)
            }))
            .on_action(cx.listener(|shell, _: &PanUp, _, cx| shell.frequency_ticks(1., cx)))
            .on_action(cx.listener(|shell, _: &PanDown, _, cx| shell.frequency_ticks(-1., cx)))
            .on_action(cx.listener(|shell, _: &PanFarUp, _, cx| shell.frequency_ticks(5., cx)))
            .on_action(cx.listener(|shell, _: &PanFarDown, _, cx| shell.frequency_ticks(-5., cx)))
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
                .left(geometry.time_ruler.left() - panel.left())
                .top(geometry.time_ruler.top() - panel.top())
                .w(if geometry.orientation.vertical() {
                    geometry.time_ruler.size.width
                } else {
                    panel.right() - geometry.time_ruler.left()
                })
                .h(if geometry.orientation.vertical() {
                    panel.bottom() - geometry.time_ruler.top()
                } else {
                    geometry.time_ruler.size.height
                })
                .children(self.unit_hint(0, geometry.time_ruler.origin, cx))
                .context_menu(move |menu, window, cx| {
                    let popup = cx.entity();
                    let _ = owner.update(cx, |shell, cx| shell.track_time_menu(&popup, window, cx));
                    time_scale_items(menu.action_context(focus.clone()), mode)
                })
                .into_any_element(),
        )
    }

    pub(super) fn orientation_button(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let mode = self.session.orientation;
        Button::new("spectrogram-orientation")
            .ghost()
            .small()
            .label(mode.label())
            .tooltip(format!(
                "Switch to {} spectrogram",
                mode.toggled().label().to_lowercase()
            ))
            .on_click(cx.listener(|shell, _, window, cx| {
                shell.session.orientation = shell.session.orientation.toggled();
                shell.time_scheme = None;
                shell.frequency_scheme = None;
                shell.tick_pan = None;
                shell.pan = None;
                shell.frequency_pan = None;
                shell.splitter_dragging = false;
                shell.pointer = None;
                shell.plot_geometry = None;
                shell.upload(window);
                shell.upload_pending = false;
                shell.orient_backdrop(window);
                shell.save();
                cx.notify();
            }))
    }

    pub(super) fn view_menu(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let focus = self.focus.clone();
        let owner = cx.entity().downgrade();
        let ruler = self.session.time_ruler;
        let show_grid = self.session.show_grid;
        Button::new("view-menu")
            .ghost()
            .small()
            .label("View")
            .dropdown_menu(move |menu, window, cx| {
                let popup = cx.entity().downgrade();
                let _ = owner.update(cx, |shell, _| shell.open_menu = Some(popup));
                let submenu_focus = focus.clone();
                let frequency_focus = focus.clone();
                menu.action_context(focus.clone())
                    .item(
                        PopupMenuItem::new("Show grid")
                            .checked(show_grid)
                            .action(Box::new(ToggleGrid)),
                    )
                    .separator()
                    .submenu("Time scale format", window, cx, move |menu, _, _| {
                        time_scale_items(menu.action_context(submenu_focus.clone()), ruler)
                    })
                    .separator()
                    .submenu("Frequency", window, cx, move |menu, _, _| {
                        menu.action_context(frequency_focus.clone())
                            .item(PopupMenuItem::new("Zoom in").action(Box::new(FrequencyZoomIn)))
                            .item(PopupMenuItem::new("Zoom out").action(Box::new(FrequencyZoomOut)))
                            .item(
                                PopupMenuItem::new("Fit frequency range")
                                    .action(Box::new(FitFrequency)),
                            )
                            .separator()
                            .item(PopupMenuItem::new("Higher frequency").action(Box::new(PanUp)))
                            .item(PopupMenuItem::new("Lower frequency").action(Box::new(PanDown)))
                            .item(
                                PopupMenuItem::new("Five frequency divisions higher")
                                    .action(Box::new(PanFarUp)),
                            )
                            .item(
                                PopupMenuItem::new("Five frequency divisions lower")
                                    .action(Box::new(PanFarDown)),
                            )
                    })
                    .item(PopupMenuItem::new("Zoom in").action(Box::new(ZoomIn)))
                    .item(PopupMenuItem::new("Zoom out").action(Box::new(ZoomOut)))
                    .item(PopupMenuItem::new("Fit capture").action(Box::new(FitCapture)))
                    .separator()
                    .item(PopupMenuItem::new("Earlier in time").action(Box::new(PanLeft)))
                    .item(PopupMenuItem::new("Later in time").action(Box::new(PanRight)))
                    .item(
                        PopupMenuItem::new("Five time divisions earlier")
                            .action(Box::new(PanFarLeft)),
                    )
                    .item(
                        PopupMenuItem::new("Five time divisions later")
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

    #[test]
    fn zoom_symbols_use_physical_shift_without_leaking_into_time_zoom() {
        let control = gpui::Modifiers {
            control: true,
            ..Default::default()
        };
        let shifted = gpui::Modifiers {
            shift: true,
            ..control
        };
        for key in ["+", "=", "add"] {
            assert!(
                zoom_key(key, control, false)
                    .unwrap()
                    .as_any()
                    .is::<ZoomIn>()
            );
            assert!(
                zoom_key(key, control, true)
                    .unwrap()
                    .as_any()
                    .is::<FrequencyZoomIn>()
            );
            assert!(
                zoom_key(key, shifted, false)
                    .unwrap()
                    .as_any()
                    .is::<FrequencyZoomIn>()
            );
        }
        for key in ["-", "_", "subtract"] {
            assert!(
                zoom_key(key, control, false)
                    .unwrap()
                    .as_any()
                    .is::<ZoomOut>()
            );
            assert!(
                zoom_key(key, control, true)
                    .unwrap()
                    .as_any()
                    .is::<FrequencyZoomOut>()
            );
            assert!(
                zoom_key(key, shifted, false)
                    .unwrap()
                    .as_any()
                    .is::<FrequencyZoomOut>()
            );
        }
        for modifiers in [
            gpui::Modifiers::default(),
            gpui::Modifiers {
                alt: true,
                ..control
            },
            gpui::Modifiers {
                platform: true,
                ..control
            },
        ] {
            assert!(zoom_key("+", modifiers, true).is_none());
        }
        for key in ["up", "down", "home", "0", "a"] {
            assert!(zoom_key(key, control, true).is_none());
        }
    }

    fn geometry() -> PlotGeometry {
        PlotGeometry {
            orientation: crate::orientation::Mode::Horizontal,
            time_ruler: Bounds::new(point(px(10.), px(150.)), size(px(100.), px(20.))),
            unit_hints: [None; 2],
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
                geometry.cursor(Some(point(px(x), px(y))), false, viewport, false),
                expected
            );
        }
        assert_eq!(
            geometry.cursor(None, true, viewport, false),
            gpui::CursorStyle::ClosedHand
        );
        assert_eq!(
            geometry.cursor(None, false, viewport, false),
            gpui::CursorStyle::Arrow
        );
        assert_eq!(
            geometry.cursor(Some(point(px(50.), px(20.))), false, None, false),
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
            gpui::CursorStyle::OpenHand
        );
        let delta = point(px(0.), px(40.));
        assert!(
            matches!(geometry.scroll(ruler, delta, gpui::Modifiers::default()), Scroll::FrequencyPan(f) if (f - 0.4).abs() < 1e-10)
        );
        let control = gpui::Modifiers {
            control: true,
            ..Default::default()
        };
        assert!(matches!(
            geometry.scroll(ruler, delta, control),
            Scroll::FrequencyZoom { anchor: 0.75, .. }
        ));
        let shift_control = gpui::Modifiers {
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
                    let modifiers = gpui::Modifiers {
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
        let control = gpui::Modifiers {
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
            gpui::CursorStyle::OpenHand
        );
        assert_eq!(
            geometry.cursor(Some(point(px(175.), px(60.))), false, viewport, true),
            gpui::CursorStyle::OpenHand
        );
        assert_eq!(
            geometry.cursor(Some(point(px(75.), px(220.))), false, viewport, true),
            gpui::CursorStyle::OpenHand
        );
    }
}
