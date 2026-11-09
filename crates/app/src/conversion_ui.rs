//! File information, conversion draft, and the lifetime of unsaved conversion results.

use super::numeric_editor::{self, NumericEditor};
use super::property_advice::{self, Advice};
use super::settings_ui::editor::{self as presentation, Name};
use super::*;
use crate::conversion::{Converted, Level, Request};
use crate::saving::{Ended, Update};
use gpui_kit::TestSupportExt;
use gpui_kit::base::actions::Confirm;
use gpui_kit::component::IconName;
use gpui_kit::component::input::InputState;
use gpui_kit::component::searchable_list::{
    SearchableGroup, SearchableListDelegate, SearchableListItem, SearchableVec,
};
use gpui_kit::component::select::{SelectEvent, SelectState};
use gpui_kit::{AnyElement, Focusable};
use std::rc::Rc;

type SampleChoices = SearchableVec<SearchableGroup<SampleChoice>>;

#[derive(Clone)]
struct SampleChoice {
    title: String,
}

impl SearchableListItem for SampleChoice {
    type Value = String;

    fn title(&self) -> gpui_kit::SharedString {
        self.title.clone().into()
    }

    fn render(&self, _: &mut Window, _: &mut gpui_kit::App) -> impl IntoElement {
        let label = self
            .title
            .split_once(" · ")
            .map_or(self.title.as_str(), |(_, format)| format)
            .to_owned();
        div()
            .id(gpui_kit::SharedString::from(format!(
                "sample-format-{}",
                self.title
            )))
            .test_support()
            .child(label)
    }

    fn value(&self) -> &String {
        &self.title
    }
}

fn flac_supports(format: argand_core::SampleFormat) -> bool {
    matches!(
        format,
        argand_core::SampleFormat::U8 | argand_core::SampleFormat::I16
    )
}

fn sample_type_for_label(label: &str) -> Option<argand_core::SampleType> {
    argand_core::SAMPLE_TYPE_TOKENS
        .iter()
        .find(|token| kind_label(token) == label)
        .and_then(|token| token.parse().ok())
}

