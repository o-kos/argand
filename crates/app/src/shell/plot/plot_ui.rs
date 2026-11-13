//! Synchronized plot painting and measured pointer geometry.

use super::navigation_ui::*;
use super::plot_view::{PlotIntent, PlotSnapshot, PlotView};
use crate::shell::*;
use gpui_kit::component::button::ButtonRounded;
use gpui_kit::component::{Disableable, Icon, IconName};
use gpui_kit::{AvailableSpace, Div};

impl Shell {
    /// The held picture view and the once-only first-paint marker, both
    /// taken from the open file when there is one.
    fn first_paint_state(
        &self,
    ) -> (
        Option<crate::navigation::PictureView>,
        Option<(Instant, Arc<AtomicBool>)>,
    ) {
        (
            self.file
                .as_ref()
                .and_then(|file| file.state.document.analysis())
                .map(|analysis| crate::navigation::PictureView::grid(&analysis.db)),
            self.file
                .as_ref()
                .map(|file| (file.opened_at, file.first_picture.clone())),
        )
    }

    /// What the plot paints this frame, or nothing when no plot is shown.
    pub(in crate::shell) fn plot_snapshot(&self, cx: &gpui_kit::App) -> Option<PlotSnapshot> {
        let Showing::Plot(extents) = self.showing() else {
            return None;
        };
        let (held, first_picture) = self.first_paint_state();
        Some(PlotSnapshot {
            extents,
            texture: self.texture.clone(),
            deep: self.deep_preview.clone(),
            backdrop: self.backdrop.clone(),
            held,
            first_picture,
            minimap: self.minimap_panel(cx),
            minimap_size: self.config.panels.minimap_size,
            time_scheme: self.time_scheme,
            frequency_scheme: self.frequency_scheme,
            frequency: self.frequency,
            show_grid: self.session.show_grid,
            tooltips_allowed: self.external_tooltips_allowed(cx),
            show_scale_ui: self.session.show_scale_ui,
            pointer_in_window: self.pointer_in_window,
            selection: self.selected(),
            selected: argand_core::Selection {
                time: self.selection,
                band: self.band,
            },
            capture_band: self.capture_band(),
            badges: self.selection_badges(),
            entry: self.entry_scale(),
            notice: match &self.save_notice {
                Some(saving_ui::Notice::Saved(name)) => Some(format!("Saved {name}").into()),
                _ => None,
            },
            commands: self.command_availability(),
        })
    }

    pub(in crate::shell) fn measure_time_scheme(
        &self,
        window: &Window,
        cx: &gpui_kit::App,
    ) -> Option<argand_core::axis::TickScheme> {
        let plot = self.plot_view()?.read(cx);
        let bounds = plot.panel_bounds?;
        let gutter = plot.gutter_floor;
        let height = panels::waveform_height(
            f32::from(
                self.session
                    .orientation
                    .axes(bounds.size.width, bounds.size.height)
                    .1,
            ),
            f32::from(window.rem_size()),
            self.config.panels.minimap_size,
            window.scale_factor(),
        );
        let (dx, dy) = self.session.orientation.axes(px(0.), px(height));
        let panel = size(bounds.size.width - dx, bounds.size.height - dy);
        // Only the time scheme is measured afresh, so the plot keeps its painted width.
        let held = axes::Held {
            time: None,
            frequency: self.frequency_scheme,
            gutter,
        };
        axes::Frame::measure_view(
            panel,
            window.scale_factor(),
            self.extents()?,
            &axes::Labels::new(window),
            held,
        )?
        .time_scheme
    }

    fn minimap_panel(&self, cx: &gpui_kit::App) -> waveform::Panel {
        let displayed = self
            .file
            .as_ref()
            .and_then(|file| file.state.displayed_settings().cloned());
        let colormap = self.settings.minimap_colormap(displayed);
        // The minimap stands on the bottom of the spectrogram's own scale in either theme.
        let [r, g, b] = colormap.gradient()[0];
        let ink = colormap.waveform_ink(true);
        let total = self
            .file
            .as_ref()
            .and_then(|file| file.state.document.meta())
            .map(|meta| meta.len_samples);
        waveform::Panel {
            waveform: self.waveform.clone(),
            viewport: self.view.zip(total),
            selection: self
                .selection
                .zip(total)
                .and_then(|(span, total)| crate::navigation::View::full(total).fractions_of(span)),
            selection_fill: selection_fill(cx),
            separator: cx.theme().border,
            paper: gpui_kit::rgb((u32::from(r) << 16) | (u32::from(g) << 8) | u32::from(b)).into(),
            ink: waveform::Ink {
                active: gpui_kit::rgb(ink.active),
                muted: gpui_kit::rgb(ink.muted),
            },
        }
    }
}

