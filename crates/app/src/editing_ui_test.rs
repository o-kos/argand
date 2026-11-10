use super::*;
use crate::analysis::{FileInfo, Update};
use argand_core::SampleSpan;
use argand_core::{Domain, SampleFormat, SampleType, SignalMeta};
use gpui_kit::TestAppContext;
use std::path::PathBuf;

fn open_window(cx: &mut TestAppContext) -> (Entity<Shell>, &mut gpui_kit::VisualTestContext) {
    cx.update(|cx| {
        gpui_kit::init(cx);
        settings_ui::init(cx);
        navigation_ui::init(cx);
        hints::init(cx);
        app_menu_ui::init(cx);
        window_keys(cx);
        crate::theme::install(ThemeMode::Dark, cx);
    });
    let (shell, cx) = cx.add_window_view(|window, cx| {
        Shell::new(Config::default(), None, Session::default(), window, cx)
    });
    cx.simulate_resize(gpui_kit::size(px(800.), px(600.)));
    cx.run_until_parked();
    (shell, cx)
}

/// A capture that has described itself, with its edits started as an opened file's are.
fn described(cx: &mut gpui_kit::VisualTestContext, shell: &Entity<Shell>, path: &str) {
    shell.update_in(cx, |shell, window, cx| {
        shell.open(Origin::new(PathBuf::from(path)), window, cx);
        let Some(file) = shell.file.as_mut() else {
            panic!("the capture never opened");
        };
        file.state.document.apply(Update::Opened(
            SignalMeta {
                sample_rate: 24_000.0,
                center_freq: 0.0,
                sample_type: SampleType::new(Domain::Iq, SampleFormat::I16),
                len_samples: 48_000,
                container: "raw",
                divisor: 32_768.0,
                source: PathBuf::from(path),
            },
            FileInfo::default(),
        ));
        shell.start_editing(cx);
        shell.view = Some(crate::navigation::View::full(48_000));
    });
    cx.run_until_parked();
}

fn length(cx: &mut gpui_kit::VisualTestContext, shell: &Entity<Shell>) -> Option<u64> {
    shell.read_with(cx, |shell, _| shell.sample_count())
}

fn select(cx: &mut gpui_kit::VisualTestContext, shell: &Entity<Shell>, a: u64, b: u64) {
    shell.update_in(cx, |shell, _, _| {
        shell.selection = SampleSpan::between(a, b)
    });
}

#[gpui_kit::test]
fn delete_undo_and_redo_follow_the_length_selection_and_title(cx: &mut TestAppContext) {
    let (shell, cx) = open_window(cx);
    described(cx, &shell, "/captures/a.iqw");
    select(cx, &shell, 1000, 3000);
    let can = shell.read_with(cx, |shell, _| shell.edit_commands_available());
    assert!(can.cut && !can.undo && !can.paste);
    shell.update_in(cx, |shell, window, cx| shell.delete_selection(window, cx));
    assert_eq!(length(cx, &shell), Some(46_000));
    assert!(shell.read_with(cx, |shell, _| shell.selection.is_none()));
    assert!(shell.read_with(cx, |shell, _| shell.file_name().starts_with('•')));
    shell.update_in(cx, |shell, window, cx| shell.undo(window, cx));
    assert_eq!(length(cx, &shell), Some(48_000));
    assert!(!shell.read_with(cx, |shell, _| shell.file_name().starts_with('•')));
    shell.update_in(cx, |shell, window, cx| shell.redo(window, cx));
    assert_eq!(length(cx, &shell), Some(46_000));
}

#[gpui_kit::test]
fn copy_and_replace_selection_paste_and_select_the_pasted_samples(cx: &mut TestAppContext) {
    let (shell, cx) = open_window(cx);
    described(cx, &shell, "/captures/a.iqw");
    select(cx, &shell, 0, 100);
    shell.update_in(cx, |shell, _, cx| shell.copy_selection(cx));
    select(cx, &shell, 1000, 3000);
    assert!(shell.read_with(cx, |shell, _| shell.edit_commands_available().replace));
    shell.update_in(cx, |shell, window, cx| {
        shell.paste(
            Placement::Replace(SampleSpan::between(1000, 3000).unwrap()),
            window,
            cx,
        );
    });
    assert_eq!(length(cx, &shell), Some(46_100));
    assert_eq!(
        shell.read_with(cx, |shell, _| shell.selection),
        SampleSpan::between(1000, 1100)
    );
}

