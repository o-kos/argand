//! Retain one wider picture to fill already-known time on zoom out.

use super::*;
use crate::analysis::Update;

pub(super) struct Refresh {
    settings: Settings,
    shading: argand_dsp::Shading,
    delivery: Delivery,
    _task: Task<()>,
}

struct PreparedStyle {
    db: Arc<argand_core::DbGrid>,
    image: argand_core::SpectrogramImage,
    settings: Settings,
    shading: argand_dsp::Shading,
}

impl Shell {
    pub(super) fn refresh_backdrop_style(
        &mut self,
        delivery: Delivery,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<Delivery> {
        let Some(backdrop) = &self.backdrop else {
            return Some(delivery);
        };
        let settings = self.settings;
        let same_transform = Settings {
            colormap: settings.colormap,
            dynamic_range: settings.dynamic_range,
            ..backdrop.settings
        }
        .equivalent(settings);
        if backdrop.settings.equivalent(settings) || !same_transform {
            return Some(delivery);
        }
        let (Update::Snapshot { analysis, .. } | Update::Ready { analysis, .. }) = &delivery.update
        else {
            return Some(delivery);
        };
        let shading = argand_dsp::Shading {
            colormap: settings.colormap,
            db_min: analysis.spectrogram.db_min,
            db_max: analysis.spectrogram.db_max,
        };
        if let Some(refresh) = &mut self.backdrop_refresh
            && refresh.settings.equivalent(settings)
            && refresh.shading == shading
        {
            refresh.delivery = delivery;
            return None;
        }
        let db = backdrop.db.clone();
        let job = cx.background_executor().spawn(async move {
            let image = argand_dsp::shade(&db, shading);
            PreparedStyle {
                db,
                image,
                settings,
                shading,
            }
        });
        let task = cx.spawn_in(window, async move |shell, cx| {
            let prepared = job.await;
            let _ = shell.update_in(cx, |shell, window, cx| {
                shell.finish_backdrop_style(prepared, window, cx);
            });
        });
        self.backdrop_refresh = Some(Refresh {
            settings,
            shading,
            delivery,
            _task: task,
        });
        None
    }

    fn finish_backdrop_style(
        &mut self,
        prepared: PreparedStyle,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let matching = self.backdrop_refresh.as_ref().is_some_and(|refresh| {
            refresh.settings.equivalent(prepared.settings) && refresh.shading == prepared.shading
        });
        if !matching {
            return;
        }
        let Some(refresh) = self.backdrop_refresh.take() else {
            return;
        };
        let PreparedStyle { db, image, .. } = prepared;
        let accepted = self
            .file
            .as_ref()
            .is_some_and(|file| file.analyst.accepts(&refresh.delivery));
        let Some(backdrop) = &mut self.backdrop else {
            return;
        };
        if !accepted
            || !Arc::ptr_eq(&db, &backdrop.db)
            || !refresh.settings.equivalent(self.settings)
        {
            return;
        }
        let Some(texture) = spectrogram::texture(&image) else {
            return;
        };
        release(
            Some(std::mem::replace(&mut backdrop.texture, texture)),
            window,
        );
        if let Some(deep) = backdrop.deep.take() {
            deep.release(window);
        }
        backdrop.image = Arc::new(image);
        backdrop.settings = refresh.settings;
        self.receive(refresh.delivery, window, cx);
    }
}

#[derive(Clone)]
pub(super) struct Backdrop {
    db: Arc<argand_core::DbGrid>,
    image: Arc<argand_core::SpectrogramImage>,
    texture: Arc<RenderImage>,
    settings: Settings,
    complete: bool,
    deep: Option<Arc<plot_ui::DeepPreview>>,
}

impl Shell {
    pub(super) fn release_backdrop(&mut self, window: &mut Window) {
        self.backdrop_refresh = None;
        if let Some(backdrop) = self.backdrop.take() {
            release(Some(backdrop.texture.clone()), window);
            if let Some(deep) = &backdrop.deep {
                deep.release(window);
            }
        }
    }

    pub(super) fn prepare_backdrop(&mut self, window: &mut Window) {
        let Some(file) = &self.file else { return };
        let Some(settings) = file.displayed_settings else {
            return;
        };
        if self
            .backdrop
            .as_ref()
            .is_some_and(|backdrop| !backdrop.settings.equivalent(settings))
        {
            self.release_backdrop(window);
        }
        self.retain_wider_picture(settings, window);
        let Some(shown) = self.extents().map(|extents| extents.seconds) else {
            return;
        };
        let Some(backdrop) = self.backdrop.as_mut() else {
            return;
        };
        backdrop.prepare(shown, window);
    }

