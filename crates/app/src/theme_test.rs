use super::*;
use gpui_kit::TestAppContext;

fn surfaces(cx: &mut TestAppContext) -> [Hsla; 4] {
    cx.update(|cx| {
        [
            cx.theme().popover,
            cx.theme().status_bar,
            sheet(cx),
            cx.theme().link,
        ]
    })
}

#[gpui_kit::test]
fn every_switch_applies_argands_surfaces(cx: &mut TestAppContext) {
    cx.update(|cx| {
        gpui_kit::init(cx);
        install(ThemeMode::Dark, cx);
    });
    let dark = [
        gpui_kit::rgb(0x2b2b2b).into(),
        gpui_kit::rgb(0x171717).into(),
        gpui_kit::rgb(0x1f1f1f).into(),
        gpui_kit::component::blue_400(),
    ];
    let light = [
        gpui_kit::rgb(0xffffff).into(),
        gpui_kit::rgb(0xf3f3f3).into(),
        gpui_kit::rgb(0xf4f4f5).into(),
        gpui_kit::component::blue_600(),
    ];
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