fn sample_choices(flac: bool) -> SampleChoices {
    SearchableVec::new(
        [("Real", "rl_"), ("I/Q", "iq_")]
            .into_iter()
            .map(|(title, prefix)| {
                SearchableGroup::new(title).items(
                    argand_core::SAMPLE_TYPE_TOKENS
                        .iter()
                        .filter(|token| token.starts_with(prefix))
                        .filter(|token| {
                            !flac
                                || token
                                    .parse::<argand_core::SampleType>()
                                    .is_ok_and(|sample_type| flac_supports(sample_type.format))
                        })
                        .map(|token| SampleChoice {
                            title: kind_label(token),
                        }),
                )
            })
            .collect::<Vec<_>>(),
    )
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum AfterCancel {
    KeepDocument,
    Close,
}

pub(super) struct State {
    pub(super) running: Option<Converting>,
    pub(super) updates: Option<Task<()>>,
    pub(super) pending: Option<PendingConversion>,
    next_reader: u64,
    after_cancel: Option<AfterCancel>,
    #[cfg(test)]
    fail_reader_open: bool,
    pub(super) hint: Entity<hints::PinnedHint>,
    _subscription: Subscription,
    result: Option<String>,
}

impl State {
    pub(super) fn new(cx: &mut Context<Shell>) -> Self {
        let owner = cx.entity().downgrade();
        let hint = cx.new(|cx| {
            hints::PinnedHint::new(move |window, cx| panel(owner.clone(), window, cx), cx)
        });
        let subscription = cx.subscribe(&hint, |shell, _, event, cx| {
            shell.status_panel_changed(settings_ui::StatusPanel::File, *event, cx);
        });
        Self {
            running: None,
            updates: None,
            pending: None,
            next_reader: 0,
            after_cancel: None,
            #[cfg(test)]
            fail_reader_open: false,
            hint,
            _subscription: subscription,
            result: None,
        }
    }
}

pub(super) struct ReaderRoute {
    token: u64,
    adopted: Arc<std::sync::atomic::AtomicU64>,
}

pub(super) struct PendingConversion {
    token: u64,
    job: Converting,
    origin: Origin,
    readers: replace_ui::Readers,
    adopted: Arc<std::sync::atomic::AtomicU64>,
}

pub(super) struct Converting {
    job: crate::saving::Job,
    progress: Option<(u64, u64)>,
    document: u64,
    version: u64,
    name: String,
    directory: std::path::PathBuf,
    output_is_iq: bool,
    container: argand_io::write::Container,
    backing: Arc<crate::scratch::Scratch>,
}

fn kind_label(token: &str) -> String {
    let (domain, format) = token.split_once('_').unwrap_or(("", token));
    let domain = if domain == "iq" { "I/Q" } else { "Real" };
    let format = match format {
        "u8" => "Int 8-bit",
        "i16" => "Int 16-bit",
        "i32" => "Int 32-bit",
        "f32" => "Float 32-bit (normalized)",
        "f16x8" => "Float 32-bit (unscaled)",
        _ => format,
    };
    format!("{domain} · {format}")
}

fn clipping_notice(count: u64, iq: bool) -> Option<String> {
    (count > 0).then(|| {
        let counted = if iq {
            "I/Q component values (I and Q counted separately)"
        } else {
            "samples"
        };
        format!(
            "Conversion clipped {} {counted}",
            crate::numbers::number(count)
        )
    })
}

fn converted_name(name: &str, container: argand_io::write::Container) -> String {
    let stem = std::path::Path::new(name)
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy();
    format!(
        "{}-converted.{}",
        stem.trim_end_matches("-converted"),
        container.extension()
    )
}

struct Editor {
    request: Request,
    rate: Entity<InputState>,
    rate_editor: Entity<NumericEditor>,
    reference: Entity<InputState>,
    reference_editor: Entity<NumericEditor>,
    reference_iq: bool,
    container: Entity<SelectState<Vec<Name>>>,
    container_widths: Vec<Pixels>,
    kind: Entity<SelectState<SampleChoices>>,
    kind_widths: Vec<Pixels>,
    unavailable_kind: Option<String>,
    format_warning: bool,
    format_advice: Advice,
    closed_selects: [FocusHandle; 2],
    _subscriptions: Vec<Subscription>,
    owner: Option<WeakEntity<Shell>>,
    document: Option<u64>,
    version: u64,
    details: Vec<(String, String)>,
    focus_on_open: bool,
    scope: FocusHandle,
}

fn frequency_editor(
    value: f64,
    positive: bool,
    ids: (&'static str, &'static str),
    window: &mut Window,
    cx: &mut gpui_kit::App,
) -> Entity<NumericEditor> {
    cx.new(|cx| {
        NumericEditor::new(
            crate::numbers::number(value),
            numeric_editor::Config {
                unit: "Hz",
                step: 1.,
                minimum: 0.,
                maximum: f64::MAX,
                minimum_inclusive: !positive,
                down: ids.0,
                up: ids.1,
            },
            window,
            cx,
        )
    })
}

impl Editor {
    fn new(request: Request, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let rate_editor = frequency_editor(
            request.output.sample_rate,
            true,
            ("rate-down", "rate-up"),
            window,
            cx,
        );
        let reference_editor = frequency_editor(
            request.output.center_freq,
            false,
            ("reference-down", "reference-up"),
            window,
            cx,
        );
        reference_editor.update(cx, |editor, cx| {
            editor.set_disabled(request.output.is_iq() == request.input.is_iq(), cx)
        });
        let rate = rate_editor.read(cx).input.clone();
        let reference = reference_editor.read(cx).input.clone();
        let kinds: Vec<String> = argand_core::SAMPLE_TYPE_TOKENS
            .iter()
            .map(|s| kind_label(s))
            .collect();
        let kind_widths = presentation::shown_widths(&kinds, window, cx);
        let groups = sample_choices(request.container == argand_io::write::Container::Flac);
        let initial_kind = kind_label(&request.output.sample_type.to_string());
        let selected = groups.position(&initial_kind);
        let unavailable_kind = selected.is_none().then_some(initial_kind);
        let kind = cx.new(|cx| SelectState::new(groups, selected, window, cx));
        let containers = vec!["WAV".to_owned(), "FLAC".to_owned()];
        let container_widths = presentation::shown_widths(&containers, window, cx);
        let selected = usize::from(request.container == argand_io::write::Container::Flac);
        let container = cx.new(|cx| {
            SelectState::new(
                containers.into_iter().map(Name).collect::<Vec<_>>(),
                Some(gpui_kit::component::IndexPath::new(selected)),
                window,
                cx,
            )
        });
        let closed_selects = [
            container.read(cx).focus_handle(cx),
            kind.read(cx).focus_handle(cx),
        ];
        let subscriptions = vec![
            cx.observe(&kind, |_, _, cx| cx.notify()),
            cx.observe(&container, |_, _, cx| cx.notify()),
            cx.subscribe(&rate_editor, |_, _, _: &numeric_editor::Changed, cx| {
                cx.notify()
            }),
            cx.subscribe(
                &reference_editor,
                |_, _, _: &numeric_editor::Changed, cx| cx.notify(),
            ),
            cx.subscribe_in(
                &container,
                window,
                |editor, _, _: &SelectEvent<Vec<Name>>, window, cx| {
                    editor.format_warning = false;
                    editor.filter_kinds(window, cx);
                    cx.notify();
                },
            ),
            cx.subscribe_in(
                &kind,
                window,
                |editor, _, _: &SelectEvent<SampleChoices>, window, cx| {
                    editor.format_warning = false;
                    if editor.kind.read(cx).selected_value().is_some() {
                        editor.unavailable_kind = None;
                    }
                    editor.sync_reference(window, cx);
                    cx.notify();
                },
            ),
        ];
        Self {
            closed_selects,
            reference_iq: request.output.is_iq(),
            request,
            container,
            container_widths,
            rate,
            rate_editor,
            reference,
            reference_editor,
            kind,
            kind_widths,
            unavailable_kind,
            format_warning: false,
            format_advice: Advice::default(),
            _subscriptions: subscriptions,
            owner: None,
            document: None,
            version: 0,
            details: Vec::new(),
            focus_on_open: true,
            scope: cx.focus_handle(),
        }
    }

    fn traverse(&self, forward: bool, window: &mut Window, cx: &mut Context<Self>) {
        numeric_editor::traverse(
            &self.scope,
            &[self.rate_editor.clone(), self.reference_editor.clone()],
            7,
            forward,
            window,
            cx,
        );
        cx.notify();
    }

    fn prepare(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if std::mem::take(&mut self.focus_on_open) {
            let focus = self.rate.read(cx).focus_handle(cx);
            window.defer(cx, move |window, cx| window.focus(&focus, cx));
        }
        if let Some(owner) = &self.owner
            && let Ok(Some(rows)) = owner.read_with(cx, |shell, _| {
                let file = shell.file.as_ref()?;
                (Some(file.id) == self.document)
                    .then(|| {
                        file.state
                            .document
                            .file_summary()
                            .map(|summary| summary.hint.rows)
                    })
                    .flatten()
            })
        {
            self.details
                .retain(|(label, _)| label != "Peak" && label != "RMS");
            self.details.extend(
                rows.into_iter()
                    .filter(|(label, _)| label == "Peak" || label == "RMS"),
            );
        }
    }

    fn filter_kinds(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let mut value = self.kind_value(cx).map(str::to_owned);
        let flac = self
            .container
            .read(cx)
            .selected_value()
            .is_some_and(|value| value == "FLAC");
        if flac
            && let Some(sample_type) = value.as_deref().and_then(sample_type_for_label)
            && !flac_supports(sample_type.format)
        {
            let fallback =
                argand_core::SampleType::new(sample_type.domain, argand_core::SampleFormat::I16);
            value = Some(kind_label(&fallback.to_string()));
            self.format_warning = true;
            self.format_advice.reset();
        }
        self.kind.update(cx, |kind, cx| {
            kind.set_items(sample_choices(flac), window, cx);
            if let Some(value) = &value {
                kind.set_selected_value(value, window, cx);
            }
        });
        self.unavailable_kind = value.filter(|_| self.kind.read(cx).selected_value().is_none());
    }

    fn kind_value<'a>(&'a self, cx: &'a gpui_kit::App) -> Option<&'a str> {
        self.kind
            .read(cx)
            .selected_value()
            .map(String::as_str)
            .or(self.unavailable_kind.as_deref())
    }

    fn sync_reference(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let iq = self
            .kind_value(cx)
            .is_some_and(|value| value.starts_with("I/Q"));
        if iq == self.reference_iq {
            return;
        }
        self.reference_iq = iq;
        let locked = iq == self.request.input.is_iq();
        let source = &self.request.input;
        let (rate, reference) = if locked {
            (source.sample_rate, source.center_freq)
        } else if source.is_iq() {
            (source.sample_rate * 2.0, source.sample_rate / 2.0)
        } else {
            (source.sample_rate / 2.0, source.sample_rate / 4.0)
        };
        self.rate.update(cx, |input, cx| {
            input.set_value(crate::numbers::number(rate), window, cx)
        });
        self.reference.update(cx, |input, cx| {
            input.set_value(crate::numbers::number(reference), window, cx)
        });
        self.reference_editor
            .update(cx, |editor, cx| editor.set_disabled(locked, cx));
    }

    fn request(&self, cx: &gpui_kit::App) -> Result<Request, String> {
        let mut request = self.request.clone();
        request.output.sample_rate = crate::numbers::parse(self.rate.read(cx).value().as_ref())
            .map_err(|_| "Enter a positive finite sample rate in Hz".to_string())?;
        let selected = self.kind_value(cx).ok_or("Choose a sample type")?;
        request.output.sample_type =
            sample_type_for_label(selected).ok_or("Choose a supported sample type")?;
        request.output.center_freq = if request.output.is_iq() == request.input.is_iq() {
            request.carrier = None;
            request.input.center_freq
        } else {
            let carrier = crate::numbers::parse(self.reference.read(cx).value().as_ref())
                .map_err(|_| "Enter a finite reference carrier in Hz".to_string())?;
            request.carrier = Some(carrier);
            crate::conversion::validate_carrier(&request.input, &request.output, carrier)?;
            0.0
        };
        request.container = match self.container.read(cx).selected_value().map(String::as_str) {
            Some("WAV") => argand_io::write::Container::Wave,
            Some("FLAC") => argand_io::write::Container::Flac,
            _ => return Err("Choose a file type".into()),
        };
        request.level = Level::Preserve;
        request.plan().map_err(|error| error.to_string())?;
        Ok(request)
    }

    fn changed(&self, request: &Request) -> bool {
        request.container != self.request.container
            || request.output.sample_rate != request.input.sample_rate
            || request.output.center_freq != request.input.center_freq
            || request.output.sample_type != request.input.sample_type
    }

    fn convert_button(&self, request: Option<Request>, cx: &mut Context<Self>) -> Button {
        presentation::outlined_button(
            "convert-capture",
            "conversion-action",
            "Convert",
            false,
            request.is_some(),
            cx,
        )
        .when(request.is_none(), |button| button.opacity(0.5))
        .on_click(cx.listener(move |editor, _, window, cx| {
            let Some(request) = request.clone() else {
                return;
            };
            editor.format_warning = false;
            let Some(owner) = &editor.owner else {
                return;
            };
            let document = editor.document;
            let version = editor.version;
            let _ = owner.update(cx, |shell, cx| {
                shell.convert_draft(request, document, version, window, cx)
            });
        }))
    }

    fn format_control(
        &self,
        tips: bool,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let choice = presentation::choice_control(
            "conversion-kind",
            &self.kind,
            &self.kind_widths,
            self.unavailable_kind.as_deref(),
            window,
            cx,
        )
        .into_any_element();
        let value = if self.format_warning {
            let warning = "Sample format changed to Int 16-bit for FLAC";
            let edge = property_advice::color(cx);
            self.format_advice.render(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(
                        div()
                            .text_color(cx.theme().popover_foreground)
                            .child(choice),
                    )
                    .child("⚠"),
                property_advice::Content {
                    anchor: "conversion-format-advice",
                    balloon: "conversion-format-balloon",
                    close: "conversion-format-balloon-close",
                    pointer: "conversion-format-balloon-pointer",
                    warning: warning.into(),
                    edge,
                    action: None,
                    hover: Rc::new(move |_, cx| {
                        metadata_tooltip(
                            MetadataHint {
                                title: "Sample format",
                                value: warning.into(),
                                explanation: String::new(),
                                rows: Vec::new(),
                            },
                            cx,
                        )
                    }),
                },
                !tips,
                cx.entity_id(),
                window,
                cx,
            )
        } else {
            div()
                .id("conversion-format-hint")
                .when(tips, |control| {
                    control.tooltip(|_, cx| {
                        metadata_tooltip(
                            MetadataHint {
                                title: "Sample format",
                                value: "Normalized: nominal −1…+1 scale".into(),
                                explanation: "Unscaled: arbitrary sample scale".into(),
                                rows: Vec::new(),
                            },
                            cx,
                        )
                    })
                })
                .child(choice)
                .into_any_element()
        };
        presentation::row("Sample format", value, cx)
    }

    fn reference_control(&self, tips: bool, cx: &mut Context<Self>) -> impl IntoElement {
        let carrier = self.reference_iq != self.request.input.is_iq();
        div()
            .id("conversion-reference-hint")
            .when(tips, |control| {
                control.tooltip(move |_, cx| {
                    metadata_tooltip(
                        MetadataHint {
                            title: "Reference",
                            value: if carrier {
                                "Carrier frequency in the real signal"
                            } else {
                                "Physical frequency of array zero"
                            }
                            .into(),
                            explanation: if carrier {
                                "The frequency where I/Q zero is placed"
                            } else {
                                "Fixed while the signal domain is unchanged"
                            }
                            .into(),
                            rows: Vec::new(),
                        },
                        cx,
                    )
                })
            })
            .child(presentation::row(
                "Reference",
                self.reference_editor.clone(),
                cx,
            ))
    }

    fn summary(&self, cx: &gpui_kit::App) -> Result<String, String> {
        let request = self.request(cx)?;
        let plan = request.plan().map_err(|error| error.to_string())?;
        if !plan.retains_input_band() {
            return Ok(format!(
                "Input frequencies outside {} … {} Hz will be discarded",
                crate::numbers::number(plan.local_input_band().0),
                crate::numbers::number(plan.local_input_band().1)
            ));
        }
        Ok(String::new())
    }
}

impl Render for Editor {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.prepare(window, cx);
        let tips = !presentation::select_open(&self.container, &self.closed_selects[0], cx)
            && !presentation::select_open(&self.kind, &self.closed_selects[1], cx);
        let summary = self.summary(cx);
        let request = self
            .request(cx)
            .ok()
            .filter(|request| self.changed(request));
        div()
            .id("file-information-panel")
            .relative()
            .child(self.format_advice.sheet(cx.entity_id()))
            .key_context("FileInformation")
            .track_focus(&self.scope)
            .on_action(cx.listener(|editor, _: &FocusNext, window, cx| {
                editor.traverse(true, window, cx);
            }))
            .on_action(cx.listener(|editor, _: &FocusPrevious, window, cx| {
                editor.traverse(false, window, cx);
            }))
            .on_action(|_: &Confirm, _, _| {})
            .flex()
            .flex_col()
            .gap_1()
            .w(presentation::property_width(window))
            .px_3()
            .py_2()
            .text_sm()
            .font_family(cx.theme().font_family.clone())
            .text_color(cx.theme().popover_foreground)
            .max_h(window.viewport_size().height - px(48.))
            .overflow_y_scroll()
            .bg(crate::theme::sheet(cx))
            .border_1()
            .border_color(cx.theme().border)
            .rounded(cx.theme().radius)
            .shadow_md()
            .child(
                div()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("File information"),
            )
            .children(
                self.details.iter().map(|(label, value)| {
                    settings_ui::detail_row(label.clone(), value.clone(), cx)
                }),
            )
            .child(div().border_t_1().border_color(cx.theme().border))
            .child(presentation::row(
                "Sample rate",
                self.rate_editor.clone(),
                cx,
            ))
            .child(self.reference_control(tips, cx))
            .child(presentation::row(
                "File type",
                presentation::choice_control(
                    "conversion-container",
                    &self.container,
                    &self.container_widths,
                    None,
                    window,
                    cx,
                ),
                cx,
            ))
            .child(self.format_control(tips, window, cx))
            .when(
                summary.as_ref().map_or(true, |warning| !warning.is_empty()),
                |panel| {
                    panel.child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().danger)
                            .child(summary.unwrap_or_else(|error| error)),
                    )
                },
            )
            .child(div().mt_1().pt_1().child(self.convert_button(request, cx)))
    }
}

