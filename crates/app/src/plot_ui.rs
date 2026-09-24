//! Synchronized plot painting and measured pointer geometry.

use super::navigation_ui::*;
use super::plot_view::{PlotIntent, PlotSnapshot, PlotView};
use super::*;
use gpui_kit::Div;
use gpui_kit::component::{Icon, IconName};

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
                .and_then(|file| file.document.analysis())
                .map(|analysis| crate::navigation::PictureView::grid(&analysis.db)),
            self.file
                .as_ref()
                .map(|file| (file.opened_at, file.first_picture.clone())),
        )
    }

    /// What the plot paints this frame, or nothing when no plot is shown.
    pub(super) fn plot_snapshot(&self, cx: &gpui_kit::App) -> Option<PlotSnapshot> {
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
            fraction: self.session.waveform_fraction,
            time_scheme: self.time_scheme,
            frequency_scheme: self.frequency_scheme,
            frequency: self.frequency,
            show_grid: self.session.show_grid,
            show_scale_ui: self.session.show_scale_ui,
        })
    }

    pub(super) fn measure_time_scheme(
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
            self.session.waveform_fraction,
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
        let displayed = self.file.as_ref().and_then(|file| file.displayed_settings);
        let ink = self
            .settings
            .minimap_colormap(displayed)
            .waveform_ink(cx.theme().mode.is_dark());
        waveform::Panel {
            waveform: self.waveform.clone(),
            viewport: self.view.zip(
                self.file
                    .as_ref()
                    .and_then(|file| file.document.meta())
                    .map(|meta| meta.len_samples),
            ),
            separator: cx.theme().border,
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
    pub(super) fn spectrogram(
        &self,
        snapshot: PlotSnapshot,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let PlotSnapshot {
            extents,
            texture,
            deep,
            backdrop,
            held: held_view,
            first_picture,
            minimap,
            fraction,
            time_scheme,
            frequency_scheme,
            show_grid,
            show_scale_ui: scale_ui_visible,
            ..
        } = snapshot;
        let orientation = extents.orientation;
        let rem = f32::from(cx.theme().font_size);
        let known_bounds = self.panel_bounds;
        let known = self.measured;
        let known_geometry = self.geometry;
        let held = axes::Held {
            time: time_scheme,
            frequency: frequency_scheme,
            gutter: self.gutter_floor,
        };
        let view = cx.entity().downgrade();
        let guides = self.cursor_guides(extents, cx);
        let colors = axis_colors(cx, show_grid);

        canvas(
            move |bounds, window, cx| {
                let scale = window.scale_factor();
                let labels = axes::Labels::new(window);
                let height = panels::waveform_height(
                    f32::from(orientation.axes(bounds.size.width, bounds.size.height).1),
                    rem,
                    fraction,
                    scale,
                );
                let (dx, dy) = orientation.axes(px(0.), px(height));
                let spectrum_size = size(bounds.size.width - dx, bounds.size.height - dy);
                let frame =
                    axes::Frame::measure_view(spectrum_size, scale, extents, &labels, held)?;
                let measured = oriented_device_size(frame.plot, scale, orientation);
                let geometry =
                    plot_geometry(bounds, &frame, &labels, height, scale, scale_ui_visible);
                if known != Some(measured)
                    || known_bounds != Some(bounds)
                    || known_geometry != Some(geometry)
                    || frame.gutter > held.gutter
                {
                    defer_layout(view.clone(), bounds, measured, geometry, frame.gutter, cx);
                }
                Some((frame, labels, height))
            },
            move |bounds, prepainted, window, cx| {
                let Some((frame, labels, height)) = prepainted else {
                    return;
                };
                let (dx, dy) = orientation.axes(px(0.), px(height));
                let spectrum_origin = bounds.origin + point(dx, dy);
                if let Some(backdrop) = &backdrop {
                    backdrop.paint(
                        &frame,
                        bounds.origin,
                        height,
                        extents.picture(),
                        held_view,
                        window,
                    );
                }
                minimap.paint(&frame, bounds.origin, height, window);
                // The picture first, then the marks over it: a grid line is
                // there to be read against the spectrogram, not under it.
                if let Some(texture) = texture
                    && let Some(held) = held_view
                {
                    let plot = Bounds {
                        origin: spectrum_origin + point(px(frame.plot.x), px(frame.plot.y)),
                        size: size(px(frame.plot.width), px(frame.plot.height)),
                    };
                    if let Some(deep) = deep {
                        deep.paint(plot, extents.picture(), orientation, window);
                    } else {
                        paint_held(texture, plot, held, extents.picture(), orientation, window);
                    }
                    if let Some((opened_at, painted)) = &first_picture
                        && !painted.swap(true, Ordering::Relaxed)
                    {
                        tracing::debug!(elapsed = ?opened_at.elapsed(), "first picture painted");
                    }
                }
                axes::paint(&frame, spectrum_origin, &labels, colors, window, cx);
                if let Some(guides) = &guides {
                    let panel = Bounds::new(
                        spectrum_origin,
                        size(bounds.size.width - dx, bounds.size.height - dy),
                    );
                    guides.paint(&frame, panel, &labels, window, cx);
                }
            },
        )
        .size_full()
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

    pub(super) fn unit_hint(
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
        if self.pan.is_some() || self.frequency_pan.is_some() || self.splitter_dragging || menu_open
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

    pub(super) fn ruler_zoom_buttons(
        &self,
        snapshot: &PlotSnapshot,
        cx: &mut Context<Self>,
    ) -> Option<gpui_kit::AnyElement> {
        if !snapshot.show_scale_ui {
            return None;
        }
        let geometry = self.geometry?;
        let origin = self.panel_bounds?.origin;
        let enabled = true;
        let [time, frequency] = geometry.zoom_zones;
        Some(
            div()
                .id("ruler-zoom-buttons")
                .absolute()
                .inset_0()
                .children(time.map(|zone| {
                    zoom_pair(
                        ZoomPair {
                            zone,
                            zoom_in: "Zoom in time",
                            zoom_out: "Zoom out time",
                        },
                        ZoomIn,
                        ZoomOut,
                        enabled,
                        [
                            self.pressed_zoom == Some("Zoom in time"),
                            self.pressed_zoom == Some("Zoom out time"),
                        ],
                        origin,
                        cx,
                    )
                }))
                .children(frequency.map(|zone| {
                    zoom_pair(
                        ZoomPair {
                            zone,
                            zoom_in: "Zoom in frequency",
                            zoom_out: "Zoom out frequency",
                        },
                        FrequencyZoomIn,
                        FrequencyZoomOut,
                        enabled,
                        [
                            self.pressed_zoom == Some("Zoom in frequency"),
                            self.pressed_zoom == Some("Zoom out frequency"),
                        ],
                        origin,
                        cx,
                    )
                }))
                .into_any_element(),
        )
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
        cx.emit(PlotIntent::Layout {
            plot: measured,
            time_length_changed: old.is_some_and(|old| old.time_length() != geometry.time_length()),
            frequency_length_changed: old
                .is_some_and(|old| old.frequency_length() != geometry.frequency_length()),
        });
        cx.notify();
    }
}

fn axis_colors(cx: &gpui_kit::App, show_grid: bool) -> axes::Colors {
    axes::Colors {
        // Keep the picture visible through the overlaid grid.
        grid: show_grid.then(|| cx.theme().border.opacity(0.55)),
        tick: cx.theme().muted_foreground,
        label: cx.theme().muted_foreground,
    }
}

/// The corner zoom zones, translated into panel coordinates: the time pair in
/// the spectrum's bottom-left corner, the frequency pair in its top-right
/// one, in both orientations, held [`SCALE_INSET`] clear of the picture's
/// edges.
///
/// The pairs exist only while the scale-controls toggle shows them, and both
/// appear or vanish together: they need the same clear span on *both* sides
/// of the picture, because below it they would overlap in the middle. The
/// span holds one pair plus the other pair's button, clear of both edges.
///
/// Each present zone is exactly the pair's frame, `[+|-]`: two squares and
/// their shared divider inside one border.
fn corner_zones(spectrum: Bounds<Pixels>, visible: bool) -> [Option<axes::Rect>; 2] {
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
    [
        Some(axes::Rect {
            x: left + SCALE_INSET,
            y: bottom - SCALE_INSET - SCALE_BUTTON,
            width: SCALE_PAIR,
            height: SCALE_BUTTON,
        }),
        Some(axes::Rect {
            x: right - SCALE_INSET - SCALE_BUTTON,
            y: top + SCALE_INSET,
            width: SCALE_BUTTON,
            height: SCALE_PAIR,
        }),
    ]
}

struct ZoomPair {
    zone: axes::Rect,
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

fn zoom_pair(
    pair: ZoomPair,
    in_action: impl Action,
    out_action: impl Action,
    enabled: bool,
    pressed: [bool; 2],
    origin: gpui_kit::Point<Pixels>,
    cx: &mut Context<PlotView>,
) -> Div {
    let frame = cx.theme().border.opacity(0.75);
    let paper = cx.theme().background.opacity(0.55);
    let horizontal = pair.zone.width > pair.zone.height;
    let zoom_in = half_button(
        IconName::Plus,
        pair.zoom_in,
        in_action,
        enabled,
        pressed[0],
        horizontal,
        cx,
    );
    let zoom_out = half_button(
        IconName::Minus,
        pair.zoom_out,
        out_action,
        enabled,
        pressed[1],
        horizontal,
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
        .left(px(pair.zone.x) - origin.x)
        .top(px(pair.zone.y) - origin.y)
        .w(px(pair.zone.width))
        .h(px(pair.zone.height))
        .border_1()
        .border_color(frame)
        .bg(paper)
        .overflow_hidden()
        .cursor(gpui_kit::CursorStyle::Arrow)
        .rounded(px(SCALE_ROUNDING))
        .child(bar)
}

/// One clickable half of a corner pair: it fills its side of the shared
/// frame, centers its glyph, presses while held, and dispatches the pair's
/// zoom action.
fn half_button(
    icon: IconName,
    hint: &'static str,
    action: impl Action + 'static,
    enabled: bool,
    pressed: bool,
    horizontal: bool,
    cx: &mut Context<PlotView>,
) -> gpui_kit::Stateful<Div> {
    let tooltip_action = Box::new(action) as Box<dyn Action>;
    let glyph = if enabled {
        cx.theme().foreground.opacity(0.85)
    } else {
        cx.theme().muted_foreground.opacity(0.5)
    };
    let accent = super::app_menu_ui::toolbar_accent(cx);
    let hover = accent.opacity(0.32);
    let press = accent.opacity(0.44);
    let mut half = div()
        .id(hint)
        .flex()
        .items_center()
        .justify_center()
        .cursor(gpui_kit::CursorStyle::Arrow)
        .child(Icon::new(icon).size(px(12.)).text_color(glyph));
    half = if horizontal {
        half.flex_1().h_full()
    } else {
        half.flex_1().w_full()
    };
    half = if enabled && pressed {
        // Pressed wins outright: a hover refinement would repaint the same
        // shade the pointer already shows while it holds the button down.
        half.bg(press)
    } else if enabled {
        half.hover(move |style| style.bg(hover))
    } else {
        half
    };
    if enabled {
        let click = tooltip_action.boxed_clone();
        half = half
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |plot, _, _, cx| {
                    plot.pressed_zoom = Some(hint);
                    cx.notify();
                }),
            )
            .on_click(cx.listener(move |plot, _, window, cx| {
                window.focus(&plot.focus, cx);
                window.dispatch_action(click.boxed_clone(), cx);
            }));
    }
    half = half
        .on_mouse_up(
            MouseButton::Left,
            cx.listener(|plot, _, _, cx| plot.release_press(cx)),
        )
        .on_mouse_up_out(
            MouseButton::Left,
            cx.listener(|plot, _, _, cx| plot.release_press(cx)),
        );
    half.tooltip(move |window, cx| {
        shortcut_tooltip(
            hint.to_owned(),
            Some(tooltip_action.boxed_clone()),
            "Plot",
            px(240.),
        )
        .build(window, cx)
    })
}

fn unit_tooltip(
    owner: WeakEntity<PlotView>,
    index: usize,
    cx: &mut gpui_kit::App,
) -> gpui_kit::AnyView {
    cx.new(|cx| {
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
    .into()
}

fn plot_geometry(
    bounds: Bounds<Pixels>,
    frame: &axes::Frame,
    labels: &axes::Labels,
    height: f32,
    scale: f32,
    scale_ui_visible: bool,
) -> navigation_ui::PlotGeometry {
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
    navigation_ui::PlotGeometry {
        orientation,
        scale,
        unit_hints: frame.unit_hints(labels).map(|hint| {
            hint.map(|mut hint| {
                hint.bounds.x += f32::from(bounds.origin.x + dx);
                hint.bounds.y += f32::from(bounds.origin.y + dy);
                hint
            })
        }),
        zoom_zones: corner_zones(spectrum, scale_ui_visible),
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
    orientation: crate::orientation::Mode,
) -> PlotSize {
    let physical = device_size(plot, scale);
    let (width, height) = orientation.axes(physical.width, physical.height);
    PlotSize { width, height }
}

pub(super) fn mapped_bounds(
    plot: Bounds<Pixels>,
    rectangle: [f32; 4],
    orientation: crate::orientation::Mode,
) -> Bounds<Pixels> {
    let [x, y, width, height] = orientation.rect(rectangle);
    Bounds::new(
        plot.origin + point(plot.size.width * x, plot.size.height * y),
        size(plot.size.width * width, plot.size.height * height),
    )
}

pub(super) fn paint_held(
    texture: Arc<RenderImage>,
    plot: Bounds<Pixels>,
    held: crate::navigation::PictureView,
    shown: crate::navigation::PictureView,
    orientation: crate::orientation::Mode,
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

fn defer_layout(
    view: WeakEntity<PlotView>,
    bounds: Bounds<Pixels>,
    measured: PlotSize,
    geometry: navigation_ui::PlotGeometry,
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
pub(super) struct DeepPreview {
    columns: std::ops::Range<usize>,
    held: (f64, f64),
    frequency: (f64, f64),
    orientation: crate::orientation::Mode,
    width: usize,
    strips: Vec<(usize, Arc<RenderImage>)>,
}

impl Shell {
    pub(super) fn release_deep_preview(&mut self, window: &mut Window, cx: &mut gpui_kit::App) {
        if let Some(deep) = self.deep_preview.take() {
            retire(self.plot_view(), deep.images(), window, cx);
        }
    }

    pub(super) fn prepare_deep_preview(&mut self, window: &mut Window, cx: &mut gpui_kit::App) {
        let Some(extents) = self.extents() else {
            return;
        };
        let Some(analysis) = self.file.as_ref().and_then(|file| file.document.analysis()) else {
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
    pub(super) fn prepare(
        image: &argand_core::SpectrogramImage,
        shown: (f64, f64),
        orientation: crate::orientation::Mode,
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
    pub(super) fn images(&self) -> Vec<Arc<RenderImage>> {
        self.strips
            .iter()
            .map(|(_, texture)| texture.clone())
            .collect()
    }

    pub(super) fn paint(
        &self,
        plot: Bounds<Pixels>,
        shown: crate::navigation::PictureView,
        orientation: crate::orientation::Mode,
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
mod tests {
    use super::*;

    fn spectrum() -> Bounds<Pixels> {
        Bounds::new(point(px(30.), px(20.)), size(px(640.), px(300.)))
    }

    #[test]
    fn corner_zones_sit_in_bottom_left_and_top_right_in_both_orientations() {
        let spectrum = spectrum();
        let [time, frequency] = corner_zones(spectrum, true);
        let time = time.unwrap();
        assert_eq!(
            (time.x, time.y, time.width, time.height),
            (
                30. + SCALE_INSET,
                20. + 300. - SCALE_INSET - SCALE_BUTTON,
                SCALE_PAIR,
                SCALE_BUTTON
            )
        );
        let frequency = frequency.unwrap();
        assert_eq!(
            (frequency.x, frequency.y, frequency.width, frequency.height),
            (
                30. + 640. - SCALE_INSET - SCALE_BUTTON,
                20. + SCALE_INSET,
                SCALE_BUTTON,
                SCALE_PAIR
            )
        );
    }

    #[test]
    fn hidden_scale_controls_leave_no_zones() {
        for zone in corner_zones(spectrum(), false) {
            assert!(zone.is_none(), "a hidden toggle leaves no pair");
        }
    }

    #[test]
    fn corner_zones_vanish_together_on_small_spectrums() {
        let tiny = Bounds::new(point(px(0.), px(0.)), size(px(40.), px(40.)));
        for zone in corner_zones(tiny, true) {
            assert!(zone.is_none(), "a 40-pixel spectrum fits no pair");
        }
        // Either side alone being too small drops both pairs: a narrow but
        // tall spectrum keeps no frequency pair.
        let narrow = Bounds::new(point(px(0.), px(0.)), size(px(40.), px(400.)));
        for zone in corner_zones(narrow, true) {
            assert!(zone.is_none(), "a 40-pixel side fits no pair");
        }
        // Below the minimum the two corner pairs would overlap in the
        // middle of the picture, so the minimum is exact.
        let cramped = Bounds::new(point(px(0.), px(0.)), size(px(82.), px(82.)));
        for zone in corner_zones(cramped, true) {
            assert!(zone.is_none(), "an 82-pixel spectrum still overlaps");
        }
        let snug = Bounds::new(point(px(0.), px(0.)), size(px(83.), px(83.)));
        let [time, frequency] = corner_zones(snug, true);
        let (time, frequency) = (time.unwrap(), frequency.unwrap());
        assert!(
            time.x + time.width <= frequency.x || frequency.x + frequency.width <= time.x,
            "the horizontal spans do not overlap"
        );
        assert!(
            frequency.y + frequency.height <= time.y || time.y + time.height <= frequency.y,
            "the vertical spans do not overlap"
        );
    }
}
