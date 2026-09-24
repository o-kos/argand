//! Time navigation gestures, actions and physical cursor readout.

use super::*;
use crate::navigation::{self, View};

actions!(
    navigation,
    [
        ToggleGrid,
        ToggleScaleUi,
        ToggleOrientation,
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

pub(super) fn init(cx: &mut gpui_kit::App) {
    cx.bind_keys([
        KeyBinding::new("ctrl-g", ToggleGrid, Some("Plot")),
        KeyBinding::new("ctrl-u", ToggleScaleUi, Some("Plot")),
        KeyBinding::new("ctrl-t", ToggleOrientation, Some("Plot")),
        KeyBinding::new("ctrl-+", ZoomIn, Some("Plot")),
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
        KeyBinding::new("ctrl-shift-0", FitFrequency, Some("Plot")),
        KeyBinding::new("up", PanUp, Some("Plot && Horizontal")),
        KeyBinding::new("down", PanDown, Some("Plot && Horizontal")),
        KeyBinding::new("ctrl-up", PanFarUp, Some("Plot && Horizontal")),
        KeyBinding::new("ctrl-down", PanFarDown, Some("Plot && Horizontal")),
    ]);
}

pub(super) fn plot_shortcut(
    key: &str,
    modifiers: gpui_kit::Modifiers,
    physical_shift: bool,
) -> Option<Box<dyn gpui_kit::Action>> {
    if !modifiers.control || modifiers.alt || modifiers.platform {
        return None;
    }
    match (key, modifiers.shift || physical_shift) {
        ("+" | "=" | "add", false) => Some(Box::new(ZoomIn)),
        ("-" | "_" | "subtract", false) => Some(Box::new(ZoomOut)),
        ("+" | "=" | "add", true) => Some(Box::new(FrequencyZoomIn)),
        ("-" | "_" | "subtract", true) => Some(Box::new(FrequencyZoomOut)),
        ("0" | ")", true) => Some(Box::new(FitFrequency)),
        _ => None,
    }
}

#[derive(Clone, Copy, PartialEq)]
pub(super) struct PlotGeometry {
    pub orientation: crate::orientation::Mode,
    pub scale: f32,
    pub time_ruler: Bounds<Pixels>,
    pub unit_hints: [Option<axes::UnitHint>; 2],
    pub zoom_zones: [Option<axes::Rect>; 2],
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
        pointer: Option<gpui_kit::Point<Pixels>>,
        dragging: bool,
        viewport: Option<(View, u64)>,
        frequency_zoomed: bool,
    ) -> gpui_kit::CursorStyle {
        if dragging {
            return gpui_kit::CursorStyle::ClosedHand;
        }
        let Some(position) = pointer else {
            return gpui_kit::CursorStyle::Arrow;
        };
        if self.controls_at(position) {
            return gpui_kit::CursorStyle::Arrow;
        }
        if self.spectrum.contains(&position) {
            return gpui_kit::CursorStyle::Crosshair;
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
            gpui_kit::CursorStyle::OpenHand
        } else {
            gpui_kit::CursorStyle::Arrow
        }
    }

    fn unit_at(self, position: gpui_kit::Point<Pixels>) -> bool {
        self.unit_hints
            .iter()
            .flatten()
            .any(|hint| rect_contains(hint.bounds, position))
    }

    fn controls_at(self, position: gpui_kit::Point<Pixels>) -> bool {
        self.unit_at(position)
            || self
                .zoom_zones
                .iter()
                .flatten()
                .any(|zone| rect_contains(*zone, position))
    }

    /// Whether the pointer sits on a corner scale button, wherever it is.
    /// Guides and gestures keep off these squares.
    pub fn over_scale_buttons(self, pointer: Option<gpui_kit::Point<Pixels>>) -> bool {
        pointer.is_some_and(|position| {
            self.zoom_zones
                .iter()
                .flatten()
                .any(|zone| rect_contains(*zone, position))
        })
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

    fn fractions(self, position: gpui_kit::Point<Pixels>) -> (f64, f64) {
        self.orientation.fractions(
            f32::from(position.x - self.spectrum.left()) as f64
                / f32::from(self.spectrum.size.width) as f64,
            f32::from(position.y - self.spectrum.top()) as f64
                / f32::from(self.spectrum.size.height) as f64,
        )
    }

    fn minimap_fraction(self, position: gpui_kit::Point<Pixels>) -> f64 {
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
        position: gpui_kit::Point<Pixels>,
        delta: gpui_kit::Point<Pixels>,
        modifiers: gpui_kit::Modifiers,
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

    fn drag_axes(self, position: gpui_kit::Point<Pixels>) -> (bool, bool) {
        if self.controls_at(position) {
            return (false, false);
        }
        let frequency_ruler = self.frequency_ruler.contains(&position);
        (
            self.navigation.contains(&position) && !frequency_ruler,
            self.spectrum.contains(&position) || frequency_ruler,
        )
    }
}

#[derive(Clone, Copy)]
pub(super) struct Pan {
    position: gpui_kit::Point<Pixels>,
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
        self.frequency_scheme = None;
        self.time_scheme = None;
        self.tick_pan = None;
        if self.settings_backup.is_some() {
            self.settings_view_backup = self.view;
            self.settings_frequency_backup = Some(self.frequency);
        }
    }

    pub(super) fn bound_view(&mut self, cx: &mut gpui_kit::App) {
        if let Some(plot) = self.plot_entity() {
            plot.update(cx, |plot, cx| plot.cancel_drags(cx));
        }
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

    /// Whether a view the plot asked for lies within the open capture.
    fn within_capture(&self, view: View) -> bool {
        self.sample_count()
            .is_some_and(|total| view.len > 0 && view.start.saturating_add(view.len) <= total)
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

    fn hold_time_scheme(&mut self, window: &Window, cx: &gpui_kit::App) {
        if self.time_scheme.is_none() {
            self.time_scheme = self.measure_time_scheme(window, cx);
        }
    }

    fn pan_ticks(&mut self, divisions: i64, window: &Window, cx: &mut Context<Self>) {
        let Some(view) = self.view else { return };
        let Some(meta) = self.file.as_ref().and_then(|file| file.document.meta()) else {
            return;
        };
        let total = meta.len_samples;
        let rate = meta.sample_rate;
        self.hold_time_scheme(window, cx);
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
        self.hold_time_scheme(window, cx);
        if let Some(view) = self.view
            && let Some(total) = self.sample_count()
        {
            self.navigate(view.pan(fraction, total), cx);
        }
    }

    /// Act on what the plot asked for, accepting only views within this capture.
    pub(super) fn apply_plot_intent(
        &mut self,
        _: &gpui_kit::Entity<plot_view::PlotView>,
        intent: &plot_view::PlotIntent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        use plot_view::{FrequencyIntent, PlotIntent, TimeIntent};
        match *intent {
            PlotIntent::Layout {
                plot,
                time_length_changed,
                frequency_length_changed,
            } => {
                if time_length_changed {
                    self.time_scheme = None;
                    self.tick_pan = None;
                }
                if frequency_length_changed {
                    self.frequency_scheme = None;
                }
                if self.plot != Some(plot) {
                    self.resize(plot, cx);
                }
                cx.notify();
            }
            PlotIntent::Pointer => {
                if !self.ready_status_dismissed && self.cursor_readout(cx).is_some() {
                    self.dismiss_ready_status(cx);
                }
                cx.notify();
            }
            PlotIntent::GestureStarted { time, frequency } => {
                if frequency {
                    self.hold_frequency_scheme(cx);
                }
                if time {
                    self.hold_time_scheme(window, cx);
                }
            }
            PlotIntent::Time(TimeIntent::Zoom { factor, anchor }) => self.zoom(factor, anchor, cx),
            PlotIntent::Time(TimeIntent::Pan(fraction)) => self.pan_by(fraction, window, cx),
            PlotIntent::Time(TimeIntent::Ticks(divisions)) => self.pan_ticks(divisions, window, cx),
            PlotIntent::Time(TimeIntent::Fit) => {
                if let Some(total) = self.sample_count() {
                    self.navigate(View::full(total), cx);
                }
            }
            PlotIntent::Time(TimeIntent::Show(view)) => {
                if self.within_capture(view) {
                    self.navigate(view, cx);
                }
            }
            PlotIntent::Frequency(FrequencyIntent::Zoom { factor, anchor }) => {
                self.zoom_frequency(factor, anchor, cx)
            }
            PlotIntent::Frequency(FrequencyIntent::Pan(fraction)) => {
                self.pan_frequency(fraction, cx)
            }
            PlotIntent::Frequency(FrequencyIntent::Ticks(divisions)) => {
                self.frequency_ticks(divisions, cx)
            }
            PlotIntent::Frequency(FrequencyIntent::Fit) => {
                self.navigate_frequency(crate::frequency::View::default(), cx)
            }
            PlotIntent::Drag { time, frequency } => {
                let mut changed = false;
                if let Some(view) = frequency {
                    changed |= self.set_frequency_view(view);
                }
                if let Some(view) = time.filter(|view| self.within_capture(*view)) {
                    changed |= self.set_time_view(view);
                }
                if changed {
                    self.ask_for_a_picture();
                }
                cx.notify();
            }
            PlotIntent::WaveformFraction(fraction) => {
                self.session.waveform_fraction = Some(fraction);
                self.save();
                cx.notify();
            }
        }
    }

    pub(super) fn cursor_readout(&self, cx: &gpui_kit::App) -> Option<(String, Option<String>)> {
        let (pointer, geometry) = self.plot_view()?.read(cx).hover()?;
        if !geometry.navigation.contains(&pointer) {
            return None;
        }
        let mut extents = self.extents()?;
        if geometry.minimap.contains(&pointer) {
            let meta = self.file.as_ref()?.document.meta()?;
            extents.seconds = (0., meta.duration_seconds());
            extents.time.view = View {
                start: 0,
                len: meta.len_samples,
            };
        }
        let (x, y) = geometry.fractions(pointer);
        let readout = axes::Readout::from_fractions(
            (x, y),
            extents,
            (
                (geometry.time_length() * geometry.scale) as f64,
                (geometry.frequency_length() * geometry.scale) as f64,
            ),
        );
        if !geometry.spectrum.contains(&pointer) {
            return Some((readout.time, None));
        }
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
        Some((
            format!("{} · {}", readout.frequency, readout.time),
            Some(level),
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

    fn hold_frequency_scheme(&mut self, cx: &gpui_kit::App) {
        if self.frequency_scheme.is_none() {
            self.frequency_scheme = self
                .plot_view()
                .and_then(|plot| plot.read(cx).geometry)
                .and_then(|geometry| geometry.frequency_scheme);
        }
    }

    fn pan_frequency(&mut self, fraction: f64, cx: &mut Context<Self>) {
        self.hold_frequency_scheme(cx);
        self.navigate_frequency(self.frequency.pan(fraction), cx);
    }

    fn frequency_ticks(&mut self, divisions: f64, cx: &mut Context<Self>) {
        self.hold_frequency_scheme(cx);
        if let Some(scheme) = self.frequency_scheme
            && let Some(extents) = self.extents()
        {
            self.pan_frequency(
                divisions * scheme.step / (extents.hertz.1 - extents.hertz.0),
                cx,
            );
        }
    }

    /// The view commands that change the session; navigation belongs to the plot.
    pub(super) fn view_commands(
        &self,
        content: gpui_kit::Stateful<gpui_kit::Div>,
        cx: &mut Context<Self>,
    ) -> gpui_kit::Stateful<gpui_kit::Div> {
        content
            .on_action(cx.listener(|shell, _: &ToggleOrientation, window, cx| {
                shell.toggle_orientation(window, cx);
            }))
            .on_action(cx.listener(|shell, _: &ToggleGrid, _, cx| {
                shell.session.show_grid = !shell.session.show_grid;
                shell.save();
                cx.notify();
            }))
            .on_action(cx.listener(|shell, _: &ToggleScaleUi, _, cx| {
                shell.session.show_scale_ui = !shell.session.show_scale_ui;
                // Hiding the controls while one is held would leave the
                // pressed mark stuck on whichever half returns.
                if !shell.session.show_scale_ui
                    && let Some(plot) = shell.plot_entity()
                {
                    plot.update(cx, |plot, cx| plot.release_press(cx));
                }
                shell.save();
                cx.notify();
            }))
            .on_action(cx.listener(|shell, _: &ClockRuler, _, cx| {
                shell.set_time_ruler(crate::time_ruler::Mode::Clock, cx)
            }))
            .on_action(cx.listener(|shell, _: &SecondsRuler, _, cx| {
                shell.set_time_ruler(crate::time_ruler::Mode::Seconds, cx)
            }))
            .on_action(cx.listener(|shell, _: &SamplesRuler, _, cx| {
                shell.set_time_ruler(crate::time_ruler::Mode::Samples, cx)
            }))
    }

    fn set_time_ruler(&mut self, mode: crate::time_ruler::Mode, cx: &mut Context<Self>) {
        if self.session.time_ruler == mode {
            return;
        }
        self.session.time_ruler = mode;
        self.time_scheme = None;
        self.tick_pan = None;
        if self.session.orientation.vertical()
            && let Some(plot) = self.plot_entity()
        {
            plot.update(cx, |plot, cx| plot.reset_gutter(cx));
        }
        self.save();
        cx.notify();
    }

    fn toggle_orientation(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.session.orientation = self.session.orientation.toggled();
        self.time_scheme = None;
        self.frequency_scheme = None;
        self.tick_pan = None;
        if let Some(plot) = self.plot_entity() {
            plot.update(cx, |plot, cx| plot.reorient(cx));
        }
        self.upload(window, cx);
        self.upload_pending = false;
        self.orient_backdrop(window, cx);
        self.save();
        cx.notify();
    }
}

impl plot_view::PlotView {
    pub(super) fn wheel(
        &mut self,
        event: &gpui_kit::ScrollWheelEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        use plot_view::{FrequencyIntent, PlotIntent, TimeIntent};
        let Some(geometry) = self.geometry else {
            return;
        };
        if !(geometry.navigation.contains(&event.position)
            || geometry.frequency_ruler.contains(&event.position))
            || self.splitter_dragging
            || geometry.controls_at(event.position)
        {
            return;
        }
        if geometry.minimap.contains(&event.position) {
            return;
        }
        self.pan = None;
        self.frequency_pan = None;
        let delta = event.delta.pixel_delta(px(40.));
        self.view_intent(
            match geometry.scroll(event.position, delta, event.modifiers) {
                Scroll::FrequencyPan(fraction) => {
                    PlotIntent::Frequency(FrequencyIntent::Pan(fraction))
                }
                Scroll::FrequencyZoom { factor, anchor } => {
                    PlotIntent::Frequency(FrequencyIntent::Zoom { factor, anchor })
                }
                Scroll::Pan(fraction) => PlotIntent::Time(TimeIntent::Pan(fraction)),
                Scroll::Zoom { factor, anchor } => {
                    PlotIntent::Time(TimeIntent::Zoom { factor, anchor })
                }
            },
            cx,
        );
        cx.stop_propagation();
    }

    pub(super) fn begin_pan(
        &mut self,
        event: &gpui_kit::MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(geometry) = self.geometry else {
            return;
        };
        let Some(snapshot) = &self.snapshot else {
            return;
        };
        if !(geometry.navigation.contains(&event.position)
            || geometry.frequency_ruler.contains(&event.position))
            || self.splitter_dragging
            || geometry.controls_at(event.position)
        {
            return;
        }
        window.focus(&self.focus, cx);
        let view = snapshot.extents.time.view;
        let total = snapshot.extents.time.total;
        let frequency_view = snapshot.frequency;
        self.pan = None;
        self.frequency_pan = None;
        let (time, frequency) = geometry.drag_axes(event.position);
        let frequency = frequency && frequency_view.span < 1.;
        let time = time && view.len < total;
        if frequency {
            self.frequency_pan = Some((event.position, frequency_view));
        }
        if time || frequency {
            cx.emit(plot_view::PlotIntent::GestureStarted { time, frequency });
        }
        if time {
            let minimap = geometry.minimap.contains(&event.position);
            if !minimap || self.minimap_press(event, geometry, view, total, cx) {
                self.pan = Some(Pan {
                    position: event.position,
                    view,
                    width: geometry.time_length(),
                    minimap,
                });
            }
        }
        cx.notify();
    }

    fn minimap_press(
        &mut self,
        event: &gpui_kit::MouseDownEvent,
        geometry: PlotGeometry,
        view: View,
        total: u64,
        cx: &mut Context<Self>,
    ) -> bool {
        use plot_view::{PlotIntent, TimeIntent};
        let fraction = geometry.minimap_fraction(event.position);
        let viewport = crate::minimap::viewport(view, total, geometry.minimap_columns);
        match crate::minimap::click(
            fraction,
            viewport,
            event.modifiers.control,
            event.click_count,
        ) {
            crate::minimap::Click::Grab => return true,
            crate::minimap::Click::Step(divisions) => {
                cx.emit(PlotIntent::Time(TimeIntent::Ticks(divisions)))
            }
            crate::minimap::Click::Center => cx.emit(PlotIntent::Time(TimeIntent::Show(
                crate::minimap::center(view, fraction, total),
            ))),
        }
        false
    }

    fn plot_pointer(&self, position: gpui_kit::Point<Pixels>) -> Option<gpui_kit::Point<Pixels>> {
        self.geometry
            .filter(|geometry| {
                geometry.navigation.contains(&position)
                    || geometry.frequency_ruler.contains(&position)
            })
            .map(|_| position)
    }

    fn set_pointer(&mut self, pointer: Option<gpui_kit::Point<Pixels>>, cx: &mut Context<Self>) {
        if self.pointer != pointer {
            self.pointer = pointer;
            cx.emit(plot_view::PlotIntent::Pointer);
        }
    }

    pub(super) fn pointer_moved(
        &mut self,
        event: &gpui_kit::MouseMoveEvent,
        window: &Window,
        cx: &mut Context<Self>,
    ) {
        self.drag_splitter(event, window, cx);
        let pointer = self.plot_pointer(event.position);
        if self.pan.is_none() && self.frequency_pan.is_none() && self.pointer == pointer {
            return;
        }
        self.set_pointer(pointer, cx);
        let mut frequency = None;
        if let Some((origin, view)) = self.frequency_pan {
            if event.dragging() {
                if let Some(geometry) = self.geometry {
                    let fraction =
                        geometry.fractions(event.position).1 - geometry.fractions(origin).1;
                    frequency = Some(view.pan(fraction));
                }
            } else {
                self.frequency_pan = None;
            }
        }
        let mut time = None;
        if let Some(pan) = self.pan
            && let Some(snapshot) = &self.snapshot
        {
            if event.dragging() {
                let total = snapshot.extents.time.total;
                let delta = pan.position - event.position;
                let fraction =
                    f32::from(snapshot.extents.orientation.axes(delta.x, delta.y).0) / pan.width;
                let fraction = if pan.minimap {
                    -f64::from(fraction) * total as f64 / pan.view.len.max(1) as f64
                } else {
                    f64::from(fraction)
                };
                time = Some(pan.view.pan(fraction, total));
            } else {
                self.pan = None;
            }
        }
        if time.is_some() || frequency.is_some() {
            cx.emit(plot_view::PlotIntent::Drag { time, frequency });
        }
        cx.notify();
    }

    pub(super) fn finish_drags(
        &mut self,
        _: &gpui_kit::MouseUpEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.dragging() {
            self.pan = None;
            self.frequency_pan = None;
            self.splitter_dragging = false;
            cx.notify();
        }
    }

    fn track_time_menu(
        &mut self,
        menu: &gpui_kit::Entity<PopupMenu>,
        window: &Window,
        cx: &mut Context<Self>,
    ) {
        self.end_gestures(cx);
        self.open_menu = Some(menu.downgrade());
        self.menu_dismiss = Some(cx.subscribe_in(menu, window, Self::time_menu_dismissed));
    }

    fn time_menu_dismissed(
        &mut self,
        menu: &gpui_kit::Entity<PopupMenu>,
        _: &gpui_kit::DismissEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self
            .open_menu
            .as_ref()
            .is_some_and(|open| open.entity_id() == menu.entity_id())
        {
            self.open_menu = None;
            self.set_pointer(self.plot_pointer(window.mouse_position()), cx);
            cx.notify();
        }
    }

    pub(super) fn time_context_menu(
        &self,
        snapshot: &plot_view::PlotSnapshot,
        cx: &mut Context<Self>,
    ) -> Option<gpui_kit::AnyElement> {
        use gpui_kit::component::menu::ContextMenuExt;
        let geometry = self.geometry?;
        let panel = self.panel_bounds?;
        let focus = self.focus.clone();
        let owner = cx.entity().downgrade();
        let mode = snapshot.extents.time.mode;
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
                .h(geometry.time_ruler.size.height)
                .children(self.unit_hint(0, geometry.time_ruler.origin, cx))
                .context_menu(move |menu, window, cx| {
                    let popup = cx.entity();
                    let _ = owner.update(cx, |plot, cx| plot.track_time_menu(&popup, window, cx));
                    time_scale_items(menu.action_context(focus.clone()), mode)
                })
                .into_any_element(),
        )
    }
}

fn rect_contains(rect: axes::Rect, position: gpui_kit::Point<Pixels>) -> bool {
    Bounds::new(
        point(px(rect.x), px(rect.y)),
        gpui_kit::size(px(rect.width), px(rect.height)),
    )
    .contains(&position)
}

fn time_scale_items(
    menu: gpui_kit::component::menu::PopupMenu,
    mode: crate::time_ruler::Mode,
) -> gpui_kit::component::menu::PopupMenu {
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
}