struct Unavailable;
impl Render for Unavailable {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().child("File information is unavailable")
    }
}

fn panel(
    owner: WeakEntity<Shell>,
    window: &mut Window,
    cx: &mut gpui_kit::App,
) -> gpui_kit::AnyView {
    let prepared = owner
        .read_with(cx, |shell, _| {
            let request = shell.conversion_request()?;
            let file = shell.file.as_ref()?;
            let mut details = file.state.document.file_summary()?.hint.rows;
            details.retain(|(label, _)| {
                !matches!(
                    label.as_str(),
                    "Sample rate" | "Sample format" | "Centre frequency" | "Container"
                )
            });
            if file.unsaved.is_some() {
                details.retain(|(label, _)| label != "File size");
            }
            Some((request, file.id, file.editing.as_ref()?.version(), details))
        })
        .ok()
        .flatten();
    let Some((request, document, version, details)) = prepared else {
        return cx.new(|_| Unavailable).into();
    };
    cx.new(|cx| {
        let mut editor = Editor::new(request, window, cx);
        if let Some(shell) = owner.upgrade() {
            editor
                ._subscriptions
                .push(cx.observe(&shell, |_, _, cx| cx.notify()));
        }
        editor.owner = Some(owner);
        editor.document = Some(document);
        editor.version = version;
        editor.details = details;
        editor
    })
    .into()
}

