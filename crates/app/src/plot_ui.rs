//! Synchronized plot painting and measured pointer geometry.

use super::*;

impl Shell {
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
        extents: axes::Extents,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let texture = self.texture.clone();
        let deep = self.deep_preview.clone();
        let backdrop = self.backdrop.clone();
        let held_view = self
            .file
            .as_ref()
            .and_then(|file| file.document.analysis())
            .map(|analysis| crate::navigation::PictureView::grid(&analysis.db));
        let first_picture = self
            .file
            .as_ref()
            .map(|file| (file.opened_at, file.first_picture.clone()));
        let minimap = self.minimap_panel(cx);
        let orientation = self.session.orientation;
        let fraction = self.session.waveform_fraction;
        let rem = f32::from(cx.theme().font_size);
        let known_bounds = self.panel_bounds;
        let known = self.plot;
        let known_geometry = self.plot_geometry;
        let time_scheme = self.time_scheme;
        let frequency_scheme = self.frequency_scheme;
        let view = cx.entity().downgrade();
        let guides = self.cursor_guides(extents, cx);
        let colors = axis_colors(cx, self.session.show_grid);

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
                let frame = axes::Frame::measure_view(
                    spectrum_size,
                    scale,
                    extents,
                    &labels,
                    time_scheme,
                    frequency_scheme,
                )?;
                let measured = oriented_device_size(frame.plot, scale, orientation);
                let geometry = plot_geometry(bounds, &frame, &labels, height, measured.width);
                if known != Some(measured)
                    || known_bounds != Some(bounds)
                    || known_geometry != Some(geometry)
                {
                    defer_layout(view.clone(), bounds, measured, geometry, cx);
                }
                Some((frame, labels, height, spectrum_size))
            },
            move |bounds, prepainted, window, cx| {
                let Some((frame, labels, height, spectrum_size)) = prepainted else {
                    return;
                };
                let (ox, oy) = orientation.spectrum_offset(px(height));
                let spectrum_origin = bounds.origin + point(ox, oy);
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
                minimap.paint(&frame, bounds, height, window);
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
                    let panel = Bounds::new(spectrum_origin, spectrum_size);
                    guides.paint(&frame, panel, &labels, window, cx);
                }
            },
        )
        .size_full()
    }

    pub(super) fn measure_time_scheme(
        &self,
        window: &Window,
    ) -> Option<argand_core::axis::TickScheme> {
        let bounds = self.panel_bounds?;
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
        axes::Frame::measure(
            panel,
            window.scale_factor(),
            self.extents()?,
            &axes::Labels::new(window),
            None,
        )?
        .time_scheme
    }

    fn minimap_panel(&self, cx: &gpui::App) -> waveform::Panel {
        waveform::Panel {
            waveform: self.waveform.clone(),
            viewport: self.view.zip(
                self.file
                    .as_ref()
                    .and_then(|file| file.document.meta())
                    .map(|meta| meta.len_samples),
            ),
            separator: cx.theme().border,
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
        (self.pointer.is_some() && self.pan.is_none() && self.frequency_pan.is_none() && !menu_open)
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
        origin: gpui::Point<Pixels>,
        cx: &Context<Self>,
    ) -> Option<gpui::AnyElement> {
        let menu_open = self
            .open_menu
            .as_ref()
            .and_then(WeakEntity::upgrade)
            .is_some();
        if self.pan.is_some() || self.frequency_pan.is_some() || self.splitter_dragging || menu_open
        {
            return None;
        }
        let hint = self.plot_geometry?.unit_hints[index]?;
        let owner = cx.entity().downgrade();
        Some(
            div()
                .id(hint.text)
                .absolute()
                .left(px(hint.bounds.x) - origin.x)
                .top(px(hint.bounds.y) - origin.y)
                .w(px(hint.bounds.width))
                .h(px(hint.bounds.height))
                .tooltip(move |_, cx| unit_tooltip(owner.clone(), index, cx))
                .into_any_element(),
        )
    }

    fn layout_panels(
        &mut self,
        bounds: Bounds<Pixels>,
        measured: PlotSize,
        geometry: navigation_ui::PlotGeometry,
        cx: &mut Context<Self>,
    ) {
        if self
            .plot_geometry
            .is_some_and(|old| old.time_length() != geometry.time_length())
        {
            self.time_scheme = None;
            self.tick_pan = None;
        }
        if self
            .plot_geometry
            .is_some_and(|old| old.frequency_length() != geometry.frequency_length())
        {
            self.frequency_scheme = None;
        }
        self.panel_bounds = Some(bounds);
        self.plot_geometry = Some(geometry);
        if self.plot != Some(measured) {
            self.resize(measured, cx);
        }
        cx.notify();
    }
}

fn axis_colors(cx: &gpui::App, show_grid: bool) -> axes::Colors {
    axes::Colors {
        // Keep the picture visible through the overlaid grid.
        grid: show_grid.then(|| cx.theme().border.opacity(0.55)),
        tick: cx.theme().muted_foreground,
        label: cx.theme().muted_foreground,
    }
}

