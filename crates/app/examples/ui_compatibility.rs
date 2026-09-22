//! Runnable stock-control compatibility probe for Issue #126.

#[path = "ui_compatibility/model.rs"]
mod model;

use gpui_kit::component::{
    ActiveTheme as _, Disableable as _, Icon, IconName, IndexPath, Root, Selectable as _, Theme,
    ThemeMode, TitleBar, WindowExt as _,
    button::{Button, ButtonVariants as _},
    input::{InputEvent, InputState, NumberInput, NumberInputEvent, StepAction},
    label::Label,
    menu::{DropdownMenu as _, PopupMenuItem},
    popover::Popover,
    resizable::{ResizablePanelGroup, ResizableState, h_resizable, resizable_panel, v_resizable},
    select::{Select, SelectState},
    tooltip::Tooltip,
};
use gpui_kit::{
    Anchor, App, AppContext as _, Bounds, Context, CursorStyle, Entity, FocusHandle, Focusable,
    InteractiveElement as _, IntoElement, KeyBinding, MouseButton, MouseDownEvent, MouseMoveEvent,
    ParentElement as _, Pixels, Point, Render, StatefulInteractiveElement as _, Styled as _,
    Subscription, Window, WindowBounds, WindowDecorations, WindowOptions, actions, canvas, div,
    hsla, px, size,
};
use model::{NumberStep, NumberValidation, Orientation, PlotPoint, ProbeModel, ZoomDirection};

const APP_ID: &str = "io.github.o_kos.argand.ui-compatibility";
const PLOT_KEY_CONTEXT: &str = "CompatibilityPlot";

actions!(ui_compatibility, [PlotProbeKey]);

type ChoiceState = Entity<SelectState<Vec<String>>>;

struct Fixture {
    model: ProbeModel,
    number_input: Entity<InputState>,
    select_input: ChoiceState,
    dialog_number_input: Entity<InputState>,
    dialog_select_input: ChoiceState,
    horizontal_panels: Entity<ResizableState>,
    vertical_panels: Entity<ResizableState>,
    plot_focus: FocusHandle,
    plot_bounds: Option<Bounds<Pixels>>,
    popover_crash_probe: bool,
    _subscriptions: Vec<Subscription>,
}

