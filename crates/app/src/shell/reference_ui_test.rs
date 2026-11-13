use super::*;
use crate::shell::replace_ui::tests::{open_window, wait_until, write_wave};
use gpui_kit::{TestAppContext, test::TestWindowExt};

fn frame(cx: &mut gpui_kit::VisualTestContext) {
    cx.run_until_parked();
    cx.update(|window, cx| {
        window.render_frame(cx);
        window.simulate_next_frame(cx);
    });
    cx.run_until_parked();
}

fn opened(
    cx: &mut TestAppContext,
) -> (
    Entity<Shell>,
    argand_io::testutil::TempDir,
    &mut gpui_kit::VisualTestContext,
) {
    let (shell, cx) = open_window(cx);
    cx.update(|window, _| window.activate_window());
    let dir = argand_io::testutil::TempDir::new("status-reference");
    let path = dir.join("capture.wav");
    write_wave(&path, 2400);
    shell.update_in(cx, |shell, window, cx| {
        shell.open(Origin::new(path), window, cx)
    });
    wait_until(cx, &shell, "reference fixture", |shell| {
        shell.is_reference_editable()
    });
    frame(cx);
    (shell, dir, cx)
}

fn editor(shell: &Entity<Shell>, cx: &gpui_kit::VisualTestContext) -> Entity<Editor> {
    shell.read_with(cx, |shell, cx| {
        shell
            .reference
            .hint
            .read(cx)
            .view()
            .unwrap()
            .downcast::<Editor>()
            .unwrap()
    })
}

#[gpui_kit::test]
fn status_reference_edits_and_saves_without_changing_samples(cx: &mut TestAppContext) {
    let (shell, dir, cx) = opened(cx);
    let path = dir.join("capture.wav");
    let before = std::fs::read(&path).unwrap()[44..].to_vec();
    cx.update(|window, cx| {
        assert!(window.try_find("reference-frequency-control").is_none());
        let file = window.find("file-information").bounds();
        let reference = window.find("reference-frequency").bounds();
        let analysis = window.find("analysis-settings").bounds();
        assert!(file.right() <= reference.left());
        assert!(reference.right() <= analysis.left());
        assert_eq!(file.top(), reference.top());
        assert_eq!(reference.top(), analysis.top());
        window.click("reference-frequency", cx);
    });
    frame(cx);
    let editor = editor(&shell, cx);
    let input = editor.read_with(cx, |editor, _| editor.input.clone());
    input.update_in(cx, |input, window, cx| {
        input.set_value("10 100 00.8", window, cx)
    });
    assert_eq!(
        input.read_with(cx, |input, _| input.value().to_string()),
        "10 100 00.8"
    );
    cx.simulate_keystrokes("enter");
    frame(cx);
    shell.read_with(cx, |shell, _| {
        assert_eq!(shell.editing().unwrap().reference(), 1_010_001.);
        assert!(shell.editing().unwrap().is_dirty());
    });
    assert!(editor.read_with(cx, |editor, _| !editor.refused));
    cx.update(|window, cx| window.click("save-reference", cx));
    wait_until(cx, &shell, "reference save", |shell| {
        shell.saving.is_none() && !shell.editing().unwrap().is_dirty()
    });
    let source = argand_io::open(&path, &Default::default()).unwrap();
    assert_eq!(source.meta().center_freq, 1_010_001.);
    let bytes = std::fs::read(&path).unwrap();
    let data = bytes.windows(4).position(|part| part == b"data").unwrap() + 8;
    assert_eq!(&bytes[data..], before);
}

#[gpui_kit::test]
fn reference_draft_rejects_invalid_and_stale_values_and_isolates_plot_keys(
    cx: &mut TestAppContext,
) {
    let (shell, _dir, cx) = opened(cx);
    cx.update(|window, cx| window.click("reference-frequency", cx));
    frame(cx);
    let editor = editor(&shell, cx);
    let input = editor.read_with(cx, |editor, _| editor.input.clone());
    input.update_in(cx, |input, window, cx| input.set_value("bad", window, cx));
    cx.simulate_keystrokes("enter");
    frame(cx);
    assert!(editor.read_with(cx, |editor, _| editor.refused));
    assert_eq!(
        shell.read_with(cx, |shell, _| shell.editing().unwrap().reference()),
        0.
    );
    let grid = shell.read_with(cx, |shell, _| shell.session.show_grid);
    cx.simulate_keystrokes("ctrl-g");
    cx.simulate_keystrokes("space");
    frame(cx);
    assert_eq!(
        shell.read_with(cx, |shell, _| shell.session.show_grid),
        grid
    );
    input.update_in(cx, |input, window, cx| input.set_value("200", window, cx));
    shell.update_in(cx, |shell, window, cx| {
        shell.set_reference(100., window, cx);
    });
    editor.update_in(cx, |editor, window, cx| {
        assert!(!editor.commit(window, cx));
    });
    assert_eq!(
        shell.read_with(cx, |shell, _| shell.editing().unwrap().reference()),
        100.
    );
    cx.simulate_keystrokes("escape");
    frame(cx);
    shell.read_with(cx, |shell, cx| {
        assert!(!shell.reference.hint.read(cx).is_open())
    });
}

#[gpui_kit::test]
fn closing_reference_commits_outside_draft_but_escape_discards_it(cx: &mut TestAppContext) {
    let (shell, _dir, cx) = opened(cx);
    cx.update(|window, cx| window.click("reference-frequency", cx));
    frame(cx);
    let input = editor(&shell, cx).read_with(cx, |editor, _| editor.input.clone());
    input.update_in(cx, |input, window, cx| input.set_value("123", window, cx));
    cx.simulate_mouse_down(
        gpui_kit::point(px(650.), px(250.)),
        MouseButton::Left,
        Default::default(),
    );
    cx.simulate_mouse_up(
        gpui_kit::point(px(650.), px(250.)),
        MouseButton::Left,
        Default::default(),
    );
    frame(cx);
    assert_eq!(
        shell.read_with(cx, |shell, _| shell.editing().unwrap().reference()),
        123.
    );
    cx.update(|window, cx| window.click("reference-frequency", cx));
    frame(cx);
    let input = editor(&shell, cx).read_with(cx, |editor, _| editor.input.clone());
    input.update_in(cx, |input, window, cx| input.set_value("456", window, cx));
    cx.simulate_keystrokes("escape");
    frame(cx);
    assert_eq!(
        shell.read_with(cx, |shell, _| shell.editing().unwrap().reference()),
        123.
    );
}

#[gpui_kit::test]
fn tab_and_enter_save_from_the_reference_panel(cx: &mut TestAppContext) {
    let (shell, dir, cx) = opened(cx);
    cx.update(|window, cx| window.click("reference-frequency", cx));
    frame(cx);
    let input = editor(&shell, cx).read_with(cx, |editor, _| editor.input.clone());
    input.update_in(cx, |input, window, cx| input.set_value("123", window, cx));
    frame(cx);
    cx.simulate_keystrokes("tab");
    frame(cx);
    cx.update(|window, _| assert_eq!(window.find("save-reference").focused(), Some(true)));
    cx.update(|window, cx| window.press("enter", cx));
    wait_until(cx, &shell, "keyboard reference save", |shell| {
        shell.saving.is_none() && !shell.editing().unwrap().is_dirty()
    });
    let source = argand_io::open(&dir.join("capture.wav"), &Default::default()).unwrap();
    assert_eq!(source.meta().center_freq, 123.);
}
