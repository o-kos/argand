use super::*;
use gpui_kit::TestAppContext;
#[gpui_kit::test]
fn failed_source_remains_unavailable_after_capture_and_description_changes(
    cx: &mut TestAppContext,
) {
    use super::super::conversion_ui::tests::panel_window;
    use super::super::replace_ui::tests::{wait_until, write_wave};
    let (shell, cx) = panel_window(cx);
    let dir = argand_io::testutil::TempDir::new("statistics-failed");
    let path = dir.join("source.wav");
    write_wave(&path, 2400);
    shell.update_in(cx, |shell, window, cx| {
        shell.open(Origin::new(path), window, cx)
    });
    wait_until(cx, &shell, "source statistics", |shell| {
        shell.statistics.finished
    });
    shell.update_in(cx, |shell, window, cx| {
        let editing = shell.editing_mut().unwrap();
        editing.fail_statistics(argand_edit::SourceId(0));
        editing.delete(argand_core::SampleSpan::between(100, 200).unwrap());
        editing.describe_units(Some((32768., 0.)));
        shell.prepare_statistics(window, cx);
        assert!(shell.statistics.finished);
        assert!(shell.statistics.task.is_none());
        let summary = shell
            .file
            .as_ref()
            .unwrap()
            .state
            .document
            .file_summary()
            .unwrap();
        assert!(summary.hint.value.contains("Peak: Unavailable"));
        assert!(!summary.hint.value.contains("Calculating"));
    });
}
#[gpui_kit::test]
fn stale_capture_metrics_are_rejected_and_view_changes_keep_the_same_query(
    cx: &mut TestAppContext,
) {
    use super::super::conversion_ui::tests::panel_window;
    use super::super::replace_ui::tests::{wait_until, write_wave};
    let (shell, cx) = panel_window(cx);
    let dir = argand_io::testutil::TempDir::new("statistics-route");
    let path = dir.join("source.wav");
    write_wave(&path, 2400);
    shell.update_in(cx, |shell, window, cx| {
        shell.open(Origin::new(path), window, cx)
    });
    wait_until(cx, &shell, "statistics source", |shell| {
        shell
            .editing()
            .is_some_and(|editing| editing.is_described())
    });
    shell.update_in(cx, |shell, _, cx| {
        let before = shell.statistics_key().unwrap();
        shell.settings.overlap = 25;
        assert!(
            shell.statistics_key().as_ref() == Some(&before),
            "FFT controls are not amplitude-query generations"
        );
        shell
            .editing_mut()
            .unwrap()
            .delete(argand_core::SampleSpan::between(100, 200).unwrap());
        shell
            .file
            .as_mut()
            .unwrap()
            .state
            .document
            .set_statistics(None);
        shell.receive_statistics(
            before,
            Ok(crate::sample_statistics::Metrics {
                peak: 999.,
                rms: 999.,
            }),
            cx,
        );
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
                .value
                .contains("Peak: Calculating…")
        );
        let current = shell.statistics_key().unwrap();
        shell.receive_statistics(
            current,
            Ok(crate::sample_statistics::Metrics { peak: 5., rms: 3. }),
            cx,
        );
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
                .value
                .contains("RMS: 3")
        );
    });
}
#[gpui_kit::test]
fn same_scale_save_rebuilds_statistics_behind_a_reused_complete_envelope(cx: &mut TestAppContext) {
    use super::super::conversion_ui::tests::panel_window;
    use super::super::replace_ui::tests::{wait_for_save_chooser, wait_until, write_wave};
    let (shell, cx) = panel_window(cx);
    let dir = argand_io::testutil::TempDir::new("statistics-rebind");
    let source = dir.join("source.wav");
    let target = dir.join("saved.wav");
    write_wave(&source, 2400);
    shell.update_in(cx, |shell, window, cx| {
        shell.open(Origin::new(source), window, cx)
    });
    wait_until(cx, &shell, "source statistics", |shell| {
        shell.statistics.finished
            && shell
                .editing()
                .is_some_and(|editing| editing.statistics_request().is_some())
    });
    shell.update_in(cx, |shell, window, cx| {
        shell
            .editing_mut()
            .unwrap()
            .delete(argand_core::SampleSpan::between(100, 200).unwrap());
        shell.edited(window, cx);
        assert!(shell.editing().unwrap().minimap().unwrap().complete);
        shell.save_as(false, window, cx);
    });
    wait_for_save_chooser(cx);
    let chosen = target.clone();
    cx.simulate_new_path_selection(move |_| Some(chosen));
    wait_until(cx, &shell, "rebound source statistics", |shell| {
        shell.editing().is_some_and(|editing| {
            editing.file().meta.source == target
                && editing.is_described()
                && editing.statistics_request().is_some()
        }) && shell.statistics.finished
    });
    shell.read_with(cx, |shell, _| {
        assert_eq!(shell.editing().unwrap().len(), 2300);
        let summary = shell
            .file
            .as_ref()
            .unwrap()
            .state
            .document
            .file_summary()
            .unwrap();
        let peak: f64 = crate::numbers::parse(
            &summary
                .hint
                .rows
                .iter()
                .find(|(label, _)| label == "Peak")
                .unwrap()
                .1,
        )
        .unwrap();
        assert!((peak - 2399. * 2f64.sqrt()).abs() < 0.001);
    });
}