impl Fixture {
    fn new(popover_crash_probe: bool, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let number_input = cx.new(|cx| InputState::new(window, cx).default_value("2048"));
        let select_input = cx.new(|cx| {
            SelectState::new(
                vec!["Peak (MAX)".into(), "Mean power".into()],
                Some(IndexPath::new(0)),
                window,
                cx,
            )
        });
        let dialog_number_input = cx.new(|cx| InputState::new(window, cx).default_value("2048"));
        let dialog_select_input = cx.new(|cx| {
            SelectState::new(
                vec!["Peak (MAX)".into(), "Mean power".into()],
                Some(IndexPath::new(0)),
                window,
                cx,
            )
        });
        let horizontal_panels = cx.new(|_| ResizableState::default());
        let vertical_panels = cx.new(|_| ResizableState::default());
        let plot_focus = cx.focus_handle().tab_stop(true);
        let number_focus = number_input.focus_handle(cx);
        let select_focus = select_input.focus_handle(cx);
        let dialog_number_focus = dialog_number_input.focus_handle(cx);
        let dialog_select_focus = dialog_select_input.focus_handle(cx);
        let subscriptions = vec![
            cx.observe(&number_input, |_, _, cx| cx.notify()),
            cx.observe(&select_input, |_, _, cx| cx.notify()),
            cx.observe(&dialog_number_input, |_, _, cx| cx.notify()),
            cx.observe(&dialog_select_input, |_, _, cx| cx.notify()),
            cx.observe(&horizontal_panels, |_, _, cx| cx.notify()),
            cx.observe(&vertical_panels, |_, _, cx| cx.notify()),
            cx.subscribe(&number_input, |fixture, input, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    fixture
                        .model
                        .validate_number(input.read(cx).value().as_ref());
                    cx.notify();
                }
            }),
            cx.subscribe_in(
                &number_input,
                window,
                |fixture, input, event: &NumberInputEvent, window, cx| {
                    fixture.handle_number_step(input, event, window, cx);
                },
            ),
            cx.subscribe_in(
                &dialog_number_input,
                window,
                |fixture, input, event: &NumberInputEvent, window, cx| {
                    fixture.handle_dialog_number_step(input, event, window, cx);
                },
            ),
            cx.on_focus(&plot_focus, window, |_, _, cx| cx.notify()),
            cx.on_blur(&plot_focus, window, |_, _, cx| cx.notify()),
            cx.on_focus(&number_focus, window, |_, _, cx| cx.notify()),
            cx.on_blur(&number_focus, window, |_, _, cx| cx.notify()),
            cx.on_focus(&select_focus, window, |_, _, cx| cx.notify()),
            cx.on_blur(&select_focus, window, |_, _, cx| cx.notify()),
            cx.on_focus(&dialog_number_focus, window, |_, _, cx| cx.notify()),
            cx.on_blur(&dialog_number_focus, window, |_, _, cx| cx.notify()),
            cx.on_focus(&dialog_select_focus, window, |_, _, cx| cx.notify()),
            cx.on_blur(&dialog_select_focus, window, |_, _, cx| cx.notify()),
        ];

        Self {
            model: ProbeModel::default(),
            number_input,
            select_input,
            dialog_number_input,
            dialog_select_input,
            horizontal_panels,
            vertical_panels,
            plot_focus,
            plot_bounds: None,
            popover_crash_probe,
            _subscriptions: subscriptions,
        }
    }

    fn toggle_theme(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let mode = if cx.theme().mode.is_dark() {
            ThemeMode::Light
        } else {
            ThemeMode::Dark
        };
        Theme::change(mode, Some(window), cx);
        cx.refresh_windows();
    }

    fn toggle_orientation(&mut self, cx: &mut Context<Self>) {
        self.model.toggle_orientation();
        cx.notify();
    }

    fn reset_counters(&mut self, cx: &mut Context<Self>) {
        self.model.reset_counters();
        cx.notify();
    }

    fn zoom(&mut self, direction: ZoomDirection, cx: &mut Context<Self>) {
        self.model.zoom(direction);
        cx.notify();
    }

    fn plot_point(&self, position: Point<Pixels>) -> Option<PlotPoint> {
        let bounds = self.plot_bounds?;
        if !bounds.contains(&position)
            || bounds.size.width <= px(0.)
            || bounds.size.height <= px(0.)
        {
            return None;
        }
        Some(PlotPoint::new(
            f32::from(position.x - bounds.left()) / f32::from(bounds.size.width),
            f32::from(position.y - bounds.top()) / f32::from(bounds.size.height),
        ))
    }

    fn pointer_moved(&mut self, event: &MouseMoveEvent, cx: &mut Context<Self>) {
        let Some(point) = self.plot_point(event.position) else {
            return;
        };
        self.model.move_pointer(point, event.dragging());
        cx.notify();
    }

    fn pointer_pressed(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(point) = self.plot_point(event.position) else {
            return;
        };
        self.plot_focus.focus(window, cx);
        self.model.press_pointer(point);
        cx.notify();
    }

    fn pointer_clicked(&mut self, position: Point<Pixels>, cx: &mut Context<Self>) {
        let Some(point) = self.plot_point(position) else {
            return;
        };
        self.model.click_pointer(point);
        cx.notify();
    }

    fn record_menu_action(&mut self, action: &'static str, cx: &mut Context<Self>) {
        self.model.record_menu_action(action);
        cx.notify();
    }

    fn record_button_action(&mut self, cx: &mut Context<Self>) {
        self.model.record_button_action();
        cx.notify();
    }

    fn record_plot_key_action(&mut self, cx: &mut Context<Self>) {
        self.model.record_plot_key_action();
        cx.notify();
    }

    fn set_passive_hint_hovered(&mut self, hovered: bool, cx: &mut Context<Self>) {
        self.model.set_passive_hint_hovered(hovered);
        cx.notify();
    }

    fn set_plot_hovered(&mut self, hovered: bool, cx: &mut Context<Self>) {
        self.model.set_plot_hovered(hovered);
        cx.notify();
    }

    fn handle_number_step(
        &mut self,
        input: &Entity<InputState>,
        event: &NumberInputEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let NumberInputEvent::Step(action) = event;
        let direction = match action {
            StepAction::Decrement => NumberStep::Decrement,
            StepAction::Increment => NumberStep::Increment,
        };
        let text = input.read(cx).value();
        if let Some(value) = self.model.step_number(text.as_ref(), direction) {
            input.update(cx, |input, cx| input.set_value(value, window, cx));
        }
        cx.notify();
    }

    fn handle_dialog_number_step(
        &mut self,
        input: &Entity<InputState>,
        event: &NumberInputEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let NumberInputEvent::Step(action) = event;
        let direction = match action {
            StepAction::Decrement => NumberStep::Decrement,
            StepAction::Increment => NumberStep::Increment,
        };
        self.model.record_dialog_number_step();
        let text = input.read(cx).value();
        if let Some(value) = model::step_number_value(text.as_ref(), direction) {
            input.update(cx, |input, cx| input.set_value(value, window, cx));
        }
        cx.notify();
    }

    fn record_resize_callback(&mut self, orientation: Orientation, cx: &mut Context<Self>) {
        self.model.record_resize_callback(orientation);
        cx.notify();
    }

    fn toolbar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let dark = cx.theme().mode.is_dark();
        let orientation = self.model.orientation;
        div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_2()
            .px_3()
            .py_2()
            .border_b_1()
            .border_color(cx.theme().border)
            .child(self.theme_button(dark, cx))
            .child(self.orientation_button(orientation, cx))
            .child(
                Button::new("open-dialog-editor")
                    .primary()
                    .icon(Icon::new(IconName::Settings2))
                    .label("Modal Dialog")
                    .tooltip("Open the supported standard Dialog composition probe")
                    .on_click(cx.listener(|fixture, _, window, cx| {
                        fixture.open_dialog_editor(window, cx);
                    })),
            )
            .child(self.popover_probe_control(cx))
            .child(self.nested_menu(cx))
            .child(
                Button::new("reset-counters")
                    .outline()
                    .icon(Icon::new(IconName::Redo2))
                    .label("Reset counters")
                    .tooltip("Reset event and callback counts; retained control state is unchanged")
                    .on_click(cx.listener(|fixture, _, _, cx| fixture.reset_counters(cx))),
            )
    }

    fn theme_button(&self, dark: bool, cx: &mut Context<Self>) -> Button {
        Button::new("theme-toggle")
            .icon(Icon::new(if dark { IconName::Sun } else { IconName::Moon }))
            .label(if dark { "Use light" } else { "Use dark" })
            .tooltip("Toggle the gpui-component theme")
            .on_click(cx.listener(|fixture, _, window, cx| fixture.toggle_theme(window, cx)))
    }

    fn orientation_button(&self, orientation: Orientation, cx: &mut Context<Self>) -> Button {
        Button::new("orientation-toggle")
            .icon(Icon::new(match orientation {
                Orientation::Horizontal => IconName::PanelBottom,
                Orientation::Vertical => IconName::PanelRight,
            }))
            .label(orientation.label())
            .tooltip("Toggle the resizable group orientation")
            .on_click(cx.listener(|fixture, _, _, cx| fixture.toggle_orientation(cx)))
    }

    fn popover_probe_control(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        if self.popover_crash_probe {
            return self.editor_popover(cx).into_any_element();
        }
        Button::new("disabled-popover-crash-probe")
            .icon(Icon::new(IconName::TriangleAlert))
            .label("Popover crash probe disabled")
            .tooltip("Relaunch with --popover-crash-probe to enable the known-crashing stock composition")
            .disabled(true)
            .into_any_element()
    }

    fn popover_probe_notice(&self, cx: &App) -> impl IntoElement {
        let (message, color) = if self.popover_crash_probe {
            (
                "DANGER: --popover-crash-probe is enabled. Opening Retained editor and then its Select can terminate the fixture in the locked nested-deferred path.",
                cx.theme().danger,
            )
        } else {
            (
                "Known stock Popover + Select crash is isolated by default. Relaunch with --popover-crash-probe only for the explicit failure reproduction; the modal Dialog is a comparison, not an approved UX replacement.",
                cx.theme().muted_foreground,
            )
        };
        div()
            .px_3()
            .py_2()
            .border_b_1()
            .border_color(cx.theme().border)
            .text_xs()
            .text_color(color)
            .child(message)
    }

    fn window_metrics(&self, window: &Window, cx: &App) -> impl IntoElement {
        div()
            .px_3()
            .py_2()
            .border_b_1()
            .border_color(cx.theme().border)
            .text_xs()
            .text_color(cx.theme().muted_foreground)
            .child(window_summary(window))
    }

    fn open_dialog_editor(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let number_input = self.dialog_number_input.clone();
        let select_input = self.dialog_select_input.clone();
        window.open_dialog(cx, move |dialog, window, cx| {
            let validation = model::validate_number(number_input.read(cx).value().as_ref());
            let focus_owner = control_focus_owner(&number_input, &select_input, window, cx);
            let confirmed_number = number_input.clone();
            dialog
                .button_props(gpui_kit::component::dialog::DialogButtonProps::default().show_cancel(true))
                .title("Standard modal Dialog comparison")
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_3()
                        .child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child(
                                    "Public Dialog composition only. Retained values survive close/reopen; focus restoration is left to WindowExt.",
                                ),
                        )
                        .child(control_row(
                            "Probe value",
                            NumberInput::new(&number_input).suffix("units"),
                            cx,
                        ))
                        .child(number_feedback(validation, cx))
                        .child(control_row("Reducer", Select::new(&select_input), cx))
                        .child(detail("Actual focus", focus_owner, cx)),
                )
                .on_ok(move |_, _, cx| {
                    model::validate_number(confirmed_number.read(cx).value().as_ref()).is_valid()
                })
        });
    }

    fn editor_popover(&self, cx: &mut Context<Self>) -> Popover {
        let number_input = self.number_input.clone();
        let select_input = self.select_input.clone();
        let number_validation = self.model.number_validation;
        Popover::new("retained-editor-popover")
            .anchor(Anchor::TopLeft)
            .track_focus(&number_input.focus_handle(cx))
            .trigger(
                Button::new("open-editor-popover")
                    .icon(Icon::new(IconName::TriangleAlert))
                    .label("Retained editor crash probe"),
            )
            .content(move |_, _, cx| {
                div()
                    .w(px(280.))
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(div().text_sm().child("Retained Popover controls"))
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child("Edit, close, reopen, and verify that both values remain."),
                    )
                    .child(control_row(
                        "Probe value",
                        NumberInput::new(&number_input).suffix("units"),
                        cx,
                    ))
                    .child(number_feedback(number_validation, cx))
                    .child(control_row("Reducer", Select::new(&select_input), cx))
            })
    }

    fn nested_menu(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let owner = cx.entity().downgrade();
        Button::new("nested-menu")
            .icon(Icon::new(IconName::Menu))
            .label("Nested menu")
            .dropdown_menu(move |menu, window, cx| {
                let owner = owner.clone();
                menu.label("Stock PopupMenu").submenu(
                    "Analysis",
                    window,
                    cx,
                    move |menu, window, cx| {
                        let palette_owner = owner.clone();
                        menu.item(menu_action("Peak (MAX)", owner.clone(), "Peak (MAX)"))
                            .item(menu_action("Mean power", owner.clone(), "Mean power"))
                            .submenu("Palette", window, cx, move |menu, _, _| {
                                menu.item(menu_action("Oceanic", palette_owner.clone(), "Oceanic"))
                                    .item(menu_action("Fire", palette_owner.clone(), "Fire"))
                            })
                    },
                )
            })
    }

    fn resizable_group(&self, window: &Window, cx: &mut Context<Self>) -> ResizablePanelGroup {
        let orientation = self.model.orientation;
        let state = match orientation {
            Orientation::Horizontal => &self.horizontal_panels,
            Orientation::Vertical => &self.vertical_panels,
        };
        let owner = cx.entity().downgrade();
        let group = match orientation {
            Orientation::Horizontal => h_resizable("horizontal-probe-panels"),
            Orientation::Vertical => v_resizable("vertical-probe-panels"),
        };
        group
            .with_state(state)
            .child(
                resizable_panel()
                    .size(px(520.))
                    .size_range(px(180.)..Pixels::MAX)
                    .child(self.plot_probe(window, cx)),
            )
            .child(
                resizable_panel()
                    .size_range(px(180.)..Pixels::MAX)
                    .child(self.probe_panel(window, cx)),
            )
            .on_resize(move |_, _, cx| {
                let _ = owner.update(cx, |fixture, cx| {
                    fixture.record_resize_callback(orientation, cx);
                });
            })
    }

    fn plot_probe(&self, window: &Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .relative()
            .size_full()
            .min_w_0()
            .min_h_0()
            .child(self.plot(window, cx))
            .child(self.passive_hint_target(cx))
    }

    fn plot(&self, window: &Window, cx: &mut Context<Self>) -> impl IntoElement {
        let known_bounds = self.plot_bounds;
        let owner = cx.entity().downgrade();
        let show_alt_guides =
            window.modifiers().alt && self.model.plot_hovered && self.model.pointer.is_some();
        let plot_focused = self.plot_focus.is_focused(window);
        div()
            .id("domain-plot")
            .key_context(PLOT_KEY_CONTEXT)
            .track_focus(&self.plot_focus)
            .relative()
            .size_full()
            .min_w_0()
            .min_h_0()
            .overflow_hidden()
            .cursor(CursorStyle::Crosshair)
            .bg(hsla(0.60, 0.55, 0.12, 1.0))
            .border_1()
            .border_color(if plot_focused {
                cx.theme().accent
            } else {
                cx.theme().border
            })
            .on_action(cx.listener(|fixture, _: &PlotProbeKey, _, cx| {
                fixture.record_plot_key_action(cx);
            }))
            .on_hover(cx.listener(|fixture, hovered, _, cx| {
                fixture.set_plot_hovered(*hovered, cx);
            }))
            .on_mouse_move(cx.listener(|fixture, event, _, cx| {
                fixture.pointer_moved(event, cx);
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|fixture, event, window, cx| {
                    fixture.pointer_pressed(event, window, cx);
                }),
            )
            .on_click(cx.listener(|fixture, event: &gpui_kit::ClickEvent, _, cx| {
                fixture.pointer_clicked(event.position(), cx);
            }))
            .on_scroll_wheel(cx.listener(|fixture, _, _, cx| {
                fixture.model.wheel();
                cx.notify();
            }))
            .child(plot_bounds_observer(known_bounds, owner))
            .children(plot_bands())
            .children(plot_grid(self.plot_bounds, cx))
            .children(self.selection_overlay(cx))
            .children(show_alt_guides.then(|| self.crosshair(cx)).flatten())
            .child(
                div()
                    .absolute()
                    .left_3()
                    .top_3()
                    .px_2()
                    .py_1()
                    .rounded_sm()
                    .bg(hsla(0.0, 0.0, 0.02, 0.76))
                    .text_color(gpui_kit::white())
                    .text_xs()
                    .child(if show_alt_guides {
                        self.model
                            .pointer
                            .map_or_else(|| "Alt guides unavailable".into(), PlotPoint::readout)
                    } else {
                        "Click to focus; hold Alt for guides; press P for the scoped plot action"
                            .into()
                    }),
            )
            .child(self.zoom_buttons(cx))
    }

    fn passive_hint_target(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("passive-hint-target")
            .absolute()
            .right_3()
            .top_3()
            .cursor(CursorStyle::Arrow)
            .on_hover(cx.listener(|fixture, hovered, _, cx| {
                fixture.set_passive_hint_hovered(*hovered, cx);
            }))
            .tooltip(|window, cx| {
                Tooltip::new(
                    "Stock Tooltip over the plot: compare cursor, Alt guides, readout, clicks, and wheel counts",
                )
                .build(window, cx)
            })
            .child(
                div()
                    .px_2()
                    .py_1()
                    .rounded_sm()
                    .border_1()
                    .border_color(cx.theme().border)
                    .bg(cx.theme().background)
                    .text_color(cx.theme().foreground)
                    .text_xs()
                    .child(Label::new("Passive hint target")),
            )
    }

    fn selection_overlay(&self, cx: &App) -> Option<impl IntoElement> {
        let bounds = self.plot_bounds?;
        let (start, end) = self.model.selection?;
        let left = start.time.min(end.time) * f32::from(bounds.size.width);
        let top = start.frequency.min(end.frequency) * f32::from(bounds.size.height);
        let width = (start.time - end.time).abs() * f32::from(bounds.size.width);
        let height = (start.frequency - end.frequency).abs() * f32::from(bounds.size.height);
        Some(
            div()
                .absolute()
                .left(px(left))
                .top(px(top))
                .w(px(width.max(1.0)))
                .h(px(height.max(1.0)))
                .border_1()
                .border_color(cx.theme().accent)
                .bg(cx.theme().accent.opacity(0.16)),
        )
    }

    fn crosshair(&self, cx: &App) -> Option<impl IntoElement> {
        let bounds = self.plot_bounds?;
        let pointer = self.model.pointer?;
        let x = px(pointer.time * f32::from(bounds.size.width));
        let y = px(pointer.frequency * f32::from(bounds.size.height));
        Some(
            div()
                .absolute()
                .inset_0()
                .child(
                    div()
                        .absolute()
                        .left(x)
                        .top_0()
                        .w(px(1.))
                        .h_full()
                        .bg(cx.theme().accent),
                )
                .child(
                    div()
                        .absolute()
                        .left_0()
                        .top(y)
                        .w_full()
                        .h(px(1.))
                        .bg(cx.theme().accent),
                ),
        )
    }

    fn zoom_buttons(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .absolute()
            .left_3()
            .bottom_3()
            .flex()
            .gap_1()
            .child(
                Button::new("zoom-in")
                    .icon(Icon::new(IconName::Plus))
                    .tooltip("Record a domain zoom-in request")
                    .on_click(cx.listener(|fixture, _, _, cx| {
                        fixture.zoom(ZoomDirection::In, cx);
                    })),
            )
            .child(
                Button::new("zoom-out")
                    .icon(Icon::new(IconName::Minus))
                    .tooltip("Record a domain zoom-out request")
                    .on_click(cx.listener(|fixture, _, _, cx| {
                        fixture.zoom(ZoomDirection::Out, cx);
                    })),
            )
    }

    fn probe_panel(&self, window: &Window, cx: &mut Context<Self>) -> impl IntoElement {
        let counters = self.model.counters;
        let orientation = self.model.orientation;
        div()
            .id("probe-panel")
            .size_full()
            .min_w_0()
            .min_h_0()
            .overflow_y_scroll()
            .p_4()
            .flex()
            .flex_col()
            .gap_4()
            .bg(cx.theme().background)
            .child(section_title("Observable fixture state", cx))
            .child(self.pointer_counters(counters, window, cx))
            .child(self.resize_observations(orientation, cx))
            .child(self.retained_values(cx))
            .child(self.button_states(cx))
    }

    fn pointer_counters(
        &self,
        counters: model::Counters,
        window: &Window,
        cx: &App,
    ) -> impl IntoElement {
        let alt_guides =
            window.modifiers().alt && self.model.plot_hovered && self.model.pointer.is_some();
        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(detail("Pointer moves", counters.pointer_moves, cx))
            .child(detail("Pointer presses", counters.pointer_presses, cx))
            .child(detail("Clicks", counters.clicks, cx))
            .child(detail("Wheel events", counters.wheel_events, cx))
            .child(detail(
                "Zoom in / out",
                format!("{} / {}", counters.zoom_in, counters.zoom_out),
                cx,
            ))
            .child(detail("Zoom steps", self.model.zoom_steps, cx))
            .child(detail("Menu actions", counters.menu_actions, cx))
            .child(detail("Button actions", counters.button_actions, cx))
            .child(detail("Plot P actions", counters.plot_key_actions, cx))
            .child(detail(
                "Tracked focus owner",
                self.focus_owner(window, cx),
                cx,
            ))
            .child(detail(
                "Plot hover target",
                if self.model.plot_hovered {
                    "hovered"
                } else {
                    "not hovered"
                },
                cx,
            ))
            .child(detail(
                "Passive hint target",
                if self.model.passive_hint_hovered {
                    "hovered"
                } else {
                    "not hovered"
                },
                cx,
            ))
            .child(detail(
                "Alt guides/readout",
                if alt_guides { "visible" } else { "hidden" },
                cx,
            ))
            .child(detail(
                "Last menu action",
                self.model.last_menu_action.unwrap_or("none"),
                cx,
            ))
    }

    fn focus_owner(&self, window: &Window, cx: &App) -> &'static str {
        if self.plot_focus.is_focused(window) {
            return "plot";
        }
        if self
            .dialog_number_input
            .focus_handle(cx)
            .contains_focused(window, cx)
        {
            return "dialog number input";
        }
        if self
            .dialog_select_input
            .focus_handle(cx)
            .contains_focused(window, cx)
        {
            return "dialog select focus scope";
        }
        if self
            .number_input
            .focus_handle(cx)
            .contains_focused(window, cx)
        {
            return "number input";
        }
        if self
            .select_input
            .focus_handle(cx)
            .contains_focused(window, cx)
        {
            return "select focus scope";
        }
        if window.focused(cx).is_some() {
            "other / untracked"
        } else {
            "none"
        }
    }

    fn resize_observations(&self, orientation: Orientation, cx: &App) -> impl IntoElement {
        let active = match orientation {
            Orientation::Horizontal => &self.horizontal_panels,
            Orientation::Vertical => &self.vertical_panels,
        };
        div()
            .flex()
            .flex_col()
            .gap_1()
            .pt_2()
            .border_t_1()
            .border_color(cx.theme().border)
            .child(section_title("Resizable observations", cx))
            .child(detail("Active orientation", orientation.label(), cx))
            .child(detail("Live panel dimensions", panel_sizes(active, cx), cx))
            .child(detail(
                "Horizontal callbacks",
                self.model.resize_callbacks(Orientation::Horizontal),
                cx,
            ))
            .child(detail(
                "Vertical callbacks",
                self.model.resize_callbacks(Orientation::Vertical),
                cx,
            ))
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child("Dimensions are read from ResizableState; callbacks count only stock on_resize calls."),
            )
    }

    fn retained_values(&self, cx: &App) -> impl IntoElement {
        let number = self.number_input.read(cx).value();
        let selected = self
            .select_input
            .read(cx)
            .selected_value()
            .map_or_else(|| "none".into(), Clone::clone);
        let dialog_number = self.dialog_number_input.read(cx).value();
        let dialog_validation = model::validate_number(dialog_number.as_ref());
        let dialog_selected = self
            .dialog_select_input
            .read(cx)
            .selected_value()
            .map_or_else(|| "none".into(), Clone::clone);
        div()
            .flex()
            .flex_col()
            .gap_1()
            .pt_2()
            .border_t_1()
            .border_color(cx.theme().border)
            .child(section_title("Retained editor state", cx))
            .child(detail("Probe number text", number, cx))
            .child(detail(
                "Number validation",
                self.model.number_validation.message(),
                cx,
            ))
            .child(detail(
                "Number step events",
                self.model.counters.number_step_events,
                cx,
            ))
            .child(detail("Reducer", selected, cx))
            .child(detail("Dialog number text", dialog_number, cx))
            .child(detail("Dialog validation", dialog_validation.message(), cx))
            .child(detail(
                "Dialog step events",
                self.model.counters.dialog_number_step_events,
                cx,
            ))
            .child(detail("Dialog reducer", dialog_selected, cx))
    }

    fn button_states(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_2()
            .pt_2()
            .border_t_1()
            .border_color(cx.theme().border)
            .child(section_title("Standard Button states", cx))
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .child(
                        Button::new("state-default")
                            .label("Default")
                            .on_click(cx.listener(|fixture, _, _, cx| {
                                fixture.record_button_action(cx);
                            })),
                    )
                    .child(
                        Button::new("state-primary")
                            .primary()
                            .label("Primary")
                            .on_click(cx.listener(|fixture, _, _, cx| {
                                fixture.record_button_action(cx);
                            })),
                    )
                    .child(
                        Button::new("state-selected")
                            .label("Selected")
                            .selected(true),
                    )
                    .child(
                        Button::new("state-disabled")
                            .label("Disabled")
                            .disabled(true),
                    ),
            )
    }
}

