//! Time navigation gestures, actions and physical cursor readout.

use crate::navigation::{self, View};
use crate::shell::*;

actions!(
    navigation,
    [
        ToggleGrid,
        ToggleScaleUi,
        ToggleOrientation,
        ClockRuler,
        SecondsRuler,
        SamplesRuler,
        SystemTheme,
        LightTheme,
        DarkTheme,
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
        PanFarDown,
        ClearSelection
    ]
);

pub(in crate::shell) fn init(cx: &mut gpui_kit::App) {
    cx.bind_keys([
        command::binding("ctrl-g", ToggleGrid, Some("Plot")),
        command::binding("ctrl-u", ToggleScaleUi, Some("Plot")),
        command::binding("ctrl-t", ToggleOrientation, Some("Plot")),
        KeyBinding::new("ctrl-+", ZoomIn, Some("Plot")),
        KeyBinding::new("ctrl--", ZoomOut, Some("Plot")),
        command::binding("ctrl-0", FitCapture, Some("Plot")),
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
        command::binding("ctrl-shift-0", FitFrequency, Some("Plot")),
        KeyBinding::new("up", PanUp, Some("Plot && Horizontal")),
        KeyBinding::new("down", PanDown, Some("Plot && Horizontal")),
        KeyBinding::new("ctrl-up", PanFarUp, Some("Plot && Horizontal")),
        KeyBinding::new("ctrl-down", PanFarDown, Some("Plot && Horizontal")),
        KeyBinding::new("escape", ClearSelection, Some("Plot")),
    ]);
}

