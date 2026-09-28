//! The application menu and toolbar; the menu itself is the toolkit's own.

use super::{navigation_ui::*, *};
use crate::app_menu::{self, Row};
use crate::orientation::Mode;
use gpui_kit::assets::IconName as Lucide;
use gpui_kit::component::Selectable;
use gpui_kit::component::button::{ButtonCustomVariant, ButtonGroup, ButtonRounded};
use gpui_kit::component::popover::Popover;
use gpui_kit::{
    Anchor, AnyElement, App, CursorStyle, DismissEvent, Entity, Focusable, IntoElement, KeyBinding,
    Styled, deferred, img,
};

actions!(
    application_menu,
    [OpenApplicationMenu, CloseApplicationMenu]
);

/// The visual height of a toolbar control, frame included.
const TOOLBAR_HEIGHT: f32 = 26.0;
/// The width of one orientation segment, which the frame wraps around.
const SEGMENT: f32 = 26.0;
/// The radius of the frame around the two orientation segments.
const FRAME_RADIUS: f32 = 6.0;

/// The key context the menu's own content adds, for what the stock menu lacks.
///
/// The toolkit's own covers the pointer, Escape, Enter and the arrows, so what
/// a person expects of a menu bar and cannot get from those lands here.
const CONTEXT: &str = "ApplicationMenu";

/// A recent row chosen by its digit, which only the File branch promises.
#[derive(Clone, PartialEq, serde::Deserialize, Action)]
#[action(namespace = application_menu, no_json)]
pub(super) struct OpenRecentRow {
    index: usize,
}

pub(super) fn init(cx: &mut gpui_kit::App) {
    let mut keys = vec![
        KeyBinding::new("f10", OpenApplicationMenu, Some(CONTEXT)),
        KeyBinding::new("tab", CloseApplicationMenu, Some(CONTEXT)),
        // The popover confirms on Space, and no menu closes that way.
        KeyBinding::new("space", gpui_kit::NoAction, Some(CONTEXT)),
    ];
    keys.extend((1..=9).map(|digit| {
        KeyBinding::new(
            &digit.to_string(),
            OpenRecentRow {
                index: (digit - 1) as usize,
            },
            Some(CONTEXT),
        )
    }));
    cx.bind_keys(keys);
}