fn unit_tooltip(owner: WeakEntity<Shell>, index: usize, cx: &mut gpui::App) -> gpui::AnyView {
    cx.new(|cx| {
        if let Some(owner) = owner.upgrade() {
            cx.observe(&owner, |_, _, cx| cx.notify()).detach();
        }
        Tooltip::element(move |window, cx| {
            let hint = owner.upgrade().and_then(|owner| {
                owner
                    .read(cx)
                    .plot_geometry
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
    labels: &dyn argand_core::axis::LabelMeasure,
    height: f32,
    minimap_columns: usize,
) -> navigation_ui::PlotGeometry {
    let orientation = frame.orientation;
    let (dx, dy) = orientation.spectrum_offset(px(height));
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
        size(
            bounds.right()
                - spectrum.right()
                - if orientation.vertical() {
                    px(height)
                } else {
                    px(0.)
                },
            spectrum.size.height,
        ),
    );
    let (time_ruler, frequency_ruler) = if orientation.vertical() {
        (right, bottom)
    } else {
        (bottom, right)
    };
    let minimap = if orientation.vertical() {
        Bounds::new(
            point(bounds.right() - px(height) + px(1.), spectrum.top()),
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
        unit_hints: frame.unit_hints(labels).map(|hint| {
            hint.map(|mut hint| {
                hint.bounds.x += f32::from(bounds.origin.x + dx);
                hint.bounds.y += f32::from(bounds.origin.y + dy);
                hint
            })
        }),
        time_scheme: frame.time_scheme,
        frequency_scheme: frame.frequency_scheme,
        minimap_columns,
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
    window.with_content_mask(Some(gpui::ContentMask { bounds: plot }), |window| {
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
    view: WeakEntity<Shell>,
    bounds: Bounds<Pixels>,
    measured: PlotSize,
    geometry: navigation_ui::PlotGeometry,
    cx: &mut gpui::App,
) {
    cx.defer(move |cx| {
        let _ = view.update(cx, |shell, cx| {
            shell.layout_panels(bounds, measured, geometry, cx)
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
    pub(super) fn release_deep_preview(&mut self, window: &mut Window) {
        if let Some(deep) = self.deep_preview.take() {
            deep.release(window);
        }
    }

    pub(super) fn prepare_deep_preview(&mut self, window: &mut Window) {
        let Some(extents) = self.extents() else {
            return;
        };
        let Some(analysis) = self.file.as_ref().and_then(|file| file.document.analysis()) else {
            return;
        };
        let image = &analysis.spectrogram;
        let held = (image.t0, image.t1);
        if crate::navigation::image_mapping(held, extents.seconds).1 <= 1024.0 {
            self.release_deep_preview(window);
            return;
        }
        if let Some(deep) = DeepPreview::prepare(
            image,
            extents.seconds,
            self.session.orientation,
            self.deep_preview.as_deref(),
        ) {
            self.release_deep_preview(window);
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

    pub(super) fn release(&self, window: &mut Window) {
        for (_, texture) in &self.strips {
            release(Some(texture.clone()), window);
        }
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
            window.with_content_mask(Some(gpui::ContentMask { bounds: plot }), |window| {
                spectrogram::paint(texture.clone(), bounds, window);
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orientation::Mode;
    use argand_core::testutil::DejaVuSans;

    #[test]
    fn minimap_and_rulers_follow_layout_without_overlapping() {
        let bounds = Bounds::new(point(px(100.), px(200.)), size(px(800.), px(600.)));
        for orientation in [Mode::Horizontal, Mode::Vertical] {
            for thickness in [48., 180.] {
                let (dx, dy) = orientation.axes(px(0.), px(thickness));
                let panel = size(bounds.size.width - dx, bounds.size.height - dy);
                let extents = axes::Extents {
                    orientation,
                    time: crate::time_ruler::Ruler::CLOCK,
                    seconds: (0., 10.),
                    hertz: (-12_000., 12_000.),
                };
                let frame = axes::Frame::measure(panel, 1.25, extents, &DejaVuSans, None).unwrap();
                let geometry = plot_geometry(bounds, &frame, &DejaVuSans, thickness, 1000);
                assert_eq!(geometry.spectrum.left(), bounds.left() + px(frame.plot.x));
                match orientation {
                    Mode::Vertical => {
                        assert_eq!(geometry.minimap.right(), bounds.right());
                        assert_eq!(geometry.minimap.left(), bounds.right() - px(thickness - 1.));
                        assert_eq!(
                            geometry.time_ruler.right(),
                            geometry.minimap.left() - px(1.)
                        );
                        assert_eq!(geometry.time_ruler.left(), geometry.spectrum.right());
                        assert_eq!(geometry.minimap.top(), geometry.spectrum.top());
                        assert_eq!(geometry.minimap.bottom(), geometry.spectrum.bottom());
                        let frequency = geometry.unit_hints[1].unwrap().bounds;
                        assert!(px(frequency.x) >= geometry.minimap.left());
                        assert!(px(frequency.x + frequency.width) < bounds.right());
                    }
                    Mode::Horizontal => {
                        assert_eq!(geometry.minimap.top(), bounds.top());
                        assert_eq!(geometry.minimap.bottom(), geometry.spectrum.top() - px(1.));
                        assert_eq!(geometry.minimap.left(), geometry.spectrum.left());
                        assert_eq!(geometry.minimap.right(), geometry.spectrum.right());
                    }
                }
            }
        }
    }
}
