//! The window itself: where GPUI meets everything decided without it.
//!
//! What the configuration says, where the window may open, when to write the
//! session, what a file is doing and what the status bar says about it are all
//! settled in [`crate::config`], [`crate::session`], [`crate::document`] and
//! [`crate::analysis`], none of which know about a toolkit and all of which are
//! tested without one. This module converts between those answers and GPUI.

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Instant;

use gpui::{
    Action, AppContext, Application, Bounds, Context, Corners, ExternalPaths, FocusHandle,
    FontWeight, InteractiveElement, IntoElement, KeyBinding, MouseButton, ParentElement,
    PathPromptOptions, Pixels, Render, RenderImage, StatefulInteractiveElement, Styled,
    Subscription, Task, TitlebarOptions, WeakEntity, Window, WindowBounds, WindowDecorations,
    WindowOptions, actions, canvas, div, point, prelude::FluentBuilder, px, size,
};
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::kbd::Kbd;
use gpui_component::menu::{DropdownMenu, PopupMenu, PopupMenuItem};
use gpui_component::tooltip::Tooltip;
use gpui_component::{ActiveTheme, Colorize, InteractiveElementExt, Sizable, ThemeMode, TitleBar};

use crate::settings::Settings;
use argand_dsp::AnalysisRequest;

#[path = "backdrop.rs"]
mod backdrop;

#[path = "navigation_ui.rs"]
mod navigation_ui;
#[path = "plot_ui.rs"]
mod plot_ui;
#[path = "settings_ui.rs"]
mod settings_ui;

use crate::analysis::{Analyst, Delivery};
use crate::axes;
use crate::chrome;
use crate::config::{Aggregation, Config, Theme};
use crate::document::{Document, Effect, MetadataHint, Origin, Status};
use crate::recent::RecentFiles;
use crate::session::{Geometry, Session, WindowState, Writer, place, restore_rectangle};
use crate::spectrogram;
use crate::{panels, waveform};

/// What the window is called, in its title bar and to the desktop environment.
const TITLE: &str = "argand";
/// Reverse-DNS identifier desktop environments group windows by.
const APP_ID: &str = "io.github.o_kos.argand";

actions!(
    shell,
    [
        FocusNext,
        FocusPrevious,
        ChooseFile,
        EditAnalysis,
        UseRecommendedRange
    ]
);

#[derive(Clone, PartialEq, serde::Deserialize, Action)]
#[action(namespace = shell, no_json)]
struct OpenRecent {
    index: usize,
}

/// Open the window and run until it closes.
pub fn run(config: Config, saved: Session, writer: Option<Writer>, opening: Option<Origin>) {
    // The toolkit's own icons -- the window controls among them -- are loaded
    // by path through an asset source. Without one they resolve to nothing and
    // the buttons render as blank space that still responds to a click.
    Application::new()
        .with_assets(gpui_component_assets::Assets)
        .run(move |cx| {
            gpui_component::init(cx);
            settings_ui::init(cx);
            navigation_ui::init(cx);
            cx.bind_keys([
                KeyBinding::new("tab", FocusNext, Some("Shell")),
                KeyBinding::new("shift-tab", FocusPrevious, Some("Shell")),
                KeyBinding::new(
                    if cfg!(target_os = "macos") {
                        "cmd-o"
                    } else {
                        "ctrl-o"
                    },
                    ChooseFile,
                    None,
                ),
            ]);
            cx.bind_keys((0..9).map(|index| {
                KeyBinding::new(
                    &format!("alt-{}", index + 1),
                    OpenRecent { index },
                    Some("StartPage"),
                )
            }));
            gpui_component::theme::Theme::change(
                theme_mode(config.theme, cx.window_appearance()),
                None,
                cx,
            );

            // Opening from a spawned task rather than straight from `run` follows
            // the toolkit's own examples and gives the platform a turn of its event
            // loop first.
            cx.spawn(async move |cx| {
                let displays: Vec<Geometry> = cx.update(|cx| {
                    cx.displays()
                        .iter()
                        .map(|display| from_bounds(display.bounds()))
                        .collect()
                })?;
                let options = window_options(&saved, &displays);
                tracing::debug!(
                    displays = displays.len(),
                    saved = ?saved.geometry,
                    opening_at = ?options.window_bounds,
                    "placing the window"
                );

                let opened = cx.open_window(options, |window, cx| {
                    tracing::debug!(
                        decorations = ?window.window_decorations(),
                        "the window is decorated by this side"
                    );
                    let shell = cx.new(|cx| Shell::new(config, writer, saved, window, cx));
                    // After the entity exists, so the task draining its
                    // updates has something to deliver them to.
                    if let Some(origin) = opening {
                        shell.update(cx, |shell, cx| shell.open(origin, window, cx));
                    } else {
                        shell.update(cx, |shell, cx| shell.check_recent(window, cx));
                    }
                    shell
                });

                // A window that will not open is the end of the run, and a task
                // whose error nobody reads would end it silently: there is nothing
                // else this program does.
                match opened {
                    Ok(window) => cx.update(|cx| Shell::bind_choose_file(window, cx))?,
                    Err(error) => {
                        tracing::error!(%error, "cannot open a window");
                        cx.update(|cx| cx.quit())?;
                    }
                }
                Ok::<_, anyhow::Error>(())
            })
            .detach();
        });
}

/// Where and how the window opens.
fn window_options(saved: &Session, displays: &[Geometry]) -> WindowOptions {
    let bounds = place(saved.geometry, displays).map(to_bounds);
    let window_bounds = bounds.map(|bounds| match saved.window_state {
        WindowState::Normal => WindowBounds::Windowed(bounds),
        WindowState::Maximized => WindowBounds::Maximized(bounds),
        WindowState::Fullscreen => WindowBounds::Fullscreen(bounds),
    });

    WindowOptions {
        // `None` leaves the placement to the platform, which is what a first
        // run and a rectangle with nowhere to go both want.
        window_bounds,
        // The application draws its own title bar, so it asks to draw the rest
        // of the frame too. Left unset, gpui requests *server-side*
        // decorations, and the desktop then draws a title bar of its own above
        // ours -- two of them, with the outer one owning the window controls.
        // `chrome` owns the frame, shadow and resize regions.
        window_decorations: Some(WindowDecorations::Client),
        titlebar: Some(TitlebarOptions {
            title: Some(TITLE.into()),
            // macOS keeps its traffic lights and hands us the rest of the bar.
            appears_transparent: true,
            traffic_light_position: Some(point(px(9.0), px(9.0))),
        }),
        app_id: Some(APP_ID.into()),
        window_min_size: Some(size(px(640.0), px(400.0))),
        ..Default::default()
    }
}

