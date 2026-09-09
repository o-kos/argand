//! One open file, and everything the window says about it.
//!
//! A document is where the file came from, how it was opened, what it turned
//! out to be, and the last analysis produced for it. Everything the window
//! draws hangs off this rather than off the shell, so the sequence a file goes
//! through -- opening, analysing, shown, or failed -- is one state machine in
//! [`Document::apply`] that can be driven without a thread or a toolkit.
//!
//! The samples themselves are not here. They belong to the thread in
//! [`crate::analysis`], which is the only thing that ever touches them.

use std::path::PathBuf;
use std::time::Duration;

use argand_core::{SampleFormat, SampleType, SignalMeta, format_duration, format_hz};
use argand_dsp::Analysis;
use argand_io::OpenHints;

use crate::analysis::{FileInfo, Update};

/// Where a document came from, and how it was read.
///
/// These two together are what it takes to open the same file the same way
/// again, which is what the recent list has to remember: a headerless capture
/// is not openable at all without the hints it was opened with the first time.
#[derive(Debug, Clone)]
pub struct Origin {
    pub path: PathBuf,
    pub hints: OpenHints,
}

impl Origin {
    /// A file opened on whatever it says about itself.
    ///
    /// What a menu and a drop can offer: a path and nothing else. A headerless
    /// capture opened this way fails and says what it needs, which is why the
    /// recent list remembers hints.
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            hints: OpenHints::default(),
        }
    }

    /// The name to put in a title bar or a recent list.
    ///
    /// The whole path is what identifies the file, and the whole path is also
    /// what nobody can read at a glance.
    pub fn name(&self) -> String {
        self.path
            .file_name()
            .unwrap_or(self.path.as_os_str())
            .to_string_lossy()
            .into_owned()
    }
}

/// Where a document is in the sequence open, analyse, show.
///
/// The picture on screen and this are deliberately independent: a re-analysis
/// puts the document back into [`Status::Analyzing`] while the previous
/// picture is still up, because a window that blanks itself on every settings change is
/// harder to use than one that shows a slightly stale spectrogram.
#[derive(Debug, Clone, PartialEq)]
pub enum Status {
    /// Reading the header, and the level scan a normalized capture needs.
    Opening,
    /// A transform is running. `total` is zero until the first report.
    Analyzing { done: u64, total: u64 },
    /// What is on screen is current.
    Ready { elapsed: Duration },
    /// Nothing came of it, and this is what to tell the person.
    Failed(String),
}

impl Status {
    pub fn hint(&self) -> Option<MetadataHint> {
        let Self::Ready { elapsed } = self else {
            return None;
        };
        Some(MetadataHint::new(
            "Analysis time",
            format!("{:.3} s", elapsed.as_secs_f64()),
            "Includes sample reading, transforms and image preparation, excluding file opening and window drawing",
        ))
    }

    /// What the status bar says about the work, as opposed to the file.
    ///
    /// A failure is named rather than quoted here: the reason can be a
    /// sentence with a path in it, and the status bar is one line at the foot
    /// of the window. Whatever went wrong is shown in full where the picture
    /// would have been.
    pub fn message(&self) -> String {
        match self {
            Self::Opening => "opening...".to_owned(),
            Self::Analyzing { done, total } => match percent(*done, *total) {
                Some(percent) => format!("analysing... {percent}%"),
                None => "analysing...".to_owned(),
            },
            Self::Ready { elapsed } => {
                format!("ready in {}", format_duration(elapsed.as_secs_f64()))
            }
            Self::Failed(_) => "cannot be read".to_owned(),
        }
    }
}

/// How far a transform has got, or `None` before it has said.
///
/// The total arrives with the first report rather than in advance, so a run
/// that has not reported yet has no fraction to show -- and one that reports a
/// total of zero has nothing to divide by.
fn percent(done: u64, total: u64) -> Option<u64> {
    (total > 0).then(|| (done.min(total) * 100) / total)
}

/// What the window has to do about an update, beyond drawing again.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effect {
    /// Only what the status bar says has changed.
    Status,
    /// The file's own description arrived, so a request can now be built for
    /// it: the span to analyse is the length it just reported.
    Opened,
    /// A new picture replaced whatever was on screen.
    Analysis,
}

/// One open file.
pub struct Document {
    origin: Origin,
    /// What the file says about itself, once it has been opened.
    meta: Option<SignalMeta>,
    /// The last analysis produced for it, kept across a re-analysis so the
    /// window has something to draw while the next one runs.
    analysis: Option<Box<Analysis>>,
    status: Status,
    waveform_peak: Option<f32>,
    file_info: FileInfo,
    sample_extrema: Option<Vec<(f32, f32)>>,
}

impl Document {
    /// A document for a file whose thread has been started but has not
    /// reported yet.
    pub fn opening(origin: Origin) -> Self {
        Self {
            origin,
            meta: None,
            analysis: None,
            status: Status::Opening,
            waveform_peak: None,
            file_info: FileInfo::default(),
            sample_extrema: None,
        }
    }

