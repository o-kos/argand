use super::super::navigation_ui::{FrequencyZoomIn, FrequencyZoomOut, ToggleGrid, ZoomIn, ZoomOut};
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
