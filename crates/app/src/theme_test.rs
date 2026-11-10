use super::*;
use gpui_kit::TestAppContext;

fn surfaces(cx: &mut TestAppContext) -> [Hsla; 3] {
    cx.update(|cx| [cx.theme().popover, cx.theme().status_bar, sheet(cx)])
}

#[gpui_kit::test]
fn every_switch_applies_argands_surfaces(cx: &mut TestAppContext) {
    cx.update(|cx| {
        gpui_kit::init(cx);
        install(ThemeMode::Dark, cx);
    });
    let dark = [0x2b2b2b, 0x171717, 0x1f1f1f].map(|c| Hsla::from(gpui_kit::rgb(c)));
    let light = [0xffffff, 0xf3f3f3, 0xf4f4f5].map(|c| Hsla::from(gpui_kit::rgb(c)));
    assert_eq!(surfaces(cx), dark, "the installed dark theme");
    for (mode, expected) in [
        (ThemeMode::Light, light),
        (ThemeMode::Dark, dark),
        (ThemeMode::Light, light),
    ] {
        cx.update(|cx| Theme::change(mode, None, cx));
        assert_eq!(surfaces(cx), expected, "after a switch to {mode:?}");
    }
}
