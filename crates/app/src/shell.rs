//! The window itself: where GPUI meets everything decided without it.
//!
//! What the configuration says, where the window may open, when to write the
//! session, what a file is doing and what the status bar says about it are all
//! settled in [`crate::config`], [`crate::session`], [`crate::document`] and
//! [`crate::analysis`], none of which know about a toolkit and all of which are
//! tested without one. This module converts between those answers and GPUI.

use std::sync::Arc;
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

use argand_dsp::AnalysisRequest;

use crate::analysis::{Analyst, Update};
use crate::axes;
use crate::chrome;
use crate::config::{Config, Theme};
use crate::document::{Document, Effect, MetadataHint, Origin, Status};
use crate::recent::RecentFiles;
use crate::session::{Geometry, Session, WindowState, Writer, place, restore_rectangle};
use crate::spectrogram;

/// What the window is called, in its title bar and to the desktop environment.
const TITLE: &str = "argand";
/// Reverse-DNS identifier desktop environments group windows by.
const APP_ID: &str = "io.github.o_kos.argand";

actions!(shell, [FocusNext, FocusPrevious, ChooseFile]);

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
            gpui_component::theme::Theme::change(theme_mode(config.theme), None, cx);

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

const fn theme_mode(theme: Theme) -> ThemeMode {
    match theme {
        Theme::Dark => ThemeMode::Dark,
        Theme::Light => ThemeMode::Light,
    }
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
    /// It arrives from the layout rather than being computed here, and it is
    /// what a request is sized to, so a window resized to twice the width is
    /// answered with twice the columns rather than with the same picture
    /// stretched.
    plot: Option<PlotSize>,
    /// The picture currently on the GPU.
    ///
    /// Held so that the one it replaces can be released: gpui keeps an
    /// uploaded image in the window's texture atlas until it is told to let go.
    texture: Option<Arc<RenderImage>>,
    title_drag_pending: bool,
    focus: FocusHandle,
    file_menu: Option<WeakEntity<PopupMenu>>,
    startup_recent: Option<RecentFiles>,
    recent_updates: Option<Task<()>>,
    /// Kept because dropping it stops the notifications.
    _bounds: Subscription,
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
        let focus = cx.focus_handle();
        window.focus(&focus);
        let bounds = cx.observe_window_bounds(window, |shell, window, _| shell.remember(window));
        Self {
            config,
            writer,
            session: saved,
            file: None,
            plot: None,
            texture: None,
            title_drag_pending: false,
            focus,
            file_menu: None,
            startup_recent: None,
            recent_updates: None,
            _bounds: bounds,
        }
    }

    /// Open a file, replacing whatever was open before it.
    ///
    /// Nothing is read here. [`crate::analysis::open`] starts a thread that
    /// does the opening as well as the transforms, because a capture asked to
    /// normalize is scanned for its peak before the first sample reaches a
    /// transform, and that is a pass over the file that must not happen on the
    /// thread drawing the window.
    fn open(&mut self, origin: Origin, window: &mut Window, cx: &mut Context<Self>) {
        tracing::info!(path = %origin.path.display(), "opening");
        self.recent_updates = None;
        self.startup_recent = None;

        // Nothing of the previous file is left standing. Its picture would
        // otherwise be drawn under this one's axes until the first transform
        // lands, and its plot size would send the first request at a width
        // this file's labels may not leave.
        self.release(window);
        self.plot = None;

        let (analyst, updates) = crate::analysis::open(origin.path.clone(), origin.hints.clone());

        // Dropping this task drops the receiver, which is half of what tells
        // the thread that nobody is waiting for it any more.
        //
        // It is spawned against the window rather than the application because
        // letting go of a texture needs one: gpui takes the window being
        // updated out of its own list, so an image released without naming it
        // stays in that window's atlas.
        let pump = cx.spawn_in(window, async move |shell, cx| {
            while let Ok(update) = updates.recv().await {
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
    fn receive(&mut self, update: Update, window: &mut Window, cx: &mut Context<Self>) {
        let Some(effect) = self.file.as_mut().map(|file| file.document.apply(update)) else {
            return;
        };
        match effect {
            Effect::Opened => {
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
            Effect::Analysis => self.upload(window),
            Effect::Status => {}
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
        self.plot = Some(plot);
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
            seconds: (0.0, meta.duration_seconds()),
            hertz: meta.frequency_span(),
        })
    }

    /// Ask the analysis thread for the picture the window can currently show.
    ///
    /// Silent when the file has not opened yet or the panel has not been laid
    /// out: both arrive on their own, and each one calls back here.
    fn ask_for_a_picture(&self) {
        let Some(file) = self.file.as_ref() else {
            return;
        };
        let Some(request) = self.request(&file.document) else {
            return;
        };
        if !file.analyst.request(request) {
            tracing::warn!("the analysis thread has stopped; nothing more will be drawn");
        }
    }

    /// What to ask for: this file, at this size, with the settings from
    /// `argand.toml`.
    fn request(&self, document: &Document) -> Option<AnalysisRequest> {
        let meta = document.meta()?;
        let plot = self.plot?;
        Some(self.config.analysis_request(meta, plot.width, plot.height))
    }

    /// Put the newest picture on the GPU and release the one it replaces.
    fn upload(&mut self, window: &mut Window) {
        let fresh = self
            .file
            .as_ref()
            .and_then(|file| file.document.analysis())
            .and_then(|analysis| spectrogram::texture(&analysis.spectrogram));
        let stale = std::mem::replace(&mut self.texture, fresh);
        release(stale, window);
    }

    /// Let go of whatever picture is on the GPU, leaving nothing to draw.
    fn release(&mut self, window: &mut Window) {
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
    }

    fn choose_file(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(menu) = self.file_menu.take() {
            let _ = menu.update(cx, |_, cx| cx.emit(gpui::DismissEvent));
            window.focus(&self.focus);
        }
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
                let _ = view.update(cx, |shell, _| shell.file_menu = Some(menu_view));
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
    fn spectrogram(&self, extents: axes::Extents, cx: &mut Context<Self>) -> impl IntoElement {
        let texture = self.texture.clone();
        let known = self.plot;
        let view = cx.entity().downgrade();
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
                let frame = axes::Frame::measure(bounds.size, scale, extents, &labels)?;
                let measured = device_size(frame.plot, scale);
                if known != Some(measured) {
                    cx.defer(move |cx| {
                        let _ = view.update(cx, |shell, cx| shell.resize(measured, cx));
                    });
                }
                Some((frame, labels))
            },
            move |bounds, prepainted, window, cx| {
                let Some((frame, labels)) = prepainted else {
                    return;
                };
                // The picture first, then the marks over it: a grid line is
                // there to be read against the spectrogram, not under it.
                if let Some(texture) = texture {
                    let plot = Bounds {
                        origin: bounds.origin + point(px(frame.plot.x), px(frame.plot.y)),
                        size: size(px(frame.plot.width), px(frame.plot.height)),
                    };
                    if let Err(error) =
                        window.paint_image(plot, Corners::default(), texture, 0, false)
                    {
                        tracing::warn!(%error, "cannot draw the spectrogram");
                    }
                }
                axes::paint(&frame, bounds.origin, &labels, colors, window, cx);
            },
        )
        .size_full()
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
                .flex_1()
                .child(self.spectrogram(extents, cx))
                .into_any_element(),
            // A file that has not said what it is yet has no axes to draw and
            // no picture to draw them around; the status bar reports it.
            Showing::Opening => div().flex_1().into_any_element(),
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
            .w(width)
            .px_2()
            .justify_start()
            .cursor_pointer()
            .child(
                div()
                    .w(width - px(24.))
                    .flex()
                    .gap_2()
                    .child(
                        div()
                            .w_6()
                            .flex_shrink_0()
                            .text_color(cx.theme().muted_foreground)
                            .child(if index < 9 {
                                (index + 1).to_string()
                            } else {
                                String::new()
                            }),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .line_clamp(1)
                            .text_ellipsis()
                            .child(label),
                    ),
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
            .w(width)
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
                        .px_3()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child("or"),
                )
            })
            .child(chooser)
    }

    /// Which of the four the window is in.
    fn showing(&self) -> Showing {
        let Some(file) = self.file.as_ref() else {
            return Showing::Nothing;
        };
        if let Status::Failed(reason) = file.document.status() {
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
        window.set_rem_size(cx.theme().font_size);
        let frame = chrome::Frame::for_window(window);
        let corners = frame.corners;
        let summary = self
            .file
            .as_ref()
            .and_then(|file| file.document.summary())
            .unwrap_or_default();
        let status = self.file.as_ref().map_or_else(
            || "ready".to_owned(),
            |file| file.document.status().message(),
        );

        let content = div()
            .size_full()
            .flex()
            .flex_col()
            .font_family(cx.theme().font_family.clone())
            .text_color(cx.theme().foreground)
            .id("shell")
            .track_focus(&self.focus)
            .key_context(if self.file.is_none() {
                "Shell StartPage"
            } else {
                "Shell"
            })
            .on_action(cx.listener(Self::open_recent))
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
            .child(
                // Two status-row heights: 3 rem, or 48 logical pixels with the default font.
                // The waveform itself arrives in #31.
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .h_12()
                            .flex_shrink_0()
                            .border_b_1()
                            .border_color(cx.theme().border),
                    )
                    .child(self.middle(window, cx)),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .px_2()
                    .h_6()
                    .flex_shrink_0()
                    .rounded_bl(corners.bottom_left)
                    .rounded_br(corners.bottom_right)
                    .border_t_1()
                    .border_color(cx.theme().border)
                    .bg(cx.theme().secondary)
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(div().flex().min_w_0().overflow_hidden().children(
                        summary.into_iter().enumerate().map(|(index, field)| {
                            div()
                                .id(("metadata", index))
                                .px_2()
                                .flex_shrink_0()
                                .when(index > 0, |field| {
                                    field.border_l_1().border_color(cx.theme().border)
                                })
                                .tooltip(move |window, cx| {
                                    metadata_tooltip(field.hint.clone()).build(window, cx)
                                })
                                .child(field.value)
                        }),
                    ))
                    .child(div().flex_shrink_0().child(status)),
            );
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
            .w(width.min(window.viewport_size().width - px(48.)))
            .flex()
            .items_start()
            .gap_3()
            .child(div().flex_1().min_w_0().child(text.clone()))
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

fn metadata_tooltip(hint: MetadataHint) -> Tooltip {
    Tooltip::element(move |window, cx| {
        let term_color = if cx.theme().is_dark() {
            cx.theme().blue_light
        } else {
            cx.theme().blue.darken(0.2)
        };
        div()
            .w(px(288.))
            .max_w(window.viewport_size().width - px(32.))
            .py_1()
            .flex()
            .flex_col()
            .gap_2()
            .child(div().font_weight(FontWeight::SEMIBOLD).child(hint.title))
            .children(hint.sections.iter().map(|&(term, explanation)| {
                div()
                    .flex()
                    .flex_col()
                    .child(div().text_color(term_color).child(term))
                    .when(!explanation.is_empty(), |section| {
                        section.child(
                            div()
                                .text_color(cx.theme().muted_foreground)
                                .child(explanation),
                        )
                    })
            }))
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