fn theme_mode(theme: Theme, appearance: gpui::WindowAppearance) -> ThemeMode {
    match theme {
        Theme::System => appearance.into(),
        Theme::Dark => ThemeMode::Dark,
        Theme::Light => ThemeMode::Light,
    }
}

fn sync_theme(theme: Theme, window: &mut Window, cx: &mut gpui::App) {
    let mode = theme_mode(theme, window.appearance());
    if mode == cx.theme().mode {
        return;
    }
    gpui_component::theme::Theme::change(mode, Some(window), cx);
    cx.refresh_windows();
}

fn from_bounds(bounds: Bounds<Pixels>) -> Geometry {
    Geometry::new(
        bounds.origin.x.into(),
        bounds.origin.y.into(),
        bounds.size.width.into(),
        bounds.size.height.into(),
    )
}

fn to_bounds(geometry: Geometry) -> Bounds<Pixels> {
    Bounds {
        origin: point(px(geometry.x), px(geometry.y)),
        size: size(px(geometry.width), px(geometry.height)),
    }
}

/// One open file, with the thread analysing it and the task delivering what
/// that thread says.
///
/// The three live and die together. The thread stops when both the request
/// sender in [`Analyst`] and the update receiver inside the task are gone, so
/// keeping them in one struct is what makes closing a document a single drop
/// rather than three that have to happen in the right order.
struct OpenFile {
    document: Document,
    analyst: Analyst,
    _updates: Task<()>,
    _minimap_updates: Option<Task<()>>,
    opened_at: Instant,
    first_picture: Arc<AtomicBool>,
    displayed_settings: Option<Settings>,
}

/// The size of the plot in device pixels, which is the size the transform is
/// asked to fill.
///
/// The plot, not the panel: the axis labels take a gutter out of the panel,
/// and a transform sized to the whole of it would be squeezed into what is
/// left. Device pixels rather than logical ones, so one column of the
/// transform is one column of the screen on a scaled display as well as on an
/// unscaled one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PlotSize {
    width: usize,
    height: usize,
}

/// The window's content: a title bar, the spectrogram, and the status bar.
struct Shell {
    /// What the person configured.
    ///
    /// The colour scheme, range mode and transform defaults go into every
    /// analysis request built below.
    config: Config,
    settings: Settings,
    settings_window: Option<gpui::WindowHandle<gpui_component::Root>>,
    analysis_hovered: bool,
    settings_backup: Option<Settings>,
    settings_view_backup: Option<crate::navigation::View>,
    settings_frequency_backup: Option<crate::frequency::View>,
    settings_error: Option<String>,

    /// Absent when the platform offers nowhere to keep state, or when the file
    /// there was written by a version this one must not overwrite. Either way
    /// the window simply does not remember itself.
    writer: Option<Writer>,
    /// What the next run should get back.
    ///
    /// Held whole rather than assembled at each offer, because two unrelated
    /// things write to it: the toolkit reports the window moving, and a person
    /// opens a file. Building a `Session` from whichever of the two happened
    /// last would have it guess at the other.
    ///
    /// Its `geometry` is the rectangle to come back to, which is not what a
    /// maximized or fullscreen window reports. `WindowBounds` documents its
    /// payload as the restore size, but the backends that omit a variant do
    /// not have one to give: X11 hands back the bounds its last configure
    /// event set, and macOS reads the live window frame. Both are the screen
    /// while the window covers it. Saving that would restore a maximized
    /// window correctly and then un-maximize it to the size of the display, so
    /// the last rectangle reported by an ordinary window is kept instead.
    /// [`restore_rectangle`] says which those are, and what a backend that
    /// misreports the state costs.
    session: Session,
    /// The file on screen, or `None` for a window nobody has opened anything
    /// in yet.
    file: Option<OpenFile>,
    /// The size the plot was last laid out at.
    ///
    /// Layout sizes the display request. The worker rebins its retained
    /// overview without restarting analysis when only these dimensions change.
    plot: Option<PlotSize>,
    view: Option<crate::navigation::View>,
    frequency: crate::frequency::View,
    frequency_scheme: Option<argand_core::axis::TickScheme>,
    frequency_pan: Option<(gpui::Point<Pixels>, crate::frequency::View)>,
    time_scheme: Option<argand_core::axis::TickScheme>,
    tick_pan: Option<crate::navigation::TickPan>,
    plot_geometry: Option<navigation_ui::PlotGeometry>,
    pointer: Option<gpui::Point<Pixels>>,
    badge_metrics: axes::BadgeMetrics,
    pan: Option<navigation_ui::Pan>,
    /// The picture currently on the GPU.
    ///
    /// Held so that the one it replaces can be released: gpui keeps an
    /// uploaded image in the window's texture atlas until it is told to let go.
    texture: Option<Arc<RenderImage>>,
    deep_preview: Option<Arc<plot_ui::DeepPreview>>,
    backdrop: Option<backdrop::Backdrop>,
    backdrop_refresh: Option<backdrop::Refresh>,
    upload_pending: bool,
    title_drag_pending: bool,
    waveform: Option<Arc<waveform::Waveform>>,
    panel_bounds: Option<Bounds<Pixels>>,
    splitter_dragging: bool,
    focus: FocusHandle,
    open_menu: Option<WeakEntity<PopupMenu>>,
    menu_dismiss: Option<gpui::Subscription>,
    startup_recent: Option<RecentFiles>,
    recent_updates: Option<Task<()>>,
    /// Kept because dropping it stops the notifications.
    _bounds: Subscription,
    _activation: Subscription,
    _appearance: Subscription,
}

