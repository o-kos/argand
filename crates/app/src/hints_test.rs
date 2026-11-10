use super::*;
use gpui_kit::component::button::Button;
use gpui_kit::test::TestWindowExt;
use gpui_kit::{
    Bounds, Modifiers, ScrollDelta, ScrollEvent, StatefulInteractiveElement, Subscription,
    TestAppContext, TestSupportExt, VisualTestContext, point, px, size,
};

gpui_kit::actions!(hint_tests, [Nudge]);

/// The band at the window's edges a frame keeps for resizing, which the backdrop leaves uncovered.
const EDGE: f32 = 10.;

/// A plot filling the window, with a pinned hint's trigger at its bottom left.
struct Harness {
    hint: Entity<PinnedHint>,
    focus: FocusHandle,
    events: Vec<Pinned>,
    nudges: usize,
    presses: usize,
    wheels: usize,
    hovered: Option<bool>,
    _events: Subscription,
}

impl Render for Harness {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let hint = self.hint.clone();
        let trigger = Button::new("trigger")
            .tab_stop(false)
            .label("FFT")
            .on_hover(move |hovered, window, cx| {
                hint.update(cx, |hint, cx| hint.hover(*hovered, window, cx))
            });
        div()
            .size_full()
            .relative()
            .child(
                div()
                    .id("plot")
                    .absolute()
                    .inset_0()
                    .track_focus(&self.focus)
                    .key_context("Plot")
                    .on_action(cx.listener(|harness, _: &Nudge, _, _| harness.nudges += 1))
                    .on_hover(
                        cx.listener(|harness, hovered, _, _| harness.hovered = Some(*hovered)),
                    )
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|harness, _, _, _| harness.presses += 1),
                    )
                    .on_scroll(cx.listener(|harness, _, _, _| harness.wheels += 1)),
            )
            .child(div().absolute().left(px(20.)).top(px(260.)).child(pinned(
                "hint",
                &self.hint,
                trigger,
                Anchor::BottomLeft,
                cx,
            )))
            .children(backdrop(
                &self.hint,
                Bounds::new(
                    point(px(EDGE), px(EDGE)),
                    size(px(400. - 2. * EDGE), px(300. - 2. * EDGE)),
                ),
                cx,
            ))
    }
}

const TRIGGER: (f32, f32) = (30., 268.);
const OUTSIDE: (f32, f32) = (300., 60.);

fn at((x, y): (f32, f32)) -> gpui_kit::Point<gpui_kit::Pixels> {
    point(px(x), px(y))
}

fn frame(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|window, cx| window.render_frame(cx));
    cx.run_until_parked();
}

fn open_window(cx: &mut TestAppContext) -> (Entity<Harness>, &mut VisualTestContext) {
    cx.update(|cx| {
        gpui_kit::init(cx);
        init(cx);
        cx.bind_keys([KeyBinding::new("left", Nudge, Some("Plot"))]);
    });
    let (harness, cx) = cx.add_window_view(|window, cx| {
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        let hint =
            cx.new(|cx| PinnedHint::new(|_, cx| cx.new(|_| Tooltip::new("Analysis")).into(), cx));
        let events = cx.subscribe(&hint, |harness: &mut Harness, _, event, _| {
            harness.events.push(*event)
        });
        Harness {
            hint,
            focus,
            events: Vec::new(),
            nudges: 0,
            presses: 0,
            wheels: 0,
            hovered: None,
            _events: events,
        }
    });
    cx.simulate_resize(size(px(400.), px(300.)));
    frame(cx);
    (harness, cx)
}

/// Hovers the trigger long enough to open the hint, then moves away from it.
fn open_and_leave(cx: &mut TestAppContext) -> (Entity<Harness>, &mut VisualTestContext) {
    let (harness, cx) = open_window(cx);
    cx.simulate_mouse_move(at(TRIGGER), None, Modifiers::default());
    cx.executor().advance_clock(Duration::from_secs(1));
    frame(cx);
    cx.simulate_mouse_move(at(OUTSIDE), None, Modifiers::default());
    cx.executor().advance_clock(Duration::from_secs(1));
    frame(cx);
    (harness, cx)
}

fn read<T>(
    cx: &mut VisualTestContext,
    harness: &Entity<Harness>,
    f: impl FnOnce(&Harness, &App) -> T,
) -> T {
    harness.read_with(cx, |harness, cx| f(harness, cx))
}

fn is_open(cx: &mut VisualTestContext, harness: &Entity<Harness>) -> bool {
    read(cx, harness, |harness, cx| harness.hint.read(cx).is_open())
}

fn events(cx: &mut VisualTestContext, harness: &Entity<Harness>) -> Vec<Pinned> {
    read(cx, harness, |harness, _| harness.events.clone())
}

fn nudge(cx: &mut VisualTestContext, harness: &Entity<Harness>) -> usize {
    cx.simulate_keystrokes("left");
    read(cx, harness, |harness, _| harness.nudges)
}

#[gpui_kit::test]
fn a_short_hover_opens_nothing(cx: &mut TestAppContext) {
    let (harness, cx) = open_window(cx);
    cx.simulate_mouse_move(at(TRIGGER), None, Modifiers::default());
    cx.simulate_mouse_move(at(OUTSIDE), None, Modifiers::default());
    cx.executor().advance_clock(Duration::from_secs(1));
    frame(cx);
    assert!(!is_open(cx, &harness));
}