    fn retain_wider_picture(&mut self, settings: Settings, window: &mut Window) {
        // A refresh owns the only pending foreground delivery. Its source grid
        // must survive until that delivery is applied or explicitly invalidated.
        let refreshing = self.backdrop_refresh.is_some();
        let Some(file) = &self.file else { return };
        let complete = matches!(file.document.status(), Status::Ready { .. });
        let Some(analysis) = file.document.analysis() else {
            return;
        };
        let incoming = crate::navigation::PictureView::grid(&analysis.db);
        if !complete
            && self
                .extents()
                .is_some_and(|extents| extents.picture() == incoming)
        {
            return;
        }
        if self
            .backdrop
            .as_ref()
            .is_some_and(|backdrop| !backdrop.should_replace(incoming, complete, refreshing))
        {
            return;
        }
        let Some(texture) = spectrogram::texture(&analysis.spectrogram) else {
            return;
        };
        let backdrop = Backdrop {
            db: Arc::new(analysis.db.clone()),
            image: Arc::new(analysis.spectrogram.clone()),
            texture,
            settings,
            complete,
            deep: None,
        };
        self.release_backdrop(window);
        self.backdrop = Some(backdrop);
    }
}

impl Backdrop {
    fn should_replace(
        &self,
        incoming: crate::navigation::PictureView,
        complete: bool,
        refreshing: bool,
    ) -> bool {
        if refreshing {
            return false;
        }
        let held = crate::navigation::PictureView::grid(&self.db);
        let width = |range: (f64, f64)| range.1 - range.0;
        let wider = width(incoming.time) >= width(held.time)
            && width(incoming.frequency) >= width(held.frequency)
            && (width(incoming.time) > width(held.time)
                || width(incoming.frequency) > width(held.frequency));
        wider || (held == incoming && complete && !self.complete)
    }

    pub(super) fn level_at(
        &self,
        shown: crate::navigation::PictureView,
        x: f64,
        y: f64,
    ) -> Option<f32> {
        if !self.complete {
            return None;
        }
        crate::navigation::level_in_view(&self.db, shown, x, y)
    }

    fn prepare(&mut self, shown: (f64, f64), window: &mut Window) {
        let held = (self.db.t0, self.db.t1);
        if crate::navigation::image_mapping(held, shown).1 <= 1024.0 {
            if let Some(deep) = self.deep.take() {
                deep.release(window);
            }
            return;
        }
        let Some(deep) = plot_ui::DeepPreview::prepare(&self.image, shown, self.deep.as_deref())
        else {
            return;
        };
        if let Some(old) = self.deep.replace(Arc::new(deep)) {
            old.release(window);
        }
    }

    pub(super) fn paint(
        &self,
        frame: &axes::Frame,
        origin: gpui::Point<Pixels>,
        height: f32,
        shown: crate::navigation::PictureView,
        foreground: Option<crate::navigation::PictureView>,
        window: &mut Window,
    ) {
        let held = crate::navigation::PictureView::grid(&self.db);
        let plot = Bounds {
            origin: origin + point(px(frame.plot.x), px(height + frame.plot.y)),
            size: size(px(frame.plot.width), px(frame.plot.height)),
        };
        for (left, top, right, bottom) in crate::navigation::uncovered_picture(shown, foreground) {
            let clip = Bounds {
                origin: plot.origin
                    + point(plot.size.width * left as f32, plot.size.height * top as f32),
                size: size(
                    plot.size.width * (right - left) as f32,
                    plot.size.height * (bottom - top) as f32,
                ),
            };
            window.with_content_mask(Some(gpui::ContentMask { bounds: clip }), |window| {
                if let Some(deep) = &self.deep {
                    deep.paint(plot, shown, window);
                } else {
                    plot_ui::paint_held(self.texture.clone(), plot, held, shown, window);
                }
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view(time: (f64, f64)) -> crate::navigation::PictureView {
        crate::navigation::PictureView {
            time,
            frequency: (0., 1000.),
        }
    }

    fn picture(complete: bool) -> Backdrop {
        let mut image = argand_core::SpectrogramImage::new(2, 1);
        image.t1 = 100.0;
        Backdrop {
            db: Arc::new(argand_core::DbGrid {
                width: 2,
                height: 1,
                values: vec![-10.0, -20.0],
                t0: 0.0,
                t1: 100.0,
                f0: 0.0,
                f1: 1000.0,
            }),
            texture: spectrogram::texture(&image).unwrap(),
            image: Arc::new(image),
            settings: Settings::from_config(&Config::default()),
            complete,
            deep: None,
        }
    }

    #[test]
    fn an_interrupted_preview_is_only_a_picture_not_a_numeric_level_source() {
        assert_eq!(picture(false).level_at(view((0.0, 100.0)), 0.25, 0.5), None);
        assert_eq!(
            picture(true).level_at(view((0.0, 100.0)), 0.25, 0.5),
            Some(-10.0)
        );
        assert_eq!(
            picture(true).level_at(view((100.0, 200.0)), 0.25, 0.5),
            None
        );
    }

    #[test]
    fn narrower_panned_results_keep_the_widest_picture_and_completion_upgrades_a_preview() {
        let held = picture(false);
        assert!(!held.should_replace(view((60.0, 110.0)), true, false));
        assert!(!held.should_replace(view((100.0, 150.0)), true, false));
        assert!(held.should_replace(view((0.0, 110.0)), false, false));
        assert!(held.should_replace(view((0.0, 100.0)), true, false));
        assert!(!picture(true).should_replace(view((0.0, 100.0)), true, false));
    }
    #[test]
    fn pending_style_delivery_defers_wider_retention_and_preview_upgrade() {
        let held = picture(false);
        for incoming in [(0.0, 100.0), (0.0, 150.0)] {
            assert!(
                !held.should_replace(view(incoming), true, true),
                "a render must not evict the source of the parked foreground delivery"
            );
            assert!(
                held.should_replace(view(incoming), true, false),
                "once the delivery is applied, the next render may retain the completed picture"
            );
        }
    }
}
