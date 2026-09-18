//! Shared keycap colours and measurement, preserving the toolkit typography.

use gpui::{
    Action, App, AvailableSpace, IntoElement, Keystroke, Pixels, Styled, Window, relative, size,
};
use gpui_component::{ActiveTheme, kbd::Kbd};

/// Uniform keycap names for the symbol zoom keys.
///
/// A literal `+` or `-` formats as "Ctrl++" and reads as noise, so the four
/// zoom commands display named keys instead: Ctrl+Plus, Ctrl+Shift+Plus,
/// Ctrl+Minus, Ctrl+Shift+Minus.
pub(super) fn zoom_keycap(action: &dyn Action) -> Option<Kbd> {
    zoom_key_stroke(action).map(Kbd::new)
}

fn zoom_key_stroke(action: &dyn Action) -> Option<Keystroke> {
    use super::navigation_ui::{FrequencyZoomIn, FrequencyZoomOut, ZoomIn, ZoomOut};
    let stroke = if action.as_any().is::<ZoomIn>() {
        "ctrl-plus"
    } else if action.as_any().is::<ZoomOut>() {
        "ctrl-minus"
    } else if action.as_any().is::<FrequencyZoomIn>() {
        "ctrl-shift-plus"
    } else if action.as_any().is::<FrequencyZoomOut>() {
        "ctrl-shift-minus"
    } else {
        return None;
    };
    Keystroke::parse(stroke).ok()
}

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

#[cfg(test)]
mod tests {
    use super::super::navigation_ui::{
        FrequencyZoomIn, FrequencyZoomOut, ToggleGrid, ZoomIn, ZoomOut,
    };
    use super::*;

    fn label(action: &dyn Action) -> String {
        match zoom_key_stroke(action) {
            Some(stroke) => Kbd::format(&stroke),
            None => String::new(),
        }
    }

    #[test]
    fn zoom_commands_show_named_symbol_keys() {
        // Modifier notation is the platform's own (Ctrl, ⌃); what must hold
        // everywhere is that the key is named, not left as a raw symbol
        // that would read "Ctrl++".
        assert!(label(&ZoomIn).ends_with("Plus"));
        assert!(label(&ZoomOut).ends_with("Minus"));
        assert!(label(&FrequencyZoomIn).ends_with("Plus"));
        assert!(label(&FrequencyZoomOut).ends_with("Minus"));
        assert!(zoom_keycap(&ToggleGrid).is_none());
    }
}