#[gpui_kit::test]
fn leaving_the_trigger_keeps_the_hint_open(cx: &mut TestAppContext) {
    let (harness, cx) = open_and_leave(cx);
    assert!(is_open(cx, &harness));
    assert_eq!(events(cx, &harness), [Pinned::Opened]);
}

#[gpui_kit::test]
fn an_open_hint_keeps_the_plot_from_input(cx: &mut TestAppContext) {
    let (harness, cx) = open_and_leave(cx);
    assert_eq!(read(cx, &harness, |h, _| h.hovered), Some(false));
    cx.simulate_event(ScrollEvent {
        position: at(OUTSIDE),
        delta: ScrollDelta::Pixels(point(px(0.), px(40.))),
        ..Default::default()
    });
    assert_eq!(nudge(cx, &harness), 0, "keys do not reach the plot");
    assert_eq!(read(cx, &harness, |h, _| h.wheels), 0, "nor does the wheel");
}

#[gpui_kit::test]
fn a_click_outside_closes_the_hint_and_goes_no_further(cx: &mut TestAppContext) {
    let (harness, cx) = open_and_leave(cx);
    cx.simulate_mouse_down(at(OUTSIDE), MouseButton::Left, Modifiers::default());
    cx.simulate_mouse_up(at(OUTSIDE), MouseButton::Left, Modifiers::default());
    frame(cx);
    assert!(!is_open(cx, &harness));
    assert_eq!(read(cx, &harness, |h, _| h.presses), 0);
    assert_eq!(
        events(cx, &harness),
        [Pinned::Opened, Pinned::Closed { revert: false }]
    );
    assert_eq!(nudge(cx, &harness), 1, "the keyboard is back");
}

#[gpui_kit::test]
fn the_backdrop_leaves_the_edges_beyond_its_area_to_what_lies_there(cx: &mut TestAppContext) {
    let (harness, cx) = open_and_leave(cx);
    let edge = point(px(EDGE / 2.), px(150.));
    cx.simulate_mouse_down(edge, MouseButton::Left, Modifiers::default());
    cx.simulate_mouse_up(edge, MouseButton::Left, Modifiers::default());
    frame(cx);
    assert_eq!(
        read(cx, &harness, |h, _| h.presses),
        1,
        "the press reaches the edge"
    );
    assert!(is_open(cx, &harness), "whose owner decides what it closes");
}

#[gpui_kit::test]
fn enter_closes_the_hint_and_keeps_the_values(cx: &mut TestAppContext) {
    let (harness, cx) = open_and_leave(cx);
    cx.simulate_keystrokes("enter");
    frame(cx);
    assert!(!is_open(cx, &harness));
    assert_eq!(
        events(cx, &harness),
        [Pinned::Opened, Pinned::Closed { revert: false }]
    );
    assert_eq!(nudge(cx, &harness), 1);
}

#[gpui_kit::test]
fn space_leaves_the_hint_open(cx: &mut TestAppContext) {
    let (harness, cx) = open_and_leave(cx);
    cx.simulate_keystrokes("space");
    frame(cx);
    assert!(is_open(cx, &harness));
    assert_eq!(events(cx, &harness), [Pinned::Opened]);
}

#[gpui_kit::test]
fn escape_closes_the_hint_and_reverts(cx: &mut TestAppContext) {
    let (harness, cx) = open_and_leave(cx);
    cx.simulate_keystrokes("escape");
    frame(cx);
    assert!(!is_open(cx, &harness));
    assert_eq!(
        events(cx, &harness),
        [Pinned::Opened, Pinned::Closed { revert: true }]
    );
    assert_eq!(nudge(cx, &harness), 1);
}

/// A zoom-half-sized button whose own hint opens partly over it.
#[derive(Default)]
struct Trigger {
    presses: usize,
}

impl Render for Trigger {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(
            div()
                .id("trigger")
                .absolute()
                .left(px(10.))
                .top(px(10.))
                .size(px(22.))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|trigger, _, _, _| trigger.presses += 1),
                )
                .tooltip(|_, cx| passive(cx, |_| observed_hint())),
        )
    }
}

fn observed_hint() -> Tooltip {
    Tooltip::element(|_, _| div().id("hint-text").test_support().child("A hint"))
}

#[gpui_kit::test]
fn a_passive_hint_leaves_its_trigger_the_click(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (trigger, cx) = cx.add_window_view(|_, _| Trigger::default());
    cx.simulate_resize(size(px(400.), px(300.)));
    let none = Modifiers::default();
    cx.simulate_mouse_move(point(px(11.), px(11.)), None, none);
    cx.executor().advance_clock(Duration::from_secs(1));
    frame(cx);
    // Still on the button, and inside the box its hint opens 13 pixels away.
    let overlap = point(px(28.), px(28.));
    cx.simulate_mouse_move(overlap, None, none);
    frame(cx);
    let text = cx
        .update(|window, _| window.try_find("hint-text"))
        .expect("the hint is shown");
    // The standard tooltip pads its text by 8 and 2 pixels inside a 1-pixel border.
    let frame = point(px(9.), px(3.));
    let hint = Bounds::from_corners(
        text.bounds().origin - frame,
        text.bounds().bottom_right() + frame,
    );
    assert!(text.visible() && hint.contains(&overlap), "{hint:?}");
    cx.simulate_mouse_down(overlap, MouseButton::Left, none);
    assert_eq!(trigger.read_with(cx, |trigger, _| trigger.presses), 1);
}
