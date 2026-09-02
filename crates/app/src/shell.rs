//! The window itself: the one place GPUI types appear.
//!
//! Everything the window needs to decide -- what the configuration says, where
//! the window may open, when to write the session -- is settled in [`crate::config`]
//! and [`crate::session`], which know nothing about a toolkit and are tested
//! without one. This module converts between those answers and GPUI, and holds
//! the placeholders the later milestones replace with real panels.

use std::path::PathBuf;
use std::time::Instant;

use gpui::{
    AppContext, Application, Bounds, Context, IntoElement, ParentElement, Pixels, Render, Styled,
    TitlebarOptions, Window, WindowBounds, WindowOptions, div, point, px, size,
};
use gpui_component::{ActiveTheme, Root, ThemeMode, TitleBar};

use crate::config::{Config, Theme};
use crate::session::{Geometry, Session, WindowState, Writer, place};

/// What the window is called, in its title bar and to the desktop environment.
const TITLE: &str = "argand";
/// Reverse-DNS identifier desktop environments group windows by.
const APP_ID: &str = "io.github.o_kos.argand";

/// Open the window and run until it closes.
pub fn run(config: Config, saved: Session, state_path: Option<PathBuf>) {
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

            let writer = state_path.map(|path| Writer::new(path, saved.clone()));
            cx.open_window(options, |window, cx| {
                cx.new(|cx| Root::new(cx.new(|_| Shell::new(writer)), window, cx))
            })?;
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
    /// Absent when the platform offers nowhere to keep state, in which case
    /// the window simply does not remember itself.
    writer: Option<Writer>,
}

impl Shell {
    const fn new(writer: Option<Writer>) -> Self {
        Self { writer }
    }

    /// Record where the window is, on the frame that draws it there.
    ///
    /// gpui 0.2 has no observer for a move or a resize, and the render path is
    /// the one thing that certainly runs while a window is being dragged. The
    /// cost of asking on every frame is one comparison: [`Writer`] decides
    /// whether anything reaches the disk.
    fn remember(&mut self, window: &Window) {
        let Some(writer) = self.writer.as_mut() else {
            return;
        };
        let bounds = window.window_bounds();
        writer.offer(
            Session {
                version: crate::session::VERSION,
                geometry: Some(from_bounds(bounds.get_bounds())),
                window_state: state_of(window),
            },
            Instant::now(),
        );
    }
}

fn state_of(window: &Window) -> WindowState {
    if window.is_fullscreen() {
        WindowState::Fullscreen
    } else if window.is_maximized() {
        WindowState::Maximized
    } else {
        WindowState::Normal
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
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.remember(window);

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(TitleBar::new().child(div().text_sm().child(TITLE)))
            .child(
                // Where the waveform, the spectrogram and the panels go. The
                // milestones after this one replace it; until then it is what
                // proves the window has a body the theme reaches.
                div()
                    .flex_1()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(cx.theme().muted_foreground)
                    .child("no signal loaded"),
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