impl Shell {
    pub(super) fn file_select_trigger(&self, cx: &gpui_kit::App) -> Option<FocusHandle> {
        let editor = self
            .conversion
            .hint
            .read(cx)
            .view()?
            .downcast::<Editor>()
            .ok()?;
        let editor = editor.read(cx);
        let handles = [
            editor.container.read(cx).focus_handle(cx),
            editor.kind.read(cx).focus_handle(cx),
        ];
        handles
            .iter()
            .zip(&editor.closed_selects)
            .find_map(|(current, closed)| (current != closed).then(|| closed.clone()))
    }

    pub(super) fn conversion_active(&self) -> bool {
        self.conversion.running.is_some() || self.conversion.pending.is_some()
    }

    pub(super) fn cancel_conversion(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.conversion
            .after_cancel
            .get_or_insert(AfterCancel::KeepDocument);
        self.conversion.pending = None;
        self.conversion.result = None;
        if let Some(job) = &self.conversion.running {
            job.job.cancel();
        } else if self.conversion.after_cancel.take() == Some(AfterCancel::Close) {
            self.request_close(window, cx);
        }
        cx.notify();
    }

    pub(super) fn close_after_conversion(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self.conversion_active() {
            return false;
        }
        self.conversion.after_cancel = Some(AfterCancel::Close);
        self.cancel_conversion(window, cx);
        true
    }