pub(in crate::shell) fn plot_shortcut(
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
pub(in crate::shell) struct PlotGeometry {
    pub orientation: crate::navigation::orientation::Mode,
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

    /// The cursor a time selection or a held Space calls for, ahead of the ordinary one.
    pub fn selection_cursor(
        self,
        pointer: Option<gpui_kit::Point<Pixels>>,
        panning: bool,
        selecting: Option<gpui_kit::CursorStyle>,
        space: bool,
    ) -> Option<gpui_kit::CursorStyle> {
        if panning {
            return None;
        }
        if let Some(cursor) = selecting {
            return Some(cursor);
        }
        let position = pointer?;
        (space && self.spectrum.contains(&position) && !self.controls_at(position))
            .then_some(gpui_kit::CursorStyle::OpenHand)
    }

    /// Whether a left press here starts a time selection rather than a pan.
    pub(in crate::shell) fn selects_at(self, position: gpui_kit::Point<Pixels>) -> bool {
        self.spectrum.contains(&position) && !self.controls_at(position)
    }

    fn unit_at(self, position: gpui_kit::Point<Pixels>) -> bool {
        self.unit_hints
            .iter()
            .flatten()
            .any(|hint| rect_contains(hint.bounds, position))
    }

    pub(in crate::shell) fn controls_at(self, position: gpui_kit::Point<Pixels>) -> bool {
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

    pub(in crate::shell) fn fractions(self, position: gpui_kit::Point<Pixels>) -> (f64, f64) {
        self.orientation.fractions(
            f32::from(position.x - self.spectrum.left()) as f64
                / f32::from(self.spectrum.size.width) as f64,
            f32::from(position.y - self.spectrum.top()) as f64
                / f32::from(self.spectrum.size.height) as f64,
        )
    }

    pub(in crate::shell) fn minimap_fraction(self, position: gpui_kit::Point<Pixels>) -> f64 {
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

/// A selection being dragged out from the boundary it was pressed at, or reshaped or moved.
#[derive(Clone, Copy)]
pub(in crate::shell) struct Selecting {
    /// The sample the drag started at, unless it selects a band alone.
    pub(in crate::shell) anchor: Option<u64>,
    /// The frequency in hertz the drag started at, when it selects a band or a rectangle.
    pub(in crate::shell) anchor_hz: Option<f64>,
    pub(in crate::shell) origin: gpui_kit::Point<Pixels>,
    /// Set once the pointer has gone far enough for this to be a drag and not a click.
    pub(in crate::shell) moved: bool,
    /// The time span kept while only a band edge moves.
    pub(in crate::shell) keep_time: Option<argand_core::SampleSpan>,
    /// The band kept while only a time edge moves.
    pub(in crate::shell) keep_band: Option<argand_core::FrequencyBand>,
    /// Whether the time follows the minimap, which spans the whole capture.
    pub(in crate::shell) minimap: bool,
    /// Whether a click that never became a drag clears the selection.
    pub(in crate::shell) clears: bool,
    /// The selection as it was when a drag inside it began to move it whole.
    pub(in crate::shell) moving: Option<argand_core::Selection>,
    /// The cursor the gesture keeps while it lasts.
    pub(in crate::shell) cursor: gpui_kit::CursorStyle,
}

impl Selecting {
    /// A new selection from the press at `origin`.
    fn fresh(origin: gpui_kit::Point<Pixels>, anchor: Option<u64>, anchor_hz: Option<f64>) -> Self {
        Self {
            anchor,
            anchor_hz,
            origin,
            moved: false,
            keep_time: None,
            keep_band: None,
            minimap: false,
            clears: true,
            moving: None,
            cursor: gpui_kit::CursorStyle::Crosshair,
        }
    }
}

/// The physical frequency at `fraction` down from the top of the visible band `hertz`.
pub(in crate::shell) fn hertz(hertz: (f64, f64), fraction: f64) -> f64 {
    hertz.1 - fraction * (hertz.1 - hertz.0)
}

/// How far a press may wander, in logical pixels, and still be a click.
const CLICK_SLOP: f32 = 3.;

#[derive(Clone, Copy)]
pub(in crate::shell) struct Pan {
    position: gpui_kit::Point<Pixels>,
    view: View,
    width: f32,
    minimap: bool,
}

impl Shell {
    pub(in crate::shell) fn reset_view(&mut self) {
        let Some(total) = self.sample_count() else {
            return;
        };
        self.view = Some(View::full(total));
        self.frequency = crate::navigation::frequency::View::default();
        self.frequency_scheme = None;
        self.time_scheme = None;
        self.tick_pan = None;
        if let Some(opening) = &mut self.hint_opening {
            opening.view = self.view;
            opening.frequency = self.frequency;
        }
    }

    pub(in crate::shell) fn bound_view(&mut self, cx: &mut gpui_kit::App) {
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

    pub(in crate::shell) fn sample_count(&self) -> Option<u64> {
        Some(self.file.as_ref()?.state.document.meta()?.len_samples)
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
        if let Some(opening) = &mut self.hint_opening {
            opening.view = Some(view);
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
        let Some(meta) = self
            .file
            .as_ref()
            .and_then(|file| file.state.document.meta())
        else {
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
    pub(in crate::shell) fn apply_plot_intent(
        &mut self,
        _: &gpui_kit::Entity<crate::shell::plot::plot_view::PlotView>,
        intent: &crate::shell::plot::plot_view::PlotIntent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        use crate::shell::plot::plot_view::{FrequencyIntent, PlotIntent, TimeIntent};
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
            PlotIntent::Select(selection) => self.select(selection.unwrap_or_default(), cx),
            PlotIntent::Pointer => {
                if self.conversion_active() {
                    return;
                }
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
                self.navigate_frequency(crate::navigation::frequency::View::default(), cx)
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
        }
    }

    /// What a selection's values are written and read against, as its badges show them.
    pub(in crate::shell) fn entry_scale(
        &self,
    ) -> Option<crate::navigation::selection_entry::Scale> {
        let meta = self.file.as_ref()?.state.document.meta()?;
        let view = self.view?;
        let extents = self.extents()?;
        let (low, high) = extents.displayed_hertz();
        let (unit, divisor) = axes::frequency_unit((low, high));
        let plot = self.plot;
        let columns = plot.map_or(1, |plot| plot.width.max(1));
        let rows = plot.map_or(1, |plot| plot.height.max(1));
        Some(crate::navigation::selection_entry::Scale {
            mode: self.session.time_ruler,
            rate: meta.sample_rate,
            total: meta.len_samples,
            seconds_per_pixel: view.len as f64 / meta.sample_rate / columns as f64,
            unit,
            divisor,
            decimals: crate::navigation::time_precision((high - low) / rows as f64 / divisor),
            band: self.capture_band()?,
            frequency_offset: self.frequency_offset(),
        })
    }

    pub(in crate::shell) fn cursor_readout(
        &self,
        cx: &gpui_kit::App,
    ) -> Option<(String, Option<String>)> {
        if self.conversion_active() {
            return None;
        }
        let (pointer, geometry) = self.plot_view()?.read(cx).hover()?;
        // The corner scale buttons are controls, not a place on the plot.
        if !geometry.navigation.contains(&pointer) || geometry.over_scale_buttons(Some(pointer)) {
            return None;
        }
        let mut extents = self.extents()?;
        if geometry.minimap.contains(&pointer) {
            let meta = self.file.as_ref()?.state.document.meta()?;
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
            .state
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
        let Some(meta) = self
            .file
            .as_ref()
            .and_then(|file| file.state.document.meta())
        else {
            return 1;
        };
        let bins = if meta.is_iq() {
            self.settings.fft_size
        } else {
            self.settings.fft_size / 2 + 1
        };
        crate::navigation::frequency::cells(meta.frequency_span(), bins)
    }

    fn navigate_frequency(
        &mut self,
        view: crate::navigation::frequency::View,
        cx: &mut Context<Self>,
    ) {
        if self.set_frequency_view(view) {
            self.ask_for_a_picture();
            cx.notify();
        }
    }

    fn set_frequency_view(&mut self, view: crate::navigation::frequency::View) -> bool {
        if self.frequency == view {
            return false;
        }
        if self.frequency.span != view.span {
            self.frequency_scheme = None;
        }
        self.frequency = view;
        if let Some(opening) = &mut self.hint_opening {
            opening.frequency = view;
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
    pub(in crate::shell) fn view_commands(
        &self,
        content: gpui_kit::Stateful<gpui_kit::Div>,
        cx: &mut Context<Self>,
    ) -> gpui_kit::Stateful<gpui_kit::Div> {
        content
            .on_action(command::listener(
                cx,
                |shell, _: &ToggleOrientation, window, cx| {
                    shell.toggle_orientation(window, cx);
                },
            ))
            .on_action(command::listener(cx, |shell, _: &ToggleGrid, _, cx| {
                shell.session.show_grid = !shell.session.show_grid;
                shell.save();
                cx.notify();
            }))
            .on_action(command::listener(cx, |shell, _: &ToggleScaleUi, _, cx| {
                shell.session.show_scale_ui = !shell.session.show_scale_ui;
                shell.save();
                cx.notify();
            }))
            .on_action(command::listener(cx, |shell, _: &ClockRuler, _, cx| {
                shell.set_time_ruler(crate::navigation::time_ruler::Mode::Clock, cx)
            }))
            .on_action(command::listener(cx, |shell, _: &SecondsRuler, _, cx| {
                shell.set_time_ruler(crate::navigation::time_ruler::Mode::Seconds, cx)
            }))
            .on_action(command::listener(cx, |shell, _: &SamplesRuler, _, cx| {
                shell.set_time_ruler(crate::navigation::time_ruler::Mode::Samples, cx)
            }))
            .on_action(command::listener(
                cx,
                |shell, _: &SystemTheme, window, cx| shell.set_theme(Theme::System, window, cx),
            ))
            .on_action(command::listener(
                cx,
                |shell, _: &LightTheme, window, cx| shell.set_theme(Theme::Light, window, cx),
            ))
            .on_action(command::listener(cx, |shell, _: &DarkTheme, window, cx| {
                shell.set_theme(Theme::Dark, window, cx)
            }))
    }

    /// The interface theme the session chose, or the configuration's while it chose none.
    pub(in crate::shell) fn theme(&self) -> Theme {
        self.session.theme.unwrap_or(self.config.theme)
    }

    /// Remembers a theme chosen in the menu and repaints every window with it.
    fn set_theme(&mut self, theme: Theme, window: &mut Window, cx: &mut Context<Self>) {
        if self.session.theme == Some(theme) {
            return;
        }
        self.session.theme = Some(theme);
        self.save();
        sync_theme(theme, window, cx);
        cx.notify();
    }

    fn set_time_ruler(
        &mut self,
        mode: crate::navigation::time_ruler::Mode,
        cx: &mut Context<Self>,
    ) {
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

    pub(in crate::shell) fn toggle_orientation(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
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

impl Shell {
    /// Take a selection a gesture made, its band clamped to the capture's.
    pub(in crate::shell) fn select(
        &mut self,
        selection: argand_core::Selection,
        cx: &mut Context<Self>,
    ) {
        if self.conversion_active() {
            return;
        }
        let band = selection.band.map(|band| {
            let (low, high) = self.capture_band().unwrap_or((band.low(), band.high()));
            let (low, high) = (band.low().max(low), band.high().min(high));
            // Clamped edges that cross mean no overlap, which `between` would read as reversed.
            (low < high)
                .then(|| argand_core::FrequencyBand::between(low, high))
                .flatten()
        });
        // A band wholly beyond the capture selects nothing, not the band's time span alone.
        let (time, band) = match band {
            Some(None) => (None, None),
            band => (selection.time, band.flatten()),
        };
        if (self.selection, self.band) != (time, band) {
            self.selection = time;
            self.band = band;
            cx.notify();
        }
    }

    pub(in crate::shell) fn frequency_offset(&self) -> f64 {
        let Some(editing) = self.editing() else {
            return 0.;
        };
        editing.reference() - editing.file().meta.center_freq
    }

    /// The capture's band in physical hertz, around its reference when complex and above it when real.
    pub(in crate::shell) fn capture_band(&self) -> Option<(f64, f64)> {
        let meta = self.file.as_ref()?.state.document.meta()?;
        let nyquist = meta.sample_rate / 2.0;
        let reference = meta.center_freq;
        Some(if meta.is_iq() {
            (reference - nyquist, reference + nyquist)
        } else {
            (reference, reference + nyquist)
        })
    }

    /// The time span alone, which edit commands act on, and none while a band is selected.
    pub(in crate::shell) fn time_selection(&self) -> Option<argand_core::SampleSpan> {
        self.selection.filter(|_| self.band.is_none())
    }
}

impl crate::shell::plot::plot_view::PlotView {
    pub(in crate::shell) fn wheel(
        &mut self,
        event: &gpui_kit::ScrollEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        use crate::shell::plot::plot_view::{FrequencyIntent, PlotIntent, TimeIntent};
        let Some(geometry) = self.geometry else {
            return;
        };
        if !(geometry.navigation.contains(&event.position)
            || geometry.frequency_ruler.contains(&event.position))
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

    /// Start the gesture a press asks for, which is decided here once.
    ///
    /// Unless Space is held, a left press selects time on the spectrum, a
    /// rectangle with Ctrl there, and a band on the frequency ruler. Every
    /// other press that lands on the plot pans.
    pub(in crate::shell) fn press(
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
            || geometry.controls_at(event.position)
        {
            return;
        }
        window.focus(&self.focus, cx);
        self.pan = None;
        self.frequency_pan = None;
        self.selecting = None;
        // A badge takes its own left presses, a double click turning it into an input.
        if event.button == MouseButton::Left
            && !self.space
            && let Some(field) = self.badge_at(event.position)
        {
            if event.click_count >= 2 {
                self.open_entry(field, window, cx);
            }
            cx.notify();
            return;
        }
        let on_spectrum = geometry.selects_at(event.position);
        let on_ruler = geometry.frequency_ruler.contains(&event.position);
        let grabbed = (event.button == MouseButton::Left && !self.space)
            .then(|| self.grab_selection(event, geometry))
            .flatten();
        if let Some(selecting) = grabbed {
            self.selecting = Some(selecting);
            if selecting.moved {
                self.select_to(event.position, cx);
            }
            cx.notify();
            return;
        }
        if event.button == MouseButton::Left && !self.space && (on_spectrum || on_ruler) {
            let (time, frequency) = geometry.fractions(event.position);
            let view = snapshot.extents.time.view;
            self.selecting = Some(Selecting::fresh(
                event.position,
                on_spectrum.then(|| view.boundary(time)),
                (on_ruler || event.modifiers.control)
                    .then(|| hertz(snapshot.extents.hertz, frequency)),
            ));
            cx.notify();
            return;
        }
        self.pan_button = event.button;
        let view = snapshot.extents.time.view;
        let total = snapshot.extents.time.total;
        let frequency_view = snapshot.frequency;
        let (time, frequency) = geometry.drag_axes(event.position);
        let frequency = frequency && frequency_view.span < 1.;
        let time = time && view.len < total;
        if frequency {
            self.frequency_pan = Some((event.position, frequency_view));
        }
        if time || frequency {
            cx.emit(crate::shell::plot::plot_view::PlotIntent::GestureStarted { time, frequency });
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
        use crate::shell::plot::plot_view::{PlotIntent, TimeIntent};
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

    pub(in crate::shell) fn plot_pointer(
        &self,
        position: gpui_kit::Point<Pixels>,
    ) -> Option<gpui_kit::Point<Pixels>> {
        self.geometry
            .filter(|geometry| {
                geometry.navigation.contains(&position)
                    || geometry.frequency_ruler.contains(&position)
            })
            .map(|_| position)
    }

    /// Take up a pointer that came to rest over the plot without a move.
    pub(in crate::shell) fn pick_up_pointer(&mut self, window: &Window, cx: &mut Context<Self>) {
        let in_window = self
            .snapshot
            .as_ref()
            .is_some_and(|snapshot| snapshot.pointer_in_window);
        if !in_window || self.pointer.is_some() || self.dragging() {
            return;
        }
        self.set_pointer(self.plot_pointer(window.mouse_position()), cx);
        cx.notify();
    }

    fn set_pointer(&mut self, pointer: Option<gpui_kit::Point<Pixels>>, cx: &mut Context<Self>) {
        if self.pointer != pointer {
            self.pointer = pointer;
            cx.emit(crate::shell::plot::plot_view::PlotIntent::Pointer);
        }
    }

    pub(in crate::shell) fn pointer_moved(
        &mut self,
        event: &gpui_kit::MouseMoveEvent,
        cx: &mut Context<Self>,
    ) {
        let pointer = self.plot_pointer(event.position);
        if !self.dragging() && self.pointer == pointer {
            return;
        }
        self.set_pointer(pointer, cx);
        self.extend_selection(event, cx);
        let held = event.pressed_button == Some(self.pan_button);
        let mut frequency = None;
        if let Some((origin, view)) = self.frequency_pan {
            if held {
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
            if held {
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
            cx.emit(crate::shell::plot::plot_view::PlotIntent::Drag { time, frequency });
        }
        cx.notify();
    }

    /// Carry a time selection to the pointer, once it has gone beyond a click.
    fn extend_selection(&mut self, event: &gpui_kit::MouseMoveEvent, cx: &mut Context<Self>) {
        if self.selecting.is_none() {
            return;
        }
        if !event.dragging() {
            self.selecting = None;
            return;
        }
        self.select_to(event.position, cx);
    }

    /// Report the selection from its anchor to `position`, unless the press is still a click.
    fn select_to(&mut self, position: gpui_kit::Point<Pixels>, cx: &mut Context<Self>) {
        let Some(mut selecting) = self.selecting else {
            return;
        };
        let (Some(geometry), Some(snapshot)) = (self.geometry, &self.snapshot) else {
            return;
        };
        let delta = position - selecting.origin;
        selecting.moved |= f32::from(delta.x).hypot(f32::from(delta.y)) >= CLICK_SLOP;
        self.selecting = Some(selecting);
        if !selecting.moved {
            return;
        }
        if let Some(from) = selecting.moving {
            let moved = self.moved_selection(from, selecting.origin, position);
            cx.emit(crate::shell::plot::plot_view::PlotIntent::Select(moved));
            return;
        }
        let (time, frequency) = geometry.fractions(position);
        let boundary = match selecting.minimap {
            true => View::full(snapshot.extents.time.total)
                .boundary(geometry.minimap_fraction(position)),
            false => snapshot.extents.time.view.boundary(time),
        };
        let span = selecting
            .anchor
            .map(|anchor| argand_core::SampleSpan::between(anchor, boundary));
        let band = selecting.anchor_hz.map(|anchor| {
            // A band edge stops at the capture's band rather than leave it and lose the selection.
            let at = hertz(snapshot.extents.hertz, frequency);
            let at = snapshot
                .capture_band
                .map_or(at, |(floor, ceiling)| at.clamp(floor, ceiling));
            argand_core::FrequencyBand::between(anchor, at)
        });
        // Each axis the gesture selects must have some extent, or nothing is selected.
        let selection = match (span, band) {
            (Some(None), _) | (_, Some(None)) => None,
            (time, band) => Some(argand_core::Selection {
                time: time.flatten().or(selecting.keep_time),
                band: band.flatten().or(selecting.keep_band),
            }),
        };
        // An edge laid on its partner leaves the selection as it last was.
        if selection.is_some() || selecting.clears {
            cx.emit(crate::shell::plot::plot_view::PlotIntent::Select(selection));
        }
    }

    pub(in crate::shell) fn finish_drags(
        &mut self,
        event: &gpui_kit::MouseUpEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if event.button == MouseButton::Left && self.selecting.is_some() {
            // The release can lie beyond the last move, so it settles the far end
            self.select_to(event.position, cx);
            if self
                .selecting
                .take()
                .is_some_and(|selecting| !selecting.moved && selecting.clears)
            {
                cx.emit(crate::shell::plot::plot_view::PlotIntent::Select(None));
            }
            cx.notify();
        }
        if self.panning() && event.button == self.pan_button {
            self.pan = None;
            self.frequency_pan = None;
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
        // The paste point outlives the menu, whose action arrives after its dismissal.
        self.paste_point = None;
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

    /// The edit commands under a right click on the spectrum, Paste here going before the clicked sample.
    pub(in crate::shell) fn spectrum_context_menu(
        &self,
        snapshot: &crate::shell::plot::plot_view::PlotSnapshot,
        cx: &mut Context<Self>,
    ) -> Option<gpui_kit::AnyElement> {
        use gpui_kit::component::menu::ContextMenuExt;
        let geometry = self.geometry?;
        let panel = self.panel_bounds?;
        let focus = self.focus.clone();
        let owner = cx.entity().downgrade();
        let commands = snapshot.commands;
        let view = snapshot.extents.time.view;
        Some(
            div()
                .id("spectrum-context")
                .absolute()
                .left(geometry.spectrum.left() - panel.left())
                .top(geometry.spectrum.top() - panel.top())
                .w(geometry.spectrum.size.width)
                .h(geometry.spectrum.size.height)
                .context_menu(move |menu, window, cx| {
                    let popup = cx.entity();
                    let at = window.mouse_position();
                    let _ = owner.update(cx, |plot, cx| {
                        plot.track_spectrum_menu(&popup, at, view, window, cx);
                    });
                    edit_items(menu.action_context(focus.clone()), commands)
                })
                .into_any_element(),
        )
    }

    fn track_spectrum_menu(
        &mut self,
        menu: &gpui_kit::Entity<PopupMenu>,
        at: gpui_kit::Point<Pixels>,
        view: View,
        window: &Window,
        cx: &mut Context<Self>,
    ) {
        self.track_time_menu(menu, window, cx);
        self.paste_point = self
            .geometry
            .map(|geometry| view.boundary(geometry.fractions(at).0));
        cx.notify();
    }

    pub(in crate::shell) fn time_context_menu(
        &self,
        snapshot: &crate::shell::plot::plot_view::PlotSnapshot,
        cx: &mut Context<Self>,
    ) -> Option<gpui_kit::AnyElement> {
        use gpui_kit::component::menu::ContextMenuExt;
        let geometry = self.geometry?;
        let panel = self.panel_bounds?;
        let focus = self.focus.clone();
        let owner = cx.entity().downgrade();
        let mode = snapshot.extents.time.mode;
        let commands = snapshot.commands;
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
                    time_scale_items(menu.action_context(focus.clone()), mode, commands)
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

fn edit_items(
    menu: gpui_kit::component::menu::PopupMenu,
    commands: command::Availability,
) -> gpui_kit::component::menu::PopupMenu {
    use crate::shell::editing_ui::{
        CopySelection, CutSelection, DeleteSelection, PasteClipboard, PasteHere, SuppressBand,
    };
    menu.item(commands.item(CutSelection, PopupMenuItem::new("Cut")))
        .item(commands.item(CopySelection, PopupMenuItem::new("Copy")))
        .item(commands.item(PasteHere, PopupMenuItem::new("Paste here")))
        .item(commands.item(PasteClipboard, PopupMenuItem::new("Replace selection")))
        .item(PopupMenuItem::separator())
        .item(commands.item(DeleteSelection, PopupMenuItem::new("Delete")))
        .item(PopupMenuItem::separator())
        .item(commands.item(SuppressBand, PopupMenuItem::new("Suppress band")))
}

fn time_scale_items(
    menu: gpui_kit::component::menu::PopupMenu,
    mode: crate::navigation::time_ruler::Mode,
    commands: command::Availability,
) -> gpui_kit::component::menu::PopupMenu {
    use crate::navigation::time_ruler::Mode;
    menu.item(commands.item(
        ClockRuler,
        PopupMenuItem::new("Hours, minutes, seconds (hms)").checked(mode == Mode::Clock),
    ))
    .item(commands.item(
        SecondsRuler,
        PopupMenuItem::new("Seconds").checked(mode == Mode::Seconds),
    ))
    .item(commands.item(
        SamplesRuler,
        PopupMenuItem::new("Sample numbers").checked(mode == Mode::Samples),
    ))
}

#[cfg(test)]
#[path = "navigation_ui_test.rs"]
mod tests;