impl PlotView {
    /// The spectrogram panel: the picture, and the axes around it.
    ///
    /// One canvas does the measuring and the drawing, because the two are the
    /// same question. How wide the frequency labels are decides how much of
    /// the panel is left for the picture, and that leftover is exactly what the
    /// transform is asked to fill -- so the size reported back from here is the
    /// plot's and not the panel's. Only the window has a font to measure the
    /// labels with, which is why this cannot be settled anywhere earlier.
    ///
    /// The measurement is deferred rather than applied on the spot: prepaint is
    /// not a moment at which the entity being painted can be borrowed again.
    pub(in crate::shell) fn spectrogram(
        &self,
        snapshot: PlotSnapshot,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let extents = snapshot.extents;
        let orientation = extents.orientation;
        let overlays = self.overlays(&snapshot, cx);
        let rem = f32::from(cx.theme().font_size);
        let known_bounds = self.panel_bounds;
        let known = self.measured;
        let known_geometry = self.geometry;
        let badges = self.badge_layout(snapshot.badges, cx);
        let held = axes::Held {
            time: snapshot.time_scheme,
            frequency: snapshot.frequency_scheme,
            gutter: self.gutter_floor,
        };
        let view = cx.entity().downgrade();
        let marks = self.marks(snapshot.selection, extents, snapshot.show_grid, cx);

        canvas(
            move |bounds, window, cx| {
                let scale = window.scale_factor();
                let labels = axes::Labels::new(window);
                let height = panels::waveform_height(
                    f32::from(orientation.axes(bounds.size.width, bounds.size.height).1),
                    rem,
                    snapshot.minimap_size,
                    scale,
                );
                let (dx, dy) = orientation.axes(px(0.), px(height));
                let spectrum_size = size(bounds.size.width - dx, bounds.size.height - dy);
                let frame =
                    axes::Frame::measure_view(spectrum_size, scale, extents, &labels, held)?;
                let measured = oriented_device_size(frame.plot, scale, orientation);
                let geometry = plot_geometry(
                    bounds,
                    &frame,
                    &labels,
                    height,
                    scale,
                    snapshot.show_scale_ui,
                );
                if known != Some(measured)
                    || known_bounds != Some(bounds)
                    || known_geometry != Some(geometry)
                    || frame.gutter > held.gutter
                {
                    defer_layout(view.clone(), bounds, measured, geometry, frame.gutter, cx);
                }
                let entry = badge_entry::entry_size(f32::from(window.rem_size()));
                let placed =
                    badges.place(&frame, &labels, entry, bounds.origin + point(dx, dy), cx);
                let overlays = overlays.prepaint(
                    bounds,
                    plot_bounds(&frame, bounds.origin + point(dx, dy)),
                    geometry.zoom_zones[0].is_some(),
                    window,
                    cx,
                );
                Some((frame, labels, height, placed, overlays))
            },
            move |bounds, prepainted, window, cx| {
                let Some((frame, labels, height, placed, overlays)) = prepainted else {
                    return;
                };
                let (dx, dy) = orientation.axes(px(0.), px(height));
                let spectrum_origin = bounds.origin + point(dx, dy);
                if let Some(backdrop) = &snapshot.backdrop {
                    backdrop.paint(
                        &frame,
                        bounds.origin,
                        height,
                        extents.picture(),
                        snapshot.held,
                        window,
                    );
                }
                snapshot
                    .minimap
                    .paint(&frame, bounds.origin, height, window);
                // The picture first, then the marks over it: a grid line is
                // there to be read against the spectrogram, not under it.
                if let Some(texture) = snapshot.texture
                    && let Some(held) = snapshot.held
                {
                    let plot = plot_bounds(&frame, spectrum_origin);
                    if let Some(deep) = snapshot.deep {
                        deep.paint(plot, extents.picture(), orientation, window);
                    } else {
                        paint_held(texture, plot, held, extents.picture(), orientation, window);
                    }
                    mark_first_paint(&snapshot.first_picture);
                }
                let panel = Bounds::new(
                    spectrum_origin,
                    size(bounds.size.width - dx, bounds.size.height - dy),
                );
                marks.paint(&frame, panel, &labels, &placed, window, cx);
                overlays.paint(window, cx);
            },
        )
        .size_full()
    }

    fn badge_layout(
        &self,
        badges: Option<axes::SelectionBadges>,
        cx: &Context<Self>,
    ) -> BadgeLayout {
        BadgeLayout {
            badges,
            known: self.badges.clone(),
            typing: self.typing.as_ref().map(|typing| typing.field),
            view: cx.entity().downgrade(),
        }
    }

    fn marks(
        &self,
        selection: Option<waveform::Selected>,
        extents: axes::Extents,
        show_grid: bool,
        cx: &Context<Self>,
    ) -> Marks {
        Marks {
            selection,
            badge_colors: [cx.theme().foreground, cx.theme().background],
            colors: axis_colors(cx, show_grid),
            guides: self.cursor_guides(extents, cx),
        }
    }

    fn cursor_guides(
        &self,
        extents: axes::Extents,
        cx: &Context<Self>,
    ) -> Option<axes::CursorGuides> {
        let menu_open = self
            .open_menu
            .as_ref()
            .and_then(WeakEntity::upgrade)
            .is_some();
        // The corner scale buttons take the pointer for themselves: no
        // Alt guides over them.
        let over_buttons = self
            .geometry
            .is_some_and(|geometry| geometry.over_scale_buttons(self.pointer));
        (self.pointer.is_some()
            && self.pan.is_none()
            && self.frequency_pan.is_none()
            && !menu_open
            && !over_buttons)
            .then_some(axes::CursorGuides {
                extents,
                metrics: self.badge_metrics.clone(),
                ink: cx.theme().foreground,
                paper: cx.theme().background,
            })
    }

