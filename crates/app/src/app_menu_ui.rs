//! The application menu and toolbar; menu navigation itself has no toolkit types.

use super::{navigation_ui::*, *};
use crate::app_menu::{self, Effect, Item, Kind, Menu};
use gpui::{AnyElement, AsKeystroke, KeyDownEvent, ScrollHandle, deferred, img};
use gpui_component::button::ButtonCustomVariant;
use gpui_component::{Disableable, Icon, IconName};
use std::{cell::Cell, rc::Rc};

actions!(application_menu, [OpenApplicationMenu]);

#[derive(Clone)]
enum Command {
    Action(Rc<dyn Action>),
    Open(Origin),
}

fn command(label: &str, action: impl Action) -> Item<Command> {
    Item::command(label, Command::Action(Rc::new(action)))
}

pub(super) struct ApplicationMenu {
    model: Menu<Command>,
    focus: FocusHandle,
    scroll: Vec<ScrollHandle>,
}

pub(super) type Anchor = Rc<Cell<Bounds<Pixels>>>;

const ROW: f32 = 26.;
const SEPARATOR: f32 = 9.;
const PADDING: f32 = 5.;

fn row_height(item: &Item<Command>) -> f32 {
    if matches!(item.kind, Kind::Separator) {
        SEPARATOR
    } else {
        ROW
    }
}

impl Shell {
    fn application_items(&self) -> Vec<Item<Command>> {
        let recent = self
            .recent_entries()
            .into_iter()
            .map(|(label, origin)| Item::command(label, Command::Open(origin)))
            .collect();
        let file = Item::branch(
            "File",
            vec![
                command("Open file...", ChooseFile),
                Item::branch("Recent", recent),
            ],
        );
        let mut items = vec![file];
        if self.view.is_some() {
            items.push(Item::branch("View", self.application_view_items()));
        }
        items
    }

    fn application_view_items(&self) -> Vec<Item<Command>> {
        use crate::time_ruler::Mode;
        let ruler = self.session.time_ruler;
        vec![
            command("Show grid", ToggleGrid).checked(self.session.show_grid),
            command("Vertical orientation", ToggleOrientation)
                .checked(self.session.orientation.vertical()),
            Item::separator(),
            Item::branch(
                "Time scale format",
                vec![
                    command("Hours, minutes, seconds (hms)", ClockRuler)
                        .checked(ruler == Mode::Clock),
                    command("Seconds", SecondsRuler).checked(ruler == Mode::Seconds),
                    command("Sample numbers", SamplesRuler).checked(ruler == Mode::Samples),
                ],
            ),
            Item::branch(
                "Frequency",
                vec![
                    command("Zoom in", FrequencyZoomIn),
                    command("Zoom out", FrequencyZoomOut),
                    command("Fit frequency range", FitFrequency),
                    Item::separator(),
                    command("Higher frequency", PanUp),
                    command("Lower frequency", PanDown),
                    command("Five frequency divisions higher", PanFarUp),
                    command("Five frequency divisions lower", PanFarDown),
                ],
            ),
            Item::separator(),
            command("Zoom in", ZoomIn),
            command("Zoom out", ZoomOut),
            command("Fit capture", FitCapture),
            Item::separator(),
            command("Earlier in time", PanLeft),
            command("Later in time", PanRight),
            command("Five time divisions earlier", PanFarLeft),
            command("Five time divisions later", PanFarRight),
            command("Go to start", GoStart),
            command("Go to end", GoEnd),
        ]
    }

    pub(super) fn dismiss_application_menu(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.application_menu.take().is_some() {
            window.focus(&self.focus);
            self.title_drag_pending = false;
            cx.notify();
        }
    }

    pub(super) fn toggle_application_menu(
        &mut self,
        _: &OpenApplicationMenu,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.application_menu.is_some() {
            self.dismiss_application_menu(window, cx);
            return;
        }
        if let Some(menu) = self.open_menu.take() {
            let _ = menu.update(cx, |_, cx| cx.emit(gpui::DismissEvent));
        }
        self.recent_files.refresh(&self.session.recent);
        let focus = cx.focus_handle();
        window.focus(&focus);
        self.application_menu = Some(ApplicationMenu {
            model: Menu::new(self.application_items()),
            focus,
            scroll: Vec::new(),
        });
        self.title_drag_pending = false;
        self.pointer = None;
        cx.notify();
    }

    fn menu_effect(
        &mut self,
        effect: Effect<Command>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match effect {
            Effect::None => cx.notify(),
            Effect::Dismiss => self.dismiss_application_menu(window, cx),
            Effect::Activate(command) => {
                self.dismiss_application_menu(window, cx);
                match command {
                    Command::Action(action) => window.dispatch_action(action.boxed_clone(), cx),
                    Command::Open(origin) => self.open(origin, window, cx),
                }
            }
        }
    }