impl Shell {
    fn new(
        config: Config,
        writer: Option<Writer>,
        saved: Session,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        // The toolkit says when the window has moved or resized, so nothing
        // here has to ask on every frame. It still says it once per step of a
        // drag, which is what [`Writer`] is for.
        crate::profiling::watch_ui(cx);
        let focus = cx.focus_handle();
        window.focus(&focus);
        let bounds = cx.observe_window_bounds(window, |shell, window, _| shell.remember(window));
        let activation = cx.observe_window_activation(window, |_, _, cx| cx.notify());
        sync_theme(config.theme, window, cx);
        let appearance = cx.observe_window_appearance(window, |shell, window, cx| {
            sync_theme(shell.config.theme, window, cx);
        });
        let settings = Settings::restored(saved.analysis_settings, &config);
        Self {
            settings,
            settings_window: None,
            analysis_hovered: false,
            settings_backup: None,
            settings_view_backup: None,
            settings_frequency_backup: None,
            settings_error: None,

            config,
            writer,
            session: saved,
            file: None,
            plot: None,
            view: None,
            frequency: crate::frequency::View::default(),
            frequency_scheme: None,
            frequency_pan: None,
            time_scheme: None,
            tick_pan: None,
            plot_geometry: None,
            pointer: None,
            badge_metrics: axes::BadgeMetrics::default(),
            pan: None,
            texture: None,
            deep_preview: None,
            backdrop: None,
            backdrop_refresh: None,
            upload_pending: false,
            title_drag_pending: false,
            waveform: None,
            panel_bounds: None,
            splitter_dragging: false,
            focus,
            open_menu: None,
            menu_dismiss: None,
            startup_recent: None,
            recent_updates: None,
            _bounds: bounds,
            _activation: activation,
            _appearance: appearance,
        }
    }

    /// Open a file, replacing whatever was open before it.
    ///
    /// Nothing is read here. [`crate::analysis::prepare`] starts a thread that
    /// does the opening as well as the transforms, because a capture asked to
    /// normalize is scanned for its peak before the first sample reaches a
    /// transform, and that is a pass over the file that must not happen on the
    /// thread drawing the window.
    fn open(&mut self, origin: Origin, window: &mut Window, cx: &mut Context<Self>) {
        let editor = self.settings_window;
        self.finish_settings(false, cx);
        if let Some(editor) = editor {
            let _ = editor.update(cx, |_, window, _| window.remove_window());
        }
        self.settings.dynamic_range = self.config.dynamic_range;
        tracing::info!(path = %origin.path.display(), "opening");
        self.settings_error = None;
        self.recent_updates = None;
        self.startup_recent = None;

        // Nothing of the previous file is left standing. Its picture would
        // otherwise be drawn under this one's axes until the first transform
        // lands, and its plot size would send the first request at a width
        // this file's labels may not leave.
        self.release(window);
        self.plot = None;
        self.view = None;
        self.time_scheme = None;
        self.tick_pan = None;
        self.plot_geometry = None;
        self.pointer = None;
        self.pan = None;

        let (analyst, updates, start) = crate::analysis::prepare(
            origin.path.clone(),
            origin.hints.clone(),
            self.config.analysis,
        );
        window.on_next_frame(move |window, _| {
            window.on_next_frame(move |_, _| start.start());
        });

        // Dropping this task drops the receiver, which is half of what tells
        // the thread that nobody is waiting for it any more.
        //
        // It is spawned against the window rather than the application because
        // letting go of a texture needs one: gpui takes the window being
        // updated out of its own list, so an image released without naming it
        // stays in that window's atlas.
        let pump = cx.spawn_in(window, async move |shell, cx| {
            while let Ok(update) = updates.recv().await {
                tracing::trace!(target: "argand::ui_latency",
                    age_us = update.prepared_at.elapsed().as_micros(), "analysis delivery received");
                if shell
                    .update_in(cx, |shell, window, cx| shell.receive(update, window, cx))
                    .is_err()
                {
                    // The window has gone; so has anything to tell.
                    break;
                }
            }
        });

        // Replacing the previous file drops both ends of its queue, which is
        // what stops its thread: a transform nobody will look at should not go
        // on holding a mapped file and a core.
        self.file = Some(OpenFile {
            document: Document::opening(origin),
            analyst,
            _updates: pump,
            _minimap_updates: None,
            opened_at: Instant::now(),
            first_picture: Arc::new(AtomicBool::new(false)),
            displayed_settings: None,
        });
        cx.notify();
    }

    fn check_recent(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let recent = RecentFiles::new(&self.session.recent);
        let updates = recent.check();
        self.startup_recent = Some(recent);
        self.recent_updates = Some(cx.spawn_in(window, async move |shell, cx| {
            while let Ok((index, exists)) = updates.recv().await {
                if shell
                    .update_in(cx, |shell, _, cx| {
                        shell.receive_recent(index, exists, cx);
                    })
                    .is_err()
                {
                    break;
                }
            }
        }));
    }

    fn receive_recent(&mut self, index: usize, exists: bool, cx: &mut Context<Self>) {
        if let Some(recent) = self.startup_recent.as_mut() {
            recent.apply(index, exists);
            cx.notify();
        }
    }

    fn open_recent(&mut self, action: &OpenRecent, window: &mut Window, cx: &mut Context<Self>) {
        if self.file.is_some() {
            return;
        }
        let entry = self
            .startup_recent
            .as_ref()
            .and_then(|recent| recent.shortcut(action.index));
        if let Some(entry) = entry {
            self.open(
                Origin {
                    path: entry.path,
                    hints: entry.hints.to_open_hints(),
                },
                window,
                cx,
            );
        }
    }

    /// Put a file at the head of the recent list, and write the session out.
    fn remember_file(&mut self, origin: &Origin) {
        self.session.remember(&origin.path, &origin.hints);
        self.save();
    }

    /// Offer the session as it now stands.
    fn save(&mut self) {
        if let Some(writer) = self.writer.as_mut() {
            writer.offer(self.session.clone(), Instant::now());
        }
    }

