use super::*;
use argand_core::{Domain, SampleFormat, SampleType, SignalMeta};
use argand_edit::{Capture, SourceId};
use gpui_kit::test::TestWindowExt;
use gpui_kit::{AppContext, TestAppContext};

fn request() -> Request {
    let meta = SignalMeta {
        sample_rate: 8000.0,
        center_freq: 100_000.0,
        sample_type: SampleType::new(Domain::Iq, SampleFormat::F32),
        len_samples: 1000,
        container: "wav",
        divisor: 1.0,
        source: "input.wav".into(),
    };
    Request {
        capture: Capture::whole(SourceId(0), 1000),
        sources: Vec::new(),
        input: meta.clone(),
        output: meta,
        target: "output.wav".into(),
        protected: Vec::new(),
        level: Level::Preserve,
        full_scale: 1.0,
        copy: None,
    }
}

#[gpui_kit::test]
fn changing_kind_keeps_both_explicit_frequencies_and_revalidates_the_band(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let mut editor = None;
    let window = cx.add_window(|window, cx| {
        let form = cx.new(|cx| Editor::new(request(), window, cx));
        editor = Some(form.clone());
        Root::new(form, window, cx)
    });
    cx.run_until_parked();
    let editor = editor.unwrap();
    window
        .update(cx, |_, window, cx| {
            let kind = editor.read(cx).kind.clone();
            kind.update(cx, |kind, cx| {
                kind.set_selected_value(&"rl_i16".to_string(), window, cx)
            });
            let converted = editor.read(cx).request(cx).unwrap();
            assert_eq!(converted.output.sample_rate, 8000.0);
            assert_eq!(converted.output.center_freq, 100_000.0);
            assert_eq!(converted.output.sample_type.to_string(), "rl_i16");
            assert!(
                editor
                    .read(cx)
                    .summary(cx)
                    .unwrap()
                    .contains("Whole capture")
            );
            let reference = editor.read(cx).reference.clone();
            reference.update(cx, |state, cx| state.set_value("200000", window, cx));
            assert!(editor.read(cx).request(cx).is_err());
            let rate = editor.read(cx).rate.clone();
            rate.update(cx, |state, cx| state.set_value("NaN", window, cx));
            assert!(editor.read(cx).request(cx).is_err());
        })
        .unwrap();
}

struct DialogHost;

impl Render for DialogHost {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div().children(Root::render_dialog_layer(window, cx))
    }
}

#[gpui_kit::test]
fn standard_dialog_inputs_lists_and_escape_keep_the_expected_focus(cx: &mut TestAppContext) {
    use std::sync::atomic::AtomicUsize;
    cx.update(gpui_kit::init);
    let mut editor = None;
    let handle = cx.add_window(|window, cx| {
        let form = cx.new(|cx| Editor::new(request(), window, cx));
        editor = Some(form);
        let host = cx.new(|_| DialogHost);
        Root::new(host, window, cx)
    });
    let editor = editor.unwrap();
    let cancelled = Arc::new(AtomicUsize::new(0));
    let events = cancelled.clone();
    cx.update_window(handle.into(), |_, window, cx| {
        let child = editor.clone();
        window.open_dialog(cx, move |dialog, _, _| {
            let events = events.clone();
            dialog
                .title("Convert capture")
                .child(child.clone())
                .on_cancel(move |_, _, _| {
                    events.fetch_add(1, Ordering::Relaxed);
                    true
                })
        });
        let focus = editor.read(cx).rate.read(cx).focus_handle(cx);
        window.focus(&focus, cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle.into(), |_, window, cx| window.render_frame(cx))
        .unwrap();
    cx.update_window(handle.into(), |_, window, cx| {
        assert!(
            editor
                .read(cx)
                .rate
                .read(cx)
                .focus_handle(cx)
                .is_focused(window)
        );
        window.press("secondary-a", cx);
        window.input("12000", cx);
        assert_eq!(
            editor.read(cx).request(cx).unwrap().output.sample_rate,
            12000.0
        );
        let focus = editor.read(cx).kind.read(cx).focus_handle(cx);
        window.focus(&focus, cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle.into(), |_, window, cx| window.render_frame(cx))
        .unwrap();
    cx.update_window(handle.into(), |_, window, cx| window.press("enter", cx))
        .unwrap();
    cx.run_until_parked();
    cx.update_window(handle.into(), |_, window, cx| window.render_frame(cx))
        .unwrap();
    cx.update_window(handle.into(), |_, window, cx| {
        window.press("escape", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle.into(), |_, window, cx| window.render_frame(cx))
        .unwrap();
    assert_eq!(
        cancelled.load(Ordering::Relaxed),
        0,
        "the list owns its first Escape"
    );
    cx.update_window(handle.into(), |_, window, cx| {
        assert!(
            editor
                .read(cx)
                .kind
                .read(cx)
                .focus_handle(cx)
                .is_focused(window)
        );
        window.press("escape", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle.into(), |_, window, cx| window.render_frame(cx))
        .unwrap();
    assert_eq!(
        cancelled.load(Ordering::Relaxed),
        1,
        "the dialog closes on the next Escape"
    );
}