    fn application_menu_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(menu) = self.application_menu.as_mut() else {
            return;
        };
        let stroke = &event.keystroke;
        if stroke.modifiers.control || stroke.modifiers.platform || stroke.modifiers.alt {
            return;
        }
        let effect = match stroke.key.as_str() {
            "up" => {
                menu.model.step(false);
                Effect::None
            }
            "down" => {
                menu.model.step(true);
                Effect::None
            }
            "home" => {
                menu.model.edge(false);
                Effect::None
            }
            "end" => {
                menu.model.edge(true);
                Effect::None
            }
            "right" => menu.model.enter(false),
            "enter" | "space" => menu.model.enter(true),
            "left" | "escape" => menu.model.back(),
            "tab" | "f10" => Effect::Dismiss,
            _ => return,
        };
        let level = menu.model.depth() - 1;
        if let (Some(index), Some(scroll)) = (menu.model.selected(level), menu.scroll.get(level)) {
            scroll.scroll_to_item(index);
        }
        cx.stop_propagation();
        self.menu_effect(effect, window, cx);
    }

    pub(super) fn toolbar(&self, window: &Window, cx: &mut Context<Self>) -> impl IntoElement {
        let anchor = self.application_menu_anchor.clone();
        let mut app = Button::new("application-menu-button")
            .custom(toolbar_style(self.application_menu.is_some(), cx))
            .small()
            .w(app_button_width(window, cx))
            .px(px(6.))
            .h(px(26.))
            .child(img("argand/app.png").size(px(22.)))
            .child(div().text_color(cx.theme().foreground).child(TITLE))
            .on_click(cx.listener(|shell, _, window, cx| {
                shell.toggle_application_menu(&OpenApplicationMenu, window, cx)
            }))
            .child(
                canvas(move |bounds, _, _| anchor.set(bounds), |_, _, _, _| {})
                    .absolute()
                    .size_full(),
            );
        app.interactivity().tooltip(|window, cx| {
            shortcut_tooltip(
                "Application menu".to_owned(),
                Some(Box::new(OpenApplicationMenu)),
                "Shell",
                px(240.),
            )
            .build(window, cx)
        });
        div()
            .id("title-toolbar")
            .occlude()
            .flex()
            .items_center()
            .gap_1()
            .flex_shrink_0()
            .on_mouse_down(MouseButton::Left, |_, window, cx| {
                window.prevent_default();
                cx.stop_propagation();
            })
            .on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation())
            .on_double_click(|_, _, cx| cx.stop_propagation())
            .child(app)
            .child(div().w(px(1.)).h(px(16.)).mx_1().bg(cx.theme().border))
            .child(self.toolbar_button(
                "spectrogram-orientation",
                if self.session.orientation.vertical() {
                    "argand/vertical.svg"
                } else {
                    "argand/horizontal.svg"
                },
                format!(
                    "Switch to {} orientation",
                    self.session.orientation.toggled().label().to_lowercase()
                ),
                ToggleOrientation,
                false,
                cx,
            ))
            .child(
                self.toolbar_button(
                    "toggle-grid",
                    "argand/grid.svg",
                    if self.session.show_grid {
                        "Hide grid"
                    } else {
                        "Show grid"
                    }
                    .to_owned(),
                    ToggleGrid,
                    self.session.show_grid,
                    cx,
                ),
            )
    }

    fn toolbar_button(
        &self,
        id: &'static str,
        icon: &'static str,
        hint: String,
        action: impl Action,
        selected: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let tooltip_action = action.boxed_clone();
        let enabled = self.view.is_some();
        let hover_foreground = toolbar_accent(cx);
        let mut button = Button::new(id)
            .custom(toolbar_style(selected, cx))
            .small()
            .w(px(26.))
            .h(px(26.))
            .px_0()
            .child(
                gpui::svg()
                    .path(icon)
                    .size(px(20.))
                    .id((id, 0_usize))
                    .text_color(cx.theme().foreground.opacity(0.85))
                    .when(enabled, |glyph| {
                        glyph.group_hover(id, |style| style.text_color(hover_foreground))
                    })
                    .when(!enabled, |glyph| {
                        glyph.text_color(cx.theme().muted_foreground.opacity(0.5))
                    }),
            )
            .disabled(!enabled)
            .on_click(cx.listener(move |shell, _, window, cx| {
                window.focus(&shell.focus);
                window.dispatch_action(action.boxed_clone(), cx);
            }));
        button.interactivity().tooltip(move |window, cx| {
            shortcut_tooltip(
                hint.clone(),
                Some(tooltip_action.boxed_clone()),
                "Plot",
                px(360.),
            )
            .build(window, cx)
        });
        div().group(id).child(button)
    }

    pub(super) fn application_menu_overlay(
        &mut self,
        area: Bounds<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let menu = self.application_menu.as_mut()?;
        let depth = menu.model.depth();
        menu.scroll.resize_with(depth, ScrollHandle::new);
        let focus = menu.focus.clone();
        let bounds = self.application_menu_anchor.get();
        let mut parent = [
            (bounds.origin.x - area.origin.x).into(),
            (bounds.origin.y - area.origin.y).into(),
            bounds.size.width.into(),
            bounds.size.height.into(),
        ];
        let viewport = area.size;
        let viewport = [viewport.width.into(), viewport.height.into()];
        let mut panels = Vec::new();
        for level in 0..depth {
            let menu = self.application_menu.as_ref()?;
            let items = menu.model.items(level);
            let height = items.iter().map(row_height).sum::<f32>() + PADDING * 2.;
            let width = menu_width(items, &self.focus, window, cx);
            let rect = app_menu::place(parent, [width, height], viewport, level > 0);
            let selected = menu.model.selected(level);
            let offset = menu.scroll[level].offset().y;
            let row_top: f32 = items
                .iter()
                .take(selected.unwrap_or(0))
                .map(row_height)
                .sum();
            parent = [
                rect[0],
                rect[1] + PADDING + row_top + f32::from(offset),
                rect[2],
                ROW,
            ];
            let screen_rect = [
                rect[0] + f32::from(area.origin.x),
                rect[1] + f32::from(area.origin.y),
                rect[2],
                rect[3],
            ];
            panels.push(self.application_menu_panel(level, screen_rect, window, cx));
        }
        Some(
            deferred(
                div()
                    .id("application-menu-overlay")
                    .absolute()
                    .top_0()
                    .left_0()
                    .w(window.viewport_size().width)
                    .h(window.viewport_size().height)
                    .track_focus(&focus)
                    .key_context("ApplicationMenu")
                    .font_family(cx.theme().font_family.clone())
                    .occlude()
                    .on_key_down(cx.listener(Self::application_menu_key))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|shell, _, window, cx| {
                            shell.dismiss_application_menu(window, cx);
                            cx.stop_propagation();
                        }),
                    )
                    .on_mouse_down(
                        MouseButton::Right,
                        cx.listener(|shell, _, window, cx| {
                            shell.dismiss_application_menu(window, cx);
                            cx.stop_propagation();
                        }),
                    )
                    .on_mouse_move(|_, _, cx| cx.stop_propagation())
                    .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
                    .on_mouse_up(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .on_double_click(|_, _, cx| cx.stop_propagation())
                    .children(panels),
            )
            .with_priority(2)
            .into_any_element(),
        )
    }

    fn application_menu_panel(
        &self,
        level: usize,
        rect: [f32; 4],
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Some(menu) = &self.application_menu else {
            return div().into_any_element();
        };
        let [x, y, width, height] = rect;
        div()
            .id(("app-menu-panel", level))
            .occlude()
            .absolute()
            .left(px(x))
            .top(px(y))
            .w(px(width))
            .h(px(height))
            .bg(cx.theme().popover)
            .text_color(cx.theme().popover_foreground)
            .border_1()
            .border_color(cx.theme().border)
            .rounded_md()
            .shadow_lg()
            .text_sm()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation())
            .child(
                div()
                    .id(("app-menu-scroll", level))
                    .size_full()
                    .p(px(PADDING - 1.))
                    .overflow_y_scroll()
                    .track_scroll(&menu.scroll[level])
                    .children(
                        menu.model
                            .items(level)
                            .iter()
                            .enumerate()
                            .map(|(index, item)| {
                                self.application_menu_row(level, index, item, window, cx)
                            }),
                    ),
            )
            .into_any_element()
    }

    fn application_menu_row(
        &self,
        level: usize,
        index: usize,
        item: &Item<Command>,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        if matches!(item.kind, Kind::Separator) {
            return div()
                .h(px(SEPARATOR))
                .flex_shrink_0()
                .py(px(4.))
                .child(div().h(px(1.)).bg(cx.theme().border))
                .into_any_element();
        }
        let selected = self
            .application_menu
            .as_ref()
            .is_some_and(|menu| menu.model.selected(level) == Some(index));
        let action = match &item.kind {
            Kind::Command(Command::Action(action)) => Some(action),
            _ => None,
        };
        let shortcut = action
            .and_then(|action| Kbd::binding_for_action_in(action.as_ref(), &self.focus, window));
        div()
            .id(("app-menu-row", level * 100 + index))
            .h(px(ROW))
            .flex_shrink_0()
            .flex()
            .items_center()
            .gap_2()
            .px_2()
            .rounded_sm()
            .when(selected, |row| {
                row.bg(cx.theme().accent)
                    .text_color(cx.theme().accent_foreground)
            })
            .when(!item.enabled, |row| {
                row.text_color(cx.theme().muted_foreground)
            })
            .on_hover(cx.listener(move |shell, hovered, _, cx| {
                if !hovered {
                    return;
                }
                if let Some(menu) = &mut shell.application_menu {
                    menu.model.hover(level, index);
                    menu.scroll.truncate(level + 1);
                }
                cx.notify();
            }))
            .on_click(cx.listener(move |shell, _, window, cx| {
                let Some(menu) = &mut shell.application_menu else {
                    return;
                };
                menu.model.select(level, index);
                menu.scroll.truncate(level + 1);
                let effect = menu.model.enter(true);
                shell.menu_effect(effect, window, cx);
                cx.stop_propagation();
            }))
            .child(div().w(px(14.)).flex_shrink_0().when(item.checked, |slot| {
                slot.child(Icon::new(IconName::Check).size(px(14.)))
            }))
            .child(
                div()
                    .min_w_0()
                    .flex_1()
                    .overflow_hidden()
                    .text_ellipsis()
                    .child(item.label.clone()),
            )
            .when_some(shortcut, |row, shortcut| {
                row.child(shortcut.appearance(false))
            })
            .when(matches!(item.kind, Kind::Branch(_)), |row| {
                row.child(Icon::new(IconName::ChevronRight).size(px(14.)))
            })
            .into_any_element()
    }
}