    pub(in crate::shell) fn unit_hint(
        &self,
        index: usize,
        origin: gpui_kit::Point<Pixels>,
        cx: &Context<Self>,
    ) -> Option<gpui_kit::AnyElement> {
        let menu_open = self
            .open_menu
            .as_ref()
            .and_then(WeakEntity::upgrade)
            .is_some();
        if self.pan.is_some()
            || self.frequency_pan.is_some()
            || menu_open
            || !self
                .snapshot
                .as_ref()
                .is_some_and(|snapshot| snapshot.tooltips_allowed)
        {
            return None;
        }
        let hint = self.geometry?.unit_hints[index]?;
        let owner = cx.entity().downgrade();
        Some(
            div()
                .id(hint.text)
                .cursor(gpui_kit::CursorStyle::Arrow)
                .absolute()
                .left(px(hint.bounds.x) - origin.x)
                .top(px(hint.bounds.y) - origin.y)
                .w(px(hint.bounds.width))
                .h(px(hint.bounds.height))
                .tooltip(move |_, cx| unit_tooltip(owner.clone(), index, cx))
                .into_any_element(),
        )
    }

    /// A finished save in the spectrum's bottom-right corner, which no zoom pair takes in either orientation.
    pub(in crate::shell) fn saved_notice(
        &self,
        snapshot: &PlotSnapshot,
        cx: &mut Context<Self>,
    ) -> Option<gpui_kit::AnyElement> {
        let text = snapshot.notice.clone()?;
        let spectrum = self.geometry?.spectrum;
        let origin = self.panel_bounds?.origin;
        Some(
            div()
                .absolute()
                .left(spectrum.left() - origin.x)
                .top(spectrum.top() - origin.y)
                .w(spectrum.size.width)
                .h(spectrum.size.height)
                .p(px(SCALE_INSET))
                .flex()
                .items_end()
                .justify_end()
                .child(
                    div()
                        .id("saved-notice")
                        .min_w_0()
                        .px_2()
                        .py_0p5()
                        .border_1()
                        .border_color(cx.theme().border.opacity(0.75))
                        .bg(cx.theme().background.opacity(0.85))
                        .rounded(px(SCALE_ROUNDING))
                        .text_xs()
                        .text_color(cx.theme().foreground)
                        .overflow_hidden()
                        .text_ellipsis()
                        .whitespace_nowrap()
                        .child(text),
                )
                .into_any_element(),
        )
    }

    /// The line marking where Paste here inserts, while the spectrum's menu is open.
    pub(in crate::shell) fn paste_marker(
        &self,
        snapshot: &PlotSnapshot,
        cx: &mut Context<Self>,
    ) -> Option<gpui_kit::AnyElement> {
        self.open_menu.as_ref()?;
        let at = self.paste_point?;
        let spectrum = self.geometry?.spectrum;
        let origin = self.panel_bounds?.origin;
        let view = snapshot.extents.time.view;
        let fraction =
            (at.saturating_sub(view.start) as f64 / view.len.max(1) as f64).clamp(0.0, 1.0) as f32;
        let ink = cx.theme().foreground;
        let paper = cx.theme().background;
        let line = div().absolute().bg(ink).border_color(paper);
        let marker = if snapshot.extents.orientation.vertical() {
            let y = spectrum.top() + spectrum.size.height * fraction - origin.y;
            line.left(spectrum.left() - origin.x)
                .top(y - px(1.))
                .w(spectrum.size.width)
                .h(px(3.))
                .border_y_1()
        } else {
            let x = spectrum.left() + spectrum.size.width * fraction - origin.x;
            line.left(x - px(1.))
                .top(spectrum.top() - origin.y)
                .w(px(3.))
                .h(spectrum.size.height)
                .border_x_1()
        };
        Some(marker.into_any_element())
    }

    fn overlays(&self, snapshot: &PlotSnapshot, cx: &mut Context<Self>) -> PlotOverlays {
        PlotOverlays {
            marker: self.paste_marker(snapshot, cx).map(|marker| {
                div()
                    .relative()
                    .size_full()
                    .child(marker)
                    .into_any_element()
            }),
            controls: snapshot
                .show_scale_ui
                .then(|| self.ruler_zoom_buttons(snapshot.extents.orientation, cx)),
        }
    }

    fn ruler_zoom_buttons(
        &self,
        orientation: crate::navigation::orientation::Mode,
        cx: &mut Context<Self>,
    ) -> gpui_kit::AnyElement {
        div()
            .id("ruler-zoom-buttons")
            .relative()
            .size_full()
            .child(zoom_pair(
                ZoomPair {
                    tooltips_allowed: self
                        .snapshot
                        .as_ref()
                        .is_some_and(|snapshot| snapshot.tooltips_allowed),
                    horizontal: !orientation.vertical(),
                    zoom_in: "Zoom in time",
                    zoom_out: "Zoom out time",
                },
                ZoomIn,
                ZoomOut,
                true,
                cx,
            ))
            .child(zoom_pair(
                ZoomPair {
                    tooltips_allowed: self
                        .snapshot
                        .as_ref()
                        .is_some_and(|snapshot| snapshot.tooltips_allowed),
                    horizontal: orientation.vertical(),
                    zoom_in: "Zoom in frequency",
                    zoom_out: "Zoom out frequency",
                },
                FrequencyZoomIn,
                FrequencyZoomOut,
                true,
                cx,
            ))
            .into_any_element()
    }

