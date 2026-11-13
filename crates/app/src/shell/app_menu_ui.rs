//! The application menu and toolbar; the menu itself is the toolkit's own.

use super::*;
use crate::app_menu::{self, Row};
use crate::navigation::orientation::Mode;
use crate::shell::plot::navigation_ui::*;
use gpui_kit::assets::IconName as Lucide;
use gpui_kit::component::button::{ButtonCustomVariant, ButtonGroup, ButtonRounded};
use gpui_kit::component::kbd::Kbd;
use gpui_kit::component::popover::Popover;
use gpui_kit::component::{Disableable, Selectable};
use gpui_kit::{
    Anchor, AnyElement, App, DismissEvent, Entity, Focusable, IntoElement, KeyBinding, Styled, img,
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
    keys.extend(
        ["1", "2", "3", "4", "5", "6", "7", "8", "9", "0"]
            .into_iter()
            .enumerate()
            .map(|(index, key)| KeyBinding::new(key, OpenRecentRow { index }, Some(CONTEXT))),
    );
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
        let commands = self.command_availability();
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
            let menu = menu
                .action_context(focus.clone())
                .max_w(px(420.))
                .scrollable(true);
            recent.into_iter().fold(menu, |menu, row| {
                menu.item(Shell::file_row(row, commands, focus.clone(), owner.clone()))
            })
        });
        let recent_focus = recent_menu
            .as_ref()
            .map(|menu| menu.read(cx).focus_handle(cx));
        let menu = PopupMenu::build(window, cx, move |menu, _, _| {
            let menu = menu.action_context(focus.clone()).max_w(px(420.));
            let mut history_added = false;
            recent.into_iter().fold(menu, |menu, row| {
                if !matches!(row, Row::Recent { .. }) {
                    return menu.item(Shell::file_row(row, enabled, focus.clone(), owner.clone()));
                }
                if history_added {
                    return menu;
                }
                history_added = true;
                menu.item(PopupMenuItem::submenu(
                    "Recent files",
                    recent_menu.as_ref().expect("history submenu").clone(),
                ))
            })
        });
        (menu, recent_focus, digits)
    }

    fn file_row(
        row: Row<Origin>,
        commands: command::Availability,
        focus: FocusHandle,
        owner: WeakEntity<Self>,
    ) -> PopupMenuItem {
        match row {
            Row::Open => action_row("Open file...", ChooseFile, false, focus, commands),
            Row::Save => action_row("Save", Save, false, focus, commands),
            Row::SaveAs => action_row("Save as...", SaveAs, false, focus, commands),
            Row::SaveSelectionAs => action_row(
                "Save selection as...",
                SaveSelectionAs,
                false,
                focus,
            )
            .disabled(!enabled.save_selection),
            Row::Information => {
                action_row("File information…", Box::new(ConvertCapture), false, focus)
                    .disabled(!enabled.information)
            }
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

    /// The branch that edits the capture, every row disabled while it cannot act.
    fn edit_menu(
        &self,
        focus: FocusHandle,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<PopupMenu> {
        let commands = self.command_availability();
        let inverted = self.edit_commands_available().inverted;
        PopupMenu::build(window, cx, move |menu, _, _| {
            menu.action_context(focus.clone())
                .item(action_row(
                    "Undo",
                    editing_ui::Undo,
                    false,
                    focus.clone(),
                    commands,
                ))
                .item(action_row(
                    "Redo",
                    editing_ui::Redo,
                    false,
                    focus.clone(),
                    commands,
                ))
                .item(PopupMenuItem::separator())
                .item(action_row(
                    "Cut",
                    editing_ui::CutSelection,
                    false,
                    focus.clone(),
                    commands,
                ))
                .item(action_row(
                    "Copy",
                    editing_ui::CopySelection,
                    false,
                    focus.clone(),
                    commands,
                ))
                .item(action_row(
                    "Paste",
                    editing_ui::PasteClipboard,
                    false,
                    focus.clone(),
                    commands,
                ))
                .item(action_row(
                    "Delete",
                    editing_ui::DeleteSelection,
                    false,
                    focus.clone(),
                    commands,
                ))
                .item(PopupMenuItem::separator())
                .item(action_row(
                    "Selection panel",
                    editing_ui::EditSelection,
                    false,
                    focus.clone(),
                    commands,
                ))
                .item(PopupMenuItem::separator())
                .item(action_row(
                    "Suppress band",
                    editing_ui::SuppressBand,
                    false,
                    focus.clone(),
                    commands,
                ))
                .item(PopupMenuItem::separator())
                .item(action_row(
                    "Invert spectrum",
                    editing_ui::InvertSpectrum,
                    inverted,
                    focus.clone(),
                    commands,
                ))
                .item(action_row(
                    "Normalize level",
                    editing_ui::NormalizeLevel,
                    false,
                    focus.clone(),
                    commands,
                ))
        })
    }

    /// The branch that carries the session's own choices.
    ///
    /// All commands stay visible; document-specific choices are disabled without one.
    fn view_menu(
        &self,
        focus: FocusHandle,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<PopupMenu> {
        let scale = self.time_scale_menu(focus.clone(), window, cx);
        let theme = self.theme_menu(focus.clone(), window, cx);
        let commands = self.command_availability();
        let show_grid = self.session.show_grid;
        let show_scale_ui = self.session.show_scale_ui;
        let vertical = self.session.orientation.vertical();
        let analysis = self.status_panel_available(settings_ui::StatusPanel::Analysis);
        PopupMenu::build(window, cx, move |menu, _, _| {
            menu.action_context(focus.clone())
                .item(action_row(
                    "Show grid",
                    ToggleGrid,
                    show_grid,
                    focus.clone(),
                    commands,
                ))
                .item(action_row(
                    "Show scale controls",
                    ToggleScaleUi,
                    show_scale_ui,
                    focus.clone(),
                    commands,
                ))
                .item(action_row(
                    "Vertical orientation",
                    ToggleOrientation,
                    vertical,
                    focus.clone(),
                    commands,
                ))
                .item(PopupMenuItem::separator())
                .item(action_row(
                    "Fit time",
                    FitCapture,
                    false,
                    focus.clone(),
                    commands,
                ))
                .item(action_row(
                    "Fit frequency",
                    FitFrequency,
                    false,
                    focus.clone(),
                    commands,
                ))
                .item(PopupMenuItem::separator())
                .item(if document {
                    PopupMenuItem::submenu("Time scale format", scale)
                } else {
                    PopupMenuItem::new("Time scale format").disabled(true)
                })
                .item(PopupMenuItem::submenu("Theme", theme))
                .item(PopupMenuItem::separator())
                .item(
                    action_row("Analysis settings…", Box::new(EditAnalysis), false, focus)
                        .disabled(!analysis),
                )
        })
    }

    /// The interface themes, one row each, the one in force checked.
    fn theme_menu(
        &self,
        focus: FocusHandle,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<PopupMenu> {
        let commands = self.command_availability();
        let theme = self.theme();
        PopupMenu::build(window, cx, move |menu, _, _| {
            menu.action_context(focus.clone())
                .item(action_row(
                    "System",
                    SystemTheme,
                    theme == Theme::System,
                    focus.clone(),
                    commands,
                ))
                .item(action_row(
                    "Light",
                    LightTheme,
                    theme == Theme::Light,
                    focus.clone(),
                    commands,
                ))
                .item(action_row(
                    "Dark",
                    DarkTheme,
                    theme == Theme::Dark,
                    focus.clone(),
                    commands,
                ))
        })
    }

    /// The time ruler's own units, one row each.
    fn time_scale_menu(
        &self,
        focus: FocusHandle,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<PopupMenu> {
        let commands = self.command_availability();
        let ruler = self.session.time_ruler;
        PopupMenu::build(window, cx, move |menu, _, _| {
            use crate::navigation::time_ruler::Mode;
            menu.action_context(focus.clone())
                .item(action_row(
                    "Hours, minutes, seconds (hms)",
                    ClockRuler,
                    ruler == Mode::Clock,
                    focus.clone(),
                    commands,
                ))
                .item(action_row(
                    "Seconds",
                    SecondsRuler,
                    ruler == Mode::Seconds,
                    focus.clone(),
                    commands,
                ))
                .item(action_row(
                    "Sample numbers",
                    SamplesRuler,
                    ruler == Mode::Samples,
                    focus.clone(),
                    commands,
                ))
        })
    }

    /// Builds the menu the application button opens, and puts the keyboard in it.
    ///
    /// The shell owns the entity, so the popover only draws it and one dismissal
    /// of any branch closes the menu along with its submenus.
    pub(super) fn open_application_menu(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.conversion_active() || self.application_menu.is_some() {
            return;
        }
        self.suppress_external_tooltips();
        self.close_hints(cx);
        self.interrupt_plot(cx);
        self.recent_files.refresh(&self.session.recent);
        let focus = self.focus_target(cx);
        let (file, recent_focus, digits) = self.file_menu(focus.clone(), window, cx);
        let edit = self.edit_menu(focus.clone(), window, cx);
        let view = self.view_menu(focus.clone(), window, cx);
        let branch = file.clone();
        let commands = self.command_availability();
        let menu = PopupMenu::build(window, cx, move |menu, _, _| {
            menu.action_context(focus.clone())
                .item(PopupMenuItem::submenu("File", branch))
                .item(PopupMenuItem::submenu("Edit", edit))
                .item(PopupMenuItem::submenu("View", view))
                .item(PopupMenuItem::separator())
                .item(action_row(
                    "About Argand…",
                    about_ui::ShowAbout,
                    false,
                    focus.clone(),
                ))
                .item(PopupMenuItem::separator())
                .item(action_row(
                    "Quit Argand",
                    Box::new(chrome::CloseWindow),
                    false,
                    focus,
                    commands,
                ))
        });
        self.application_menu_dismissed =
            Some(cx.subscribe_in(&menu, window, Self::application_menu_dismissed));
        self.application_menu_focus = Some(cx.on_focus_lost(window, Self::recover_menu_focus));
        self.application_file_menu = Some(file);
        self.application_recent_focus = recent_focus;
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
        self.application_recent_focus = None;
        self.application_file_rows = None;
        self.application_menu_dismissed = None;
        self.application_menu_focus = None;
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

    /// Hands the keyboard back to the menu when its focused branch stops being drawn.
    ///
    /// The stock menu draws a submenu only while its row is selected, and a hover
    /// moves that selection, so a focused submenu can vanish and take the keys with
    /// it, because a handle missing from the frame routes them to the root.
    fn recover_menu_focus(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(menu) = self.application_menu.clone() else {
            return;
        };
        window.focus(&menu.focus_handle(cx), cx);
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

    /// Opens the recent capture a digit names, but only from File or its Recent files branch.
    ///
    /// The digits belong to the frozen recent rows, so a digit typed while the menu bar
    /// itself holds the keyboard is no recent row and does nothing.
    fn open_recent_row(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let in_file = self
            .application_file_menu
            .as_ref()
            .is_some_and(|file| file.read(cx).focus_handle(cx).is_focused(window));
        let in_recent = self
            .application_recent_focus
            .as_ref()
            .is_some_and(|focus| focus.is_focused(window));
        if !in_file && !in_recent {
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
    /// Covers `area` while the menu is open, leaving the frame's resize edges beyond it live.
    pub(super) fn application_menu_backdrop(
        &self,
        area: gpui_kit::Bounds<Pixels>,
        window: &Window,
    ) -> AnyElement {
        chrome::menu_backdrop(area, chrome::menu_caption_width(window))
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
            .disabled(self.conversion_active())
            .tab_stop(false)
            .custom(toolbar_style(state, cx))
            .when_some(on_surface(state, cx), |button, surface| button.bg(surface))
            .small()
            .w(app_button_width(window, cx))
            .px(px(6.))
            .h(px(TOOLBAR_HEIGHT))
            .child(img("argand/app.png").size(px(22.)))
            .child(div().text_color(cx.theme().foreground).child(TITLE));
        if self.external_tooltips_allowed(cx) {
            app.interactivity().tooltip(move |_, cx| {
                shortcut_tooltip(
                    "Application menu".to_owned(),
                    Some(Box::new(OpenApplicationMenu)),
                    "Shell",
                    px(240.),
                    cx,
                )
            });
        }
        div()
            .id("title-toolbar")
            .flex()
            .items_center()
            .gap_1()
            .flex_shrink_0()
            .on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation())
            .child(title_control(
                "title-app-control",
                self.application_menu_popover(app, cx),
            ))
            // Without a document these act on nothing, and each consumes only
            // its own box, so the gaps between them stay bare title-bar pixels.
            .when(self.view.is_some(), |bar| {
                let separator = div().w(px(1.)).h(px(16.)).mx_1().bg(cx.theme().border);
                let orientation = self.orientation_segments(cx);
                let grid = self.grid_button(cx);
                let selection = self.selection_segments(cx);
                bar.child(separator)
                    .child(title_control("title-orientation-control", orientation))
                    .child(title_control("title-grid-control", grid))
                    .child(title_control("title-selection-control", selection))
            })
    }

    /// The two orientation segments in one frame, with the mode in force selected.
    fn orientation_segments(&self, cx: &mut Context<Self>) -> ButtonGroup {
        let vertical = self.session.orientation.vertical();
        let horizontal = self.orientation_segment(
            "orientation-horizontal",
            Lucide::PanelTop.path(),
            Mode::Horizontal,
            !vertical,
            false,
            cx,
        );
        let vertical = self.orientation_segment(
            "orientation-vertical",
            Lucide::PanelLeft.path(),
            Mode::Vertical,
            vertical,
            true,
            cx,
        );
        ButtonGroup::new("orientation-segments")
            .disabled(
                !self
                    .command_availability()
                    .is_enabled::<ToggleOrientation>(),
            )
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
                    window.dispatch_action(Box::new(ToggleOrientation), cx);
                }
            }))
    }

    /// One orientation segment, which is an action for the mode it names.
    fn orientation_segment(
        &self,
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
        if self.external_tooltips_allowed(cx) {
            segment.interactivity().tooltip(move |_, cx| {
                let action = other.then(|| Box::new(ToggleOrientation) as Box<dyn Action>);
                shortcut_tooltip(hint.clone(), action, "Plot", px(360.), cx)
            });
        }
        segment
    }

    /// What the selection holds, Time, Band or Both, with the selection panel hanging under it.
    ///
    /// A segment selects its kind and opens the panel, or toggles the panel where
    /// it is the kind already selected. Each takes its own click, since the
    /// group's handler hears the mouse only.
    fn selection_segments(&self, cx: &mut Context<Self>) -> AnyElement {
        let current = self.selection_kind();
        let kinds = super::plot::selection_panel::KINDS;
        // The arrows run along the axis they select as the plot is turned.
        let (along_time, across_time) = match self.session.orientation.vertical() {
            false => (Lucide::MoveHorizontal, Lucide::MoveVertical),
            true => (Lucide::MoveVertical, Lucide::MoveHorizontal),
        };
        let icons = [along_time, across_time, Lucide::SquareDashed];
        let segments =
            kinds
                .iter()
                .zip(icons)
                .enumerate()
                .map(|(index, (&(id, label, kind), icon))| {
                    let state = if kind == current {
                        ToolbarState::Selected
                    } else {
                        ToolbarState::Off
                    };
                    let mut segment = Button::new(id)
                        .tab_stop(false)
                        .custom(toolbar_style(state, cx))
                        .selected(kind == current)
                        .rounded(ButtonRounded::Size(px(FRAME_RADIUS - 1.)))
                        .small()
                        .w(px(SEGMENT))
                        .h(px(TOOLBAR_HEIGHT - 2.))
                        .px_0()
                        .child(Self::toolbar_glyph(id, icon.path(), state, cx))
                        .on_click(cx.listener(move |shell, _, window, cx| {
                            shell.choose_selection_kind(kind, window, cx)
                        }));
                    // Painted, because a border takes the colour of the button's own states.
                    if index > 0 {
                        let line = div().absolute().left_0().top_0().bottom_0().w(px(1.));
                        segment = segment.relative().child(line.bg(frame_colour(cx)));
                    }
                    let hint = match label {
                        "Both" => "Select time and band".to_owned(),
                        label => format!("Select {}", label.to_lowercase()),
                    };
                    if self.external_tooltips_allowed(cx) {
                        segment.interactivity().tooltip(move |_, cx| {
                            shortcut_tooltip(hint.clone(), None, "Plot", px(360.), cx)
                        });
                    }
                    segment
                });
        let group = ButtonGroup::new("selection-segments")
            .disabled(self.conversion_active())
            .rounded(px(FRAME_RADIUS))
            .border_1()
            .border_color(frame_colour(cx))
            .h(px(TOOLBAR_HEIGHT))
            .children(segments);
        super::hints::pinned(
            "selection-hint",
            &self.selection_hint,
            GroupTrigger(group),
            gpui_kit::Anchor::TopLeft,
            cx,
        )
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
            .disabled(!self.command_availability().is_enabled::<ToggleGrid>())
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
        if self.external_tooltips_allowed(cx) {
            button.interactivity().tooltip(move |_, cx| {
                shortcut_tooltip(
                    hint.to_owned(),
                    Some(Box::new(ToggleGrid)),
                    "Plot",
                    px(360.),
                    cx,
                )
            });
        }
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

/// The selection segments as the panel's trigger, which the popover may not mark selected.
struct GroupTrigger(ButtonGroup);

impl Selectable for GroupTrigger {
    fn selected(self, _selected: bool) -> Self {
        self
    }

    fn is_selected(&self) -> bool {
        false
    }
}

impl IntoElement for GroupTrigger {
    type Element = AnyElement;

    fn into_element(self) -> Self::Element {
        self.0.into_any_element()
    }

    fn into_any_element(self) -> AnyElement {
        self.0.into_any_element()
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

/// Opens the recent row a digit names, but only from File or its Recent files branch.
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

/// A command row that keeps the stock dispatch and carries the shared keycap.
///
/// The stock menu paints its own keycaps, borderless and transparent, where every
/// other shortcut in the application uses the framed one. The row is an element
/// for that reason alone, and keeps `.action(..)` and `.checked(..)` so the menu
/// still dispatches, checks, selects and navigates it.
fn action_row<A: command::Command>(
    label: &str,
    action: A,
    checked: bool,
    focus: FocusHandle,
    commands: command::Availability,
) -> PopupMenuItem {
    command_row(label, action, checked, focus, None, commands)
}

fn command_row<A: command::Command>(
    label: &str,
    action: A,
    checked: bool,
    focus: FocusHandle,
    shortcut_context: Option<&'static str>,
    commands: command::Availability,
) -> PopupMenuItem {
    let label = label.to_owned();
    let shortcut = action.boxed_clone();
    commands.item(
        action,
        PopupMenuItem::element(move |window, cx| {
            let key = match shortcut_context {
                Some(context) => Kbd::binding_for_action(shortcut.as_ref(), Some(context), window),
                None => Kbd::binding_for_action_in(shortcut.as_ref(), &focus, window),
            };
            div()
                .flex()
                .w_full()
                .min_w_0()
                .gap_3()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_ellipsis_middle()
                        .child(label.clone()),
                )
                .when_some(key, |row, key| row.child(shortcuts::keycap(key, cx)))
        })
        .checked(checked),
    )
}

/// The row one recent capture is drawn in: its digit, then its name.
fn recent_row(number: Option<u8>, label: String, muted: gpui_kit::Hsla) -> impl IntoElement {
    div()
        .flex()
        .w_full()
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
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .text_ellipsis_middle()
                .child(label),
        )
}

/// The width the title reserves for the toolbar.
pub(super) fn toolbar_width(window: &Window, cx: &gpui_kit::App, document: bool) -> Pixels {
    let app = app_button_width(window, cx);
    if !document {
        return app;
    }
    // The two framed segment groups, Grid, the separator and four gaps.
    app + px(SEGMENT * 5. + 4. + TOOLBAR_HEIGHT + 1.) + window.rem_size() * 1.5
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

/// A chrome control's box, which keeps the keyboard where it was when it is
/// pressed with a button other than the left one. The toolkit's own
/// left-press handler prevents the focusing default on the button itself, but
/// the right and middle press have no such handler, and a `tab_stop(false)`
/// button still takes focus from a click. The prevention runs in the capture
/// phase, because the button's automatic focus transfer listens in the bubble
/// phase and would have taken the keyboard before an ordinary handler on this
/// box could answer. The left press is left alone there, because the pressed
/// state that paints the control's active surface settles in a bubble
/// listener gated on the same flag, and preventing the default for it in the
/// capture phase would arrive before that listener and drop the highlight.
pub(super) fn keeps_focus(
    id: &'static str,
    child: impl IntoElement,
) -> gpui_kit::Stateful<gpui_kit::Div> {
    div()
        .id(id)
        .flex()
        .capture_any_mouse_down(|event, window, _| {
            if event.button != gpui_kit::MouseButton::Left {
                window.prevent_default();
            }
        })
        .child(child)
}

/// One toolbar control's box, which consumes its own left press and double
/// click so neither arms the title-bar drag that the bare pixels around it,
/// including the toolbar's gaps, fall through to.
pub(super) fn title_control(
    id: &'static str,
    child: impl IntoElement,
) -> gpui_kit::Stateful<gpui_kit::Div> {
    keeps_focus(id, child)
        .on_mouse_down(MouseButton::Left, |_, _, cx| {
            cx.stop_propagation();
        })
        .on_double_click(|_, _, cx| cx.stop_propagation())
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
pub(super) fn on_surface(state: ToolbarState, cx: &gpui_kit::App) -> Option<gpui_kit::Hsla> {
    (state == ToolbarState::On).then(|| toolbar_accent(cx).opacity(0.30))
}

/// The opaque frame of the segments, so the divider keeps its colour over a selected one.
fn frame_colour(cx: &gpui_kit::App) -> gpui_kit::Hsla {
    cx.theme()
        .title_bar
        .blend(cx.theme().foreground.opacity(0.24))
}

#[cfg(test)]
#[path = "app_menu_ui_test.rs"]
mod tests;