impl Render for Fixture {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .on_modifiers_changed(cx.listener(|_, _, _, cx| cx.notify()))
            .child(
                TitleBar::new().child(
                    div()
                        .flex()
                        .items_center()
                        .h_full()
                        .text_sm()
                        .child("Argand UI compatibility fixture"),
                ),
            )
            .child(self.toolbar(cx))
            .child(self.popover_probe_notice(cx))
            .child(self.window_metrics(window, cx))
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .child(self.resizable_group(window, cx)),
            )
            .children(Root::render_dialog_layer(window, cx))
    }
}

fn control_focus_owner(
    number_input: &Entity<InputState>,
    select_input: &ChoiceState,
    window: &Window,
    cx: &App,
) -> &'static str {
    if number_input.focus_handle(cx).contains_focused(window, cx) {
        return "number input";
    }
    if select_input.focus_handle(cx).contains_focused(window, cx) {
        return "select focus scope";
    }
    if window.focused(cx).is_some() {
        "dialog container / other"
    } else {
        "none"
    }
}

fn control_row(label: &'static str, control: impl IntoElement, cx: &App) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap_3()
        .child(
            div()
                .w(px(72.))
                .flex_shrink_0()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child(label),
        )
        .child(div().flex_1().min_w_0().child(control))
}

