//! Shared keycap colours and measurement, preserving the toolkit typography.

use gpui::{App, AvailableSpace, IntoElement, Pixels, Styled, Window, relative, size};
use gpui_component::{ActiveTheme, kbd::Kbd};

pub(super) fn keycap(shortcut: Kbd, cx: &App) -> Kbd {
    let theme = cx.theme();
    // Kbd replaces its whole text refinement: retain all of its text defaults.
    shortcut
        .text_center()
        .line_height(relative(1.))
        .text_xs()
        .whitespace_normal()
        .text_color(theme.muted_foreground.blend(theme.foreground.opacity(0.2)))
        .border_color(theme.border.blend(theme.foreground.opacity(0.15)))
}

pub(super) fn width(shortcut: Kbd, window: &mut Window, cx: &mut App) -> Pixels {
    keycap(shortcut, cx)
        .into_any_element()
        .layout_as_root(
            size(AvailableSpace::MaxContent, AvailableSpace::MaxContent),
            window,
            cx,
        )
        .width
        .ceil()
}
