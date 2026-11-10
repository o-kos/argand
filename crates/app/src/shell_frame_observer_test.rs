use super::*;
use gpui_kit::test::TestWindowExt;

#[gpui_kit::test]
fn frame_dismissal_updates_shell_through_root_without_reborrowing_it(
    cx: &mut gpui_kit::TestAppContext,
) {
    cx.update(|cx| {
        gpui_kit::init(cx);
        Root::register_plugin(cx, |_, _| FrameObserver);
        settings_ui::init(cx);
        hints::init(cx);
        window_keys(cx);
    });
    let (root, cx) = cx.add_window_view(|window, cx| {
        let shell =
            cx.new(|cx| Shell::new(Config::default(), None, Session::default(), window, cx));
        Root::new(shell, window, cx)
    });
    let shell = cx.update(|_, cx| root.read(cx).view().clone().downcast::<Shell>().unwrap());
    shell.update_in(cx, |shell, window, cx| {
        shell.open(Origin::new("/captures/frame-test.iqw".into()), window, cx);
    });
    cx.run_until_parked();
    shell.update_in(cx, |shell, window, cx| {
        shell.edit_analysis(&EditAnalysis, window, cx)
    });
    cx.run_until_parked();
    cx.update(|window, cx| window.render_frame(cx));
    assert!(shell.read_with(cx, |shell, cx| shell.analysis_hint.read(cx).is_open()));
    cx.update(Shell::dismiss_frame_overlays);
    cx.run_until_parked();
    cx.update(|window, cx| window.render_frame(cx));
    assert!(!shell.read_with(cx, |shell, cx| shell.analysis_hint.read(cx).is_open()));
}
