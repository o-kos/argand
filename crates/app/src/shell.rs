//! The window itself: the one place GPUI types appear.
//!
//! Everything the window needs to decide -- what the configuration says, where
//! the window may open, when to write the session -- is settled in [`crate::config`]
//! and [`crate::session`], which know nothing about a toolkit and are tested
//! without one. This module converts between those answers and GPUI, and holds
//! the placeholders the later milestones replace with real panels.

use std::time::Instant;

use gpui::{
    AppContext, Application, Bounds, Context, IntoElement, ParentElement, Pixels, Render, Styled,
    Subscription, TitlebarOptions, Window, WindowBounds, WindowOptions, div, point, px, relative,
    size,
};
use gpui_component::{ActiveTheme, Root, ThemeMode, TitleBar};

use crate::config::{Config, Theme};
use crate::session::{Geometry, Session, WindowState, Writer, place};

/// What the window is called, in its title bar and to the desktop environment.
const TITLE: &str = "argand";
/// Reverse-DNS identifier desktop environments group windows by.
const APP_ID: &str = "io.github.o_kos.argand";

/// Open the window and run until it closes.
pub fn run(config: Config, saved: Session, writer: Option<Writer>) {
    Application::new().run(move |cx| {
        gpui_component::init(cx);
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
                let shell = cx.new(|cx| Shell::new(config, writer, saved.geometry, window, cx));
                cx.new(|cx| Root::new(shell, window, cx))
            });

            // A window that will not open is the end of the run, and a task
            // whose error nobody reads would end it silently: there is nothing
            // else this program does.
            if let Err(error) = opened {
                tracing::error!(%error, "cannot open a window");
                cx.update(|cx| cx.quit())?;
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
        titlebar: Some(TitlebarOptions {
            title: Some(TITLE.into()),
            ..Default::default()
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

/// The window's content: a title bar, the area the panels will fill, and the
/// status bar under it.
struct Shell {
    /// What the person configured.
    ///
    /// The panel proportions are used below. The colour scheme, the range mode
    /// and the transform defaults have nothing to act on until a signal is
    /// loaded, so they are carried rather than applied: this milestone settles
    /// where they are read from and what they mean, and the milestones that
    /// draw with them take them from here.
    config: Config,
    /// Absent when the platform offers nowhere to keep state, or when the file
    /// there was written by a version this one must not overwrite. Either way
    /// the window simply does not remember itself.
    writer: Option<Writer>,
    /// The rectangle to come back to, which is not what a maximized or
    /// fullscreen window reports.
    ///
    /// `WindowBounds` documents its payload as the restore size, but the
    /// backends that omit a variant do not have one to give: X11 hands back the
    /// bounds its last configure event set, and macOS reads the live window
    /// frame. Both are the screen while the window covers it. Saving that would
    /// restore a maximized window correctly and then un-maximize it to the size
    /// of the display, so the last rectangle seen in the ordinary state is kept
    /// instead.
    last_normal: Option<Geometry>,
    /// Kept because dropping it stops the notifications.
    _bounds: Subscription,
}

impl Shell {
    fn new(
        config: Config,
        writer: Option<Writer>,
        last_normal: Option<Geometry>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        // The toolkit says when the window has moved or resized, so nothing
        // here has to ask on every frame. It still says it once per step of a
        // drag, which is what [`Writer`] is for.
        let bounds = cx.observe_window_bounds(window, |shell, window, _| shell.remember(window));
        Self {
            config,
            writer,
            last_normal,
            _bounds: bounds,
        }
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
        // Only an ordinary window reports the rectangle worth coming back to.
        if window_state == WindowState::Normal {
            self.last_normal = Some(from_bounds(bounds.get_bounds()));
        }
        let Some(writer) = self.writer.as_mut() else {
            return;
        };
        writer.offer(
            Session {
                version: crate::session::VERSION,
                geometry: self.last_normal,
                window_state,
            },
            Instant::now(),
        );
    }
}

impl Drop for Shell {
    fn drop(&mut self) {
        if let Some(writer) = self.writer.as_mut() {
            writer.flush(Instant::now());
        }
    }
}

impl Render for Shell {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(TitleBar::new().child(div().text_sm().child(TITLE)))
            .child(
                // Where the waveform, the spectrogram and the panels go, split
                // in the proportion the configuration asks for. The milestones
                // after this one fill these two; until then they are what shows
                // that the split is read from the file and reaches the layout.
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .h(relative(self.config.panels.waveform_fraction))
                            .border_b_1()
                            .border_color(cx.theme().border),
                    )
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_color(cx.theme().muted_foreground)
                            .child("no signal loaded"),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .px_2()
                    .h(px(24.0))
                    .border_t_1()
                    .border_color(cx.theme().border)
                    .bg(cx.theme().secondary)
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child("ready"),
            )
    }
}