    pub const fn origin(&self) -> &Origin {
        &self.origin
    }

    pub const fn meta(&self) -> Option<&SignalMeta> {
        self.meta.as_ref()
    }

    pub fn analysis(&self) -> Option<&Analysis> {
        self.analysis.as_deref()
    }

    pub fn waveform_peak(&self) -> Option<f32> {
        self.waveform_peak
            .or_else(|| self.analysis().map(|analysis| analysis.time_peak))
    }

    pub const fn status(&self) -> &Status {
        &self.status
    }

    /// Fold one update from the analysis thread into the document.
    ///
    /// This is the whole sequence a file goes through, in one place and with
    /// no toolkit in sight, so what the window shows at each step is decided
    /// here and merely drawn there.
    pub fn apply(&mut self, update: Update) -> Effect {
        match update {
            Update::Opened(meta, info) => {
                self.file_info = info;
                self.meta = Some(meta);
                // Nothing has been asked for yet; the window builds the first
                // request from the length this update just brought.
                self.status = Status::Analyzing { done: 0, total: 0 };
                Effect::Opened
            }
            Update::Progress { done, total } => {
                self.status = Status::Analyzing { done, total };
                Effect::Status
            }
            Update::Snapshot {
                analysis,
                coverage,
                waveform_peak,
            } => {
                self.analysis = Some(analysis);
                self.waveform_peak = Some(waveform_peak);
                self.status = Status::Analyzing {
                    done: coverage.refined_columns as u64,
                    total: coverage.width as u64,
                };
                Effect::Analysis
            }
            Update::Ready { analysis, elapsed } => {
                self.waveform_peak = None;
                self.sample_extrema = analysis.waveform.as_ref().map(|waveform| {
                    (0..waveform.channels)
                        .map(|channel| {
                            let low = waveform
                                .min
                                .iter()
                                .skip(channel)
                                .step_by(waveform.channels)
                                .copied()
                                .fold(f32::INFINITY, f32::min);
                            let high = waveform
                                .max
                                .iter()
                                .skip(channel)
                                .step_by(waveform.channels)
                                .copied()
                                .fold(f32::NEG_INFINITY, f32::max);
                            (low, high)
                        })
                        .collect()
                });
                self.analysis = Some(analysis);
                self.status = Status::Ready { elapsed };
                Effect::Analysis
            }
            Update::Failed(error) => {
                // The chain, because the outer message names the step and the
                // inner one says what actually went wrong.
                self.status = Status::Failed(format!("{error:#}"));
                Effect::Status
            }
        }
    }

    pub fn file_summary(&self) -> Option<MetadataField> {
        let meta = self.meta()?;
        let fields = self.summary()?;
        let value = format!(
            "{} · {} {} · {} · {}",
            meta.container,
            if meta.is_iq() { "iq" } else { "real" },
            meta.sample_type.format.as_str(),
            format_hz(meta.sample_rate),
            compact_capture_duration(meta.duration_seconds())
        );
        let mut details = fields
            .iter()
            .map(|field| format!("{}: {}", field.hint.title, field.hint.value))
            .collect::<Vec<_>>();
        details.push(format!(
            "{}: {}",
            if meta.is_iq() { "I/Q pairs" } else { "Samples" },
            meta.len_samples
        ));
        details.push(self.file_info.bytes.map_or_else(
            || "File size: unavailable".into(),
            |bytes| {
                format!(
                    "File size: {bytes} bytes ({:.2} MiB)",
                    bytes as f64 / 1048576.0
                )
            },
        ));
        details.extend(self.extrema_details(meta));
        Some(MetadataField::new(
            value,
            MetadataHint::new(
                "Signal file",
                details.join("\n"),
                if meta.is_iq() {
                    "Sample rate and count refer to I/Q pairs; extrema are decoded original sample values"
                } else {
                    "Extrema are decoded original sample values before normalization and gain"
                },
            ),
        ))
    }

    fn extrema_details(&self, meta: &SignalMeta) -> Vec<String> {
        let Some(extrema) = &self.sample_extrema else {
            return vec!["Sample minimum / maximum: awaiting complete analysis".into()];
        };
        let Some((scale, offset)) = self.file_info.sample_units else {
            return vec!["Original sample minimum / maximum: unavailable".into()];
        };
        let number = |value: f32| {
            let value = f64::from(value) * scale + offset;
            if !value.is_finite() {
                return "unavailable".into();
            }
            match meta.sample_type.format {
                SampleFormat::U8 | SampleFormat::I16 | SampleFormat::I32 => format!("{value:.0}"),
                _ => format!("{value:.7}"),
            }
        };
        extrema
            .iter()
            .enumerate()
            .map(|(channel, &(low, high))| {
                let label = match (meta.is_iq(), channel) {
                    (true, 0) => "I",
                    (true, _) => "Q",
                    _ => "Sample",
                };
                format!(
                    "{label} minimum / maximum: {} / {}",
                    number(low),
                    number(high)
                )
            })
            .collect()
    }

