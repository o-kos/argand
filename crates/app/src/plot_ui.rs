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
        let held_time = self
            .file
            .as_ref()
            .and_then(|file| file.document.analysis())
            .map(|analysis| (analysis.db.t0, analysis.db.t1));
        let first_picture = self
            .file
            .as_ref()
            .map(|file| (file.opened_at, file.first_picture.clone()));
        let waveform = self.waveform.clone();
        let fraction = self.session.waveform_fraction;
        let rem = f32::from(cx.theme().font_size);
        let known_bounds = self.panel_bounds;
        let known = self.plot;
        let known_geometry = self.plot_geometry;
        let view = cx.entity().downgrade();
        let separator_color = cx.theme().border;
        let colors = axes::Colors {
            // Over the picture rather than beside it, so it is drawn to be
            // read through: an opaque line hides a column of the spectrogram,
            // and a column is what a person is looking at.
            grid: cx.theme().border.opacity(0.55),
            tick: cx.theme().muted_foreground,
            label: cx.theme().muted_foreground,
        };

        canvas(
            move |bounds, window, cx| {
                let scale = window.scale_factor();
                let labels = axes::Labels::new(window);
                let height =
                    panels::waveform_height(f32::from(bounds.size.height), rem, fraction, scale);
                let spectrum_size = size(bounds.size.width, bounds.size.height - px(height));
                let frame = axes::Frame::measure(spectrum_size, scale, extents, &labels)?;
                let measured = device_size(frame.plot, scale);
                let spectrum = Bounds {
                    origin: bounds.origin + point(px(frame.plot.x), px(height + frame.plot.y)),
                    size: size(px(frame.plot.width), px(frame.plot.height)),
                };
                let geometry = navigation_ui::PlotGeometry {
                    spectrum,
                    both: Bounds {
                        origin: point(spectrum.left(), bounds.top()),
                        size: size(spectrum.size.width, spectrum.bottom() - bounds.top()),
                    },
                };
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
                if let Some(waveform) = &waveform {
                    waveform.paint(&frame, bounds.origin, height, extents.seconds, window);
                }
                window.paint_quad(gpui::fill(
                    Bounds {
                        origin: bounds.origin + point(px(frame.plot.x), px(height - 1.0)),
                        size: size(px(frame.plot.width), px(1.0)),
                    },
                    separator_color,
                ));
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
            },
        )
        .size_full()
    }

    fn layout_panels(
        &mut self,
        bounds: Bounds<Pixels>,
        measured: PlotSize,
        geometry: navigation_ui::PlotGeometry,
        cx: &mut Context<Self>,
    ) {
        self.panel_bounds = Some(bounds);
        self.plot_geometry = Some(geometry);
        if self.plot != Some(measured) {
            self.resize(measured, cx);
        }
        cx.notify();
    }
}

fn paint_held(
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
        if let Err(error) = window.paint_image(image, Corners::default(), texture, 0, false) {
            tracing::warn!(%error, "cannot draw the spectrogram");
        }
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
            for (_, texture) in &deep.strips {
                release(Some(texture.clone()), window);
            }
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
        let columns = crate::navigation::visible_columns(held, extents.seconds, image.width);
        if self
            .deep_preview
            .as_ref()
            .is_some_and(|deep| deep.columns == columns)
        {
            return;
        }
        let strips = columns
            .clone()
            .filter_map(|column| {
                spectrogram::column_texture(image, column).map(|texture| (column, texture))
            })
            .collect();
        let deep = DeepPreview {
            columns,
            held,
            width: image.width,
            strips,
        };
        self.release_deep_preview(window);
        self.deep_preview = Some(Arc::new(deep));
    }
}

impl DeepPreview {
    fn paint(&self, plot: Bounds<Pixels>, shown: (f64, f64), window: &mut Window) {
        for (column, texture) in &self.strips {
            let (left, right) =
                crate::navigation::column_mapping(self.held, shown, self.width, *column);
            let bounds = Bounds {
                origin: plot.origin + point(plot.size.width * left as f32, px(0.)),
                size: size(plot.size.width * (right - left) as f32, plot.size.height),
            };
            window.with_content_mask(Some(gpui::ContentMask { bounds: plot }), |window| {
                if let Err(error) =
                    window.paint_image(bounds, Corners::default(), texture.clone(), 0, false)
                {
                    tracing::warn!(%error, "cannot draw the held spectrogram column");
                }
            });
        }
    }
}
