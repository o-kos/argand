use super::*;
use crate::shell::replace_ui::tests::{open_window, wait_until, write_wave};
use gpui_kit::TestAppContext;

#[gpui_kit::test]
fn unavailable_file_shortcuts_are_inert_and_follow_document_lifetime(cx: &mut TestAppContext) {
    let (shell, cx) = open_window(cx);
    cx.update(|_, cx| Shell::bind_choose_file(shell.downgrade(), cx));
    for keys in ["ctrl-s", "ctrl-shift-s", "ctrl-alt-s", "ctrl-,", "f11"] {
        let keys = if cfg!(target_os = "macos") {
            keys.replace("ctrl-", "cmd-")
        } else {
            keys.to_owned()
        };
        cx.simulate_keystrokes(&keys);
        cx.run_until_parked();
    }
    shell.read_with(cx, |shell, cx| {
        assert!(shell.saving.is_none());
        assert!(shell.save_notice.is_none());
        assert!(!shell.analysis_hint.read(cx).is_open());
        assert!(!shell.conversion.hint.read(cx).is_open());
        assert!(!shell.command_availability().is_enabled::<SaveAs>());
        assert!(!shell.command_availability().is_enabled::<EditAnalysis>());
        assert!(!shell.command_availability().is_enabled::<ConvertCapture>());
    });
    let dir = argand_io::testutil::TempDir::new("command-lifetime");
    let path = dir.join("capture.wav");
    write_wave(&path, 2400);
    shell.update_in(cx, |shell, window, cx| {
        shell.open(Origin::new(path), window, cx)
    });
    wait_until(cx, &shell, "document commands", |shell| {
        shell.command_availability().is_enabled::<ConvertCapture>()
    });
    shell.read_with(cx, |shell, _| {
        assert!(shell.command_availability().is_enabled::<SaveAs>());
        assert!(shell.command_availability().is_enabled::<EditAnalysis>());
        assert!(!shell.command_availability().is_enabled::<Save>());
    });
    cx.simulate_keystrokes("f11");
    cx.run_until_parked();
    shell.read_with(cx, |shell, cx| {
        assert!(shell.conversion.hint.read(cx).is_open())
    });
}
