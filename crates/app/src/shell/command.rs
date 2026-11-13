//! Shared availability for commands reached through menus and keyboard dispatch.

use super::plot::navigation_ui::*;
use super::*;

pub(super) trait Command: Action {
    fn is_enabled(availability: Availability) -> bool;
}

macro_rules! commands {
    ($($action:ty => |$state:ident| $enabled:expr),+ $(,)?) => {$ (
        impl Command for $action {
            fn is_enabled($state: Availability) -> bool {
                $state.idle && $enabled
            }
        }
    )+};
}

commands! {
    ChooseFile => |s| true,
    Save => |s| s.save, SaveAs => |s| s.save_as, SaveSelectionAs => |s| s.save_selection,
    EditAnalysis => |s| s.file, ConvertCapture => |s| s.convert,
    editing_ui::Undo => |s| s.edit.undo, editing_ui::Redo => |s| s.edit.redo,
    editing_ui::CutSelection => |s| s.edit.cut, editing_ui::CopySelection => |s| s.edit.cut,
    editing_ui::PasteClipboard => |s| s.edit.replace, editing_ui::PasteHere => |s| s.edit.paste,
    editing_ui::DeleteSelection => |s| s.edit.cut, editing_ui::SuppressBand => |s| s.edit.suppress,
    editing_ui::InvertSpectrum => |s| s.edit.invert, editing_ui::NormalizeLevel => |s| s.edit.normalize,
    editing_ui::EditSelection => |s| s.selection,
    ToggleGrid => |s| true,
    ToggleScaleUi => |s| true, ToggleOrientation => |s| true,
    FitCapture => |s| s.document, FitFrequency => |s| s.document,
    ClockRuler => |s| true, SecondsRuler => |s| true, SamplesRuler => |s| true,
    SystemTheme => |s| true, LightTheme => |s| true, DarkTheme => |s| true,
    about_ui::ShowAbout => |s| true,
}

/// A presentation snapshot. Execution obtains a fresh snapshot from the owner.
#[derive(Clone, Copy, Default)]
pub(super) struct Availability {
    pub idle: bool,
    pub document: bool,
    file: bool,
    save: bool,
    save_as: bool,
    save_selection: bool,
    convert: bool,
    selection: bool,
    edit: editing_ui::EditCommands,
}

impl Availability {
    #[cfg(test)]
    pub(super) fn for_plot_test() -> Self {
        Self {
            idle: true,
            document: true,
            ..Self::default()
        }
    }

    pub(super) fn is_enabled<A: Command>(self) -> bool {
        A::is_enabled(self)
    }

    pub(super) fn item<A: Command>(self, action: A, item: PopupMenuItem) -> PopupMenuItem {
        item.disabled(!self.is_enabled::<A>())
            .action(Box::new(action))
    }
}

impl Shell {
    pub(super) fn command_availability(&self) -> Availability {
        Availability {
            idle: !self.conversion_active(),
            document: self.view.is_some(),
            file: self.file.is_some(),
            save: self.can_save_edits(),
            save_as: self.can_save(false),
            save_selection: self.can_save(true),
            convert: self.can_save(false) && !self.capture_changed(),
            selection: self.entry_scale().is_some(),
            edit: self.edit_commands_available(),
        }
    }
}

/// A focused handler and a menu row use the same typed command policy.
pub(super) fn listener<A: Command>(
    cx: &Context<Shell>,
    handler: impl Fn(&mut Shell, &A, &mut Window, &mut Context<Shell>) + 'static,
) -> impl Fn(&A, &mut Window, &mut gpui_kit::App) + 'static {
    cx.listener(move |shell, action: &A, window, cx| {
        if shell.command_availability().is_enabled::<A>() {
            handler(shell, action, window, cx);
        }
    })
}

/// Popup focus can require app dispatch; check again after its deferred delivery.
pub(super) fn bind<A: Command>(
    shell: WeakEntity<Shell>,
    cx: &mut gpui_kit::App,
    handler: fn(&mut Shell, &mut Window, &mut Context<Shell>),
) {
    cx.on_action(move |_: &A, cx| {
        let shell = shell.clone();
        cx.defer(move |cx| {
            let _ = shell.update_in(cx, |shell, window, cx| {
                if shell.command_availability().is_enabled::<A>() {
                    handler(shell, window, cx);
                }
            });
        });
    });
}

/// Plot navigation keeps its focus owner and uses the Shell-supplied policy.
pub(super) fn plot_listener<A: Command>(
    cx: &Context<plot::plot_view::PlotView>,
    handler: impl Fn(
        &mut plot::plot_view::PlotView,
        &A,
        &mut Window,
        &mut Context<plot::plot_view::PlotView>,
    ) + 'static,
) -> impl Fn(&A, &mut Window, &mut gpui_kit::App) + 'static {
    cx.listener(move |plot, action: &A, window, cx| {
        if plot
            .snapshot
            .as_ref()
            .is_some_and(|s| s.commands.is_enabled::<A>())
        {
            handler(plot, action, window, cx);
        }
    })
}

/// Register a shortcut only for a command with the shared execution policy.
pub(super) fn binding<A: Command>(
    key: &'static str,
    action: A,
    context: Option<&str>,
) -> KeyBinding {
    KeyBinding::new(key, action, context)
}

#[cfg(test)]
#[path = "command_test.rs"]
mod tests;
