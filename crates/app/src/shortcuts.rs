//! Measure the unmodified keycap used by the Edit settings reference.

use gpui::{App, AvailableSpace, IntoElement, Pixels, Window, size};
use gpui_component::kbd::Kbd;

pub(super) fn width(shortcut: Kbd, window: &mut Window, cx: &mut App) -> Pixels {
    shortcut
        .into_any_element()
        .layout_as_root(
            size(AvailableSpace::MaxContent, AvailableSpace::MaxContent),
            window,
            cx,
        )
        .width
        .ceil()
}
