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
        let held_time = self
            .file
            .as_ref()
            .and_then(|file| file.document.analysis())
            .map(|analysis| (analysis.db.t0, analysis.db.t1));
        let first_picture = self
            .file
            .as_ref()
            .map(|file| (file.opened_at, file.first_picture.clone()));
        let minimap = self.minimap_panel(cx);
        let fraction = self.session.waveform_fraction;
        let rem = f32::from(cx.theme().font_size);
        let known_bounds = self.panel_bounds;
        let known = self.plot;
        let known_geometry = self.plot_geometry;
        let time_scheme = self.time_scheme;
        let view = cx.entity().downgrade();
        let guides = self.cursor_guides(extents, cx);
        let colors = axis_colors(cx);

        canvas(
            move |bounds, window, cx| {
                let scale = window.scale_factor();
                let labels = axes::Labels::new(window);
                let height =
                    panels::waveform_height(f32::from(bounds.size.height), rem, fraction, scale);
                let spectrum_size = size(bounds.size.width, bounds.size.height - px(height));
                let frame =
                    axes::Frame::measure(spectrum_size, scale, extents, &labels, time_scheme)?;
                let measured = device_size(frame.plot, scale);
                let geometry = plot_geometry(bounds, &frame, &labels, height, measured.width);
                if known != Some(measured)
                    || known_bounds != Some(bounds)
                    || known_geometry != Some(geometry)
                {
                    defer_layout(view.clone(), bounds, measured, geometry, cx);
                }
                Some((frame, labels, height))
            },
            move |bounds, prepainted, window, cx| {
                let Some((frame, labels, height)) = prepainted else {
                    return;
                };
                let spectrum_origin = bounds.origin + point(px(0.0), px(height));
                if let Some(backdrop) = &backdrop {
                    backdrop.paint(
                        &frame,
                        bounds.origin,
                        height,
                        extents.seconds,
                        held_time,
                        window,
                    );
                }
                minimap.paint(&frame, bounds.origin, height, window);
                // The picture first, then the marks over it: a grid line is
                // there to be read against the spectrogram, not under it.
                if let Some(texture) = texture
                    && let Some(held) = held_time
                {
                    let plot = Bounds {
                        origin: spectrum_origin + point(px(frame.plot.x), px(frame.plot.y)),
                        size: size(px(frame.plot.width), px(frame.plot.height)),
                    };
                    if let Some(deep) = deep {
                        deep.paint(plot, extents.seconds, window);
                    } else {
                        paint_held(texture, plot, held, extents.seconds, window);
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
                        size(bounds.size.width, bounds.size.height - px(height)),
                    );
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
            f32::from(bounds.size.height),
            f32::from(window.rem_size()),
            self.session.waveform_fraction,
            window.scale_factor(),
        );
        let panel = size(bounds.size.width, bounds.size.height - px(height));
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
        (self.pointer.is_some() && self.pan.is_none() && !menu_open).then_some(axes::CursorGuides {
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
        if self.pan.is_some() || self.splitter_dragging || menu_open {
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
            .is_some_and(|old| old.navigation.size.width != geometry.navigation.size.width)
        {
            self.time_scheme = None;
            self.tick_pan = None;
        }
        self.panel_bounds = Some(bounds);
        self.plot_geometry = Some(geometry);
        if self.plot != Some(measured) {
            self.resize(measured, cx);
        }
        cx.notify();
    }
}

fn axis_colors(cx: &gpui::App) -> axes::Colors {
    axes::Colors {
        // Keep the picture visible through the overlaid grid.
        grid: cx.theme().border.opacity(0.55),
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
    labels: &axes::Labels,
    height: f32,
    minimap_columns: usize,
) -> navigation_ui::PlotGeometry {
    let spectrum = Bounds {
        origin: bounds.origin + point(px(frame.plot.x), px(height + frame.plot.y)),
        size: size(px(frame.plot.width), px(frame.plot.height)),
    };
    navigation_ui::PlotGeometry {
        unit_hints: frame.unit_hints(labels).map(|hint| {
            hint.map(|mut hint| {
                hint.bounds.x += f32::from(bounds.origin.x);
                hint.bounds.y += f32::from(bounds.origin.y) + height;
                hint
            })
        }),
        time_scheme: frame.time_scheme,
        minimap_columns,
        frequency_ruler: Bounds::new(
            point(spectrum.right(), spectrum.top()),
            size(bounds.right() - spectrum.right(), spectrum.size.height),
        ),
        spectrum,
        minimap: Bounds::new(
            point(spectrum.left(), bounds.top()),
            size(spectrum.size.width, px((height - 1.).max(0.))),
        ),
        navigation: Bounds {
            origin: point(spectrum.left(), bounds.top()),
            size: size(spectrum.size.width, bounds.size.height),
        },
    }
}

pub(super) fn paint_held(
    texture: Arc<RenderImage>,
    plot: Bounds<Pixels>,
    held: (f64, f64),
    shown: (f64, f64),
    window: &mut Window,
) {
    if shown.0 >= shown.1 || held.1 <= shown.0 || held.0 >= shown.1 {
        return;
    }
    let (offset, stretch) = crate::navigation::image_mapping(held, shown);
    let image = Bounds {
        origin: plot.origin + point(plot.size.width * offset as f32, px(0.)),
        size: size(plot.size.width * stretch as f32, plot.size.height),
    };
    window.with_content_mask(Some(gpui::ContentMask { bounds: plot }), |window| {
        spectrogram::paint(texture, image, window);
    });
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
        if let Some(deep) =
            DeepPreview::prepare(image, extents.seconds, self.deep_preview.as_deref())
        {
            self.release_deep_preview(window);
            self.deep_preview = Some(Arc::new(deep));
        }
    }
}

impl DeepPreview {
    pub(super) fn prepare(
        image: &argand_core::SpectrogramImage,
        shown: (f64, f64),
        previous: Option<&Self>,
    ) -> Option<Self> {
        let held = (image.t0, image.t1);
        let columns = crate::navigation::visible_columns(held, shown, image.width);
        if previous.is_some_and(|deep| {
            deep.columns == columns && deep.held == held && deep.width == image.width
        }) {
            return None;
        }
        let strips = columns
            .clone()
            .filter_map(|column| {
                spectrogram::column_texture(image, column).map(|texture| (column, texture))
            })
            .collect();
        Some(Self {
            columns,
            held,
            width: image.width,
            strips,
        })
    }

    pub(super) fn release(&self, window: &mut Window) {
        for (_, texture) in &self.strips {
            release(Some(texture.clone()), window);
        }
    }

    pub(super) fn paint(&self, plot: Bounds<Pixels>, shown: (f64, f64), window: &mut Window) {
        for (column, texture) in &self.strips {
            let (left, right) =
                crate::navigation::column_mapping(self.held, shown, self.width, *column);
            let bounds = Bounds {
                origin: plot.origin + point(plot.size.width * left as f32, px(0.)),
                size: size(plot.size.width * (right - left) as f32, plot.size.height),
            };
            window.with_content_mask(Some(gpui::ContentMask { bounds: plot }), |window| {
                spectrogram::paint(texture.clone(), bounds, window);
            });
        }
    }
}
