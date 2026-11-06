//! File information, conversion draft, and the lifetime of unsaved conversion results.

use super::*;
use crate::conversion::{Converted, Level, Request};
use crate::saving::{Ended, Update};
use gpui_kit::base::actions::Confirm;
use gpui_kit::component::Disableable;
use gpui_kit::component::IconName;
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::select::{Select, SelectEvent, SelectState};
use gpui_kit::{AnyElement, Focusable};

pub(super) struct State {
    pub(super) running: Option<Converting>,
    pub(super) updates: Option<Task<()>>,
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
            hint,
            _subscription: subscription,
            result: None,
        }
    }
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
    kind: Entity<SelectState<Vec<String>>>,
    level: Entity<SelectState<Vec<String>>>,
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
        let kind = cx.new(|cx| SelectState::new(kinds, selected, window, cx));
        let level = cx.new(|cx| {
            SelectState::new(
                vec!["Preserve level".into(), "Normalize peak".into()],
                Some(gpui_kit::component::IndexPath::new(0)),
                window,
                cx,
            )
        });
        let subscriptions = vec![
            cx.subscribe(&rate, |_, _, _: &InputEvent, cx| cx.notify()),
            cx.subscribe(&reference, |_, _, _: &InputEvent, cx| cx.notify()),
            cx.subscribe(&kind, |_, _, _: &SelectEvent<Vec<String>>, cx| cx.notify()),
            cx.subscribe(&level, |_, _, _: &SelectEvent<Vec<String>>, cx| cx.notify()),
        ];
        Self {
            request,
            rate,
            reference,
            kind,
            level,
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

fn field(label: &'static str, control: impl IntoElement) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(div().text_xs().child(label))
        .child(control)
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
            .gap_3()
            .w(px(360.).min(window.viewport_size().width - px(48.)))
            .p_3()
            .bg(crate::theme::sheet(cx))
            .border_1()
            .border_color(cx.theme().border)
            .rounded(cx.theme().radius)
            .shadow_lg()
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
            .child(field("Sample rate (Hz)", Input::new(&self.rate).small()))
            .child(field(
                "Reference frequency (Hz)",
                Input::new(&self.reference).small(),
            ))
            .child(field(
                "Sample format",
                Select::new(&self.kind).id("conversion-kind").small(),
            ))
            .child(field(
                "Level",
                Select::new(&self.level).id("conversion-level").small(),
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
            .child(
                Button::new("convert-capture")
                    .primary()
                    .label("Convert")
                    .disabled(request.is_none())
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
                    })),
            )
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
        let button = Button::new("file-information")
            .small()
            .h_5()
            .px_2()
            .tab_stop(false)
            .ghost()
            .disabled(self.conversion_request().is_none())
            .label(value)
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
                        self.open_document(
                            origin,
                            Some(UnsavedDocument {
                                name: job.name,
                                backing: job.backing,
                            }),
                            window,
                            cx,
                        );
                        self.refresh_titles(window);
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

    pub(super) fn conversion_item(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let text = if let Some(job) = &self.conversion.running {
            let percent = job
                .progress
                .filter(|(_, total)| *total > 0)
                .map_or(0, |(done, total)| {
                    (100.0 * done as f64 / total as f64) as u32
                });
            format!("Converting capture… {percent}%")
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
                            if let Some(job) = &shell.conversion.running {
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