    /// Fold one update from the analysis thread into the document, and do
    /// whatever it asks for.
    fn receive(&mut self, delivery: Delivery, window: &mut Window, cx: &mut Context<Self>) {
        let Some(file) = self.file.as_mut() else {
            return;
        };
        if !file.analyst.accepts(&delivery) {
            return;
        }
        let Some(delivery) = self.refresh_backdrop_style(delivery, window, cx) else {
            return;
        };
        let Some(file) = self.file.as_mut() else {
            return;
        };
        let effect = file.document.apply(delivery.update);
        if effect == Effect::Analysis {
            file.displayed_settings = Some(self.settings);
        }

        match effect {
            Effect::Opened => {
                self.reset_view();
                self.start_minimap(window, cx);
                // Remembered now rather than when it was asked for. A file
                // that will not open must not overwrite the hints of the entry
                // that did: a raw capture first opened with `--raw iq_i16@2M`
                // and later picked from a dialog with nothing would lose the
                // one spelling that reads it.
                if let Some(origin) = self
                    .file
                    .as_ref()
                    .map(|file| file.document.origin().clone())
                {
                    self.remember_file(&origin);
                }
                // The span to analyse is the length the file has just
                // reported, so this is the first moment a request can be built
                // at all.
                self.ask_for_a_picture();
            }
            Effect::Analysis => self.upload_pending = true,
            Effect::Status => {}
        }
        cx.notify();
    }

    fn start_minimap(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(file) = &mut self.file else { return };
        let Some(meta) = file.document.meta().cloned() else {
            return;
        };
        let updates = crate::minimap::start(file.document.origin().clone(), meta);
        file._minimap_updates = Some(cx.spawn_in(window, async move |shell, cx| {
            while let Ok(update) = updates.recv().await {
                if shell
                    .update_in(cx, |shell, _, cx| {
                        shell.receive_minimap(update, cx);
                    })
                    .is_err()
                {
                    break;
                }
            }
        }));
    }

    fn receive_minimap(
        &mut self,
        update: anyhow::Result<Arc<crate::minimap::Snapshot>>,
        cx: &mut Context<Self>,
    ) {
        let Some(file) = &mut self.file else { return };
        match update {
            Ok(snapshot) => {
                file.document.minimap_ready(&snapshot);
                self.waveform = Some(Arc::new(waveform::Waveform::new(snapshot)));
            }
            Err(error) => {
                tracing::warn!(%error, "minimap unavailable");
                file.document.minimap_failed(format!("{error:#}"));
                self.waveform = None;
            }
        }
        cx.notify();
    }

    /// Note the size the plot was laid out at, and ask for a picture that
    /// size.
    ///
    /// Called only when the size actually changed, so a window merely being
    /// redrawn asks for nothing.
    fn resize(&mut self, plot: PlotSize, cx: &mut Context<Self>) {
        tracing::debug!(
            width = plot.width,
            height = plot.height,
            "the plot was laid out"
        );
        let width_changed = self.plot.is_none_or(|old| old.width != plot.width);
        self.plot = Some(plot);
        if width_changed {
            self.time_scheme = None;
            self.tick_pan = None;
            self.bound_view();
        }
        self.ask_for_a_picture();
        cx.notify();
    }

    /// What the axes span, which is what the file says it holds.
    ///
    /// `None` until the file has been opened. The extents come from the file
    /// rather than from a finished analysis so that the labels can be measured,
    /// and the plot sized, before the first transform runs.
    fn extents(&self) -> Option<axes::Extents> {
        let meta = self.file.as_ref()?.document.meta()?;
        Some(axes::Extents {
            orientation: self.session.orientation,
            time: crate::time_ruler::Ruler {
                mode: self.session.time_ruler,
                view: self.view?,
                total: meta.len_samples,
            },
            seconds: self.view?.seconds(meta.sample_rate),
            hertz: self.frequency.hertz(meta.frequency_span()),
        })
    }

    /// Ask the analysis thread for the picture the window can currently show.
    ///
    /// Silent when the file has not opened yet or the panel has not been laid
    /// out: both arrive on their own, and each one calls back here.
    fn ask_for_a_picture(&mut self) {
        let Some(file) = self.file.as_ref() else {
            return;
        };
        let Some(request) = self.request(&file.document) else {
            return;
        };
        let frequency = self.extents().map(|extents| extents.hertz);
        let Some(file) = self.file.as_mut() else {
            return;
        };
        file.document.requested_range(request.range);
        if !file.analyst.request_view(request, frequency) {
            tracing::warn!("the analysis thread has stopped; nothing more will be drawn");
        }
    }

    /// What to ask for: this file, at this size, with the settings from
    /// `argand.toml`.
    fn request(&self, document: &Document) -> Option<AnalysisRequest> {
        let meta = document.meta()?;
        let plot = self.plot?;
        let mut request = self
            .settings
            .analysis_request(meta, plot.width, plot.height);
        request.range = self.view?.range();
        request.waveform_columns = None;
        Some(request)
    }

    /// Put the newest picture on the GPU and release the one it replaces.
    fn upload(&mut self, window: &mut Window) {
        self.release_deep_preview(window);
        let started = Instant::now();
        let fresh = self
            .file
            .as_ref()
            .and_then(|file| file.document.analysis())
            .and_then(|analysis| {
                spectrogram::texture(&analysis.spectrogram, self.session.orientation)
            });
        let stale = std::mem::replace(&mut self.texture, fresh);
        release(stale, window);
        tracing::trace!(target: "argand::ui_latency", elapsed_us = started.elapsed().as_micros(),
            "texture prepared");
    }

    /// Let go of whatever picture is on the GPU, leaving nothing to draw.
    fn release(&mut self, window: &mut Window) {
        self.waveform = None;
        self.release_backdrop(window);
        self.release_deep_preview(window);
        release(self.texture.take(), window);
    }

