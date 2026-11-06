//! File information, conversion draft, and the lifetime of unsaved conversion results.

use super::settings_ui::editor::{self as presentation, Name};
use super::*;
use crate::conversion::{Converted, Level, Request};
use crate::saving::{Ended, Update};
use gpui_kit::base::actions::Confirm;
use gpui_kit::component::Disableable;
use gpui_kit::component::IconName;
use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::component::select::{SelectEvent, SelectState};
use gpui_kit::{AnyElement, Focusable};

pub(super) struct State {
    pub(super) running: Option<Converting>,
    pub(super) updates: Option<Task<()>>,
    pub(super) pending: Option<PendingConversion>,
    next_reader: u64,
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
            if matches!(event, hints::Pinned::Opened) {
                shell.close_analysis_hint(cx);
                shell.close_selection_hint(cx);
            }
            cx.notify();
        });
        Self {
            running: None,
            updates: None,
            pending: None,
            next_reader: 0,
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
    backing: Arc<crate::scratch::Scratch>,
}

fn kind_label(token: &str) -> String {
    let (domain, format) = token.split_once('_').unwrap_or(("", token));
    let domain = if domain == "iq" { "I/Q" } else { "Real" };
    let format = match format {
        "u8" => "Unsigned 8-bit",
        "i16" => "Signed 16-bit",
        "i32" => "Signed 32-bit",
        "f32" => "Float 32-bit",
        "f16x8" => "Float 16×8",
        _ => format,
    };
    format!("{domain} · {format}")
}

fn converted_name(name: &str) -> String {
    let stem = std::path::Path::new(name)
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy();
    format!("{}-converted.wav", stem.trim_end_matches("-converted"))
}

struct Editor {
    request: Request,
    rate: Entity<InputState>,
    reference: Entity<InputState>,
    kind: Entity<SelectState<Vec<Name>>>,
    level: Entity<SelectState<Vec<Name>>>,
    kind_widths: Vec<Pixels>,
    level_widths: Vec<Pixels>,
    _subscriptions: Vec<Subscription>,
    owner: Option<WeakEntity<Shell>>,
    document: Option<u64>,
    version: u64,
    details: Vec<(String, String)>,
    focus_on_open: bool,
}

impl Editor {
    fn new(request: Request, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let rate = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(crate::numbers::input(request.output.sample_rate))
        });
        let reference = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(crate::numbers::input(request.output.center_freq))
        });
        let kinds: Vec<String> = argand_core::SAMPLE_TYPE_TOKENS
            .iter()
            .map(|s| kind_label(s))
            .collect();
        let selected = kinds
            .iter()
            .position(|s| *s == kind_label(&request.output.sample_type.to_string()))
            .map(gpui_kit::component::IndexPath::new);
        let kind_widths = presentation::shown_widths(&kinds, window, cx);
        let kind = cx.new(|cx| {
            SelectState::new(
                kinds.into_iter().map(Name).collect::<Vec<_>>(),
                selected,
                window,
                cx,
            )
        });
        let levels = vec!["Preserve level".into(), "Normalize peak".into()];
        let level_widths = presentation::shown_widths(&levels, window, cx);
        let level = cx.new(|cx| {
            SelectState::new(
                levels.into_iter().map(Name).collect::<Vec<_>>(),
                Some(gpui_kit::component::IndexPath::new(0)),
                window,
                cx,
            )
        });
        let subscriptions = vec![
            cx.subscribe(&rate, |_, _, _: &InputEvent, cx| cx.notify()),
            cx.subscribe(&reference, |_, _, _: &InputEvent, cx| cx.notify()),
            cx.subscribe(&kind, |_, _, _: &SelectEvent<Vec<Name>>, cx| cx.notify()),
            cx.subscribe(&level, |_, _, _: &SelectEvent<Vec<Name>>, cx| cx.notify()),
        ];
        Self {
            request,
            rate,
            reference,
            kind,
            level,
            kind_widths,
            level_widths,
            _subscriptions: subscriptions,
            owner: None,
            document: None,
            version: 0,
            details: Vec::new(),
            focus_on_open: true,
        }
    }

    fn request(&self, cx: &gpui_kit::App) -> Result<Request, String> {
        let mut request = self.request.clone();
        request.output.sample_rate = crate::numbers::parse(self.rate.read(cx).value().as_ref())
            .map_err(|_| "Enter a positive finite sample rate in Hz".to_string())?;
        request.output.center_freq =
            crate::numbers::parse(self.reference.read(cx).value().as_ref())
                .map_err(|_| "Enter a finite reference frequency in Hz".to_string())?;
        let selected = self
            .kind
            .read(cx)
            .selected_value()
            .ok_or("Choose a sample type")?;
        request.output.sample_type = argand_core::SAMPLE_TYPE_TOKENS
            .iter()
            .find(|token| kind_label(token) == *selected)
            .ok_or("Choose a supported sample type")?
            .parse()
            .map_err(|_| "Choose a supported sample type")?;
        request.level = if self
            .level
            .read(cx)
            .selected_value()
            .is_some_and(|s| s == "Normalize peak")
        {
            Level::Normalize
        } else {
            Level::Preserve
        };
        request.plan().map_err(|error| error.to_string())?;
        Ok(request)
    }

    fn changed(&self, request: &Request) -> bool {
        request.output.sample_rate != request.input.sample_rate
            || request.output.center_freq != request.input.center_freq
            || request.output.sample_type != request.input.sample_type
            || request.level == Level::Normalize
    }

    fn convert_button(&self, request: Option<Request>, cx: &mut Context<Self>) -> Button {
        presentation::outlined_button("convert-capture", "conversion-action", "Convert", false, cx)
            .disabled(request.is_none())
            .when(request.is_none(), |button| button.opacity(0.5))
            .on_click(cx.listener(move |editor, _, window, cx| {
                let Some(request) = request.clone() else {
                    return;
                };
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

    fn summary(&self, cx: &gpui_kit::App) -> Result<String, String> {
        let request = self.request(cx)?;
        let plan = request.plan().map_err(|error| error.to_string())?;
        if plan.band != plan.input_band {
            return Ok(format!(
                "Frequencies outside {} … {} Hz will be discarded",
                crate::numbers::number(plan.band.0),
                crate::numbers::number(plan.band.1)
            ));
        }
        Ok(String::new())
    }
}

fn frequency_control(
    state: &Entity<InputState>,
    window: &Window,
    cx: &gpui_kit::App,
) -> impl IntoElement {
    let focused = state.read(cx).focus_handle(cx).is_focused(window);
    let width = presentation::number_width(state.read(cx).value().as_ref(), window, cx);
    presentation::number_control(state, width, "Hz", focused, cx)
}

impl Render for Editor {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if std::mem::take(&mut self.focus_on_open) {
            let focus = self.rate.read(cx).focus_handle(cx);
            window.defer(cx, move |window, cx| window.focus(&focus, cx));
        }
        let summary = self.summary(cx);
        let request = self
            .request(cx)
            .ok()
            .filter(|request| self.changed(request));
        div()
            .id("file-information-panel")
            .key_context("FileInformation")
            .on_action(|_: &FocusNext, window, cx| window.focus_next(cx))
            .on_action(|_: &FocusPrevious, window, cx| window.focus_prev(cx))
            .on_action(|_: &Confirm, _, _| {})
            .flex()
            .flex_col()
            .gap_1()
            .w(px(370.).min(window.viewport_size().width - px(48.)))
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
                frequency_control(&self.rate, window, cx),
                cx,
            ))
            .child(presentation::row(
                "Reference",
                frequency_control(&self.reference, window, cx),
                cx,
            ))
            .child(presentation::row(
                "Sample format",
                presentation::choice_control(
                    "conversion-kind",
                    &self.kind,
                    &self.kind_widths,
                    window,
                    cx,
                ),
                cx,
            ))
            .child(presentation::row(
                "Level",
                presentation::choice_control(
                    "conversion-level",
                    &self.level,
                    &self.level_widths,
                    window,
                    cx,
                ),
                cx,
            ))
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
                details.insert(0, ("Document".into(), "Unsaved".into()));
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
        editor.owner = Some(owner);
        editor.document = Some(document);
        editor.version = version;
        editor.details = details;
        editor
    })
    .into()
}

