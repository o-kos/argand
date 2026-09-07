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

use crate::analysis::Update;

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
/// picture is still up, because a window that blanks itself on every resize is
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
            Update::Opened(meta) => {
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
            Update::Ready { analysis, elapsed } => {
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

    /// Separate status fields, available once the header has been read.
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
                    vec![(
                        "Samples per second",
                        if meta.is_iq() {
                            "Each I/Q pair counts as one complex sample."
                        } else {
                            "Each value counts as one real sample."
                        },
                    )],
                ),
            ),
            MetadataField::new(
                capture_duration(meta.duration_seconds()),
                MetadataHint::new(
                    "Signal duration",
                    vec![
                        ("m", "Minutes"),
                        ("ss", "Seconds"),
                        ("ms", "Milliseconds (three digits)"),
                    ],
                ),
            ),
        ];
        if meta.center_freq != 0.0 {
            fields.push(MetadataField::new(
                format_hz(meta.center_freq),
                MetadataHint::new(
                    "Signal centre frequency",
                    vec![(
                        "Tuning reference",
                        "The frequency represented by zero in the baseband signal.",
                    )],
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
    pub sections: Vec<(&'static str, &'static str)>,
}

impl MetadataHint {
    fn new(title: &'static str, sections: Vec<(&'static str, &'static str)>) -> Self {
        Self { title, sections }
    }

    fn container(container: &str) -> Self {
        let section = match container {
            "wav" => (
                "WAVE container",
                "Stores samples with a header describing their format.",
            ),
            "rf64" => (
                "RF64 container",
                "A WAVE extension for captures larger than 4 GiB.",
            ),
            "bw64" => (
                "BW64 container",
                "A broadcast WAVE extension for captures larger than 4 GiB.",
            ),
            "flac" => (
                "Free Lossless Audio Codec",
                "Stores samples with lossless compression and a format header.",
            ),
            "raw" => (
                "Headerless samples",
                "The sample format and rate are supplied when opening the file.",
            ),
            _ => (
                "File container",
                "Describes how samples and metadata are stored in the file.",
            ),
        };
        Self::new("File container type", vec![section])
    }

    fn samples(sample_type: SampleType) -> Self {
        let domain = if sample_type.is_iq() {
            (
                "Complex I/Q",
                "Interleaved in-phase and quadrature components.",
            )
        } else {
            ("Real signal", "One scalar value per sample.")
        };
        let storage = match sample_type.format {
            SampleFormat::U8 => (
                "Unsigned 8-bit integer",
                "Zero is stored as 128 (offset binary).",
            ),
            SampleFormat::I16 => ("Signed 16-bit integer", ""),
            SampleFormat::I32 => ("Signed 32-bit integer", ""),
            SampleFormat::F32 => ("32-bit floating point", "Nominal full scale is -1 to +1."),
            SampleFormat::F16x8 => (
                "CoolEdit 16x8",
                "32-bit floating-point samples with arbitrary scale, not 16-bit integers.",
            ),
        };
        Self::new("Samples format", vec![domain, storage])
    }
}

fn capture_duration(seconds: f64) -> String {
    let millis = (seconds * 1000.0).round() as u64;
    format!(
        "{}:{:02}.{:03}",
        millis / 60_000,
        millis / 1000 % 60,
        millis % 1000
    )
}

#[cfg(test)]
mod tests {
    include!("document_tests.rs");
}