fn number_feedback(validation: NumberValidation, cx: &App) -> impl IntoElement {
    div()
        .ml(px(84.))
        .text_xs()
        .text_color(if validation.is_valid() {
            cx.theme().muted_foreground
        } else {
            cx.theme().danger
        })
        .child(validation.message())
}

fn menu_action(
    label: &'static str,
    owner: gpui_kit::WeakEntity<Fixture>,
    action: &'static str,
) -> PopupMenuItem {
    PopupMenuItem::new(label).on_click(move |_, _, cx| {
        let _ = owner.update(cx, |fixture, cx| {
            fixture.record_menu_action(action, cx);
        });
    })
}

fn plot_bounds_observer(
    known_bounds: Option<Bounds<Pixels>>,
    owner: gpui_kit::WeakEntity<Fixture>,
) -> impl IntoElement {
    canvas(
        move |bounds, _, cx| {
            if known_bounds != Some(bounds) {
                let owner = owner.clone();
                cx.defer(move |cx| {
                    let _ = owner.update(cx, |fixture, cx| {
                        fixture.plot_bounds = Some(bounds);
                        cx.notify();
                    });
                });
            }
        },
        |_, _, _, _| {},
    )
    .absolute()
    .size_full()
}

fn plot_bands() -> impl Iterator<Item = impl IntoElement> {
    (0..12).map(|index| {
        let hue = 0.54 + index as f32 * 0.012;
        div()
            .absolute()
            .left_0()
            .top(gpui_kit::relative(index as f32 / 12.0))
            .w_full()
            .h(gpui_kit::relative(1.0 / 12.0))
            .bg(hsla(hue, 0.68, 0.12 + index as f32 * 0.012, 1.0))
    })
}

