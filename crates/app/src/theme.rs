//! Argand's light and dark interface themes, derived from the toolkit's defaults.
//!
//! The toolkit applies the configuration it holds for a mode on every switch, so the
//! colours written here survive system appearance changes without being repeated.

use gpui_kit::App;
use gpui_kit::component::theme::{Theme, ThemeConfig, ThemeConfigColors, ThemeMode, ThemeRegistry};
use std::rc::Rc;

/// Holds Argand's themes in the global theme and applies the one for `mode`.
pub fn install(mode: ThemeMode, cx: &mut App) {
    let registry = ThemeRegistry::global(cx);
    let light = derive(registry.default_light_theme(), light);
    let dark = derive(registry.default_dark_theme(), dark);
    // The first change creates the global theme that holds the two configurations.
    Theme::change(mode, None, cx);
    let theme = Theme::global_mut(cx);
    theme.light_theme = Rc::new(light);
    theme.dark_theme = Rc::new(dark);
    Theme::change(mode, None, cx);
}

fn derive(base: &ThemeConfig, adjust: fn(&mut ThemeConfigColors)) -> ThemeConfig {
    let mut config = base.clone();
    adjust(&mut config.colors);
    config
}

fn dark(colors: &mut ThemeConfigColors) {
    // Hints and menus stand lighter than the panels and the picture they cover.
    colors.popover = Some("#212121".into());
    colors.border = Some("#363636".into());
    // The window's outline stays visible beside another dark window.
    colors.window_border = Some("#4d4d4d".into());
}

fn light(_: &mut ThemeConfigColors) {}
