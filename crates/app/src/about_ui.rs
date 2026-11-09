//! About uses the same retained pinned surface as the property panels.

use super::*;
use crate::about;
use gpui_kit::base::Link;
use gpui_kit::component::Icon;
use gpui_kit::{Anchor, img};

actions!(about_panel, [ShowAbout]);

pub(super) struct About {
    hint: WeakEntity<hints::PinnedHint>,
}

pub(super) fn hint(cx: &mut Context<hints::PinnedHint>) -> hints::PinnedHint {
    let hint = cx.weak_entity();
    hints::PinnedHint::new(
        move |_, cx| cx.new(|_| About { hint: hint.clone() }).into(),
        cx,
    )
}

impl Render for About {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let clicked = self.hint.clone();
        let linked = self.hint.clone();
        let other_click = self.hint.clone();
        let link = Link::new("about-repository")
            .accessibility_label("Argand repository")
            .href(about::REPOSITORY)
            .open_with(|url, _, _, cx| cx.open_url(url))
            .on_activate(move |_, _, cx| close(&linked, cx))
            .text_color(cx.theme().link)
            .underline()
            .cursor_pointer()
            .focus_visible(|style| {
                style
                    .bg(cx.theme().accent)
                    .text_color(cx.theme().accent_foreground)
            })
            .child(about::REPOSITORY);
        div()
            .id("about-panel")
            .w_96()
            .max_w((window.viewport_size().width - window.rem_size() * 3.).max(px(0.)))
            .px_3()
            .py_2()
            .flex()
            .flex_col()
            .gap_2()
            .text_sm()
            .font_family(cx.theme().font_family.clone())
            .text_color(cx.theme().popover_foreground)
            .bg(crate::theme::sheet(cx))
            .border_1()
            .border_color(cx.theme().border)
            .rounded(cx.theme().radius)
            .shadow_md()
            .on_click(move |_, _, cx| close(&clicked, cx))
            .on_any_mouse_down(move |event, _, cx| {
                if event.button != MouseButton::Left {
                    close(&other_click, cx);
                }
            })
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(img("argand/about.png").size_12())
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(div().font_weight(FontWeight::SEMIBOLD).child(about::NAME))
                            .child(format!("Version {}", about::VERSION)),
                    ),
            )
            .child(about::DESCRIPTION)
            .child(settings_ui::detail_row("Licence", about::LICENSE, cx))
            .child(settings_ui::detail_row("Tag", about::TAG, cx))
            .child(settings_ui::detail_row("Commit", about::HASH, cx))
            .child(settings_ui::detail_row("Commit date", about::DATE, cx))
            .child(link)
    }
}

fn close(hint: &WeakEntity<hints::PinnedHint>, cx: &mut gpui_kit::App) {
    let _ = hint.update(cx, |hint, cx| hint.close(false, cx));
}

impl Shell {
    pub(super) fn close_about(&mut self, cx: &mut Context<Self>) {
        self.about_request += 1;
        self.about_hint.update(cx, |hint, cx| hint.close(false, cx));
    }

    pub(super) fn show_about(
        &mut self,
        _: &ShowAbout,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.about_hint.read(cx).is_open() {
            return;
        }
        self.close_hints(cx);
        self.dismiss_application_menu(window, cx);
        self.interrupt_plot(cx);
        let request = self.about_request;
        let owner = cx.entity().downgrade();
        window.defer(cx, move |window, cx| {
            let _ = owner.update(cx, |shell, cx| {
                if shell.about_request == request {
                    shell
                        .about_hint
                        .update(cx, |hint, cx| hint.open(window, cx));
                }
            });
        });
    }

    pub(super) fn about_changed(
        &mut self,
        _: &Entity<hints::PinnedHint>,
        event: &hints::Pinned,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            hints::Pinned::Opened => {
                self.close_analysis_hint(cx);
                self.close_selection_hint(cx);
                self.conversion
                    .hint
                    .update(cx, |hint, cx| hint.close(false, cx));
                self.dismiss_application_menu(window, cx);
                self.interrupt_plot(cx);
            }
            hints::Pinned::Closed { .. } => {
                let focus = self.about_hint.read(cx).focus_handle();
                if focus.contains_focused(window, cx) {
                    window.focus(&self.focus_target(cx), cx);
                }
            }
        }
        cx.notify();
    }

    pub(super) fn about_button(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let open = self.about_hint.read(cx).is_open();
        let state = if open {
            app_menu_ui::ToolbarState::On
        } else {
            app_menu_ui::ToolbarState::Off
        };
        let mut button = Button::new("about-button")
            .accessibility_label("About Argand")
            .tab_stop(false)
            .small()
            .w_7()
            .h_7()
            .custom(app_menu_ui::toolbar_style(state, cx))
            .when_some(app_menu_ui::on_surface(state, cx), |button, surface| {
                button.bg(surface)
            })
            .child(Icon::new(gpui_kit::assets::IconName::Info).small())
            .on_click(cx.listener(|shell, _, window, cx| shell.show_about(&ShowAbout, window, cx)));
        button
            .interactivity()
            .tooltip(|_, cx| hints::passive(cx, |_| Tooltip::new(about::tooltip())));
        hints::pinned(
            "about-popover",
            &self.about_hint,
            button,
            Anchor::TopLeft,
            cx,
        )
    }
}

#[cfg(test)]
#[path = "about_ui_tests.rs"]
mod tests;