fn menu_width(
    items: &[Item<Command>],
    focus: &FocusHandle,
    window: &Window,
    cx: &gpui::App,
) -> f32 {
    let style = gpui::TextStyle {
        font_family: cx.theme().font_family.clone(),
        ..Default::default()
    };
    let measure = |text: &str| {
        f32::from(
            window
                .text_system()
                .shape_line(
                    text.to_owned().into(),
                    window.rem_size() * 0.875,
                    &[style.to_run(text.len())],
                    None,
                )
                .width
                .ceil(),
        )
    };
    items
        .iter()
        .map(|item| {
            let shortcut = match &item.kind {
                Kind::Command(Command::Action(action)) => window
                    .highest_precedence_binding_for_action_in(action.as_ref(), focus)
                    .and_then(|binding| {
                        binding
                            .keystrokes()
                            .first()
                            .map(|key| Kbd::format(key.as_keystroke()))
                    })
                    .map_or(0., |text| measure(&text) + 20.),
                _ => 0.,
            };
            measure(&item.label) + shortcut + 64.
        })
        .fold(150., f32::max)
        .min(420.)
}

pub(super) fn toolbar_width(window: &Window, cx: &gpui::App) -> Pixels {
    // Three controls, separator, three gaps and the separator's two margins.
    app_button_width(window, cx) + px(26. * 2. + 1.) + window.rem_size() * 1.25
}

