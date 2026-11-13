use super::*;
use gpui_kit::{Focusable, TestAppContext, test::TestWindowExt};

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
    Entity<PlotView>,
    argand_io::testutil::TempDir,
    &mut gpui_kit::VisualTestContext,
) {
    use crate::shell::replace_ui::tests::{open_window, wait_until, write_wave};
    let (shell, cx) = open_window(cx);
    cx.update(|window, _| window.activate_window());
    let dir = argand_io::testutil::TempDir::new("inline-reference");
    let path = dir.join("capture.wav");
    write_wave(&path, 2400);
    shell.update_in(cx, |shell, window, cx| {
        shell.open(Origin::new(path), window, cx)
    });
    wait_until(cx, &shell, "reference control", |shell| {
        shell.file.as_ref().is_some_and(|file| {
            matches!(
                file.state.document.status(),
                crate::document::Status::Ready { .. }
            )
        })
    });
    let plot = shell.read_with(cx, |shell, _| shell.plot_view().unwrap().clone());
    (shell, plot, dir, cx)
}

#[gpui_kit::test]
fn inline_reference_commits_cancels_and_preserves_file_metadata(cx: &mut TestAppContext) {
    let (shell, plot, _dir, cx) = opened(cx);
    cx.update(|window, cx| window.click("reference-frequency", cx));
    frame(cx);
    let input = plot.read_with(cx, |plot, _| {
        plot.reference_entry.as_ref().unwrap().input.clone()
    });
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
        assert!(
            shell
                .file
                .as_ref()
                .unwrap()
                .state
                .document
                .file_summary()
                .unwrap()
                .hint
                .rows
                .iter()
                .all(|(label, _)| label != "Reference frequency")
        );
    });
    assert!(plot.read_with(cx, |plot, _| plot.reference_entry.is_none()));
    plot.update_in(cx, |plot, window, cx| plot.open_reference(window, cx));
    frame(cx);
    let input = plot.read_with(cx, |plot, _| {
        plot.reference_entry.as_ref().unwrap().input.clone()
    });
    frame(cx);
    input.update_in(cx, |input, window, cx| input.set_value("bad", window, cx));
    cx.simulate_keystrokes("enter");
    frame(cx);
    assert!(plot.read_with(cx, |plot, _| plot.reference_entry.as_ref().unwrap().refused));
    cx.simulate_keystrokes("escape");
    frame(cx);
    assert!(plot.read_with(cx, |plot, _| plot.reference_entry.is_none()));
    assert_eq!(
        shell.read_with(cx, |shell, _| shell.editing().unwrap().reference()),
        1_010_001.
    );
    plot.update_in(cx, |plot, window, cx| plot.open_reference(window, cx));
    frame(cx);
    let input = plot.read_with(cx, |plot, _| {
        plot.reference_entry.as_ref().unwrap().input.clone()
    });
    input.update_in(cx, |input, window, cx| input.set_value("0", window, cx));
    let focus = plot.read_with(cx, |plot, _| plot.focus.clone());
    cx.update(|window, cx| window.focus(&focus, cx));
    frame(cx);
    assert_eq!(
        shell.read_with(cx, |shell, _| shell.editing().unwrap().reference()),
        0.
    );
}
#[gpui_kit::test]
fn reference_input_keeps_plot_shortcuts_and_space_in_the_editor(cx: &mut TestAppContext) {
    let (shell, plot, _dir, cx) = opened(cx);
    shell.update_in(cx, |shell, window, cx| {
        shell.set_reference(1_000_000., window, cx);
        window.focus(&shell.focus_target(cx), cx);
        window.dispatch_action(Box::new(super::super::navigation_ui::ZoomIn), cx);
    });
    frame(cx);
    let before = shell.read_with(cx, |shell, _| {
        (
            shell.session.baseband_frequencies,
            shell.session.show_grid,
            shell.view,
        )
    });
    cx.update(|window, cx| window.click("reference-frequency", cx));
    frame(cx);
    let input = plot.read_with(cx, |plot, _| {
        plot.reference_entry.as_ref().unwrap().input.clone()
    });
    input.update_in(cx, |input, window, cx| input.set_value("12", window, cx));
    for keys in ["ctrl-b", "ctrl-g", "ctrl-0", "space"] {
        cx.simulate_keystrokes(keys);
        frame(cx);
    }
    assert_eq!(
        shell.read_with(cx, |shell, _| (
            shell.session.baseband_frequencies,
            shell.session.show_grid,
            shell.view
        )),
        before
    );
    assert!(!plot.read_with(cx, |plot, _| plot.space));
    assert_eq!(
        input.read_with(cx, |input, _| input.value().to_string()),
        "12 "
    );
    cx.update(|window, cx| assert!(input.read(cx).focus_handle(cx).is_focused(window)));
}