fn plot_grid(bounds: Option<Bounds<Pixels>>, cx: &App) -> Vec<gpui_kit::AnyElement> {
    let Some(bounds) = bounds else {
        return Vec::new();
    };
    let mut lines = Vec::with_capacity(10);
    for index in 1..6 {
        let x = bounds.size.width * (index as f32 / 6.0);
        lines.push(
            div()
                .absolute()
                .left(x)
                .top_0()
                .w(px(1.))
                .h_full()
                .bg(cx.theme().border.opacity(0.65))
                .into_any_element(),
        );
    }
    for index in 1..5 {
        let y = bounds.size.height * (index as f32 / 5.0);
        lines.push(
            div()
                .absolute()
                .left_0()
                .top(y)
                .w_full()
                .h(px(1.))
                .bg(cx.theme().border.opacity(0.65))
                .into_any_element(),
        );
    }
    lines
}

fn section_title(title: &'static str, cx: &App) -> impl IntoElement {
    div()
        .text_sm()
        .font_weight(gpui_kit::FontWeight::SEMIBOLD)
        .text_color(cx.theme().foreground)
        .child(title)
}

fn detail(label: &'static str, value: impl ToString, cx: &App) -> impl IntoElement {
    div()
        .flex()
        .items_start()
        .gap_3()
        .text_xs()
        .child(
            div()
                .w(px(132.))
                .flex_shrink_0()
                .text_color(cx.theme().muted_foreground)
                .child(label),
        )
        .child(div().min_w_0().child(value.to_string()))
}