    pub(super) fn conversion_cover(&self, area: gpui_kit::Bounds<Pixels>) -> Option<AnyElement> {
        self.conversion_active().then(|| {
            gpui_kit::deferred(
                div()
                    .id("conversion-busy-cover")
                    .absolute()
                    .left(area.origin.x)
                    .top(area.origin.y)
                    .w(area.size.width)
                    .h(area.size.height)
                    .occlude()
                    .cursor(gpui_kit::CursorStyle::OperationNotAllowed)
                    .on_any_mouse_down(|_, window, cx| {
                        window.prevent_default();
                        cx.stop_propagation();
                    })
                    .on_scroll_wheel(|_, _, cx| cx.stop_propagation()),
            )
            .with_priority(1)
            .into_any_element()
        })
    }

    pub(super) fn conversion_request(&self) -> Option<Request> {
        if !self.can_save(false) || self.capture_changed() {
            return None;
        }
        let editing = self.editing()?;
        let mut input = editing.file().meta.clone();
        input.len_samples = editing.len();
        let container = self.file.as_ref()?.unsaved.as_ref().map_or_else(
            || {
                if input.container == "flac" {
                    argand_io::write::Container::Flac
                } else {
                    argand_io::write::Container::Wave
                }
            },
            |unsaved| unsaved.container,
        );
        Some(Request {
            capture: editing.capture().clone(),
            sources: editing.source_files(),
            output: input.clone(),
            carrier: None,
            input,
            target: std::path::PathBuf::new(),
            protected: editing.protected(),
            container,
            level: Level::Preserve,
            full_scale: editing.full_scale().unwrap_or(0.0),
            copy: editing.save_request(None, std::path::PathBuf::new()),
        })
    }