#[gpui_kit::test]
fn reference_input_rejects_a_stale_commit_without_dropping_the_draft(cx: &mut TestAppContext) {
    let (shell, plot, _dir, cx) = opened(cx);
    cx.update(|window, cx| window.click("reference-frequency", cx));
    frame(cx);
    let input = plot.read_with(cx, |plot, _| {
        plot.reference_entry.as_ref().unwrap().input.clone()
    });
    input.update_in(cx, |input, window, cx| input.set_value("12345", window, cx));
    // The owner changed before the next snapshot reached the focused editor.
    shell.update(cx, |shell, _| {
        shell
            .file
            .as_mut()
            .unwrap()
            .state
            .document
            .mark_changed(None)
    });
    cx.simulate_keystrokes("enter");
    frame(cx);
    shell.read_with(cx, |shell, _| {
        assert!(!shell.is_reference_editable());
        assert_eq!(shell.editing().unwrap().reference(), 0.);
    });
    // An already queued request must be rejected by the fresh owner policy too.
    plot.update(cx, |_, cx| {
        cx.emit(PlotIntent::Reference {
            value: 12345.,
            opened: 0.,
            focus: true,
        })
    });
    frame(cx);
    assert!(plot.read_with(cx, |plot, _| plot.reference_entry.as_ref().unwrap().refused));
    assert_eq!(
        input.read_with(cx, |input, _| input.value().to_string()),
        "12345"
    );
    cx.simulate_keystrokes("escape");
    frame(cx);
    cx.update(|window, cx| window.click("reference-frequency", cx));
    frame(cx);
    assert!(plot.read_with(cx, |plot, _| plot.reference_entry.is_none()));
}

#[gpui_kit::test]
fn reference_input_fits_whole_hertz_at_default_and_enlarged_font(cx: &mut TestAppContext) {
    let (shell, plot, _dir, cx) = opened(cx);
    shell.update_in(cx, |shell, window, cx| {
        shell.set_reference(f64::from(u32::MAX), window, cx);
    });
    frame(cx);
    cx.update(|window, cx| window.click("reference-frequency", cx));
    for rem in [16., 24.] {
        cx.update(|window, cx| {
            gpui_kit::component::theme::Theme::update(cx, |theme| theme.font_size = px(rem));
            window.set_rem_size(px(rem));
        });
        frame(cx);
        cx.update(|window, cx| {
            let field = window.find("reference-frequency-value").bounds();
            assert!(field.size.width <= px(rem * INPUT_WIDTH));
            assert!(field.size.height <= px(rem * HEIGHT), "{field:?}");
            assert!(field.left() >= px(0.) && field.right() <= window.viewport_size().width);
            let value = crate::numbers::number(f64::from(u32::MAX));
            let style = gpui_kit::TextStyle {
                font_family: cx.theme().font_family.clone(),
                ..Default::default()
            };
            let text = window.text_system().shape_line(
                value.clone().into(),
                px(rem * 0.75),
                &[style.to_run(value.len())],
                None,
            );
            assert!(
                field.size.width >= text.width + px(rem * 2.5),
                "the complete value, suffix and padding fit"
            );
        });
        assert!(plot.read_with(cx, |plot, _| plot.reference_entry.is_some()));
    }
}