fn panel_sizes(state: &Entity<ResizableState>, cx: &App) -> String {
    let sizes = state.read(cx).sizes();
    if sizes.is_empty() {
        return "not laid out".into();
    }
    sizes
        .iter()
        .map(|value| format!("{:.0} px", f32::from(*value)))
        .collect::<Vec<_>>()
        .join(" / ")
}

fn window_summary(window: &Window) -> String {
    let reported = window.window_bounds();
    let reported_bounds = reported.get_bounds();
    let viewport = window.viewport_size();
    let reported_variant = match reported {
        WindowBounds::Windowed(_) => "Windowed",
        WindowBounds::Maximized(_) => "Maximized",
        WindowBounds::Fullscreen(_) => "Fullscreen",
    };
    let state = if window.is_fullscreen() || matches!(reported, WindowBounds::Fullscreen(_)) {
        "fullscreen"
    } else if window.is_maximized() || matches!(reported, WindowBounds::Maximized(_)) {
        "maximized"
    } else {
        "windowed"
    };
    format!(
        "{state} | viewport {:.0}×{:.0} | reported {reported_variant} bounds {:.0}×{:.0} at {:.0},{:.0}",
        f32::from(viewport.width),
        f32::from(viewport.height),
        f32::from(reported_bounds.size.width),
        f32::from(reported_bounds.size.height),
        f32::from(reported_bounds.origin.x),
        f32::from(reported_bounds.origin.y),
    )
}

