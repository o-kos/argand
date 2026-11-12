//! Client-side frame geometry and its resize regions.

use gpui_kit::component::{ActiveTheme, Icon, IconName, Sizable};
use gpui_kit::{
    App, Bounds, Decorations, FocusHandle, Hsla, InteractiveElement, IntoElement, MouseButton,
    ParentElement, Pixels, StatefulInteractiveElement, Styled, Window, div, point,
    prelude::FluentBuilder, px, size,
};

/// The title bar's colours, muted while the window is not the active one.
#[derive(Clone, Copy)]
pub struct TitlePalette {
    pub background: Hsla,
    pub border: Hsla,
    pub foreground: Hsla,
    /// A caption icon at rest, under the pointer and held.
    icon: [Hsla; 3],
}

impl TitlePalette {
    pub fn for_window(window: &Window, cx: &App) -> Self {
        let theme = cx.theme();
        if window.is_window_active() {
            let ink = theme.foreground;
            return Self {
                background: theme.title_bar,
                border: theme.title_bar_border,
                foreground: ink,
                icon: [ink.opacity(0.72), ink, ink.opacity(0.45)],
            };
        }
        let ink = theme.muted_foreground;
        Self {
            background: theme.background,
            border: theme.title_bar_border,
            foreground: ink,
            icon: [ink.opacity(0.7), theme.foreground, ink.opacity(0.45)],
        }
    }
}

const CAPTION_ICONS: [IconName; 3] = [
    IconName::WindowMinimize,
    IconName::WindowMaximize,
    IconName::WindowClose,
];

/// The native-caption-sized controls share their reserved backdrop width.
pub fn controls_width() -> Pixels {
    gpui_kit::component::TITLE_BAR_HEIGHT * CAPTION_ICONS.len()
}

/// Width reserved for the controls actually painted by the window's title bar.
pub fn menu_caption_width(window: &Window) -> Pixels {
    caption_width(std::env::consts::OS, window.window_controls())
}

fn caption_width(platform: &str, controls: gpui_kit::WindowControls) -> Pixels {
    match platform {
        "linux" => controls_width(),
        "windows" => {
            let count = 1 + usize::from(controls.minimize) + usize::from(controls.maximize);
            gpui_kit::component::TITLE_BAR_HEIGHT * count
        }
        _ => px(0.),
    }
}

/// Covers menu-dismissal input while leaving the title bar's caption controls live.
pub fn menu_backdrop(area: Bounds<Pixels>, caption_width: Pixels) -> gpui_kit::AnyElement {
    let caption = if caption_width > px(0.) {
        gpui_kit::component::TITLE_BAR_HEIGHT
    } else {
        px(0.)
    };
    let cover = |id, left, top, width, height| {
        div()
            .id(id)
            .absolute()
            .left(left)
            .top(top)
            .w(width)
            .h(height)
            .occlude()
            .cursor(gpui_kit::CursorStyle::Arrow)
    };
    let backdrop = div()
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .child(cover(
            "application-menu-backdrop",
            area.origin.x,
            area.origin.y + caption,
            area.size.width,
            area.size.height - caption,
        ))
        .when(caption > px(0.), |backdrop| {
            backdrop.child(cover(
                "application-menu-title-backdrop",
                area.origin.x,
                area.origin.y,
                (area.size.width - caption_width).max(px(0.)),
                caption,
            ))
        });
    gpui_kit::deferred(backdrop)
        .with_priority(1)
        .into_any_element()
}