    /// Keep the measured layout and tell the shell what it changes.
    fn layout(
        &mut self,
        bounds: Bounds<Pixels>,
        measured: PlotSize,
        geometry: PlotGeometry,
        gutter: f32,
        cx: &mut Context<Self>,
    ) {
        let old = self.geometry;
        self.gutter_floor = self.gutter_floor.max(gutter);
        self.panel_bounds = Some(bounds);
        self.geometry = Some(geometry);
        self.measured = Some(measured);
        let pointer = self.pointer.and_then(|pointer| self.plot_pointer(pointer));
        if pointer != self.pointer {
            self.pointer = pointer;
            cx.emit(PlotIntent::Pointer);
        }
        cx.emit(PlotIntent::Layout {
            plot: measured,
            time_length_changed: old.is_some_and(|old| old.time_length() != geometry.time_length()),
            frequency_length_changed: old
                .is_some_and(|old| old.frequency_length() != geometry.frequency_length()),
        });
        cx.notify();
    }
}

/// The picture's own rectangle inside the spectrum panel.
fn plot_bounds(frame: &axes::Frame, spectrum_origin: gpui_kit::Point<Pixels>) -> Bounds<Pixels> {
    Bounds {
        origin: spectrum_origin + point(px(frame.plot.x), px(frame.plot.y)),
        size: size(px(frame.plot.width), px(frame.plot.height)),
    }
}

impl Shell {
    /// The start, end and length of the time selection and the width of a band, where any of them shows.
    fn selection_badges(&self) -> Option<axes::SelectionBadges> {
        let badges = axes::SelectionBadges {
            time: self.time_badges(),
            band: self.band_badge(),
        };
        (badges != axes::SelectionBadges::default()).then_some(badges)
    }

    fn time_badges(&self) -> Option<axes::AxisBadges> {
        let view = self.view?;
        let span = self.selection?;
        let visible = view.fractions_of(span)?;
        let edge = |sample: u64| {
            (view.start..=view.start + view.len)
                .contains(&sample)
                .then(|| (sample - view.start) as f64 / view.len as f64)
        };
        Some(axes::AxisBadges {
            fields: crate::navigation::selection_entry::Field::TIME,
            texts: self.entry_scale()?.time_texts(span),
            first: edge(span.start()),
            second: edge(span.end()),
            visible,
            around: self.across_screen(false)?,
            alone: self.band.is_none(),
        })
    }

    /// What the selection covers across the screen of the axis other than time's, or than the band's with `band`.
    fn across_screen(&self, band: bool) -> Option<(f64, f64)> {
        let selected = self.selected()?;
        let vertical = self.session.orientation.vertical();
        Some(match (band, vertical) {
            // Time runs right, or down, as its fractions do.
            (true, _) => selected.time,
            // Frequency runs down from the top, or right with its top at the right edge.
            (false, false) => selected.frequency,
            (false, true) => (1. - selected.frequency.1, 1. - selected.frequency.0),
        })
    }

    fn band_badge(&self) -> Option<axes::AxisBadges> {
        let band = self.band?;
        let (low, high) = self.extents()?.hertz;
        let [low_text, high_text, width] = self.entry_scale()?.band_texts(band);
        // Down from the top of the visible band, or to the right where frequency runs right.
        let vertical = self.session.orientation.vertical();
        let along = |hertz: f64| {
            let down = (high - hertz) / (high - low);
            if vertical { 1. - down } else { down }
        };
        let shown = |hertz: f64| Some(along(hertz)).filter(|at| (0. ..=1.).contains(at));
        let (from, to) = self.selected()?.frequency;
        let visible = match vertical {
            true => (1. - to, 1. - from),
            false => (from, to),
        };
        use crate::navigation::selection_entry::Field;
        let (first, second, texts, fields) = match vertical {
            true => (
                band.low(),
                band.high(),
                [low_text, high_text, width],
                Field::BAND,
            ),
            false => (
                band.high(),
                band.low(),
                [high_text, low_text, width],
                [Field::BandHigh, Field::BandLow, Field::BandWidth],
            ),
        };
        Some(axes::AxisBadges {
            texts,
            fields,
            first: shown(first),
            second: shown(second),
            visible,
            around: self.across_screen(true)?,
            alone: self.selection.is_none(),
        })
    }

    /// The selection as fractions of the visible picture, none where none of it shows.
    pub(in crate::shell) fn selected(&self) -> Option<waveform::Selected> {
        let view = self.view?;
        let time = match self.selection {
            Some(span) => view.fractions_of(span)?,
            None if self.band.is_some() => (0., 1.),
            None => return None,
        };
        let frequency = match self.band {
            Some(band) => {
                let (low, high) = self.extents()?.hertz;
                let from_top = |hertz: f64| ((high - hertz) / (high - low)).clamp(0., 1.);
                let fractions = (from_top(band.high()), from_top(band.low()));
                (fractions.0 < fractions.1).then_some(fractions)?
            }
            None => (0., 1.),
        };
        Some(waveform::Selected { time, frequency })
    }
}

struct PlotOverlays {
    marker: Option<gpui_kit::AnyElement>,
    controls: Option<gpui_kit::AnyElement>,
}