fn main() {
    let popover_crash_probe = std::env::args().any(|argument| argument == "--popover-crash-probe");
    gpui_kit::application()
        .with_assets(gpui_kit::assets::Assets)
        .run(move |cx| {
            gpui_kit::init(cx);
            cx.bind_keys([KeyBinding::new("p", PlotProbeKey, Some(PLOT_KEY_CONTEXT))]);
            Theme::change(ThemeMode::Light, None, cx);
            let options = window_options(cx);
            // The borderless window is offset a little so both compositions
            // are on screen together: the stock Root beside the borderless
            // one is the before/after the #137 checkpoint asks for.
            let mut borderless_options = window_options(cx);
            if let Some(WindowBounds::Windowed(ref mut bounds)) = borderless_options.window_bounds {
                bounds.origin.x += px(48.);
                bounds.origin.y += px(48.);
            }
            cx.spawn(async move |cx| {
                cx.open_window(options, |window, cx| {
                    window.set_window_title("Argand UI compatibility fixture");
                    let fixture = cx.new(|cx| Fixture::new(popover_crash_probe, window, cx));
                    cx.new(|cx| Root::new(fixture, window, cx))
                })?;
                cx.open_window(borderless_options, |window, cx| {
                    window.set_window_title("Argand fixture: borderless Root");
                    let fixture = cx.new(|cx| Fixture::new(popover_crash_probe, window, cx));
                    cx.new(|cx| Root::new(fixture, window, cx).bordered(false))
                })?;
                Ok::<_, anyhow::Error>(())
            })
            .detach();
        });
}

fn window_options(cx: &App) -> WindowOptions {
    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
            None,
            size(px(1120.), px(760.)),
            cx,
        ))),
        window_min_size: Some(size(px(720.), px(520.))),
        window_decorations: Some(WindowDecorations::Client),
        titlebar: Some(TitleBar::title_bar_options()),
        app_id: Some(APP_ID.into()),
        ..Default::default()
    }
}
