use super::*;
use gpui_kit::Edges;

#[test]
fn partially_tiled_frame_keeps_only_free_edges_clear() {
    let area = overlay_bounds(
        size(px(1000.), px(700.)),
        Edges {
            top: px(0.),
            right: px(20.),
            bottom: px(20.),
            left: px(0.),
        },
        true,
    );
    assert_eq!(
        area.0,
        Bounds::new(point(px(0.), px(0.)), size(px(976.), px(676.)))
    );
}

#[test]
fn an_undersized_viewport_cannot_create_negative_overlay_sizes() {
    let area = overlay_bounds(size(px(10.), px(10.)), Edges::all(px(20.)), true);
    assert_eq!(area.0.size, size(px(0.), px(0.)));
}

struct Observed {
    presses: std::rc::Rc<std::cell::Cell<usize>>,
    popup: bool,
}
impl gpui_kit::Render for Observed {
    fn render(&mut self, _: &mut Window, _: &mut gpui_kit::Context<Self>) -> impl IntoElement {
        let presses = self.presses.clone();
        let surface = div().size_full().relative().when(self.popup, |surface| {
            surface.child(gpui_kit::deferred(
                div()
                    .absolute()
                    .left(px(0.))
                    .top(px(0.))
                    .w(px(40.))
                    .h(px(80.))
                    .occlude()
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation()),
            ))
        });
        observe_frame(
            surface,
            |_| Bounds::new(point(px(24.), px(24.)), size(px(352.), px(252.))),
            move |_, _| presses.set(presses.get() + 1),
        )
    }
}

#[gpui_kit::test]
fn frame_press_bubbles_but_a_popup_above_the_edge_keeps_its_input(
    cx: &mut gpui_kit::TestAppContext,
) {
    use gpui_kit::test::TestWindowExt;
    let presses = std::rc::Rc::new(std::cell::Cell::new(0));
    let counted = presses.clone();
    let (view, cx) = cx.add_window_view(|_, _| Observed {
        presses: counted,
        popup: false,
    });
    cx.simulate_resize(size(px(400.), px(300.)));
    cx.run_until_parked();
    cx.update(|window, cx| window.render_frame(cx));
    cx.simulate_click(point(px(10.), px(40.)), gpui_kit::Modifiers::default());
    assert_eq!(presses.get(), 1, "an uncovered edge press bubbles");
    cx.simulate_click(point(px(200.), px(150.)), gpui_kit::Modifiers::default());
    assert_eq!(presses.get(), 1, "content presses do not dismiss overlays");
    view.update(cx, |view, cx| {
        view.popup = true;
        cx.notify();
    });
    cx.run_until_parked();
    cx.update(|window, cx| window.render_frame(cx));
    cx.simulate_click(point(px(10.), px(40.)), gpui_kit::Modifiers::default());
    assert_eq!(
        presses.get(),
        1,
        "a blocking popup owns a press over the edge"
    );
}

struct MenuCaption {
    controls: gpui_kit::WindowControls,
    pressed: std::rc::Rc<std::cell::Cell<usize>>,
    underlying: std::rc::Rc<std::cell::Cell<usize>>,
}

impl gpui_kit::Render for MenuCaption {
    fn render(&mut self, window: &mut Window, _: &mut gpui_kit::Context<Self>) -> impl IntoElement {
        let underlying = self.underlying.clone();
        let supported = self.controls;
        let controls = [
            (supported.minimize, gpui_kit::WindowControlArea::Min),
            (supported.maximize, gpui_kit::WindowControlArea::Max),
            (true, gpui_kit::WindowControlArea::Close),
        ]
        .into_iter()
        .filter(|(shown, _)| *shown)
        .map(|(_, area)| {
            let pressed = self.pressed.clone();
            div()
                .w(gpui_kit::component::TITLE_BAR_HEIGHT)
                .h_full()
                .window_control_area(area)
                .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                    pressed.set(pressed.get() + 1);
                    cx.stop_propagation();
                })
        });
        div()
            .relative()
            .size_full()
            .child(
                div()
                    .absolute()
                    .size_full()
                    .on_mouse_down(MouseButton::Left, move |_, _, _| {
                        underlying.set(underlying.get() + 1)
                    }),
            )
            .child(
                div()
                    .absolute()
                    .right_0()
                    .top_0()
                    .h(gpui_kit::component::TITLE_BAR_HEIGHT)
                    .flex()
                    .children(controls),
            )
            .child(menu_backdrop(
                Bounds::new(point(px(0.), px(0.)), window.viewport_size()),
                caption_width("windows", supported),
            ))
    }
}

#[gpui_kit::test]
fn menu_backdrop_keeps_windows_caption_hitboxes_live_and_other_input_blocked(
    cx: &mut gpui_kit::TestAppContext,
) {
    use gpui_kit::test::TestWindowExt;
    let pressed = std::rc::Rc::new(std::cell::Cell::new(0));
    let underlying = std::rc::Rc::new(std::cell::Cell::new(0));
    let (view, cx) = cx.add_window_view(|_, _| MenuCaption {
        controls: gpui_kit::WindowControls::default(),
        pressed: pressed.clone(),
        underlying: underlying.clone(),
    });
    cx.simulate_resize(size(px(800.), px(600.)));
    for (minimize, maximize) in [(true, true), (false, true), (true, false), (false, false)] {
        view.update(cx, |view, cx| {
            view.controls.minimize = minimize;
            view.controls.maximize = maximize;
            cx.notify();
        });
        cx.run_until_parked();
        cx.update(|window, cx| window.render_frame(cx));
        let count = 1 + usize::from(minimize) + usize::from(maximize);
        let before = pressed.get();
        for index in 0..count {
            let width = gpui_kit::component::TITLE_BAR_HEIGHT;
            cx.simulate_click(
                point(px(800.) - width * index - width / 2., width / 2.),
                gpui_kit::Modifiers::default(),
            );
        }
        assert_eq!(
            pressed.get(),
            before + count,
            "each standard caption hitbox receives its first press"
        );
        let left = px(800.) - gpui_kit::component::TITLE_BAR_HEIGHT * count - px(1.);
        cx.simulate_click(point(left, px(14.)), gpui_kit::Modifiers::default());
        cx.simulate_click(point(px(400.), px(300.)), gpui_kit::Modifiers::default());
        assert_eq!(
            underlying.get(),
            0,
            "caption-adjacent drag and body stay occluded"
        );
    }
}