fn app_button_width(window: &Window, cx: &gpui::App) -> Pixels {
    let style = gpui::TextStyle {
        font_family: cx.theme().font_family.clone(),
        ..Default::default()
    };
    let label = window.text_system().shape_line(
        TITLE.into(),
        window.rem_size() * 0.875,
        &[style.to_run(TITLE.len())],
        None,
    );
    // Artwork, text gap, horizontal padding and border.
    label.width.ceil() + px(22. + 12. + 2.) + window.rem_size() * 0.25
}

fn toolbar_accent(cx: &gpui::App) -> gpui::Hsla {
    if cx.theme().is_dark() {
        cx.theme().blue_light
    } else {
        cx.theme().blue.darken(0.2)
    }
}

fn toolbar_style(selected: bool, cx: &gpui::App) -> ButtonCustomVariant {
    let accent = toolbar_accent(cx);
    ButtonCustomVariant::new(cx)
        .color(if selected {
            accent.opacity(0.18)
        } else {
            cx.theme().title_bar.darken(0.035)
        })
        .foreground(cx.theme().foreground.darken(0.12))
        .border(if selected {
            accent.opacity(0.55)
        } else {
            cx.theme().border
        })
        .hover(accent.opacity(0.32))
        .active(accent.opacity(0.44))
}