impl Shell {
    fn conversion_request(&self) -> Option<Request> {
        if !self.can_save(false) || self.capture_changed() {
            return None;
        }
        let editing = self.editing()?;
        let mut input = editing.file().meta.clone();
        input.len_samples = editing.len();
        Some(Request {
            capture: editing.capture().clone(),
            sources: editing.source_files(),
            output: input.clone(),
            input,
            target: std::path::PathBuf::new(),
            protected: editing.protected(),
            level: Level::Preserve,
            full_scale: editing.full_scale().unwrap_or(0.0),
            copy: editing.save_request(None, std::path::PathBuf::new()),
        })
    }

    pub(super) fn file_control(&self, value: String, cx: &mut Context<Self>) -> impl IntoElement {
        let hint = self.conversion.hint.clone();
        let hover = hint.clone();
        let lit = hint.read(cx).is_open();
        let button =
            settings_ui::status_control("file-information", "file-summary-hover", value, lit, cx)
                .disabled(self.conversion_request().is_none())
                .on_hover(move |hovered, window, cx| {
                    hover.update(cx, |hint, cx| hint.hover(*hovered, window, cx));
                })
                .on_click(cx.listener(|shell, _, window, cx| shell.convert_dialog(window, cx)));
        hints::pinned(
            "file-information-hint",
            &hint,
            button,
            gpui_kit::Anchor::BottomLeft,
            cx,
        )
    }

    pub(super) fn convert_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.conversion_request().is_none() {
            return;
        }
        self.dismiss_application_menu(window, cx);
        self.close_analysis_hint(cx);
        self.close_selection_hint(cx);
        let hint = self.conversion.hint.clone();
        window.defer(cx, move |window, cx| {
            hint.update(cx, |hint, cx| hint.open(window, cx));
        });
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
        let document = file.id;
        let version = editing.version();
        let name = converted_name(&file.unsaved.as_ref().map_or_else(
            || file.state.document.origin().name(),
            |unsaved| unsaved.name.clone(),
        ));
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
            backing,
        });
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
                match outcome {
                    Ended::Done(result) => {
                        if self.file.as_ref().map(|file| file.id) != Some(job.document)
                            || self.editing().map(|editing| editing.version()) != Some(job.version)
                        {
                            return;
                        }
                        self.conversion.result = Some(format!(
                            "Converted — gain {:.3} dB, {} clipped scalar values",
                            20.0 * result.gain.log10(),
                            result.clipped
                        ));
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
                .text_sm()
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
                        .on_click(cx.listener(|shell, _, _, cx| {
                            if shell.conversion.pending.take().is_some() {
                                shell.conversion.result = None;
                            } else if let Some(job) = &shell.conversion.running {
                                job.job.cancel();
                            } else {
                                shell.conversion.result = None;
                            }
                            cx.notify();
                        })),
                )
                .into_any_element(),
        )
    }
}

#[cfg(test)]
#[path = "conversion_ui_tests.rs"]
mod tests;
