use super::*;
use gpui_kit::WindowAppearance;

#[test]
fn system_follows_appearance_and_explicit_themes_remain_fixed() {
    for (appearance, expected) in [
        (WindowAppearance::Light, ThemeMode::Light),
        (WindowAppearance::VibrantLight, ThemeMode::Light),
        (WindowAppearance::Dark, ThemeMode::Dark),
        (WindowAppearance::VibrantDark, ThemeMode::Dark),
    ] {
        assert_eq!(theme_mode(Theme::System, appearance), expected);
        assert_eq!(theme_mode(Theme::Dark, appearance), ThemeMode::Dark);
        assert_eq!(theme_mode(Theme::Light, appearance), ThemeMode::Light);
    }
}