    /// Record where the window is and what state it is in.
    fn remember(&mut self, window: &Window) {
        if self.writer.is_none() {
            return;
        }
        // The rectangle comes from `WindowBounds`, which is the one that
        // carries the size to restore *to* rather than the screen a maximized
        // window currently covers.
        //
        // The state cannot come from that variant alone. Each backend answers
        // with only the variants it tracks: X11 returns `Maximized` or
        // `Windowed` and never `Fullscreen`, macOS returns `Fullscreen` or
        // `Windowed` and never `Maximized`. Reading the variant alone would
        // record a fullscreen X11 window, or a maximized macOS one, as
        // ordinary. Every backend reports what it omits through one of the two
        // predicates, so both are asked.
        let bounds = window.window_bounds();
        let window_state =
            if window.is_fullscreen() || matches!(bounds, WindowBounds::Fullscreen(_)) {
                WindowState::Fullscreen
            } else if window.is_maximized() || matches!(bounds, WindowBounds::Maximized(_)) {
                WindowState::Maximized
            } else {
                WindowState::Normal
            };
        self.session.window_state = window_state;
        // Only some reports say anything about the rectangle to come back to;
        // the rest leave the last one that did.
        if let Some(rectangle) = restore_rectangle(from_bounds(bounds.get_bounds()), window_state) {
            self.session.geometry = Some(rectangle);
        }
        self.save();
    }
}

impl Drop for Shell {
    fn drop(&mut self) {
        if let Some(writer) = self.writer.as_mut() {
            writer.flush(Instant::now());
        }
    }
}

impl Shell {
    fn bind_choose_file(window: gpui::WindowHandle<Self>, cx: &mut gpui::App) {
        // Popup focus sits outside the shell subtree; defer until dispatch releases the window.
        cx.on_action(move |_: &ChooseFile, cx| {
            cx.defer(move |cx| {
                let _ = window.update(cx, Self::choose_file);
            });
        });
        cx.on_action(move |_: &UseRecommendedRange, cx| {
            cx.defer(move |cx| {
                let _ = window.update(cx, |shell, _, cx| shell.use_recommended_range(cx));
            });
        });
        cx.on_action(move |_: &EditAnalysis, cx| {
            cx.defer(move |cx| {
                let _ = window.update(cx, |shell, window, cx| {
                    shell.edit_analysis(&EditAnalysis, window, cx)
                });
            });
        });
    }

    fn choose_file(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(menu) = self.open_menu.take() {
            let _ = menu.update(cx, |_, cx| cx.emit(gpui::DismissEvent));
        }
        window.focus(&self.focus);
        cx.notify();
        Self::choose(&cx.entity().downgrade(), window, cx);
    }