impl PlotOverlays {
    fn prepaint(
        mut self,
        panel: Bounds<Pixels>,
        plot: Bounds<Pixels>,
        controls_visible: bool,
        window: &mut Window,
        cx: &mut gpui_kit::App,
    ) -> Self {
        if let Some(marker) = &mut self.marker {
            marker.prepaint_as_root(
                panel.origin,
                panel.size.map(AvailableSpace::Definite),
                window,
                cx,
            );
        }
        self.controls = self.controls.filter(|_| controls_visible);
        if let Some(controls) = &mut self.controls {
            controls.prepaint_as_root(
                plot.origin,
                plot.size.map(AvailableSpace::Definite),
                window,
                cx,
            );
        }
        self
    }

    fn paint(mut self, window: &mut Window, cx: &mut gpui_kit::App) {
        for element in [&mut self.marker, &mut self.controls].into_iter().flatten() {
            element.paint(window, cx);
        }
    }
}

/// What is painted over the picture, in order from the selection's tint to the Alt guides.
struct Marks {
    selection: Option<waveform::Selected>,
    badge_colors: [gpui_kit::Hsla; 2],
    colors: axes::Colors,
    guides: Option<axes::CursorGuides>,
}

impl Marks {
    fn paint(
        &self,
        frame: &axes::Frame,
        panel: Bounds<Pixels>,
        labels: &axes::Labels,
        badges: &axes::PlacedBadges,
        window: &mut Window,
        cx: &mut gpui_kit::App,
    ) {
        paint_selection(frame, panel.origin, self.selection, window, cx);
        axes::paint(frame, panel.origin, labels, self.colors, window, cx);
        badges.paint(frame, panel.origin, labels, self.badge_colors, window, cx);
        if let Some(guides) = &self.guides {
            guides.paint(frame, panel, labels, window, cx);
        }
    }
}

/// Dim the picture outside the selection and tint it inside, given as fractions of the visible picture, under the grid.
///
/// The dimming shows the selection on a bright picture, the tint on a dark one.
fn paint_selection(
    frame: &axes::Frame,
    spectrum_origin: gpui_kit::Point<Pixels>,
    selection: Option<waveform::Selected>,
    window: &mut Window,
    cx: &gpui_kit::App,
) {
    let Some(selected) = selection else {
        return;
    };
    let plot = plot_bounds(frame, spectrum_origin);
    let rect = waveform::selected_rect(plot, frame.orientation, selected, window.scale_factor());
    for part in waveform::outside(rect, plot) {
        window.paint_quad(gpui_kit::fill(part, OUTSIDE_SELECTION));
    }
    window.paint_quad(gpui_kit::fill(rect, selection_fill(cx)));
}

/// The shade over the picture outside a selection, dark because the owner chose dimming over tinting.
const OUTSIDE_SELECTION: gpui_kit::Hsla = gpui_kit::Hsla {
    h: 0.,
    s: 0.,
    l: 0.,
    a: 0.55,
};

/// The tint laid over the selection, light because the picture under it is dark in either theme.
fn selection_fill(cx: &gpui_kit::App) -> gpui_kit::Hsla {
    cx.theme().blue_light.opacity(0.3)
}

fn axis_colors(cx: &gpui_kit::App, show_grid: bool) -> axes::Colors {
    axes::Colors {
        // Keep the picture visible through the overlaid grid.
        grid: show_grid.then(|| cx.theme().border.opacity(0.55)),
        tick: cx.theme().muted_foreground,
        label: cx.theme().muted_foreground,
    }
}

/// The corner zoom zones `[time, frequency]`, translated into panel
/// coordinates: each pair runs along the ruler of the axis it zooms, as a row
/// in the spectrum's bottom-left corner beside the bottom ruler or as a column
/// in its top-right one beside the right ruler, held [`SCALE_INSET`] clear of
/// the picture's edges.
///
/// The pairs exist only while the scale-controls toggle shows them, and both
/// appear or vanish together: they need the same clear span on *both* sides
/// of the picture, because below it they would overlap in the middle. The
/// span holds one pair plus the other pair's button, clear of both edges.
///
/// Each present zone is exactly the pair's frame, `[+|-]`: two squares and
/// their shared divider inside one border.
fn corner_zones(
    spectrum: Bounds<Pixels>,
    orientation: crate::navigation::orientation::Mode,
    visible: bool,
) -> [Option<axes::Rect>; 2] {
    const MIN_SPAN: f32 = 2.0 * SCALE_INSET + SCALE_PAIR + SCALE_BUTTON;
    if !visible {
        return [None, None];
    }
    let left = f32::from(spectrum.left());
    let top = f32::from(spectrum.top());
    let right = left + f32::from(spectrum.size.width);
    let bottom = top + f32::from(spectrum.size.height);
    if right - left < MIN_SPAN || bottom - top < MIN_SPAN {
        return [None, None];
    }
    let bottom_row = axes::Rect {
        x: left + SCALE_INSET,
        y: bottom - SCALE_INSET - SCALE_BUTTON,
        width: SCALE_PAIR,
        height: SCALE_BUTTON,
    };
    let right_column = axes::Rect {
        x: right - SCALE_INSET - SCALE_BUTTON,
        y: top + SCALE_INSET,
        width: SCALE_BUTTON,
        height: SCALE_PAIR,
    };
    let (time, frequency) = orientation.axes(bottom_row, right_column);
    [Some(time), Some(frequency)]
}

struct ZoomPair {
    tooltips_allowed: bool,
    horizontal: bool,
    zoom_in: &'static str,
    zoom_out: &'static str,
}

