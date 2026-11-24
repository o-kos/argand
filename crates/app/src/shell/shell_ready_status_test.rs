use super::*;
use argand_core::{Domain, SampleFormat, SampleType, SignalMeta};
use gpui_kit::test::TestWindowExt;
use gpui_kit::{TestAppContext, VisualTestContext};
use std::time::Duration;

fn open(cx: &mut TestAppContext) -> (Entity<Shell>, &mut VisualTestContext) {
    cx.update(|cx| {
        gpui_kit::init(cx);
        settings_ui::init(cx);
        plot::navigation_ui::init(cx);
        hints::init(cx);
        app_menu_ui::init(cx);
        window_keys(cx);
    });
    let (root, cx) = cx.add_window_view(|window, cx| {
        let shell =
            cx.new(|cx| Shell::new(Config::default(), None, Session::default(), window, cx));
        Root::new(shell, window, cx)
    });
    let shell = cx.update(|_, cx| root.read(cx).view().clone().downcast::<Shell>().unwrap());
    describe(&shell, cx, "/captures/first.iqw");
    (shell, cx)
}

fn describe(shell: &Entity<Shell>, cx: &mut VisualTestContext, path: &str) {
    shell.update_in(cx, |shell, window, cx| {
        shell.open_test_document(Origin::new(path.into()), window, cx);
        shell
            .file
            .as_mut()
            .unwrap()
            .state
            .document
            .apply(crate::analysis::Update::Opened(
                SignalMeta {
                    sample_rate: 8000.,
                    center_freq: 0.,
                    sample_type: SampleType::new(Domain::Iq, SampleFormat::I16),
                    len_samples: 8000,
                    container: "wav",
                    divisor: 32768.,
                    source: path.into(),
                },
                crate::analysis::FileInfo::default(),
            ));
        shell.view = Some(crate::navigation::View::full(8000));
        shell.attach_plot(window, cx);
        cx.notify();
    });
    frame(cx);
}

fn frame(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|window, cx| window.render_frame(cx));
    cx.run_until_parked();
}

fn ready(shell: &Entity<Shell>, cx: &mut VisualTestContext) {
    shell.update_in(cx, |shell, window, cx| {
        shell.receive(
            crate::analysis::for_test(
                crate::analysis::Update::Ready {
                    analysis: settings_ui::warned_analysis(),
                    elapsed: Duration::from_millis(1250),
                },
                None,
                None,
            ),
            window,
            cx,
        );
    });
    frame(cx);
}

fn advance(cx: &mut VisualTestContext, seconds: u64) {
    cx.executor().advance_clock(Duration::from_secs(seconds));
    frame(cx);
}

fn visible(cx: &mut VisualTestContext) -> bool {
    cx.update(|window, _| window.try_find("analysis-status").is_some())
}

#[gpui_kit::test]
fn ready_status_expires_without_input_and_preserves_saved_notice(cx: &mut TestAppContext) {
    let _cleanup = crate::test_context::cleanup(cx);
    let (shell, cx) = open(cx);
    ready(&shell, cx);
    shell.update(cx, |shell, cx| {
        shell.save_notice = Some(saving_ui::Notice::Saved("capture.iqw".into()));
        cx.notify();
    });
    advance(cx, 4);
    assert!(visible(cx));
    advance(cx, 1);
    assert!(!visible(cx));
    assert!(shell.read_with(cx, |shell, _| matches!(
        shell.save_notice,
        Some(saving_ui::Notice::Saved(_))
    )));
    assert!(shell.read_with(cx, |shell, _| matches!(shell.file.as_ref().unwrap().state.document.status(), Status::Ready { elapsed } if *elapsed == Duration::from_millis(1250))));
}

#[gpui_kit::test]
fn a_new_ready_or_document_does_not_inherit_an_older_timeout(cx: &mut TestAppContext) {
    let _cleanup = crate::test_context::cleanup(cx);
    let (shell, cx) = open(cx);
    ready(&shell, cx);
    advance(cx, 4);
    ready(&shell, cx);
    advance(cx, 1);
    assert!(
        visible(cx),
        "old completion deadline cannot hide newer completion"
    );
    advance(cx, 3);
    describe(&shell, cx, "/captures/second.iqw");
    ready(&shell, cx);
    advance(cx, 1);
    assert!(
        visible(cx),
        "old document deadline cannot hide new document"
    );
    advance(cx, 4);
    assert!(!visible(cx));
}

#[gpui_kit::test]
fn shortcut_completion_expires_with_stationary_pointer_but_progress_does_not(
    cx: &mut TestAppContext,
) {
    let _cleanup = crate::test_context::cleanup(cx);
    let (shell, cx) = open(cx);
    ready(&shell, cx);
    let before = shell.read_with(cx, |shell, _| shell.view.unwrap());
    cx.simulate_keystrokes("ctrl-+");
    frame(cx);
    assert!(
        !visible(cx),
        "shortcut dismisses existing completion immediately"
    );
    assert_ne!(shell.read_with(cx, |shell, _| shell.view.unwrap()), before);
    shell.update_in(cx, |shell, window, cx| {
        shell.receive(
            crate::analysis::for_test(
                crate::analysis::Update::Progress { done: 1, total: 10 },
                None,
                None,
            ),
            window,
            cx,
        );
    });
    advance(cx, 10);
    assert!(visible(cx), "active analysis has no timeout");
    ready(&shell, cx);
    assert!(
        visible(cx),
        "accepted completion reappears without pointer motion"
    );
    advance(cx, 5);
    assert!(!visible(cx));
}

#[gpui_kit::test]
fn rejected_completion_does_not_postpone_ready_expiry(cx: &mut TestAppContext) {
    let _cleanup = crate::test_context::cleanup(cx);
    let (shell, cx) = open(cx);
    ready(&shell, cx);
    advance(cx, 4);
    shell.update_in(cx, |shell, window, cx| {
        let generation = shell
            .file
            .as_ref()
            .unwrap()
            .state
            .analyst
            .generation_for_test();
        let stale = crate::analysis::for_test(
            crate::analysis::Update::Ready {
                analysis: settings_ui::warned_analysis(),
                elapsed: Duration::from_secs(99),
            },
            Some(generation.wrapping_add(1)),
            None,
        );
        assert!(!shell.file.as_ref().unwrap().state.analyst.accepts(&stale));
        shell.receive(stale, window, cx);
    });
    advance(cx, 1);
    assert!(
        !visible(cx),
        "rejected delivery cannot restart the deadline"
    );
}