    /// Ask the desktop for a file, and open whatever comes back.
    ///
    /// The dialog is the platform's own -- the file chooser portal on Linux,
    /// the native panel on Windows and macOS -- so it looks and behaves like
    /// every other one on the desktop, and nothing here has to be drawn.
    ///
    /// A file chosen this way is opened on what it says about itself. A
    /// headerless capture has nothing to say, so it fails and says how; the
    /// recent list is what carries hints back for the next time.
    fn choose(view: &WeakEntity<Self>, window: &mut Window, cx: &mut gpui::App) {
        let chosen = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: None,
        });
        let view = view.clone();
        window
            .spawn(cx, async move |cx| {
                // A cancelled dialog, a platform without one, and a dialog that
                // failed all mean the same thing here: no file was chosen.
                let Ok(Ok(Some(paths))) = chosen.await else {
                    return;
                };
                let Some(path) = paths.into_iter().next() else {
                    return;
                };
                let _ = view.update_in(cx, |shell, window, cx| {
                    shell.open(Origin::new(path), window, cx);
                });
            })
            .detach();
    }

    /// The menu in the title bar.
    ///
    /// The window draws its own title bar on every platform, so the menu is
    /// drawn there too rather than handed to a platform menu bar that only one
    /// of the three has.
    fn menu(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .child(self.file_menu(cx))
            .when(self.view.is_some(), |bar| {
                bar.child(self.view_menu(cx))
                    .child(self.orientation_button(cx))
            })
    }

    fn file_menu(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity().downgrade();
        let focus = self.focus.clone();
        // Rebuilt from what the session holds when the menu is opened, so an
        // entry added since the last time it was shown is in it.
        let recent = self.recent_entries();
        Button::new("file-menu")
            .ghost()
            .small()
            .label("File")
            .dropdown_menu(move |mut menu, _, cx| {
                let menu_view = cx.entity().downgrade();
                let _ = view.update(cx, |shell, _| shell.open_menu = Some(menu_view));
                menu = menu
                    .action_context(focus.clone())
                    .item(PopupMenuItem::new("Open file...").action(Box::new(ChooseFile)));
                if recent.is_empty() {
                    return menu;
                }
                menu = menu.separator().label("Recent");
                for (label, origin) in &recent {
                    let view = view.clone();
                    let origin = origin.clone();
                    menu = menu.item(PopupMenuItem::new(label.clone()).on_click(
                        move |_, window, cx| {
                            let origin = origin.clone();
                            let _ = view.update(cx, |shell, cx| shell.open(origin, window, cx));
                        },
                    ));
                }
                menu
            })
    }

    /// The recent list as a menu shows it: a name to read, and everything it
    /// takes to open the file again.
    ///
    /// The hints are what make the entry worth storing. A headerless capture
    /// opened once as `iq_i16@2M` cannot be reopened from its path alone, so
    /// choosing it here does not ask for those flags a second time.
    fn recent_entries(&self) -> Vec<(String, Origin)> {
        let labels = crate::session::recent_labels(&self.session.recent);
        self.session
            .recent
            .iter()
            .zip(labels)
            .map(|(entry, label)| {
                let origin = Origin {
                    path: entry.path.clone(),
                    hints: entry.hints.to_open_hints(),
                };
                (label, origin)
            })
            .collect()
    }

    /// The window's title: the application, and the file if there is one.
    fn title(&self) -> String {
        match self.file.as_ref() {
            Some(file) => format!("{} - {TITLE}", file.document.origin().name()),
            None => TITLE.to_owned(),
        }
    }

    fn title_bar(
        &self,
        corners: Corners<Pixels>,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let bar = if cfg!(target_os = "linux") {
            div()
                .id("title-bar")
                .flex()
                .items_center()
                .h(gpui_component::TITLE_BAR_HEIGHT)
                .pl_3()
                .rounded_tl(corners.top_left)
                .rounded_tr(corners.top_right)
                .border_b_1()
                .border_color(cx.theme().title_bar_border)
                .bg(cx.theme().title_bar)
                .on_double_click(|_, window, _| window.zoom_window())
                .on_mouse_down_out(cx.listener(|shell, _, _, _| shell.title_drag_pending = false))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|shell, _, _, _| shell.title_drag_pending = true),
                )
                .on_mouse_up(
                    MouseButton::Left,
                    cx.listener(|shell, _, _, _| shell.title_drag_pending = false),
                )
                .on_mouse_move(cx.listener(|shell, _, window, _| {
                    if std::mem::take(&mut shell.title_drag_pending) {
                        window.start_window_move();
                    }
                }))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .flex_1()
                        .h_full()
                        .on_mouse_down(MouseButton::Right, |event, window, _| {
                            window.show_window_menu(event.position)
                        })
                        .child(self.menu(cx)),
                )
                .child(chrome::controls(corners.top_right, window, cx))
                .into_any_element()
        } else {
            TitleBar::new().child(self.menu(cx)).into_any_element()
        };
        div().relative().flex_shrink_0().child(bar).child(
            // This non-interactive overlay spans the whole bar, including
            // the controls, and leaves its drag/double-click hitbox intact.
            div()
                .absolute()
                .left(px(128.0))
                .right(px(128.0))
                .top_0()
                .h_full()
                .flex()
                .items_center()
                .justify_center()
                .overflow_hidden()
                .text_sm()
                .child(
                    div()
                        .w_full()
                        .min_w_0()
                        .text_center()
                        .line_clamp(1)
                        .text_ellipsis()
                        .child(self.title()),
                ),
        )
    }

    fn finish_splitter(&mut self, _: &gpui::MouseUpEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.splitter_dragging {
            self.splitter_dragging = false;
            cx.notify();
        }
    }

    fn content(&self, window: &Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .when(!matches!(self.showing(), Showing::Plot(_)), |content| {
                content.child(
                    div()
                        .h_12()
                        .flex_shrink_0()
                        .border_b_1()
                        .border_color(cx.theme().border),
                )
            })
            .child(self.middle(window, cx))
    }

    fn splitter(&self, window: &Window, cx: &mut Context<Self>) -> impl IntoElement {
        let total = self.panel_bounds.map_or(0.0, |bounds| {
            f32::from(
                self.session
                    .orientation
                    .axes(bounds.size.width, bounds.size.height)
                    .1,
            )
        });
        let height = panels::waveform_height(
            total,
            f32::from(cx.theme().font_size),
            self.session.waveform_fraction,
            window.scale_factor(),
        );
        let divider = div().id("waveform-splitter").absolute();
        let divider = if self.session.orientation.vertical() {
            divider
                .top_0()
                .bottom_0()
                .left(px((height - 3.).max(0.)))
                .w(px(5.))
                .cursor(gpui::CursorStyle::ResizeLeftRight)
        } else {
            divider
                .left_0()
                .right_0()
                .top(px((height - 3.).max(0.)))
                .h(px(5.))
                .cursor(gpui::CursorStyle::ResizeUpDown)
        };
        divider.on_mouse_down(
            MouseButton::Left,
            cx.listener(|shell, _, _, cx| {
                shell.splitter_dragging = true;
                cx.stop_propagation();
            }),
        )
    }

    fn drag_splitter(
        &mut self,
        event: &gpui::MouseMoveEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.splitter_dragging {
            return;
        }
        if !event.dragging() {
            self.splitter_dragging = false;
            cx.notify();
            return;
        }
        let Some(bounds) = self.panel_bounds else {
            return;
        };
        let orientation = self.session.orientation;
        let total = f32::from(orientation.axes(bounds.size.width, bounds.size.height).1);
        if total <= 0.0 {
            return;
        }
        let delta = event.position - bounds.origin;
        let requested = f32::from(orientation.axes(delta.x, delta.y).1) / total;
        let height = panels::waveform_height(
            total,
            f32::from(cx.theme().font_size),
            Some(requested.clamp(0.0, 1.0)),
            window.scale_factor(),
        );
        self.session.waveform_fraction = Some(height / total);
        self.save();
        cx.notify();
    }

    /// What fills the middle of the window.
    ///
    /// Decided in one place and drawn in another, so that neither has to hold
    /// the other's conditions: four states, and each of them one element.
    fn middle(&self, window: &Window, cx: &mut Context<Self>) -> gpui::AnyElement {
        let notice = |text: String, color| {
            div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .px_4()
                .text_color(color)
                .child(text)
                .into_any_element()
        };

        match self.showing() {
            Showing::Plot(extents) => div()
                .id("time-plot")
                .cursor(
                    self.plot_geometry
                        .map_or(gpui::CursorStyle::Arrow, |geometry| {
                            geometry.cursor(
                                self.pointer,
                                self.pan.is_some() || self.frequency_pan.is_some(),
                                self.view.zip(
                                    self.file
                                        .as_ref()
                                        .and_then(|file| file.document.meta())
                                        .map(|meta| meta.len_samples),
                                ),
                                self.frequency.span < 1.,
                            )
                        }),
                )
                .on_scroll_wheel(cx.listener(Self::wheel))
                .on_mouse_down(MouseButton::Left, cx.listener(Self::begin_pan))
                .on_hover(cx.listener(|shell, hovered, _, cx| {
                    if !hovered {
                        shell.pointer = None;
                        cx.notify();
                    }
                }))
                .flex_1()
                .min_h_0()
                .relative()
                .child(self.spectrogram(extents, cx))
                .child(self.splitter(window, cx))
                .children(self.time_context_menu(cx))
                .children(
                    self.panel_bounds
                        .and_then(|bounds| self.unit_hint(1, bounds.origin, cx)),
                )
                .into_any_element(),
            // Physical labels need the metadata; their boundaries can appear immediately.
            Showing::Opening => div()
                .flex_1()
                .min_h_0()
                .p_1()
                .pr(px(52.0))
                .pb(px(24.0))
                .child(
                    div()
                        .size_full()
                        .border_r_1()
                        .border_b_1()
                        .border_color(cx.theme().muted_foreground),
                )
                .into_any_element(),
            Showing::Nothing => self.start_page(window, cx).into_any_element(),
            Showing::Failed(reason) => notice(reason, cx.theme().danger),
        }
    }

    fn recent_button(
        entry: crate::session::Recent,
        label: String,
        index: usize,
        width: Pixels,
        cx: &mut Context<Self>,
    ) -> Button {
        let tooltip = entry.path.display().to_string();
        let mut button = Button::new(("start-recent", index))
            .ghost()
            .small()
            .h_6()
            .max_w(width)
            .px_2()
            .justify_start()
            .cursor_pointer()
            .child(
                div()
                    .max_w(width - px(24.))
                    .flex()
                    .gap_2()
                    .child(
                        div()
                            .w_6()
                            .flex_shrink_0()
                            .text_color(cx.theme().muted_foreground)
                            .child(if index < 9 {
                                crate::numbers::number(index + 1)
                            } else {
                                String::new()
                            }),
                    )
                    .child(div().min_w_0().line_clamp(1).text_ellipsis().child(label)),
            )
            .on_click(cx.listener(move |shell, _, window, cx| {
                shell.open(
                    Origin {
                        path: entry.path.clone(),
                        hints: entry.hints.to_open_hints(),
                    },
                    window,
                    cx,
                );
            }));
        button.interactivity().tooltip(move |window, cx| {
            let action = (index < 9).then(|| Box::new(OpenRecent { index }) as Box<dyn Action>);
            shortcut_tooltip(tooltip.clone(), action, "StartPage", width).build(window, cx)
        });
        button
    }

    fn start_page(&self, window: &Window, cx: &mut Context<Self>) -> impl IntoElement {
        let recent = self
            .startup_recent
            .as_ref()
            .map(RecentFiles::visible)
            .unwrap_or_default();
        let labels = crate::session::recent_labels(&recent);
        let width = (window.viewport_size().width - px(96.)).min(px(560.));
        let empty = recent.is_empty();
        let list_height = cx.theme().font_size * (recent.len() as f32 * 1.75 - 0.25).max(0.);
        let mut chooser = Button::new("start-open")
            .ghost()
            .small()
            .h_6()
            .px_2()
            .max_w(width)
            .justify_start()
            .cursor_pointer()
            .label("Open a signal file…")
            .on_click(|_, window, cx| window.dispatch_action(Box::new(ChooseFile), cx));
        chooser.interactivity().tooltip(move |window, cx| {
            shortcut_tooltip(
                "Open a signal file".to_owned(),
                Some(Box::new(ChooseFile)),
                "Shell",
                width,
            )
            .build(window, cx)
        });
        div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .items_center()
            .p_4()
            .gap_2()
            .when(empty, |page| page.justify_center())
            .when(!empty, |page| {
                page.child(
                    div()
                        .w(width)
                        .flex_shrink_0()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child("Recent files"),
                )
                .child(
                    div()
                        .id("start-links")
                        .w(width)
                        .h(list_height)
                        .min_h_0()
                        .overflow_y_scroll()
                        .flex()
                        .flex_col()
                        .items_start()
                        .gap_1()
                        .children(recent.into_iter().zip(labels).enumerate().map(
                            |(index, (entry, label))| {
                                Self::recent_button(entry, label, index, width, cx)
                            },
                        )),
                )
                .child(
                    div()
                        .w(width)
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child("or"),
                )
            })
            .child(
                div()
                    .w(width)
                    .flex()
                    .flex_shrink_0()
                    .when(empty, |row| row.justify_center())
                    // Match the recent row's number column and gap; button padding is shared.
                    .when(!empty, |row| row.pl_8())
                    .child(chooser),
            )
    }

    /// Which of the four the window is in.
    fn showing(&self) -> Showing {
        let Some(file) = self.file.as_ref() else {
            return Showing::Nothing;
        };
        if let Status::Failed(reason) = file.document.status()
            && file.document.analysis().is_none()
        {
            return Showing::Failed(reason.clone());
        }
        match self.extents() {
            Some(extents) => Showing::Plot(extents),
            None => Showing::Opening,
        }
    }
}