    /// Detailed fields combined in the file hint.
    pub fn summary(&self) -> Option<Vec<MetadataField>> {
        let meta = self.meta.as_ref()?;
        let domain = if meta.is_iq() { "iq" } else { "real" };
        let mut fields = vec![
            MetadataField::new(meta.container, MetadataHint::container(meta.container)),
            MetadataField::new(
                format!("{domain} · {}", meta.sample_type.format.as_str()),
                MetadataHint::samples(meta.sample_type),
            ),
            MetadataField::new(
                format_hz(meta.sample_rate),
                MetadataHint::new(
                    "Signal sample rate",
                    format_hz(meta.sample_rate),
                    if meta.is_iq() {
                        "The rate counts I/Q pairs per second"
                    } else {
                        "The rate counts real samples per second"
                    },
                ),
            ),
            MetadataField::new(
                compact_capture_duration(meta.duration_seconds()),
                MetadataHint::new(
                    "Signal duration",
                    capture_duration(meta.duration_seconds()),
                    "hms.ms",
                ),
            ),
        ];
        if meta.center_freq != 0.0 {
            fields.push(MetadataField::new(
                format_hz(meta.center_freq),
                MetadataHint::new(
                    "Signal centre frequency",
                    format_hz(meta.center_freq),
                    "This is the frequency represented by zero in the baseband signal",
                ),
            ));
        }
        Some(fields)
    }
}

pub struct MetadataField {
    pub value: String,
    pub hint: MetadataHint,
}

impl MetadataField {
    fn new(value: impl Into<String>, hint: MetadataHint) -> Self {
        Self {
            value: value.into(),
            hint,
        }
    }
}

#[derive(Clone)]
pub struct MetadataHint {
    pub title: &'static str,
    pub value: String,
    pub explanation: String,
}

impl MetadataHint {
    fn new(title: &'static str, value: impl Into<String>, explanation: impl Into<String>) -> Self {
        Self {
            title,
            value: value.into(),
            explanation: explanation.into(),
        }
    }

    fn container(container: &str) -> Self {
        let explanation = match container {
            "wav" => "The WAVE container stores samples and their format metadata",
            "rf64" => "The RF64 container extends WAVE for captures larger than 4 GiB",
            "bw64" => "The BW64 container extends broadcast WAVE for captures larger than 4 GiB",
            "flac" => "The FLAC container compresses samples losslessly and stores their format",
            "raw" => "Headerless samples use the format and rate supplied when opening the file",
            _ => "The container defines how samples and metadata are stored in the file",
        };
        Self::new("File container type", container, explanation)
    }

    fn samples(sample_type: SampleType) -> Self {
        let (domain, explanation) = if sample_type.is_iq() {
            ("iq", "Each sample stores interleaved I and Q components")
        } else {
            ("real", "Each sample stores one scalar value")
        };
        let storage = match sample_type.format {
            SampleFormat::U8 => "unsigned 8-bit integers, with zero stored as 128",
            SampleFormat::I16 => "signed 16-bit integers",
            SampleFormat::I32 => "signed 32-bit integers",
            SampleFormat::F32 => "32-bit floating point with nominal full scale -1 to +1",
            SampleFormat::F16x8 => "32-bit floating point with arbitrary scale (CoolEdit 16x8)",
        };
        Self::new(
            "Samples format",
            format!("{domain} · {}", sample_type.format.as_str()),
            format!("{explanation} as {storage}"),
        )
    }
}

fn duration_millis(seconds: f64) -> u64 {
    (seconds * 1000.0).round() as u64
}

fn capture_duration(seconds: f64) -> String {
    let millis = duration_millis(seconds);
    format!(
        "{}:{:02}:{:02}.{:03}",
        millis / 3_600_000,
        millis / 60_000 % 60,
        millis / 1000 % 60,
        millis % 1000
    )
}

fn compact_capture_duration(seconds: f64) -> String {
    let millis = duration_millis(seconds);
    let hours = millis / 3_600_000;
    let minutes = millis / 60_000 % 60;
    let seconds = millis / 1000 % 60;
    let fraction = millis % 1000;
    let hours = if hours > 0 {
        format!("{hours}h")
    } else {
        String::new()
    };
    let minutes = if minutes > 0 {
        format!("{minutes}m")
    } else {
        String::new()
    };
    let seconds = match (seconds, fraction) {
        (_, 1..) => {
            format!("{seconds}.{fraction:03}")
                .trim_end_matches('0')
                .to_owned()
                + "s"
        }
        (0, 0) if millis > 0 => String::new(),
        _ => format!("{seconds}s"),
    };
    format!("{hours}{minutes}{seconds}")
}

#[cfg(test)]
mod tests {
    include!("document_tests.rs");
}