impl Shell {
    /// The branch that asks for a file, opens a recent capture, and does settings.
    ///
    /// The recent rows stand where `app_menu` places them, and a click hands the
    /// capture to the shell. The captures the digits name come back with it, so
    /// a digit cannot answer from a list the probes have changed since.
    fn file_menu(
        &self,
        focus: FocusHandle,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> (Entity<PopupMenu>, Vec<Origin>) {
        let recent = app_menu::file_items(self.recent_entries());
        let digits = recent
            .iter()
            .filter_map(|row| match row {
                Row::Recent {
                    number: Some(_),
                    origin,
                    ..
                } => Some(origin.clone()),
                _ => None,
            })
            .collect();
        let owner = cx.entity().downgrade();
        let menu = PopupMenu::build(window, cx, move |menu, _, _| {
            let menu = menu.action_context(focus).max_w(px(420.)).scrollable(true);
            recent.into_iter().fold(menu, |menu, row| {
                menu.item(Shell::file_row(row, owner.clone()))
            })
        });
        (menu, digits)
    }

    fn file_row(row: Row<Origin>, owner: WeakEntity<Self>) -> PopupMenuItem {
        match row {
            Row::Open => PopupMenuItem::new("Open file...").action(Box::new(ChooseFile)),
            Row::Settings => PopupMenuItem::new("Settings").action(Box::new(EditAnalysis)),
            Row::Separator => PopupMenuItem::separator(),
            Row::Recent {
                number,
                label,
                origin,
            } => PopupMenuItem::element(move |_, cx| {
                recent_row(number, label.clone(), cx.theme().muted_foreground)
            })
            .on_click(move |_, window, cx| {
                // The menu dismisses as this returns, so the open waits a frame.
                let owner = owner.clone();
                let origin = origin.clone();
                window.defer(cx, move |window, cx| {
                    open_capture_from(&owner, origin.clone(), window, cx);
                });
            }),
        }
    }

    /// The branch that carries the session's own choices.
    ///
    /// Absent while no document is open, which is what the menu bar offers.
    fn view_menu(
        &self,
        focus: FocusHandle,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<PopupMenu> {
        let scale = self.time_scale_menu(focus.clone(), window, cx);
        let show_grid = self.session.show_grid;
        let show_scale_ui = self.session.show_scale_ui;
        let vertical = self.session.orientation.vertical();
        PopupMenu::build(window, cx, move |menu, _, _| {
            menu.action_context(focus)
                .item(
                    PopupMenuItem::new("Show grid")
                        .action(Box::new(ToggleGrid))
                        .checked(show_grid),
                )
                .item(
                    PopupMenuItem::new("Show scale controls")
                        .action(Box::new(ToggleScaleUi))
                        .checked(show_scale_ui),
                )
                .item(
                    PopupMenuItem::new("Vertical orientation")
                        .action(Box::new(ToggleOrientation))
                        .checked(vertical),
                )
                .item(PopupMenuItem::separator())
                .item(PopupMenuItem::new("Fit time").action(Box::new(FitCapture)))
                .item(PopupMenuItem::new("Fit frequency").action(Box::new(FitFrequency)))
                .item(PopupMenuItem::separator())
                .item(PopupMenuItem::submenu("Time scale format", scale))
        })
    }

    /// The time ruler's own units, one row each.
    fn time_scale_menu(
        &self,
        focus: FocusHandle,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<PopupMenu> {
        let ruler = self.session.time_ruler;
        PopupMenu::build(window, cx, move |menu, _, _| {
            use crate::time_ruler::Mode;
            menu.action_context(focus)
                .item(
                    PopupMenuItem::new("Hours, minutes, seconds (hms)")
                        .action(Box::new(ClockRuler))
                        .checked(ruler == Mode::Clock),
                )
                .item(
                    PopupMenuItem::new("Seconds")
                        .action(Box::new(SecondsRuler))
                        .checked(ruler == Mode::Seconds),
                )
                .item(
                    PopupMenuItem::new("Sample numbers")
                        .action(Box::new(SamplesRuler))
                        .checked(ruler == Mode::Samples),
                )
        })
    }

    /// Builds the menu the application button opens, and puts the keyboard in it.
    ///
    /// The shell owns the entity, so the popover only draws it and one dismissal
    /// of any branch closes the menu along with its submenus.
    pub(super) fn open_application_menu(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.application_menu.is_some() {
            return;
        }
        self.close_analysis_hint(cx);
        self.interrupt_plot(cx);
        self.recent_files.refresh(&self.session.recent);
        let focus = self.focus_target(cx);
        let (file, digits) = self.file_menu(focus.clone(), window, cx);
        let view = self
            .view
            .is_some()
            .then(|| self.view_menu(focus.clone(), window, cx));
        let branch = file.clone();
        let menu = PopupMenu::build(window, cx, move |menu, _, _| {
            let menu = menu
                .action_context(focus)
                .item(PopupMenuItem::submenu("File", branch));
            match view {
                Some(view) => menu.item(PopupMenuItem::submenu("View", view)),
                None => menu,
            }
        });
        self.application_menu_dismissed =
            Some(cx.subscribe_in(&menu, window, Self::application_menu_dismissed));
        self.application_file_menu = Some(file);
        self.application_file_rows = Some(digits);
        self.application_menu = Some(menu.clone());
        self.title_drag_pending = false;
        window.focus(&menu.focus_handle(cx), cx);
        cx.notify();
    }

    /// Closes the menu and hands the keyboard back where it came from.
    pub(super) fn dismiss_application_menu(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.application_menu.take().is_none() {
            return;
        }
        self.application_file_menu = None;
        self.application_file_rows = None;
        window.focus(&self.focus_target(cx), cx);
        self.title_drag_pending = false;
        cx.notify();
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
        self.open_application_menu(window, cx);
    }

    fn application_menu_dismissed(
        &mut self,
        menu: &Entity<PopupMenu>,
        _: &DismissEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self
            .application_menu
            .as_ref()
            .is_some_and(|open| open.entity_id() == menu.entity_id())
        {
            self.dismiss_application_menu(window, cx);
        }
    }

    /// Opens the recent capture a digit names, but only from the File branch.
    ///
    /// The digits belong to the File rows, so a digit typed while the menu bar
    /// itself holds the keyboard is no recent row and does nothing.
    fn open_recent_row(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let in_file = self
            .application_file_menu
            .as_ref()
            .is_some_and(|file| file.read(cx).focus_handle(cx).is_focused(window));
        if !in_file {
            return;
        }
        let Some(origin) = self
            .application_file_rows
            .as_ref()
            .and_then(|rows| rows.get(index))
            .cloned()
        else {
            return;
        };
        self.open(origin, window, cx);
    }

    /// Covers the window beneath the open menu, so an outside click only closes it.
    ///
    /// Drawn below the menu, which the toolkit paints at the window's topmost
    /// priority.
    pub(super) fn application_menu_backdrop(&self) -> AnyElement {
        deferred(
            div()
                .id("application-menu-backdrop")
                .absolute()
                .inset_0()
                .occlude()
                .cursor(CursorStyle::Arrow),
        )
        .with_priority(1)
        .into_any_element()
    }

    /// The popover that carries the menu, which only the shell opens or closes.
    ///
    /// The tracked handle is the menu's own, so the popover's open leaves the
    /// keyboard inside the menu rather than taking it for itself.
    fn application_menu_popover(&self, button: Button, cx: &mut Context<Self>) -> Popover {
        let trigger = Trigger(button);
        let open = self.application_menu.is_some();
        let menu = self.application_menu.clone();
        let focus = self
            .application_menu
            .as_ref()
            .map(|menu| menu.focus_handle(cx));
        let owner = cx.entity().downgrade();
        let changed = owner.clone();
        Popover::new("application-menu")
            .anchor(Anchor::TopLeft)
            .appearance(false)
            .overlay_closable(false)
            .flex_shrink_0()
            .open(open)
            .when_some(focus, |popover, focus| popover.track_focus(&focus))
            .on_open_change(move |open, window, cx| open_from(&changed, *open, window, cx))
            .trigger(trigger)
            .content(move |_, _, _| menu_content(owner.clone(), menu.clone()))
    }

    /// The application's own window furniture, above and below the content.
    pub(super) fn toolbar(&self, window: &Window, cx: &mut Context<Self>) -> impl IntoElement {
        let open = self.application_menu.is_some();
        let state = if open {
            ToolbarState::On
        } else {
            ToolbarState::Off
        };
        // The popover owns the press, so the button needs no handler.
        let mut app = Button::new("application-menu-button")
            .tab_stop(false)
            .custom(toolbar_style(state, cx))
            .when_some(on_surface(state, cx), |button, surface| button.bg(surface))
            .small()
            .w(app_button_width(window, cx))
            .px(px(6.))
            .h(px(TOOLBAR_HEIGHT))
            .child(img("argand/app.png").size(px(22.)))
            .child(div().text_color(cx.theme().foreground).child(TITLE));
        // The button is pressed while the menu is open, so it shows no hint.
        let shell = cx.entity().downgrade();
        app.interactivity().tooltip(move |_, cx| {
            let open = shell
                .upgrade()
                .is_some_and(|shell| shell.read(cx).application_menu.is_some());
            if open {
                gpui_kit::AnyView::from(cx.new(|_| NoTooltip))
            } else {
                shortcut_tooltip(
                    "Application menu".to_owned(),
                    Some(Box::new(OpenApplicationMenu)),
                    "Shell",
                    px(240.),
                    cx,
                )
            }
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
            .child(self.application_menu_popover(app, cx))
            // Without a document these act on nothing, and their hitbox would block a title drag.
            .when(self.view.is_some(), |bar| {
                let separator = div().w(px(1.)).h(px(16.)).mx_1().bg(cx.theme().border);
                let orientation = self.orientation_segments(cx);
                let grid = self.grid_button(cx);
                bar.child(separator).child(orientation).child(grid)
            })
    }

    /// The two orientation segments in one frame, with the mode in force selected.
    fn orientation_segments(&self, cx: &mut Context<Self>) -> ButtonGroup {
        let vertical = self.session.orientation.vertical();
        let horizontal = Self::orientation_segment(
            "orientation-horizontal",
            Lucide::PanelTop.path(),
            Mode::Horizontal,
            !vertical,
            false,
            cx,
        );
        let vertical = Self::orientation_segment(
            "orientation-vertical",
            Lucide::PanelLeft.path(),
            Mode::Vertical,
            vertical,
            true,
            cx,
        );
        ButtonGroup::new("orientation-segments")
            .rounded(px(FRAME_RADIUS))
            .border_1()
            .border_color(frame_colour(cx))
            .h(px(TOOLBAR_HEIGHT))
            .child(horizontal)
            .child(vertical)
            .on_click(cx.listener(|shell, selected: &Vec<usize>, window, cx| {
                let Some(&next) = selected.first() else {
                    return;
                };
                // A click on either segment hands the keyboard back to the owner.
                window.focus(&shell.focus_target(cx), cx);
                if next != usize::from(shell.session.orientation.vertical()) {
                    shell.toggle_orientation(window, cx);
                }
            }))
    }

    /// One orientation segment, which is an action for the mode it names.
    fn orientation_segment(
        id: &'static str,
        icon: gpui_kit::SharedString,
        mode: Mode,
        current: bool,
        divided: bool,
        cx: &mut Context<Self>,
    ) -> Button {
        let state = if current {
            ToolbarState::Selected
        } else {
            ToolbarState::Off
        };
        let mut segment = Button::new(id)
            .tab_stop(false)
            .custom(toolbar_style(state, cx))
            .selected(current)
            .rounded(ButtonRounded::Size(px(FRAME_RADIUS - 1.)))
            .small()
            .w(px(SEGMENT))
            .h(px(TOOLBAR_HEIGHT - 2.))
            .px_0()
            .child(Self::toolbar_glyph(id, icon, state, cx));
        // The mode in force is not an action, so its segment joins no hover group.
        if !current {
            segment = segment.group(id);
        }
        // Painted, because a border takes the colour of the button's own states.
        if divided {
            let line = div().absolute().left_0().top_0().bottom_0().w(px(1.));
            segment = segment.relative().child(line.bg(frame_colour(cx)));
        }
        // Only the segment that changes the mode offers Ctrl+T.
        let other = !current;
        let hint = format!("{} orientation", mode.label());
        segment.interactivity().tooltip(move |_, cx| {
            let action = other.then(|| Box::new(ToggleOrientation) as Box<dyn Action>);
            shortcut_tooltip(hint.clone(), action, "Plot", px(360.), cx)
        });
        segment
    }

    /// The grid toggle, whose two states are the option on and the option off.
    fn grid_button(&self, cx: &mut Context<Self>) -> Button {
        let id = "toggle-grid";
        let show = self.session.show_grid;
        let state = if show {
            ToolbarState::On
        } else {
            ToolbarState::Off
        };
        let hint = if show { "Hide grid" } else { "Show grid" };
        let mut button = Button::new(id)
            .tab_stop(false)
            .group(id)
            .custom(toolbar_style(state, cx))
            .when_some(on_surface(state, cx), |button, surface| button.bg(surface))
            .toggled(show)
            .small()
            .w(px(TOOLBAR_HEIGHT))
            .h(px(TOOLBAR_HEIGHT))
            .px_0()
            .child(Self::toolbar_glyph(id, "argand/grid.svg".into(), state, cx))
            .on_click(cx.listener(|shell, _, window, cx| {
                window.focus(&shell.focus_target(cx), cx);
                window.dispatch_action(Box::new(ToggleGrid), cx);
            }));
        button.interactivity().tooltip(move |_, cx| {
            shortcut_tooltip(
                hint.to_owned(),
                Some(Box::new(ToggleGrid)),
                "Plot",
                px(360.),
                cx,
            )
        });
        button
    }

    /// The icon of a toolbar control, which tints while an off control is hovered.
    fn toolbar_glyph(
        id: &'static str,
        icon: gpui_kit::SharedString,
        state: ToolbarState,
        cx: &gpui_kit::App,
    ) -> impl IntoElement {
        let accent = toolbar_accent(cx);
        let colour = match state {
            ToolbarState::Off => cx.theme().foreground.opacity(0.85),
            ToolbarState::On | ToolbarState::Selected => accent,
        };
        gpui_kit::svg()
            .path(icon)
            .size(px(20.))
            .id((id, 0_usize))
            .text_color(colour)
            .when(state == ToolbarState::Off, |glyph| {
                glyph.group_hover(id, |style| style.text_color(accent))
            })
            .into_any_element()
    }
}

/// The application button as a trigger the popover may not mark selected.
///
/// `Popover::trigger` selects its trigger while the popover is open, and a
/// selected button loses its hover and pressed surfaces and paints the variant's
/// active colour, which is not the on state #130 accepted for this button.
struct Trigger(Button);

impl Selectable for Trigger {
    fn selected(self, _selected: bool) -> Self {
        self
    }

    fn is_selected(&self) -> bool {
        false
    }
}

impl Styled for Trigger {
    fn style(&mut self) -> &mut gpui_kit::StyleRefinement {
        self.0.style()
    }
}

impl IntoElement for Trigger {
    type Element = AnyElement;

    fn into_element(self) -> Self::Element {
        self.0.into_any_element()
    }

    fn into_any_element(self) -> AnyElement {
        self.0.into_any_element()
    }
}

/// Runs a shell change from an element that owns no shell context of its own.
///
/// The menu's content is handed an app alone, and the window a key or a click
/// arrived in is the one the shell draws into.
fn with_shell(
    owner: &WeakEntity<Shell>,
    window: &mut Window,
    cx: &mut App,
    change: impl FnOnce(&mut Shell, &mut Window, &mut Context<Shell>),
) {
    let Some(shell) = owner.upgrade() else {
        return;
    };
    shell.update(cx, move |shell, cx| change(shell, window, cx));
}

/// The menu with the key context the stock menu has none of.
///
/// F10, Tab and the File rows' digits land here, and the popover hands this
/// closure an app alone, so every change reaches the shell through `with_shell`.
fn menu_content(owner: WeakEntity<Shell>, menu: Option<Entity<PopupMenu>>) -> impl IntoElement {
    div()
        .key_context(CONTEXT)
        .on_action({
            let owner = owner.clone();
            move |_: &CloseApplicationMenu, window, cx| dismiss_from(&owner, window, cx)
        })
        .on_action(move |action: &OpenRecentRow, window, cx| {
            open_recent_from(&owner, action.index, window, cx)
        })
        .children(menu)
}

/// Closes the menu from its own content, which is what F10 and Tab do.
fn dismiss_from(owner: &WeakEntity<Shell>, window: &mut Window, cx: &mut App) {
    with_shell(owner, window, cx, |shell, window, cx| {
        shell.dismiss_application_menu(window, cx);
    });
}

/// Opens the recent row a digit names, but only from the File branch.
fn open_recent_from(owner: &WeakEntity<Shell>, index: usize, window: &mut Window, cx: &mut App) {
    with_shell(owner, window, cx, move |shell, window, cx| {
        shell.open_recent_row(index, window, cx);
    });
}

/// Opens a capture a recent row names, once the menu is gone.
fn open_capture_from(owner: &WeakEntity<Shell>, origin: Origin, window: &mut Window, cx: &mut App) {
    with_shell(owner, window, cx, move |shell, window, cx| {
        shell.open(origin, window, cx);
    });
}

/// Follows the popover's own open state, which only the shell moves.
fn open_from(owner: &WeakEntity<Shell>, open: bool, window: &mut Window, cx: &mut App) {
    with_shell(owner, window, cx, move |shell, window, cx| {
        if open {
            shell.open_application_menu(window, cx);
        } else {
            shell.dismiss_application_menu(window, cx);
        }
    });
}

/// The row one recent capture is drawn in: its digit, then its name.
fn recent_row(number: Option<u8>, label: String, muted: gpui_kit::Hsla) -> impl IntoElement {
    div()
        .flex()
        .flex_1()
        .min_w_0()
        .items_center()
        .gap_2()
        .when_some(number, |row, number| {
            row.child(
                div()
                    .w_6()
                    .flex_shrink_0()
                    .text_color(muted)
                    .child(crate::numbers::number(usize::from(number))),
            )
        })
        .child(div().min_w_0().text_ellipsis().child(label))
}

/// The width the title reserves for the toolbar.
pub(super) fn toolbar_width(window: &Window, cx: &gpui_kit::App, document: bool) -> Pixels {
    let app = app_button_width(window, cx);
    if !document {
        return app;
    }
    // The framed segment group and Grid, the separator, three gaps and its margins.
    app + px(SEGMENT * 2. + 2. + TOOLBAR_HEIGHT + 1.) + window.rem_size() * 1.25
}

fn app_button_width(window: &Window, cx: &gpui_kit::App) -> Pixels {
    let style = gpui_kit::TextStyle {
        font_family: cx.theme().font_family.clone(),
        ..Default::default()
    };
    let label = window.text_system().shape_line(
        TITLE.into(),
        window.rem_size() * 0.875,
        &[style.to_run(TITLE.len())],
        None,
    );
    // Artwork, text gap and horizontal padding.
    label.width.ceil() + px(22. + 12.) + window.rem_size() * 0.25
}

pub(super) fn toolbar_accent(cx: &gpui_kit::App) -> gpui_kit::Hsla {
    if cx.theme().is_dark() {
        cx.theme().blue_light
    } else {
        cx.theme().blue.darken(0.2)
    }
}

/// The state a toolbar control is in, which decides its whole surface.
#[derive(Clone, Copy, PartialEq)]
pub(super) enum ToolbarState {
    /// An ordinary control, which tints its glyph while it is hovered.
    Off,
    /// A control that is on, which stays accent while it is pressed.
    On,
    /// The segment in force, which does not react to the pointer at all.
    Selected,
}

/// The toolbar surface of one control. An on control keeps the accent shade
/// under the hover and pressed surfaces.
pub(super) fn toolbar_style(state: ToolbarState, cx: &gpui_kit::App) -> ButtonCustomVariant {
    let accent = toolbar_accent(cx);
    let (color, hover, active) = match state {
        ToolbarState::Off => (
            cx.theme().title_bar.darken(0.035),
            accent.opacity(0.32),
            accent.opacity(0.44),
        ),
        ToolbarState::On => (
            accent.opacity(0.30),
            accent.opacity(0.40),
            accent.opacity(0.52),
        ),
        // A selected button is painted with the variant's active colour.
        ToolbarState::Selected => (
            accent.opacity(0.30),
            accent.opacity(0.30),
            accent.opacity(0.30),
        ),
    };
    ButtonCustomVariant::new(cx)
        .color(color)
        .foreground(cx.theme().foreground.darken(0.12))
        .hover(hover)
        .active(active)
}

/// The resting surface of an on control, which a custom variant would fade to a fifth.
fn on_surface(state: ToolbarState, cx: &gpui_kit::App) -> Option<gpui_kit::Hsla> {
    (state == ToolbarState::On).then(|| toolbar_accent(cx).opacity(0.30))
}

/// The opaque frame of the segments, so the divider keeps its colour over a selected one.
fn frame_colour(cx: &gpui_kit::App) -> gpui_kit::Hsla {
    cx.theme()
        .title_bar
        .blend(cx.theme().foreground.opacity(0.24))
}

/// The tooltip stand-in for the pressed application button: renders nothing.
struct NoTooltip;

impl gpui_kit::Render for NoTooltip {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        gpui_kit::div()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session;
    use gpui_kit::test::TestWindowExt;
    use gpui_kit::{TestAppContext, WindowHandle};
    use std::path::{Path, PathBuf};

    /// The controls the toolbar offers, in the order they are drawn.
    const CONTROLS: [&str; 4] = [
        "application-menu-button",
        "orientation-horizontal",
        "orientation-vertical",
        "toggle-grid",
    ];

    fn open(cx: &mut TestAppContext) -> WindowHandle<Shell> {
        cx.update(|cx| {
            gpui_kit::init(cx);
            settings_ui::init(cx);
            navigation_ui::init(cx);
            hints::init(cx);
            app_menu_ui::init(cx);
            window_keys(cx);
        });
        let handle = cx.add_window(|window, cx| {
            Shell::new(Config::default(), None, Session::default(), window, cx)
        });
        frame(cx, handle);
        handle
    }

    fn frame(cx: &mut TestAppContext, handle: WindowHandle<Shell>) {
        cx.update_window(handle.into(), |_, window, cx| window.render_frame(cx))
            .unwrap();
        cx.run_until_parked();
    }

    /// The window as a person would find it, which is a range to navigate.
    fn open_document(cx: &mut TestAppContext, handle: WindowHandle<Shell>) {
        handle
            .update(cx, |shell, _, cx| {
                shell.view = Some(crate::navigation::View::full(1000));
                cx.notify();
            })
            .unwrap();
        frame(cx, handle);
    }

    /// Which of the toolbar controls the last frame drew.
    fn controls(cx: &mut TestAppContext, handle: WindowHandle<Shell>) -> Vec<bool> {
        cx.update_window(handle.into(), |_, window, _| {
            CONTROLS
                .iter()
                .map(|id| window.try_find(*id).is_some())
                .collect()
        })
        .unwrap()
    }

    fn vertical(cx: &mut TestAppContext, handle: WindowHandle<Shell>) -> bool {
        handle
            .read_with(cx, |shell, _| shell.session.orientation.vertical())
            .unwrap()
    }

    fn owner_focused(cx: &mut TestAppContext, handle: WindowHandle<Shell>) -> bool {
        handle
            .update(cx, |shell, window, _| shell.focus.is_focused(window))
            .unwrap()
    }

    #[gpui_kit::test]
    fn the_document_controls_join_the_toolbar(cx: &mut TestAppContext) {
        let handle = open(cx);
        assert_eq!(
            controls(cx, handle),
            vec![true, false, false, false],
            "the start page shows the application button alone"
        );
        open_document(cx, handle);
        assert_eq!(controls(cx, handle), vec![true, true, true, true]);
    }

    #[gpui_kit::test]
    fn the_title_reserves_exactly_what_the_toolbar_draws(cx: &mut TestAppContext) {
        let handle = open(cx);
        let app = handle
            .update(cx, |_, window, cx| app_button_width(window, cx))
            .unwrap();
        let reserved = |cx: &mut TestAppContext, document: bool| {
            handle
                .update(cx, |_, window, cx| toolbar_width(window, cx, document))
                .unwrap()
        };
        assert_eq!(
            reserved(cx, false),
            app,
            "the application button is all the toolbar holds without a document"
        );
        open_document(cx, handle);
        // The first and the last control touch the toolbar's own edges.
        let drawn = cx
            .update_window(handle.into(), |_, window, _| {
                let first = window.find("application-menu-button").bounds();
                let last = window.find("toggle-grid").bounds();
                last.right() - first.origin.x
            })
            .unwrap();
        assert_eq!(
            reserved(cx, true),
            drawn,
            "the title reserves the segments, the separator and the grid"
        );
    }

    #[gpui_kit::test]
    fn the_segment_group_carries_its_own_frame(cx: &mut TestAppContext) {
        let handle = open(cx);
        open_document(cx, handle);
        let bounds = |cx: &mut TestAppContext, id: &'static str| {
            cx.update_window(handle.into(), |_, window, _| window.find(id).bounds())
                .unwrap()
        };
        let first = bounds(cx, "orientation-horizontal");
        let second = bounds(cx, "orientation-vertical");
        let grid = bounds(cx, "toggle-grid");
        assert_eq!(first.size, second.size, "the segments are equal");
        assert_eq!(first.size.width, px(26.));
        assert_eq!(
            first.size.height,
            px(24.),
            "a segment fills the frame's inside"
        );
        assert_eq!(second.origin.x - first.origin.x, px(26.));
        assert_eq!(first.origin.y, second.origin.y, "the pair is one row");
        assert_eq!(
            grid.size.height,
            px(26.),
            "the group and the grid keep one height"
        );
        assert_eq!(
            first.origin.y - grid.origin.y,
            px(1.),
            "the frame is one pixel above the segments"
        );
    }

    #[gpui_kit::test]
    fn only_the_other_orientation_segment_switches(cx: &mut TestAppContext) {
        let handle = open(cx);
        open_document(cx, handle);
        assert!(!vertical(cx, handle), "horizontal is the default");
        // Another control held the keyboard, as it would after a toolbar click.
        handle
            .update(cx, |_, window, cx| {
                let elsewhere = cx.focus_handle();
                window.focus(&elsewhere, cx);
            })
            .unwrap();
        assert!(!owner_focused(cx, handle), "the keyboard left the owner");
        cx.update_window(handle.into(), |_, window, cx| {
            window.click("orientation-horizontal", cx);
        })
        .unwrap();
        assert!(
            owner_focused(cx, handle),
            "a click on either segment hands the keyboard back"
        );
        assert!(!vertical(cx, handle), "the mode in force is not an action");
        cx.update_window(handle.into(), |_, window, cx| {
            window.click("orientation-vertical", cx);
        })
        .unwrap();
        assert!(vertical(cx, handle), "one click is one switch");
        cx.update_window(handle.into(), |_, window, cx| {
            window.click("orientation-vertical", cx);
        })
        .unwrap();
        assert!(vertical(cx, handle), "and it stays put");
        cx.update_window(handle.into(), |_, window, cx| {
            window.click("orientation-horizontal", cx);
        })
        .unwrap();
        assert!(!vertical(cx, handle));
    }

    /// The window as a person finds it, with the keys the window itself binds.
    fn open_window(cx: &mut TestAppContext) -> (Entity<Shell>, &mut gpui_kit::VisualTestContext) {
        cx.update(|cx| {
            gpui_kit::init(cx);
            settings_ui::init(cx);
            navigation_ui::init(cx);
            hints::init(cx);
            app_menu_ui::init(cx);
            window_keys(cx);
        });
        let (shell, cx) = cx.add_window_view(|window, cx| {
            Shell::new(Config::default(), None, Session::default(), window, cx)
        });
        cx.simulate_resize(gpui_kit::size(px(800.), px(600.)));
        draw(cx);
        (shell, cx)
    }

    fn draw(cx: &mut gpui_kit::VisualTestContext) {
        cx.run_until_parked();
        cx.update(|window, cx| window.render_frame(cx));
        cx.run_until_parked();
    }

    /// A document, which is what binds the window's own keys to a focus path.
    ///
    /// The capture is nowhere near a real file and nothing analyses it, because
    /// only the document's existence matters to a key context.
    fn open_capture(cx: &mut gpui_kit::VisualTestContext, shell: &Entity<Shell>) {
        shell.update_in(cx, |shell, window, cx| {
            shell.open(
                Origin::new(PathBuf::from("/captures/session.iqw")),
                window,
                cx,
            );
        });
        draw(cx);
        shell.update_in(cx, |shell, _, cx| {
            shell.view = Some(crate::navigation::View::full(1000));
            cx.notify();
        });
        draw(cx);
    }

    /// The middle of the window, which is over the content and not the menu.
    fn outside() -> gpui_kit::Point<gpui_kit::Pixels> {
        gpui_kit::point(px(400.), px(400.))
    }

    /// Where a drawn control is, so a click lands where a person would put it.
    fn at(
        cx: &mut gpui_kit::VisualTestContext,
        id: &'static str,
    ) -> gpui_kit::Point<gpui_kit::Pixels> {
        cx.update(|window, _| window.find(id).bounds().center())
    }

    fn click(cx: &mut gpui_kit::VisualTestContext, id: &'static str) {
        let at = at(cx, id);
        cx.simulate_click(at, gpui_kit::Modifiers::default());
        draw(cx);
    }

    fn click_outside(cx: &mut gpui_kit::VisualTestContext) {
        cx.simulate_click(outside(), gpui_kit::Modifiers::default());
        draw(cx);
    }

    fn press(cx: &mut gpui_kit::VisualTestContext, keys: &str) {
        cx.simulate_keystrokes(keys);
        draw(cx);
    }

    /// Whether the shell has built the menu, which is its own answer.
    fn menu_open(cx: &mut gpui_kit::VisualTestContext, shell: &Entity<Shell>) -> bool {
        shell.read_with(cx, |shell, _| shell.application_menu.is_some())
    }

    /// Whether the last frame drew the menu itself.
    fn menu_drawn(cx: &mut gpui_kit::VisualTestContext) -> bool {
        cx.update(|window, _| window.try_find("popup-menu").is_some())
    }

    /// Whether the keyboard is in the menu, which is where it belongs.
    fn menu_focused(cx: &mut gpui_kit::VisualTestContext, shell: &Entity<Shell>) -> bool {
        shell.update_in(cx, |shell, window, cx| {
            shell
                .application_menu
                .as_ref()
                .is_some_and(|menu| menu.read(cx).focus_handle(cx).is_focused(window))
        })
    }

    /// Whether the keyboard is where the menu found it, which is what dismissal
    /// hands it back to.
    fn keyboard_at_owner(cx: &mut gpui_kit::VisualTestContext, shell: &Entity<Shell>) -> bool {
        shell.update_in(cx, |shell, window, _| shell.focus.is_focused(window))
    }

    /// Whether the File branch holds the keyboard, which a digit row needs.
    fn file_focused(cx: &mut gpui_kit::VisualTestContext, shell: &Entity<Shell>) -> bool {
        shell.update_in(cx, |shell, window, cx| {
            shell
                .application_file_menu
                .as_ref()
                .is_some_and(|file| file.read(cx).focus_handle(cx).is_focused(window))
        })
    }

    /// One available capture in the recent list, with a digit waiting for it.
    fn recent(cx: &mut gpui_kit::VisualTestContext, shell: &Entity<Shell>, path: &str) {
        available_recent(cx, shell, std::slice::from_ref(&path.to_owned()), true);
    }

    /// Recent captures in a given availability, newest first.
    fn available_recent(
        cx: &mut gpui_kit::VisualTestContext,
        shell: &Entity<Shell>,
        paths: &[String],
        available: bool,
    ) {
        shell.update_in(cx, |shell, _, cx| {
            shell.session.recent = paths
                .iter()
                .map(|path| session::Recent {
                    path: PathBuf::from(path),
                    hints: Default::default(),
                })
                .collect();
            shell.recent_files.refresh(&shell.session.recent);
            for path in paths {
                shell.recent_files.apply(PathBuf::from(path), available);
            }
            cx.notify();
        });
        draw(cx);
    }

    /// A capture a probe has just re-checked while the menu is open.
    fn recheck(cx: &mut gpui_kit::VisualTestContext, shell: &Entity<Shell>, path: &str) {
        shell.update_in(cx, |shell, _, cx| {
            shell.recent_files.apply(PathBuf::from(path), false);
            cx.notify();
        });
        draw(cx);
    }

    /// Where a stock menu row is, which the toolkit records under its index.
    fn row(cx: &mut gpui_kit::VisualTestContext, index: u64) -> gpui_kit::Point<gpui_kit::Pixels> {
        cx.update(|window, _| {
            window
                .find(gpui_kit::ElementId::Integer(index))
                .bounds()
                .center()
        })
    }

    /// The settings window the menu's Settings row opens.
    fn settings_open(cx: &mut gpui_kit::VisualTestContext, shell: &Entity<Shell>) -> bool {
        shell.read_with(cx, |shell, _| shell.settings_window.is_some())
    }

    /// The document the open menu replaced, once one of its rows opened something.
    fn opened(cx: &mut gpui_kit::VisualTestContext, shell: &Entity<Shell>) -> Option<PathBuf> {
        shell.read_with(cx, |shell, _| {
            shell
                .file
                .as_ref()
                .map(|file| file.document.origin().path.clone())
        })
    }

    fn grid(cx: &mut gpui_kit::VisualTestContext, shell: &Entity<Shell>) -> bool {
        shell.read_with(cx, |shell, _| shell.session.show_grid)
    }

    #[gpui_kit::test]
    fn the_button_and_the_key_open_the_menu_with_the_keyboard_in_it(cx: &mut TestAppContext) {
        let (shell, cx) = open_window(cx);
        open_capture(cx, &shell);
        click(cx, "application-menu-button");
        assert!(menu_open(cx, &shell), "the button opens the menu");
        assert!(menu_drawn(cx), "and the menu is drawn");
        assert!(menu_focused(cx, &shell), "with the keyboard inside it");
        click(cx, "application-menu-button");
        assert!(!menu_open(cx, &shell), "a second click closes it");
        press(cx, "f10");
        assert!(menu_open(cx, &shell), "the key opens it too");
        assert!(menu_focused(cx, &shell));
    }

    #[gpui_kit::test]
    fn a_click_on_the_button_closes_the_menu_and_leaves_it_closed(cx: &mut TestAppContext) {
        let (shell, cx) = open_window(cx);
        open_capture(cx, &shell);
        press(cx, "f10");
        assert!(menu_open(cx, &shell));
        click(cx, "application-menu-button");
        assert!(!menu_open(cx, &shell));
        draw(cx);
        draw(cx);
        assert!(!menu_drawn(cx), "and nothing opens it again");
    }

    #[gpui_kit::test]
    fn the_key_escape_and_an_outside_click_each_close_the_menu(cx: &mut TestAppContext) {
        let (shell, cx) = open_window(cx);
        open_capture(cx, &shell);
        for key in ["f10", "tab", "escape"] {
            press(cx, "f10");
            press(cx, key);
            assert!(!menu_open(cx, &shell), "{key} closes the menu");
            assert!(!menu_drawn(cx), "{key} leaves nothing drawn");
            assert!(
                keyboard_at_owner(cx, &shell),
                "{key} hands the keyboard back"
            );
        }
        press(cx, "f10");
        assert!(menu_open(cx, &shell));
        click_outside(cx);
        assert!(!menu_open(cx, &shell), "an outside click closes it");
        assert!(keyboard_at_owner(cx, &shell));
        // The next key reaches the window again, which is what dismissal promised.
        press(cx, "f10");
        assert!(menu_open(cx, &shell));
    }

    #[gpui_kit::test]
    fn an_outside_click_and_wheel_reach_nothing_under_the_menu(cx: &mut TestAppContext) {
        let (shell, cx) = open_window(cx);
        open_capture(cx, &shell);
        let view = shell.read_with(cx, |shell, _| shell.view);
        press(cx, "f10");
        assert!(menu_open(cx, &shell));
        click_outside(cx);
        assert!(!menu_open(cx, &shell), "the click closed the menu");
        assert_eq!(
            shell.read_with(cx, |shell, _| shell.view),
            view,
            "and moved nothing under it"
        );
        assert!(
            !shell.read_with(cx, |shell, _| shell.title_drag_pending),
            "nor began a title drag"
        );
        press(cx, "f10");
        cx.simulate_event(gpui_kit::ScrollWheelEvent {
            position: outside(),
            delta: gpui_kit::ScrollDelta::Pixels(gpui_kit::point(px(0.), px(80.))),
            ..Default::default()
        });
        draw(cx);
        assert!(menu_open(cx, &shell), "the wheel leaves the menu open");
        assert_eq!(
            shell.read_with(cx, |shell, _| shell.view),
            view,
            "and changes no view"
        );
    }

    #[gpui_kit::test]
    fn the_backdrop_keeps_the_window_beneath_the_menu(cx: &mut TestAppContext) {
        let (shell, cx) = open_window(cx);
        open_capture(cx, &shell);
        let before = grid(cx, &shell);
        press(cx, "f10");
        assert!(menu_open(cx, &shell));
        // A control under the menu must not answer the click that closed it.
        click(cx, "toggle-grid");
        assert!(
            !menu_open(cx, &shell),
            "the control's click dismissed the menu"
        );
        assert_eq!(grid(cx, &shell), before, "and did not reach the control");
        // A press on the title bar must not become a window drag either.
        press(cx, "f10");
        let bar = gpui_kit::point(px(400.), px(14.));
        let none = gpui_kit::Modifiers::default();
        cx.simulate_mouse_down(bar, gpui_kit::MouseButton::Left, none);
        draw(cx);
        assert!(!menu_open(cx, &shell), "the press dismissed the menu");
        assert!(
            !shell.read_with(cx, |shell, _| shell.title_drag_pending),
            "and the title bar never saw it"
        );
        cx.simulate_mouse_up(bar, gpui_kit::MouseButton::Left, none);
        draw(cx);
    }

    #[gpui_kit::test]
    fn enter_on_a_view_row_dispatches_its_action_once(cx: &mut TestAppContext) {
        let (shell, cx) = open_window(cx);
        open_capture(cx, &shell);
        let before = grid(cx, &shell);
        press(cx, "f10");
        // Down to the View branch, right into it, then confirm its first row.
        press(cx, "down down right enter");
        assert!(!menu_open(cx, &shell), "the menu closed");
        assert_ne!(grid(cx, &shell), before, "one confirmation is one toggle");
    }

    #[gpui_kit::test]
    fn a_digit_answers_only_in_the_file_branch(cx: &mut TestAppContext) {
        let (shell, cx) = open_window(cx);
        open_capture(cx, &shell);
        recent(cx, &shell, "/captures/hfdl.iqw");
        press(cx, "f10");
        press(cx, "1");
        assert!(menu_open(cx, &shell), "a digit in the menu bar is no row");
        assert_eq!(
            opened(cx, &shell).as_deref(),
            Some(Path::new("/captures/session.iqw")),
            "and opened nothing"
        );
        press(cx, "down right");
        assert!(file_focused(cx, &shell), "Right enters the File branch");
        press(cx, "1");
        assert!(!menu_open(cx, &shell), "the capture replaced the menu");
        assert_eq!(
            opened(cx, &shell).as_deref(),
            Some(Path::new("/captures/hfdl.iqw"))
        );
    }

    #[gpui_kit::test]
    fn a_recent_row_opens_its_capture(cx: &mut TestAppContext) {
        let (shell, cx) = open_window(cx);
        open_capture(cx, &shell);
        recent(cx, &shell, "/captures/beacon.iqw");
        press(cx, "f10");
        // Into File, past the command and its separator, onto the capture.
        press(cx, "down right");
        assert!(file_focused(cx, &shell));
        press(cx, "down enter");
        draw(cx);
        assert!(!menu_open(cx, &shell), "the menu closed");
        assert_eq!(
            opened(cx, &shell).as_deref(),
            Some(Path::new("/captures/beacon.iqw"))
        );
    }

    #[gpui_kit::test]
    fn the_settings_window_opens_from_the_menu_itself(cx: &mut TestAppContext) {
        let (shell, cx) = open_window(cx);
        open_capture(cx, &shell);
        press(cx, "f10");
        // Down to File, right into it, then past the command to Settings.
        press(cx, "down right");
        assert!(file_focused(cx, &shell));
        press(cx, "down enter");
        draw(cx);
        assert!(!menu_open(cx, &shell), "the menu closed");
        assert!(settings_open(cx, &shell), "and the settings window opened");
        shell.update_in(cx, |shell, _, cx| shell.finish_settings(false, cx));
        draw(cx);
    }

    #[gpui_kit::test]
    fn a_click_on_a_recent_row_opens_that_capture(cx: &mut TestAppContext) {
        let (shell, cx) = open_window(cx);
        open_capture(cx, &shell);
        recent(cx, &shell, "/captures/beacon.iqw");
        press(cx, "f10");
        // The File branch is on screen, and its third row is the first capture.
        press(cx, "down right");
        assert!(file_focused(cx, &shell));
        let capture = row(cx, 2);
        cx.simulate_click(capture, gpui_kit::Modifiers::default());
        draw(cx);
        assert!(!menu_open(cx, &shell), "the menu closed");
        assert_eq!(
            opened(cx, &shell).as_deref(),
            Some(Path::new("/captures/beacon.iqw"))
        );
    }

    #[gpui_kit::test]
    fn a_digit_opens_the_row_that_was_drawn(cx: &mut TestAppContext) {
        let (shell, cx) = open_window(cx);
        open_capture(cx, &shell);
        let paths = [
            "/captures/hfdl.iqw".to_owned(),
            "/captures/beacon.iqw".to_owned(),
        ];
        available_recent(cx, &shell, &paths, true);
        press(cx, "f10");
        press(cx, "down right");
        assert!(file_focused(cx, &shell));
        // A probe finds the first capture gone while the menu stands open.
        recheck(cx, &shell, "/captures/hfdl.iqw");
        assert!(
            menu_open(cx, &shell),
            "the menu keeps the rows it was drawn with"
        );
        press(cx, "1");
        assert_eq!(
            opened(cx, &shell).as_deref(),
            Some(Path::new("/captures/hfdl.iqw")),
            "the digit opened the row that was drawn, not the first available"
        );
    }

    #[gpui_kit::test]
    fn enter_runs_no_branch_where_the_stock_menu_ignores_one(cx: &mut TestAppContext) {
        let (shell, cx) = open_window(cx);
        open_capture(cx, &shell);
        let before = grid(cx, &shell);
        press(cx, "f10");
        // Down twice reaches the View branch, and Enter on a branch runs nothing.
        press(cx, "down down enter");
        assert!(menu_open(cx, &shell), "the menu stays open");
        assert!(
            menu_focused(cx, &shell),
            "and the keyboard stays in the menu"
        );
        assert_eq!(grid(cx, &shell), before, "with no row run");
        press(cx, "right");
        assert!(!menu_focused(cx, &shell), "Right enters the branch");
        press(cx, "enter");
        assert_ne!(grid(cx, &shell), before, "where Enter runs its row");
    }

    #[test]
    fn the_popover_trigger_reports_itself_unselected() {
        let trigger = Trigger(Button::new("application-menu-button"));
        assert!(!trigger.is_selected());
        assert!(!trigger.selected(true).is_selected());
    }
}