/// What the middle of the window has to show.
enum Showing {
    /// No file has been opened.
    Nothing,
    /// A file is being opened and has not described itself yet.
    Opening,
    /// A file that could not be read, and why.
    Failed(String),
    /// The picture and the axes around it.
    Plot(axes::Extents),
}

impl Render for Shell {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.upload_pending {
            self.upload_pending = false;
            self.upload(window);
        }
        self.prepare_backdrop(window);
        self.prepare_deep_preview(window);
        window.set_rem_size(cx.theme().font_size);
        let frame = chrome::Frame::for_window(window);
        let corners = frame.corners;
        let content =
            div()
                .size_full()
                .flex()
                .flex_col()
                .font_family(cx.theme().font_family.clone())
                .text_color(cx.theme().foreground)
                .id("shell")
                .track_focus(&self.focus)
                .key_context(if self.file.is_none() {
                    "Shell StartPage"
                } else if self.session.orientation.vertical() {
                    "Shell Plot Vertical"
                } else {
                    "Shell Plot Horizontal"
                })
                .on_mouse_move(cx.listener(Self::drag_splitter))
                .on_mouse_move(cx.listener(Self::pointer_moved))
                .on_modifiers_changed(cx.listener(|_, _, _, cx| cx.notify()))
                .on_mouse_up(MouseButton::Left, cx.listener(Self::finish_pan))
                .on_mouse_up_out(MouseButton::Left, cx.listener(Self::finish_pan))
                .on_mouse_up(MouseButton::Left, cx.listener(Self::finish_splitter))
                .on_mouse_up_out(MouseButton::Left, cx.listener(Self::finish_splitter))
                .on_action(cx.listener(Self::open_recent))
                .on_action(cx.listener(Self::edit_analysis))
                .on_action(cx.listener(|shell, _: &UseRecommendedRange, _, cx| {
                    shell.use_recommended_range(cx)
                }))
                .on_action(|_: &FocusNext, window, _| window.focus_next())
                .on_action(|_: &FocusPrevious, window, _| window.focus_prev())
                // A capture dropped anywhere on the window opens, which is where a
                // person aims when the window is showing the wrong file.
                .on_drop(cx.listener(|shell, dropped: &ExternalPaths, window, cx| {
                    if let Some(path) = dropped.paths().first() {
                        shell.open(Origin::new(path.clone()), window, cx);
                    }
                }))
                .child(self.title_bar(corners, window, cx))
                .child(self.content(window, cx))
                .child(self.status_bar(corners, cx));
        let content = self.navigation_actions(content, cx);
        frame.render(content, cx)
    }
}