/// The square overlay side of one corner pair: 22 logical pixels; the whole
/// pair `[+|-]` spans 45 pixels including its one-pixel divider.
const SCALE_BUTTON: f32 = 22.0;
const SCALE_DIVIDER: f32 = 1.0;
const SCALE_PAIR: f32 = 2.0 * SCALE_BUTTON + SCALE_DIVIDER;

/// The gap between a corner pair and the spectrum's edges, in logical pixels.
const SCALE_INSET: f32 = 8.0;
/// The radius of a pair's outward corner, away from the picture's edges.
const SCALE_ROUNDING: f32 = 6.0;

/// Round the two corners of a half that face away from its pair, to the frame's
/// own radius less its one-pixel border.
fn round_outer(half: Button, horizontal: bool, leading: bool) -> Button {
    let radius = px(SCALE_ROUNDING - 1.);
    match (horizontal, leading) {
        (true, true) => half.rounded_tl(radius).rounded_bl(radius),
        (true, false) => half.rounded_tr(radius).rounded_br(radius),
        (false, true) => half.rounded_tl(radius).rounded_tr(radius),
        (false, false) => half.rounded_bl(radius).rounded_br(radius),
    }
}

fn zoom_pair(
    pair: ZoomPair,
    in_action: impl Action,
    out_action: impl Action,
    enabled: bool,
    cx: &mut Context<PlotView>,
) -> Div {
    let frame = cx.theme().border.opacity(0.75);
    let paper = cx.theme().background.opacity(0.55);
    let horizontal = pair.horizontal;
    let zoom_in = half_button(
        IconName::Plus,
        pair.zoom_in,
        in_action,
        (enabled, pair.tooltips_allowed),
        horizontal,
        true,
        cx,
    );
    let zoom_out = half_button(
        IconName::Minus,
        pair.zoom_out,
        out_action,
        (enabled, pair.tooltips_allowed),
        horizontal,
        false,
        cx,
    );
    let divider = if horizontal {
        div().w(px(SCALE_DIVIDER)).h_full().bg(frame)
    } else {
        div().h(px(SCALE_DIVIDER)).w_full().bg(frame)
    };
    let bar = if horizontal {
        div()
            .flex()
            .w_full()
            .h_full()
            .child(zoom_in)
            .child(divider)
            .child(zoom_out)
    } else {
        div()
            .flex()
            .flex_col()
            .w_full()
            .h_full()
            .child(zoom_in)
            .child(divider)
            .child(zoom_out)
    };
    div()
        .absolute()
        .when(horizontal, |pair| {
            pair.left(px(SCALE_INSET))
                .bottom(px(SCALE_INSET))
                .w(px(SCALE_PAIR))
                .h(px(SCALE_BUTTON))
        })
        .when(!horizontal, |pair| {
            pair.right(px(SCALE_INSET))
                .top(px(SCALE_INSET))
                .w(px(SCALE_BUTTON))
                .h(px(SCALE_PAIR))
        })
        .border_1()
        .border_color(frame)
        .bg(paper)
        .overflow_hidden()
        .cursor(gpui_kit::CursorStyle::Arrow)
        .rounded(px(SCALE_ROUNDING))
        .child(bar)
}

/// One clickable half of a corner pair: a standard button that fills its side
/// of the shared frame, centers its glyph and dispatches the pair's zoom
/// action, leaving keyboard focus on the plot.
fn half_button(
    icon: IconName,
    hint: &'static str,
    action: impl Action + 'static,
    (enabled, tooltips_allowed): (bool, bool),
    horizontal: bool,
    leading: bool,
    cx: &mut Context<PlotView>,
) -> Button {
    let tooltip_action = Box::new(action) as Box<dyn Action>;
    let glyph = if enabled {
        cx.theme().foreground.opacity(0.85)
    } else {
        cx.theme().muted_foreground.opacity(0.5)
    };
    let accent = crate::shell::app_menu_ui::toolbar_accent(cx);
    // The frame supplies the paper and the rounding, so a half paints only the
    // surfaces the pointer gives it.
    let style = ButtonCustomVariant::new(cx)
        .color(cx.theme().transparent)
        .hover(accent.opacity(0.32))
        .active(accent.opacity(0.44));
    let half = Button::new(hint)
        .tab_stop(false)
        .custom(style)
        // The component rounds all four corners, so the inner ones stay square.
        .rounded(ButtonRounded::None)
        .px_0()
        .flex_1();
    let half = round_outer(half, horizontal, leading);
    let half = if horizontal {
        half.h_full()
    } else {
        half.w_full()
    };
    let click = tooltip_action.boxed_clone();
    let mut half = half
        .child(Icon::new(icon).size(px(12.)).text_color(glyph))
        .disabled(!enabled)
        .on_click(cx.listener(move |plot, _, window, cx| {
            window.focus(&plot.focus, cx);
            window.dispatch_action(click.boxed_clone(), cx);
        }));
    if tooltips_allowed {
        half.interactivity().tooltip(move |_, cx| {
            shortcut_tooltip(
                hint.to_owned(),
                Some(tooltip_action.boxed_clone()),
                "Plot",
                px(240.),
                cx,
            )
        });
    }
    half
}

