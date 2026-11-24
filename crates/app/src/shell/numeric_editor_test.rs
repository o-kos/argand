use super::*;
use crate::shell::replace_ui::tests::open_window;
use gpui_kit::TestAppContext;

#[gpui_kit::test]
fn frequency_drafts_keep_physical_values_units_bounds_and_steps(cx: &mut TestAppContext) {
    let _cleanup = crate::test_context::cleanup(cx);
    let (_, cx) = open_window(cx);
    cx.update(|window, cx| {
        let original = 12_582_550.25;
        let editor = cx.new(|cx| {
            NumericEditor::frequency(
                original,
                FrequencyUnit::MHz,
                Config {
                    unit: "Hz",
                    integer: false,
                    step: 1.,
                    minimum: 0.,
                    maximum: 20_000_000.,
                    minimum_inclusive: true,
                    down: "frequency-down",
                    up: "frequency-up",
                },
                window,
                cx,
            )
        });
        editor.update(cx, |editor, cx| {
            assert_eq!(editor.value_hz(cx), Some(original));
            editor.sync_frequency_unit(FrequencyUnit::KHz, window, cx);
            assert_eq!(editor.value_hz(cx), Some(original));
            assert_eq!(editor.input.read(cx).value().as_ref(), "12,582.55025");
            editor
                .input
                .update(cx, |input, cx| input.set_value("12.58255 MHz", window, cx));
            // A dirty draft retains the meaning of its explicit suffix across context changes.
            editor.sync_frequency_unit(FrequencyUnit::Hz, window, cx);
            assert_eq!(editor.value_hz(cx), Some(12_582_550.));
            editor.format(window, cx);
            assert_eq!(editor.value_hz(cx), Some(12_582_550.));
            assert!(editor.step(StepAction::Increment, window, cx));
            assert_eq!(editor.value_hz(cx), Some(12_582_551.));
            editor
                .input
                .update(cx, |input, cx| input.set_value("21 MHz", window, cx));
            assert_eq!(editor.value_hz(cx), None);
        });
    });
}
