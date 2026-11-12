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

pub(super) struct About;

fn hint(cx: &mut Context<hints::PinnedHint>) -> hints::PinnedHint {
    hints::PinnedHint::new(move |_, cx| cx.new(|_| About).into(), cx)
}

pub(super) struct State {
    pub(super) hint: Entity<hints::PinnedHint>,
    _events: Subscription,
}

impl State {
    pub(super) fn new(cx: &mut Context<Shell>) -> Self {
        let hint = cx.new(hint);
        let events = cx.subscribe(&hint, |shell, _, event, cx| {
            shell.information_panel_changed(hints::Panel::About, *event, cx);
        });
        Self {
            hint,
            _events: events,
        }
    }
}

impl Render for About {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let link = Link::new("repository-link")
            .accessibility_label("Argand repository")
            .key_context("AboutRepository")
            .href(about::REPOSITORY)
            .open_with(|url, _, _, cx| cx.open_url(url))
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
                    "Version {} · {} · {}",
                    about::VERSION,
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

impl Shell {
    pub(super) fn close_about(&mut self, cx: &mut Context<Self>) {
        self.information_panel_request += 1;
        self.about.hint.update(cx, |hint, cx| hint.close(false, cx));
    }

    pub(super) fn show_about(
        &mut self,
        _: &ShowAbout,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.activate_information_panel(hints::Panel::About, window, cx);
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
            .label(about::VERSION);
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