fn unit_tooltip(
    owner: WeakEntity<PlotView>,
    index: usize,
    cx: &mut gpui_kit::App,
) -> gpui_kit::AnyView {
    hints::passive(cx, |cx| {
        if let Some(owner) = owner.upgrade() {
            cx.observe(&owner, |_, _, cx| cx.notify()).detach();
        }
        Tooltip::element(move |window, cx| {
            let hint = owner.upgrade().and_then(|owner| {
                owner
                    .read(cx)
                    .geometry
                    .and_then(|geometry| geometry.unit_hints[index])
            });
            div()
                .flex()
                .flex_col()
                .gap_1()
                .max_w(px(320.).min(window.viewport_size().width - px(48.)))
                .when_some(hint, |content, hint| {
                    content.child(hint.text).child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(hint.resolution()),
                    )
                })
        })
    })
}

fn plot_geometry(
    bounds: Bounds<Pixels>,
    frame: &axes::Frame,
    labels: &axes::Labels,
    height: f32,
    scale: f32,
    scale_ui_visible: bool,
) -> crate::shell::plot::navigation_ui::PlotGeometry {
    let orientation = frame.orientation;
    let (dx, dy) = orientation.axes(px(0.), px(height));
    let spectrum = Bounds {
        origin: bounds.origin + point(dx + px(frame.plot.x), dy + px(frame.plot.y)),
        size: size(px(frame.plot.width), px(frame.plot.height)),
    };
    let bottom = Bounds::new(
        point(spectrum.left(), spectrum.bottom()),
        size(spectrum.size.width, bounds.bottom() - spectrum.bottom()),
    );
    let right = Bounds::new(
        point(spectrum.right(), spectrum.top()),
        size(bounds.right() - spectrum.right(), spectrum.size.height),
    );
    let (time_ruler, frequency_ruler) = if orientation.vertical() {
        (right, bottom)
    } else {
        (bottom, right)
    };
    let minimap = if orientation.vertical() {
        Bounds::new(
            point(bounds.left(), spectrum.top()),
            size(px((height - 1.).max(0.)), spectrum.size.height),
        )
    } else {
        Bounds::new(
            point(spectrum.left(), bounds.top()),
            size(spectrum.size.width, px((height - 1.).max(0.))),
        )
    };
    crate::shell::plot::navigation_ui::PlotGeometry {
        orientation,
        scale,
        unit_hints: frame.unit_hints(labels).map(|hint| {
            hint.map(|mut hint| {
                hint.bounds.x += f32::from(bounds.origin.x + dx);
                hint.bounds.y += f32::from(bounds.origin.y + dy);
                hint
            })
        }),
        zoom_zones: corner_zones(spectrum, orientation, scale_ui_visible),
        time_scheme: frame.time_scheme,
        frequency_scheme: frame.frequency_scheme,
        minimap_columns: oriented_device_size(frame.plot, scale, orientation).width,
        time_ruler,
        frequency_ruler,
        spectrum,
        minimap,
        navigation: if orientation.vertical() {
            Bounds::new(
                point(bounds.left(), spectrum.top()),
                size(bounds.size.width, spectrum.size.height),
            )
        } else {
            Bounds::new(
                point(spectrum.left(), bounds.top()),
                size(spectrum.size.width, bounds.size.height),
            )
        },
    }
}

fn oriented_device_size(
    plot: axes::Rect,
    scale: f32,
    orientation: crate::navigation::orientation::Mode,
) -> PlotSize {
    let physical = device_size(plot, scale);
    let (width, height) = orientation.axes(physical.width, physical.height);
    PlotSize { width, height }
}

pub(in crate::shell) fn mapped_bounds(
    plot: Bounds<Pixels>,
    rectangle: [f32; 4],
    orientation: crate::navigation::orientation::Mode,
) -> Bounds<Pixels> {
    let [x, y, width, height] = orientation.rect(rectangle);
    Bounds::new(
        plot.origin + point(plot.size.width * x, plot.size.height * y),
        size(plot.size.width * width, plot.size.height * height),
    )
}

pub(in crate::shell) fn paint_held(
    texture: Arc<RenderImage>,
    plot: Bounds<Pixels>,
    held: crate::navigation::PictureView,
    shown: crate::navigation::PictureView,
    orientation: crate::navigation::orientation::Mode,
    window: &mut Window,
) {
    if !held.intersects(shown) {
        return;
    }
    let (offset, stretch) = crate::navigation::image_mapping(held.time, shown.time);
    let (y, height) = frequency_mapping(held.frequency, shown.frequency);
    let image = mapped_bounds(
        plot,
        [offset as f32, y, stretch as f32, height],
        orientation,
    );
    window.with_content_mask(Some(gpui_kit::ContentMask { bounds: plot }), |window| {
        spectrogram::paint(texture, image, window);
    });
}

fn frequency_mapping(held: (f64, f64), shown: (f64, f64)) -> (f32, f32) {
    let span = shown.1 - shown.0;
    (
        ((shown.1 - held.1) / span) as f32,
        ((held.1 - held.0) / span) as f32,
    )
}

/// The selection's badges, where the plot last saw them and the one an input stands over.
struct BadgeLayout {
    badges: Option<axes::SelectionBadges>,
    known: Vec<(crate::navigation::selection_entry::Field, Bounds<Pixels>)>,
    typing: Option<crate::navigation::selection_entry::Field>,
    view: WeakEntity<PlotView>,
}