    pub(super) fn file_control(&self, value: String, cx: &mut Context<Self>) -> impl IntoElement {
        self.status_panel_control(settings_ui::StatusPanel::File, value, cx)
    }

    pub(super) fn convert_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.open_status_panel(settings_ui::StatusPanel::File, false, window, cx);
    }

    fn convert_draft(
        &mut self,
        request: Request,
        document: Option<u64>,
        version: u64,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.file.as_ref().map(|file| file.id) != document
            || self.editing().map(|editing| editing.version()) != Some(version)
            || self.conversion_request().is_none()
        {
            return;
        }
        self.close_hints(cx);
        self.start_conversion(request, window, cx);
    }

    fn start_conversion(
        &mut self,
        mut request: Request,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(folder) = crate::scratch::folder() else {
            self.save_notice = Some(saving_ui::Notice::Failed(
                "Conversion failed",
                "No temporary storage is available".into(),
            ));
            cx.notify();
            return;
        };
        let Some(file) = &self.file else {
            return;
        };
        let Some(editing) = file.editing.as_ref() else {
            return;
        };
        let directory = self.save_presentation(&editing.file().meta).1;
        let document = file.id;
        let version = editing.version();
        let container = request.container;
        let output_is_iq = request.output.is_iq();
        let name = converted_name(
            &file.unsaved.as_ref().map_or_else(
                || file.state.document.origin().name(),
                |unsaved| unsaved.name.clone(),
            ),
            container,
        );
        request.target = folder.new_path("wav");
        let backing = Arc::new(crate::scratch::Scratch::new(request.target.clone()));
        let retained = backing.clone();
        let scratch = editing.scratches();
        let (job, updates) = crate::saving::spawn(
            "argand-convert",
            move |progress, cancel| {
                let _retained = (retained, scratch);
                crate::saving::ended(
                    crate::conversion::run(&request, progress, cancel),
                    "conversion",
                )
            },
            Ended::Failed,
        );
        self.conversion.updates = Some(cx.spawn_in(window, async move |shell, cx| {
            while let Ok(update) = updates.recv().await {
                if shell
                    .update_in(cx, |shell, window, cx| {
                        shell.receive_conversion(update, window, cx)
                    })
                    .is_err()
                {
                    break;
                }
            }
        }));
        self.conversion.running = Some(Converting {
            job,
            progress: None,
            document,
            version,
            name,
            directory,
            output_is_iq,
            container,
            backing,
        });
        self.dismiss_application_menu(window, cx);
        self.close_hints(cx);
        self.interrupt_plot(cx);
        if let Some(plot) = self.plot_view() {
            plot.update(cx, |plot, cx| plot.clear_pointer(cx));
        }
        window.focus(&self.focus_target(cx), cx);
        self.save_notice = None;
        cx.notify();
    }

    fn receive_conversion(
        &mut self,
        update: Update<Ended<Converted>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match update {
            Update::Progress { done, total } => {
                if let Some(job) = &mut self.conversion.running {
                    job.progress = Some((done, total));
                }
            }
            Update::Finished(outcome) => {
                let Some(job) = self.conversion.running.take() else {
                    return;
                };
                if let Some(after) = self.conversion.after_cancel.take() {
                    drop(job);
                    if after == AfterCancel::Close {
                        self.request_close(window, cx);
                    }
                    cx.notify();
                    return;
                }
                match outcome {
                    Ended::Done(result) => {
                        if self.file.as_ref().map(|file| file.id) != Some(job.document)
                            || self.editing().map(|editing| editing.version()) != Some(job.version)
                        {
                            return;
                        }
                        self.conversion.result = clipping_notice(result.clipped, job.output_is_iq);
                        let mut origin = Origin::new(result.saved.path);
                        origin.hints.normalize = Some(argand_io::Normalize::None);
                        self.prepare_conversion_reader(job, origin, window, cx);
                    }
                    Ended::Cancelled => {}
                    Ended::Failed(error) => {
                        self.save_notice =
                            Some(saving_ui::Notice::Failed("Conversion failed", error))
                    }
                }
            }
        }
        cx.notify();
    }

    fn prepare_conversion_reader(
        &mut self,
        job: Converting,
        origin: Origin,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        #[cfg(test)]
        if std::mem::take(&mut self.conversion.fail_reader_open) {
            std::fs::remove_file(&origin.path)
                .expect("remove published scratch before reader preparation");
        }
        self.conversion.next_reader += 1;
        let token = self.conversion.next_reader;
        let adopted = Arc::new(std::sync::atomic::AtomicU64::new(0));
        let route = ReaderRoute {
            token,
            adopted: adopted.clone(),
        };
        let readers =
            self.preparing_readers(&origin, Some(route), Some(job.backing.clone()), window, cx);
        let start = readers.start.clone();
        self.conversion.pending = Some(PendingConversion {
            token,
            job,
            origin,
            readers,
            adopted,
        });
        start.start();
    }

    pub(super) fn receive_conversion_reader(
        &mut self,
        route: &ReaderRoute,
        delivery: Delivery,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let adopted = route.adopted.load(Ordering::Acquire);
        if adopted != 0 {
            if self.file.as_ref().map(|file| file.id) == Some(adopted) {
                self.receive(delivery, window, cx);
            }
            return;
        }
        if !self
            .conversion
            .pending
            .as_ref()
            .is_some_and(|pending| pending.token == route.token)
        {
            return;
        }
        match &delivery.update {
            crate::analysis::Update::Opened(_, _) => {}
            crate::analysis::Update::Failed(error) => {
                self.conversion.pending = None;
                self.conversion.result = None;
                self.save_notice = Some(saving_ui::Notice::Failed(
                    "Conversion failed",
                    error.to_string(),
                ));
                cx.notify();
                return;
            }
            _ => return,
        }
        let Some(pending) = self.conversion.pending.take() else {
            return;
        };
        if self.file.as_ref().map(|file| file.id) != Some(pending.job.document)
            || self.editing().map(|editing| editing.version()) != Some(pending.job.version)
        {
            return;
        }
        self.open_document(
            pending.origin,
            Some(UnsavedDocument {
                name: pending.job.name,
                directory: pending.job.directory,
                container: pending.job.container,
                backing: pending.job.backing,
            }),
            Some(pending.readers),
            window,
            cx,
        );
        if let Some(file) = &self.file {
            pending.adopted.store(file.id, Ordering::Release);
        }
        self.receive(delivery, window, cx);
        self.refresh_titles(window);
    }

    pub(super) fn conversion_item(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let text = if let Some(job) = &self.conversion.running {
            let percent = job
                .progress
                .filter(|(_, total)| *total > 0)
                .map_or(0, |(done, total)| {
                    (100.0 * done as f64 / total as f64) as u32
                });
            format!("Converting capture… {percent}%")
        } else if self.conversion.pending.is_some() {
            "Opening converted capture…".into()
        } else {
            self.conversion.result.clone()?
        };
        Some(
            div()
                .id("conversion-status")
                .flex()
                .items_center()
                .gap_1()
                .text_xs()
                .min_w_0()
                .max_w(px(420.))
                .tooltip({
                    let text = text.clone();
                    move |_, cx| {
                        metadata_tooltip(
                            MetadataHint {
                                title: "Conversion",
                                value: text.clone(),
                                explanation: String::new(),
                                rows: Vec::new(),
                            },
                            cx,
                        )
                    }
                })
                .child(
                    div()
                        .min_w_0()
                        .overflow_hidden()
                        .text_ellipsis()
                        .whitespace_nowrap()
                        .child(text),
                )
                .child(
                    Button::new("conversion-cancel")
                        .ghost()
                        .xsmall()
                        .icon(IconName::Close)
                        .tab_stop(false)
                        .on_click(cx.listener(|shell, _, window, cx| {
                            shell.cancel_conversion(window, cx);
                        })),
                )
                .into_any_element(),
        )
    }
}

#[cfg(test)]
#[path = "conversion_ui_tests.rs"]
pub(super) mod tests;
