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

use argand_core::{SignalMeta, format_duration, format_hz};
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
    Ready,
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
            Self::Ready => "ready".to_owned(),
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
            Update::Ready(analysis) => {
                self.analysis = Some(analysis);
                self.status = Status::Ready;
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

    /// What the status bar says about the file itself, in the wording `aspec`
    /// uses for the same fields.
    ///
    /// `None` until the file has been opened, because until then nothing here
    /// is known -- not even whether the file is a signal.
    pub fn summary(&self) -> Option<String> {
        let meta = self.meta.as_ref()?;
        let mut fields = format!(
            "{} {}, {}, {}",
            meta.container,
            meta.sample_type,
            format_hz(meta.sample_rate),
            format_duration(meta.duration_seconds())
        );
        // Baseband is the default and says nothing; a tuned capture is the
        // whole reason the frequency axis reads in megahertz.
        if meta.center_freq != 0.0 {
            fields.push_str(&format!(", centre {}", format_hz(meta.center_freq)));
        }
        Some(fields)
    }
}

#[cfg(test)]
mod tests {
    include!("document_tests.rs");
}