impl BadgeLayout {
    /// Settle the badges, hand the plot where they are, and leave out the one an input stands over.
    fn place(
        &self,
        frame: &axes::Frame,
        labels: &axes::Labels,
        entry: (f32, f32),
        panel: gpui_kit::Point<Pixels>,
        cx: &mut gpui_kit::App,
    ) -> axes::PlacedBadges {
        let mut placed = self
            .badges
            .as_ref()
            .map(|badges| badges.place(frame, labels, entry))
            .unwrap_or_default();
        let rects: Vec<_> = placed
            .badges
            .iter()
            .map(|(field, _, rect)| {
                let origin = panel + point(px(rect.x), px(rect.y));
                (
                    *field,
                    Bounds::new(origin, size(px(rect.width), px(rect.height))),
                )
            })
            .collect();
        if rects != self.known {
            let view = self.view.clone();
            cx.defer(move |cx| {
                let _ = view.update(cx, |plot, cx| plot.place_badges(rects, cx));
            });
        }
        placed
            .badges
            .retain(|(field, ..)| Some(*field) != self.typing);
        placed
    }
}

fn defer_layout(
    view: WeakEntity<PlotView>,
    bounds: Bounds<Pixels>,
    measured: PlotSize,
    geometry: crate::shell::plot::navigation_ui::PlotGeometry,
    gutter: f32,
    cx: &mut gpui_kit::App,
) {
    cx.defer(move |cx| {
        let _ = view.update(cx, |plot, cx| {
            plot.layout(bounds, measured, geometry, gutter, cx)
        });
    });
}

/// Small source-column textures keep coordinates and draw counts bounded.
pub(in crate::shell) struct DeepPreview {
    columns: std::ops::Range<usize>,
    held: (f64, f64),
    frequency: (f64, f64),
    orientation: crate::navigation::orientation::Mode,
    width: usize,
    strips: Vec<(usize, Arc<RenderImage>)>,
}

impl Shell {
    pub(in crate::shell) fn release_deep_preview(
        &mut self,
        window: &mut Window,
        cx: &mut gpui_kit::App,
    ) {
        if let Some(deep) = self.deep_preview.take() {
            retire(self.plot_view(), deep.images(), window, cx);
        }
    }

    pub(in crate::shell) fn prepare_deep_preview(
        &mut self,
        window: &mut Window,
        cx: &mut gpui_kit::App,
    ) {
        let Some(extents) = self.extents() else {
            return;
        };
        let Some(analysis) = self
            .file
            .as_ref()
            .and_then(|file| file.state.document.analysis())
        else {
            return;
        };
        let image = &analysis.spectrogram;
        let held = (image.t0, image.t1);
        if crate::navigation::image_mapping(held, extents.seconds).1 <= 1024.0 {
            self.release_deep_preview(window, cx);
            return;
        }
        if let Some(deep) = DeepPreview::prepare(
            image,
            extents.seconds,
            self.session.orientation,
            self.deep_preview.as_deref(),
        ) {
            self.release_deep_preview(window, cx);
            self.deep_preview = Some(Arc::new(deep));
        }
    }
}

impl DeepPreview {
    pub(in crate::shell) fn prepare(
        image: &argand_core::SpectrogramImage,
        shown: (f64, f64),
        orientation: crate::navigation::orientation::Mode,
        previous: Option<&Self>,
    ) -> Option<Self> {
        let held = (image.t0, image.t1);
        let columns = crate::navigation::visible_columns(held, shown, image.width);
        if previous.is_some_and(|deep| {
            deep.columns == columns
                && deep.held == held
                && deep.width == image.width
                && deep.orientation == orientation
        }) {
            return None;
        }
        let strips = columns
            .clone()
            .filter_map(|column| {
                spectrogram::column_texture(image, column, orientation)
                    .map(|texture| (column, texture))
            })
            .collect();
        Some(Self {
            columns,
            held,
            frequency: (image.f0, image.f1),
            orientation,
            width: image.width,
            strips,
        })
    }

    /// The strips to retire when this preview is replaced.
    pub(in crate::shell) fn images(&self) -> Vec<Arc<RenderImage>> {
        self.strips
            .iter()
            .map(|(_, texture)| texture.clone())
            .collect()
    }

    pub(in crate::shell) fn paint(
        &self,
        plot: Bounds<Pixels>,
        shown: crate::navigation::PictureView,
        orientation: crate::navigation::orientation::Mode,
        window: &mut Window,
    ) {
        let held = crate::navigation::PictureView {
            time: self.held,
            frequency: self.frequency,
        };
        if !held.intersects(shown) {
            return;
        }
        for (column, texture) in &self.strips {
            let (left, right) =
                crate::navigation::column_mapping(self.held, shown.time, self.width, *column);
            let (y, height) = frequency_mapping(self.frequency, shown.frequency);
            let bounds = mapped_bounds(
                plot,
                [left as f32, y, (right - left) as f32, height],
                orientation,
            );
            window.with_content_mask(Some(gpui_kit::ContentMask { bounds: plot }), |window| {
                spectrogram::paint(texture.clone(), bounds, window);
            });
        }
    }
}

#[cfg(test)]
#[path = "plot_ui_test.rs"]
mod tests;

fn mark_first_paint(first: &Option<(Instant, Arc<AtomicBool>)>) {
    if let Some((opened_at, painted)) = first
        && !painted.swap(true, Ordering::Relaxed)
    {
        tracing::debug!(elapsed = ?opened_at.elapsed(), "first picture painted");
    }
}