fn shortcut_tooltip(
    text: String,
    action: Option<Box<dyn Action>>,
    context: &'static str,
    width: Pixels,
) -> Tooltip {
    Tooltip::element(move |window, cx| {
        let shortcut = action
            .as_deref()
            .and_then(|action| Kbd::binding_for_action(action, Some(context), window));
        let color = if cx.theme().is_dark() {
            cx.theme().blue_light
        } else {
            cx.theme().blue.darken(0.2)
        };
        div()
            .max_w(width.min(window.viewport_size().width - px(48.)))
            .flex()
            .items_start()
            .gap_3()
            .child(div().min_w_0().child(text.clone()))
            .when_some(shortcut, |hint, shortcut| {
                hint.child(
                    div()
                        .flex_shrink_0()
                        .text_color(color)
                        .child(shortcut.appearance(false)),
                )
            })
    })
}

fn metadata_hint_width(hint: &MetadataHint, window: &Window, cx: &gpui::App) -> Pixels {
    let limit = px(320.).min(window.viewport_size().width - px(48.));
    [
        (hint.title, 0.875, FontWeight::SEMIBOLD),
        (hint.value.as_str(), 0.875, FontWeight::NORMAL),
        (hint.explanation.as_str(), 0.75, FontWeight::NORMAL),
    ]
    .into_iter()
    .flat_map(|(text, scale, weight)| {
        let style = gpui::TextStyle {
            font_family: cx.theme().font_family.clone(),
            font_weight: weight,
            ..Default::default()
        };
        text.lines().map(move |line| {
            window
                .text_system()
                .shape_line(
                    line.to_owned().into(),
                    window.rem_size() * scale,
                    &[style.to_run(line.len())],
                    None,
                )
                .width
                .ceil()
        })
    })
    .fold(px(0.), Pixels::max)
    .min(limit)
}

fn metadata_tooltip(hint: MetadataHint) -> Tooltip {
    Tooltip::element(move |window, cx| {
        if !hint.rows.is_empty() {
            return div()
                .w(px(330.).min(window.viewport_size().width - px(48.)))
                .py_1()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .font_weight(FontWeight::SEMIBOLD)
                        .mb_1()
                        .child(hint.title),
                )
                .children(hint.rows.iter().map(|(label, value)| {
                    settings_ui::detail_row(label.clone(), value.clone(), cx)
                }))
                .child(
                    div()
                        .mt_1()
                        .pt_2()
                        .border_t_1()
                        .border_color(cx.theme().border)
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(hint.explanation.clone()),
                );
        }
        // A definite content width lets wrapped lines contribute their full layout height.
        let width = metadata_hint_width(&hint, window, cx);
        div()
            .w(width)
            .py_1()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .w_full()
                    .flex_shrink_0()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(hint.title),
            )
            .child(
                div()
                    .w_full()
                    .flex_shrink_0()
                    .text_color(cx.theme().muted_foreground)
                    .child(hint.value.clone()),
            )
            .when(!hint.explanation.is_empty(), |tooltip| {
                tooltip.child(
                    div()
                        .w_full()
                        .flex_shrink_0()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(hint.explanation.clone()),
                )
            })
    })
}

/// Hand one uploaded picture back to the toolkit.
///
/// An uploaded image stays in the window's texture atlas until gpui is told to
/// let go of it. A spectrogram is the size of the plot and a resize produces
/// one per step, so saying nothing fills the atlas with pictures nobody can
/// see any more.
fn release(texture: Option<Arc<RenderImage>>, window: &mut Window) {
    if let Some(texture) = texture {
        // Blade's atlas destroys an unreferenced texture immediately, while
        // the last submitted frame can still be sampling it. Draw its
        // replacement first: that draw waits for the preceding GPU frame.
        window.on_next_frame(move |window, _| {
            window.refresh();
            window.on_next_frame(move |window, cx| {
                cx.drop_image(texture, Some(window));
            });
        });
    }
}

/// A laid-out rectangle in the pixels the display actually has.
///
/// The transform is sized in these rather than in logical pixels, so a column
/// of the spectrogram is a column of the screen whatever the display is scaled
/// to. [`axes::Frame::measure`] has already put both edges of the plot on
/// device pixels, so this rounding lands on a whole number rather than
/// choosing one.
fn device_size(plot: axes::Rect, scale: f32) -> PlotSize {
    let edge = |logical: f32| (logical * scale).round().max(0.0) as usize;
    PlotSize {
        width: edge(plot.width),
        height: edge(plot.height),
    }
}

#[cfg(test)]
mod theme_tests {
    use super::*;
    use gpui::WindowAppearance;

    #[test]
    fn system_follows_appearance_and_explicit_themes_remain_fixed() {
        for (appearance, expected) in [
            (WindowAppearance::Light, ThemeMode::Light),
            (WindowAppearance::VibrantLight, ThemeMode::Light),
            (WindowAppearance::Dark, ThemeMode::Dark),
            (WindowAppearance::VibrantDark, ThemeMode::Dark),
        ] {
            assert_eq!(theme_mode(Theme::System, appearance), expected);
            assert_eq!(theme_mode(Theme::Dark, appearance), ThemeMode::Dark);
            assert_eq!(theme_mode(Theme::Light, appearance), ThemeMode::Light);
        }
    }
}
