//! Shared keycap appearance and measurement for shortcut hints.

use gpui::{App, AvailableSpace, FontWeight, IntoElement, Pixels, Styled, Window, size};
use gpui_component::kbd::Kbd;

pub(super) fn keycap(shortcut: Kbd) -> Kbd {
    shortcut
        .appearance(true)
        .font_weight(FontWeight::NORMAL)
        .whitespace_nowrap()
}

pub(super) fn width(shortcut: Kbd, window: &mut Window, cx: &mut App) -> Pixels {
    keycap(shortcut)
        .into_any_element()
        .layout_as_root(
            size(AvailableSpace::MaxContent, AvailableSpace::MaxContent),
            window,
            cx,
        )
        .width
        .ceil()
}
