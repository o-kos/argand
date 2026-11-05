//! Standard conversion dialog and the lifetime of its background job.

use super::*;
use crate::conversion::{Converted, Level, Request};
use crate::saving::{Ended, Update};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::select::{Select, SelectEvent, SelectState};
use gpui_kit::component::{IconName, WindowExt as _};
use gpui_kit::{AnyElement, Focusable};

#[derive(Default)]
pub(super) struct State {
    pub(super) running: Option<Converting>,
    pub(super) updates: Option<Task<()>>,
    result: Option<String>,
}

pub(super) struct Converting {
    job: crate::saving::Job,
    progress: Option<(u64, u64)>,
    _scratch: Vec<Arc<crate::scratch::Scratch>>,
}

struct Editor {
    request: Request,
    rate: Entity<InputState>,
    reference: Entity<InputState>,
    kind: Entity<SelectState<Vec<String>>>,
    level: Entity<SelectState<Vec<String>>>,
    _subscriptions: Vec<Subscription>,
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
            .map(|s| (*s).into())
            .collect();
        let selected = kinds
            .iter()
            .position(|s| *s == request.output.sample_type.to_string())
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
        }
    }

    fn request(&self, cx: &gpui_kit::App) -> Result<Request, String> {
        let mut request = self.request.clone();
        request.output.sample_rate = crate::numbers::parse(self.rate.read(cx).value().as_ref())
            .map_err(|_| "Enter a positive finite sample rate in Hz".to_string())?;
        request.output.center_freq =
            crate::numbers::parse(self.reference.read(cx).value().as_ref())
                .map_err(|_| "Enter a finite reference frequency in Hz".to_string())?;
        request.output.sample_type = self
            .kind
            .read(cx)
            .selected_value()
            .ok_or_else(|| "Choose a sample type".to_string())?
            .parse()
            .map_err(|_| "Choose a supported sample type".to_string())?;
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

    fn summary(&self, cx: &gpui_kit::App) -> Result<String, String> {
        let request = self.request(cx)?;
        let plan = request.plan().map_err(|error| error.to_string())?;
        let output = request.output.frequency_span();
        let filtering = if plan.is_identity() {
            "No filtering or rate conversion".to_string()
        } else {
            "Filter transitions occupy 1% of the retained width at each edge; native real DC/Nyquist are kept".into()
        };
        Ok(format!(
            "Whole capture: {} frames → {} frames\nTarget band: {} … {} Hz\nRetained intersection: {} … {} Hz{}\nPassband interior: {} … {} Hz\n{}\nIntegers: nearest rounding, no dither; clipped scalar values reported (I/Q separately). DSP precision is f32. Float 16x8 preserves arbitrary units.",
            request.capture.len(),
            plan.samples,
            crate::numbers::number(output.0),
            crate::numbers::number(output.1),
            crate::numbers::number(plan.band.0),
            crate::numbers::number(plan.band.1),
            if plan.band != plan.input_band {
                "; frequencies outside are discarded"
            } else {
                ""
            },
            crate::numbers::number(plan.passband.0),
            crate::numbers::number(plan.passband.1),
            filtering
        ))
    }
}

impl Render for Editor {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let summary = self.summary(cx);
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .child("Sample rate (Hz)")
                    .child(Input::new(&self.rate)),
            )
            .child(
                div()
                    .child("Reference frequency (Hz)")
                    .child(Input::new(&self.reference)),
            )
            .child(
                div()
                    .child("Sample type (rl = real, iq = complex)")
                    .child(Select::new(&self.kind).id("conversion-kind")),
            )
            .child(
                div()
                    .child("Level")
                    .child(Select::new(&self.level).id("conversion-level")),
            )
            .child(
                div()
                    .text_sm()
                    .when(summary.is_err(), |d| d.text_color(cx.theme().danger))
                    .child(summary.unwrap_or_else(|error| error)),
            )
    }
}

impl Shell {
    pub(super) fn convert_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.can_save(false) || self.capture_changed() {
            return;
        }
        self.dismiss_application_menu(window, cx);
        self.close_hints(cx);
        self.interrupt_plot(cx);
        let Some(editing) = self.editing() else {
            return;
        };
        let mut input = editing.file().meta.clone();
        input.len_samples = editing.len();
        let request = Request {
            capture: editing.capture().clone(),
            sources: editing.source_files(),
            output: input.clone(),
            input,
            target: std::path::PathBuf::new(),
            protected: editing.protected(),
            level: Level::Preserve,
            full_scale: editing.full_scale().unwrap_or(0.0),
            copy: editing.save_request(None, std::path::PathBuf::new()),
        };
        let document = self.file.as_ref().map(|file| file.id);
        let version = editing.version();
        let editor = cx.new(|cx| Editor::new(request, window, cx));
        let focus = editor.read(cx).rate.read(cx).focus_handle(cx);
        let owner = cx.entity().downgrade();
        window.open_dialog(cx, move |dialog, _, _| {
            let editor = editor.clone();
            let owner = owner.clone();
            dialog
                .title("Convert capture")
                .button_props(
                    gpui_kit::component::dialog::DialogButtonProps::default()
                        .ok_text("Convert…")
                        .show_cancel(true),
                )
                .w(px(480.))
                .overlay_closable(false)
                .child(editor.clone())
                .on_ok(move |_, window, cx| {
                    let Ok(request) = editor.read(cx).request(cx) else {
                        return false;
                    };
                    let _ = owner.update(cx, |shell, cx| {
                        shell.choose_conversion(request, document, version, window, cx);
                    });
                    true
                })
                .on_cancel(|_, _, _| true)
        });
        window.focus(&focus, cx);
    }

    fn choose_conversion(
        &mut self,
        mut request: Request,
        document: Option<u64>,
        version: u64,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let directory = crate::saving::directory(&request.input.source);
        let stem = request
            .input
            .source
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy();
        let name = format!("{stem}_converted.wav");
        let chosen = cx.prompt_for_new_path(&directory, Some(&name));
        cx.spawn_in(window, async move |shell, cx| {
            let Ok(Ok(Some(target))) = chosen.await else {
                return;
            };
            request.target = target;
            let _ = shell.update_in(cx, |shell, window, cx| {
                if shell.file.as_ref().map(|file| file.id) != document
                    || shell.editing().map(|e| e.version()) != Some(version)
                    || !shell.can_save(false)
                {
                    return;
                }
                shell.start_conversion(request, window, cx);
            });
        })
        .detach();
    }

    fn start_conversion(&mut self, request: Request, window: &mut Window, cx: &mut Context<Self>) {
        let scratch = self.editing().map(|e| e.scratches()).unwrap_or_default();
        let (job, updates) = crate::saving::spawn(
            "argand-convert",
            move |progress, cancel| {
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
            _scratch: scratch,
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
                if self.conversion.running.take().is_none() {
                    return;
                }
                match outcome {
                    Ended::Done(result) => {
                        self.conversion.result = Some(format!(
                            "Converted to {} — gain {:.3} dB, {} clipped scalar values",
                            result.saved.path.display(),
                            20.0 * result.gain.log10(),
                            result.clipped
                        ));
                        let mut origin = Origin::new(result.saved.path);
                        origin.hints.normalize = Some(argand_io::Normalize::None);
                        self.open(origin, window, cx);
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
