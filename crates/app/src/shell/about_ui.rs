//! About uses the same retained pinned surface as the property panels.

use super::*;
use crate::about;
use gpui_kit::base::Link;
use gpui_kit::component::{Icon, Selectable};
use gpui_kit::{Anchor, AnyElement, img, relative};

actions!(about_panel, [ShowAbout]);

pub(super) fn init(cx: &mut gpui_kit::App) {
    // Let the standard Link handle Enter on key-up instead of confirming the popover.
    cx.bind_keys([KeyBinding::new(
        "enter",
        gpui_kit::NoAction,
        Some("AboutRepository"),
    )]);
}

pub(super) struct About {
    hint: WeakEntity<hints::PinnedHint>,
}

fn hint(cx: &mut Context<hints::PinnedHint>) -> hints::PinnedHint {
    let hint = cx.weak_entity();
    hints::PinnedHint::new(
        move |_, cx| cx.new(|_| About { hint: hint.clone() }).into(),
        cx,
    )
}

pub(super) struct State {
    pub(super) hint: Entity<hints::PinnedHint>,
    pub(super) request: u64,
    _events: Subscription,
    _focus: Subscription,
}

impl State {
    pub(super) fn new(window: &mut Window, cx: &mut Context<Shell>) -> Self {
        let hint = cx.new(hint);
        let events = cx.subscribe_in(&hint, window, Shell::about_changed);
        let focus = cx.on_focus_out(&hint.read(cx).focus_handle(), window, |shell, _, _, cx| {
            shell.close_about(cx)
        });
        Self {
            hint,
            request: 0,
            _events: events,
            _focus: focus,
        }
    }
}

impl Render for About {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let clicked = self.hint.clone();
        let linked = self.hint.clone();
        let other_click = self.hint.clone();
        let link = Link::new("repository-link")
            .accessibility_label("Argand repository")
            .key_context("AboutRepository")
            .href(about::REPOSITORY)
            .open_with(|url, _, _, cx| cx.open_url(url))
            .on_activate(move |_, _, cx| close(&linked, cx))
            .text_color(cx.theme().link)
            .underline()
            .cursor_pointer()
            .text_xs()
            .focus_visible(|style| {
                style
                    .bg(cx.theme().accent)
                    .text_color(cx.theme().accent_foreground)
            })
            .child(about::REPOSITORY);
        div()
            .id("about-panel")
            .map(gpui_kit::TestSupportExt::test_support)
            .w_80()
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
                    .items_start()
                    .gap_3()
                    .child(img("argand/about.png").size_12())
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .h_12()
                            // Match glyph bearings to the artwork’s transparent inset.
                            .py(px(1.))
                            .justify_between()
                            .child(
                                div()
                                    .text_lg()
                                    .line_height(relative(1.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(about::NAME),
                            )
                            .child(
                                div()
                                    .relative()
                                    .top(-px(1.))
                                    .line_height(relative(1.))
                                    .text_color(cx.theme().muted_foreground)
                                    .child(format!("Version {}", about::VERSION)),
                            ),
                    ),
            )
            .child(about::DESCRIPTION)
            .child(settings_ui::detail_row("Licence", about::LICENSE, cx))
            .child(settings_ui::detail_row("Tag", about::TAG, cx))
            .child(settings_ui::detail_row("Commit", about::HASH, cx))
            .child(settings_ui::detail_row(
                "Commit date",
                about::display_date(about::DATE),
                cx,
            ))
            .child(
                div()
                    .id("about-repository")
                    .map(gpui_kit::TestSupportExt::test_support)
                    .flex()
                    .justify_end()
                    .child(link),
            )
    }
}

pub(super) fn button_width(window: &Window, cx: &gpui_kit::App) -> Pixels {
    let style = gpui_kit::TextStyle {
        font_family: cx.theme().font_family.clone(),
        ..Default::default()
    };
    let label = window.text_system().shape_line(
        about::VERSION.into(),
        window.rem_size() * 0.75,
        &[style.to_run(about::VERSION.len())],
        None,
    );
    // Tag, content gap and the button's horizontal padding.
    label.width.ceil() + window.rem_size() * 2.
}

fn tooltip_content(window: &mut Window, cx: &mut gpui_kit::App) -> AnyElement {
    div()
        .id("about-tooltip")
        .map(gpui_kit::TestSupportExt::test_support)
        .w_64()
        .max_w((window.viewport_size().width - window.rem_size() * 3.).max(px(0.)))
        .whitespace_normal()
        .flex()
        .flex_col()
        .gap_1()
        .child("About Argand")
        .child(
            div()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child(format!(
                    "{} · {} · {}",
                    about::title(),
                    about::TAG,
                    about::HASH
                )),
        )
        .child(
            div()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child(about::display_date(about::DATE)),
        )
        .into_any_element()
}

fn close(hint: &WeakEntity<hints::PinnedHint>, cx: &mut gpui_kit::App) {
    let _ = hint.update(cx, |hint, cx| hint.close(false, cx));
}

impl Shell {
    pub(super) fn close_about(&mut self, cx: &mut Context<Self>) {
        self.about.request += 1;
        self.about.hint.update(cx, |hint, cx| hint.close(false, cx));
    }

    pub(super) fn show_about(
        &mut self,
        _: &ShowAbout,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.about.hint.read(cx).is_open() {
            return;
        }
        self.close_hints(cx);
        self.dismiss_application_menu(window, cx);
        self.interrupt_plot(cx);
        let request = self.about.request;
        let owner = cx.entity().downgrade();
        window.defer(cx, move |window, cx| {
            let _ = owner.update(cx, |shell, cx| {
                if shell.about.request == request {
                    shell
                        .about
                        .hint
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
                self.suppress_external_tooltips();
                self.close_analysis_hint(cx);
                self.close_selection_hint(cx);
                self.conversion
                    .hint
                    .update(cx, |hint, cx| hint.close(false, cx));
                self.dismiss_application_menu(window, cx);
                self.interrupt_plot(cx);
            }
            hints::Pinned::Closed { .. } => {
                let focus = self.about.hint.read(cx).focus_handle();
                if focus.contains_focused(window, cx) {
                    window.focus(&self.focus_target(cx), cx);
                }
            }
        }
        cx.notify();
    }

    pub(super) fn about_button(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let open = self.about.hint.read(cx).is_open();
        let mut button = Button::new("about-button")
            .accessibility_label("About Argand")
            .tab_stop(false)
            .xsmall()
            .h_7()
            .ghost()
            .open(open)
            .px_2()
            .font_weight(FontWeight::NORMAL)
            .text_color(cx.theme().muted_foreground)
            .icon(Icon::new(gpui_kit::assets::IconName::Tag))
            .label(about::VERSION)
            .on_click(cx.listener(|shell, _, window, cx| shell.show_about(&ShowAbout, window, cx)));
        if self.external_tooltips_allowed(cx) {
            button
                .interactivity()
                .tooltip(move |_, cx| hints::passive(cx, |_| Tooltip::element(tooltip_content)));
        }
        hints::pinned(
            "about-popover",
            &self.about.hint,
            button,
            Anchor::TopRight,
            cx,
        )
    }
}

#[cfg(test)]
#[path = "about_ui_test.rs"]
mod tests;
