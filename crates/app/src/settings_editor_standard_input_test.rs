use super::*;
use gpui_kit::component::input::NumberInput;
use gpui_kit::test::TestWindowExt;
use gpui_kit::{App, TestAppContext, WindowHandle};

type Handle = WindowHandle<Root>;

gpui_kit::actions!(standard_input_tests, [OuterEscape]);

/// An outer context whose escape stands for the surface around the controls.
const OUTER: &str = "StandardInputs";

/// A window's worth of standard controls, with nothing of Argand's own.
struct Form {
    number: Entity<InputState>,
    select: Entity<SelectState<Vec<String>>>,
    /// How often the outer context saw an escape.
    outer_escape: usize,
}

impl Form {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let number = cx.new(|cx| InputState::new(window, cx).default_value("50"));
        let select = cx.new(|cx| {
            SelectState::new(
                ["1024", "2048", "4096"]
                    .into_iter()
                    .map(String::from)
                    .collect::<Vec<String>>(),
                Some(gpui_kit::component::IndexPath::new(0)),
                window,
                cx,
            )
        });
        Self {
            number,
            select,
            outer_escape: 0,
        }
    }
}

impl Render for Form {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_4()
            .p_4()
            .key_context(OUTER)
            .on_action(cx.listener(|form, _: &OuterEscape, _, _| {
                form.outer_escape += 1;
            }))
            .child(NumberInput::new(&self.number))
            .child(Select::new(&self.select).id("select").w_full())
    }
}

/// Opens a window whose top-level entity is a standard `Root`, as the main window is.
fn open(cx: &mut TestAppContext) -> Handle {
    cx.update(|cx| {
        gpui_kit::init(cx);
        cx.bind_keys([KeyBinding::new("escape", OuterEscape, Some(OUTER))]);
    });
    let handle = cx.add_window(|window, cx| {
        let form = cx.new(|cx| Form::new(window, cx));
        Root::new(form, window, cx)
    });
    frame(cx, handle);
    handle
}

fn frame(cx: &mut TestAppContext, handle: Handle) {
    cx.update_window(handle.into(), |_, window, cx| window.render_frame(cx))
        .expect("the window is open");
    cx.run_until_parked();
}

/// Reaches the form through the window's `Root`, which is the only way to it.
fn with_form<R>(
    cx: &mut TestAppContext,
    handle: Handle,
    f: impl FnOnce(&Entity<Form>, &mut Window, &mut App) -> R,
) -> R {
    cx.update_window(handle.into(), |_, window, cx| {
        let form = Root::read(window, cx)
            .view()
            .clone()
            .downcast::<Form>()
            .expect("the window's Root owns the form");
        f(&form, window, cx)
    })
    .expect("the window is open")
}

/// Clicks the select trigger, which opens the standard list.
fn open_list(cx: &mut TestAppContext, handle: Handle) {
    cx.update_window(handle.into(), |_, window, cx| {
        window.within("select").click("input", cx);
    })
    .expect("the window is open");
    cx.run_until_parked();
}

/// The number input's text as the window holds it now.
fn number(cx: &mut TestAppContext, handle: Handle) -> String {
    with_form(cx, handle, |form, _, cx| {
        form.read(cx).number.read(cx).value().to_string()
    })
}

/// How many escapes reached the outer context instead of the select.
fn outer_escape(cx: &mut TestAppContext, handle: Handle) -> usize {
    with_form(cx, handle, |form, _, cx| form.read(cx).outer_escape)
}

/// Whether the window is still open and the keyboard is on this handle.
fn focused(cx: &mut TestAppContext, handle: Handle, target: &FocusHandle) -> bool {
    with_form(cx, handle, |_, window, _| target.is_focused(window))
}

/// The number input's own focus handle.
fn typed_handle(cx: &mut TestAppContext, handle: Handle) -> FocusHandle {
    with_form(cx, handle, |form, _, cx| {
        form.read(cx).number.focus_handle(cx)
    })
}

/// A shut select answers with its own handle, so this is the trigger's.
fn trigger_handle(cx: &mut TestAppContext, handle: Handle) -> FocusHandle {
    with_form(cx, handle, |form, _, cx| {
        form.read(cx).select.focus_handle(cx)
    })
}

/// Sends one key to whatever holds the keyboard.
fn press(cx: &mut TestAppContext, handle: Handle, key: &str) {
    cx.update_window(handle.into(), |_, window, cx| window.press(key, cx))
        .expect("the window is open");
    cx.run_until_parked();
}

#[gpui_kit::test]
fn standard_inputs_type_choose_and_dismiss_inside_a_root_window(cx: &mut TestAppContext) {
    let handle = open(cx);
    with_form(cx, handle, |form, window, cx| {
        form.update(cx, |form, cx| {
            form.number.read(cx).focus_handle(cx).focus(window, cx);
        });
    });
    let typed = typed_handle(cx, handle);
    let trigger = trigger_handle(cx, handle);
    cx.update_window(handle.into(), |_, window, cx| {
        window.press("secondary-a", cx);
        window.input("4096", cx);
    })
    .expect("the window is open");
    cx.run_until_parked();
    assert_eq!(
        number(cx, handle),
        "4096",
        "the standard input took the typed text"
    );

    open_list(cx, handle);
    assert!(
        !focused(cx, handle, &typed) && !focused(cx, handle, &trigger),
        "the open list took the keyboard from the input and the trigger"
    );
    press(cx, handle, "escape");
    assert!(
        focused(cx, handle, &trigger),
        "escape closed the list and left the keyboard on the select"
    );
    assert_eq!(
        number(cx, handle),
        "4096",
        "escape closed the list, not the window"
    );
    assert_eq!(
        outer_escape(cx, handle),
        0,
        "the list consumed its escape instead of letting it reach the outer context"
    );
    press(cx, handle, "escape");
    assert_eq!(
        outer_escape(cx, handle),
        1,
        "a second escape, with the list shut, reaches the outer context"
    );

    open_list(cx, handle);
    press(cx, handle, "down");
    press(cx, handle, "enter");
    assert_eq!(
        with_form(cx, handle, |form, _, cx| form
            .read(cx)
            .select
            .read(cx)
            .selected_value()
            .cloned()),
        Some(String::from("2048")),
        "the standard list confirmed its second entry"
    );
    assert!(
        focused(cx, handle, &trigger),
        "choosing returns the keyboard to the select"
    );
}