#[gpui_kit::test]
fn opening_another_file_asks_about_unsaved_edits_first(cx: &mut TestAppContext) {
    let (shell, cx) = open_window(cx);
    described(cx, &shell, "/captures/a.iqw");
    select(cx, &shell, 0, 100);
    shell.update_in(cx, |shell, window, cx| shell.delete_selection(window, cx));
    let opened = |cx: &mut gpui_kit::VisualTestContext| {
        shell.read_with(cx, |shell, _| {
            shell
                .file
                .as_ref()
                .map(|file| file.state.document.origin().path.clone())
        })
    };
    shell.update_in(cx, |shell, window, cx| {
        shell.open(Origin::new(PathBuf::from("/captures/b.iqw")), window, cx);
    });
    assert!(cx.has_pending_prompt());
    cx.simulate_prompt_answer("Cancel");
    cx.run_until_parked();
    assert_eq!(opened(cx), Some(PathBuf::from("/captures/a.iqw")));
    shell.update_in(cx, |shell, window, cx| {
        shell.open(Origin::new(PathBuf::from("/captures/b.iqw")), window, cx);
    });
    cx.simulate_prompt_answer("Discard");
    cx.run_until_parked();
    assert_eq!(opened(cx), Some(PathBuf::from("/captures/b.iqw")));
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
#[gpui_kit::test]
fn without_a_system_dialog_the_toolkit_dialog_asks(cx: &mut TestAppContext) {
    open_window(cx);
    let mut shell = None;
    let handle = cx.add_window(|window, cx| {
        let view = cx.new(|cx| Shell::new(Config::default(), None, Session::default(), window, cx));
        shell = Some(view.clone());
        gpui_kit::component::Root::new(view, window, cx)
    });
    let shell = shell.unwrap();
    let cx = &mut gpui_kit::VisualTestContext::from_window(handle.into(), cx);
    described(cx, &shell, "/captures/a.iqw");
    select(cx, &shell, 0, 100);
    shell.update_in(cx, |shell, window, cx| shell.delete_selection(window, cx));
    shell.update_in(cx, |shell, window, cx| {
        shell.open(Origin::new(PathBuf::from("/captures/b.iqw")), window, cx);
    });
    cx.run_until_parked();
    assert!(!cx.has_pending_prompt());
    assert!(cx.update(|window, cx| window.has_active_dialog(cx)));
    cx.simulate_keystrokes("escape");
    cx.run_until_parked();
    assert!(!cx.update(|window, cx| window.has_active_dialog(cx)));
    let opened = shell.read_with(cx, |shell, _| {
        shell
            .file
            .as_ref()
            .map(|file| file.state.document.origin().path.clone())
    });
    assert_eq!(
        opened,
        Some(PathBuf::from("/captures/a.iqw")),
        "Escape cancels"
    );
}

#[gpui_kit::test]
fn a_band_is_clamped_to_the_capture_and_leaves_nothing_to_edit(cx: &mut TestAppContext) {
    let (shell, cx) = open_window(cx);
    described(cx, &shell, "/captures/a.iqw");
    shell.update_in(cx, |shell, _, cx| {
        shell.select(
            argand_core::Selection {
                time: SampleSpan::between(0, 100),
                band: argand_core::FrequencyBand::between(-1e9, 1_000.),
            },
            cx,
        );
    });
    shell.read_with(cx, |shell, _| {
        assert_eq!(
            shell.band,
            argand_core::FrequencyBand::between(-12_000., 1_000.)
        );
        assert_eq!(
            shell.time_selection(),
            None,
            "a rectangle is not a time selection"
        );
        assert!(!shell.edit_commands_available().cut);
    });
    shell.update_in(cx, |shell, _, cx| {
        shell.select(
            argand_core::Selection {
                time: SampleSpan::between(0, 100),
                band: argand_core::FrequencyBand::between(20_000., 30_000.),
            },
            cx,
        );
    });
    shell.read_with(cx, |shell, _| {
        assert_eq!(
            (shell.selection, shell.band),
            (None, None),
            "a band beyond the capture selects nothing"
        );
    });
}

#[gpui_kit::test]
fn the_close_button_asks_about_unsaved_edits(cx: &mut TestAppContext) {
    let (shell, cx) = open_window(cx);
    described(cx, &shell, "/captures/a.iqw");
    select(cx, &shell, 0, 100);
    shell.update_in(cx, |shell, window, cx| shell.delete_selection(window, cx));
    cx.update(|window, cx| window.dispatch_action(Box::new(chrome::CloseWindow), cx));
    cx.run_until_parked();
    assert!(cx.has_pending_prompt(), "the window asks before closing");
    cx.simulate_prompt_answer("Cancel");
}

#[gpui_kit::test]
fn a_save_finished_after_more_edits_keeps_the_document_and_its_unsaved_state(
    cx: &mut TestAppContext,
) {
    let (shell, cx) = open_window(cx);
    described(cx, &shell, "/captures/a.iqw");
    select(cx, &shell, 0, 100);
    shell.update_in(cx, |shell, window, cx| shell.delete_selection(window, cx));
    let (document, version) = shell.read_with(cx, |shell, _| {
        let file = shell.file.as_ref().map_or(0, |file| file.id);
        (file, shell.editing().map_or(0, Editing::version))
    });
    select(cx, &shell, 0, 100);
    shell.update_in(cx, |shell, window, cx| shell.delete_selection(window, cx));
    shell.update_in(cx, |shell, window, cx| {
        shell.saved(
            PathBuf::from("/captures/saved.iqw"),
            saving_ui::SaveOf::whole_for_test(document, version),
            window,
            cx,
        );
    });
    cx.run_until_parked();
    let (path, dirty) = shell.read_with(cx, |shell, _| {
        (
            shell
                .file
                .as_ref()
                .map(|file| file.state.document.origin().path.clone()),
            shell.editing().is_some_and(Editing::is_dirty),
        )
    });
    assert_eq!(
        path,
        Some(PathBuf::from("/captures/a.iqw")),
        "the window stays on its capture"
    );
    assert!(dirty, "the later edit is still unsaved");
}

#[gpui_kit::test]
fn undo_reaches_a_capture_edited_down_to_nothing(cx: &mut TestAppContext) {
    let (shell, cx) = open_window(cx);
    described(cx, &shell, "/captures/a.iqw");
    select(cx, &shell, 0, 48_000);
    shell.update_in(cx, |shell, window, cx| {
        shell.delete_selection(window, cx);
        window.focus(&shell.focus, cx);
    });
    assert_eq!(length(cx, &shell), Some(0));
    cx.simulate_keystrokes(if cfg!(target_os = "macos") {
        "cmd-z"
    } else {
        "ctrl-z"
    });
    cx.run_until_parked();
    assert_eq!(length(cx, &shell), Some(48_000));
}