/// Linux controls paint their own corners; GPUI only clips children rectangularly.
pub fn controls(corner: Pixels, focus: FocusHandle, window: &Window, cx: &App) -> impl IntoElement {
    let palette = TitlePalette::for_window(window, cx);
    let maximize_icon = if window.is_maximized() {
        IconName::WindowRestore
    } else {
        IconName::WindowMaximize
    };
    div().flex().h_full().flex_shrink_0().children(
        [
            CAPTION_ICONS[0].clone(),
            maximize_icon,
            CAPTION_ICONS[2].clone(),
        ]
        .into_iter()
        .enumerate()
        .map(|(index, icon)| {
            let focus = focus.clone();
            let close = index == 2;
            let [resting, hovered, held] = palette.icon;
            // Close keeps its danger surface, with its icon dimmed while held like the others.
            let (hover, active, hovered, held) = if close {
                let ink = cx.theme().danger_foreground;
                (
                    cx.theme().danger,
                    cx.theme().danger_active,
                    ink,
                    ink.opacity(0.6),
                )
            } else {
                (
                    cx.theme().secondary_hover,
                    cx.theme().secondary_active,
                    hovered,
                    held,
                )
            };
            div()
                .id(("window-control", index))
                .w(gpui_kit::component::TITLE_BAR_HEIGHT)
                .h_full()
                .flex()
                .items_center()
                .justify_center()
                .when(close, |button| button.rounded_tr(corner))
                .text_color(resting)
                .hover(move |style| style.bg(hover).text_color(hovered))
                .active(move |style| style.bg(active).text_color(held))
                .on_mouse_down(MouseButton::Left, |_, window, cx| {
                    window.prevent_default();
                    cx.stop_propagation();
                })
                .on_click(move |_, window, cx| {
                    cx.stop_propagation();
                    match index {
                        0 => window.minimize_window(),
                        1 => window.zoom_window(),
                        // The window decides whether it may close, which unsaved edits can refuse.
                        _ => focus.dispatch_action(&CloseWindow, window, cx),
                    }
                })
                .child(Icon::new(icon).small())
        }),
    )
}

gpui_kit::actions!(chrome, [CloseWindow]);

/// The content area left to overlays by the toolkit frame's resize band.
/// Coordinates are relative to Shell, inside the standard Root surface.
pub fn overlay_area(window: &Window) -> Bounds<Pixels> {
    let paddings = gpui_kit::component::window_paddings(window);
    let viewport = window.viewport_size();
    let client = matches!(window.window_decorations(), Decorations::Client { .. });
    overlay_bounds(viewport, paddings, client).0
}

pub fn overlay_window_area(window: &Window) -> Bounds<Pixels> {
    let paddings = gpui_kit::component::window_paddings(window);
    let client = matches!(window.window_decorations(), Decorations::Client { .. });
    overlay_bounds(window.viewport_size(), paddings, client).1
}

fn overlay_bounds(
    viewport: gpui_kit::Size<Pixels>,
    paddings: gpui_kit::Edges<Pixels>,
    client: bool,
) -> (Bounds<Pixels>, Bounds<Pixels>) {
    // Locked WindowBorder uses a four-pixel inner hit band including its one-pixel border.
    let inset = |padding: Pixels| {
        if client && padding > px(0.) {
            px(3.)
        } else {
            px(0.)
        }
    };
    let left = inset(paddings.left);
    let top = inset(paddings.top);
    let right = inset(paddings.right);
    let bottom = inset(paddings.bottom);
    let border = |padding: Pixels| {
        if client && padding > px(0.) {
            px(1.)
        } else {
            px(0.)
        }
    };
    let local = Bounds::new(
        point(left, top),
        size(
            (viewport.width
                - paddings.left
                - paddings.right
                - border(paddings.left)
                - border(paddings.right)
                - left
                - right)
                .max(px(0.)),
            (viewport.height
                - paddings.top
                - paddings.bottom
                - border(paddings.top)
                - border(paddings.bottom)
                - top
                - bottom)
                .max(px(0.)),
        ),
    );
    let mut global = local;
    global.origin += point(
        paddings.left + border(paddings.left),
        paddings.top + border(paddings.top),
    );
    (local, global)
}

/// The wrapper observes bubbled presses; the toolkit frame owns resizing.
pub fn observe_frame(
    surface: impl IntoElement,
    content: impl Fn(&Window) -> Bounds<Pixels> + 'static,
    pressed: impl Fn(&mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id("frame-input-observer")
        .size_full()
        .child(surface)
        .on_mouse_down(MouseButton::Left, move |event, window, cx| {
            if !content(window).contains(&event.position) {
                pressed(window, cx);
            }
        })
}

#[cfg(test)]
#[path = "chrome_test.rs"]
mod tests;
