//! One document's edits and the clipboard, without a window.
//!
//! `argand-edit` keeps the piece table and its versions; this module knows
//! which file each source id stands for, what may be pasted where, and what
//! the analysis worker, the minimap and the writer need from the result.

use std::sync::Arc;

use argand_core::{SampleSpan, SignalMeta};
use argand_edit::{Capture, Clip, History, SourceId};
use argand_io::OpenHints;
use argand_io::write::{SaveRequest, Segment, SourceFile, SourceStamp, Storage};

use crate::analysis::EditState;
use crate::minimap::Snapshot;

/// A file a capture reads from, with what is known about it so far.
#[derive(Debug, Clone)]
pub struct Source {
    pub meta: SignalMeta,
    pub hints: OpenHints,
    /// Taken off the window's thread after opening, and absent until then.
    pub stamp: Option<SourceStamp>,
    /// How it stores its samples, which decides what can be pasted into it.
    pub storage: Option<Storage>,
}

impl Source {
    pub fn new(meta: SignalMeta, hints: OpenHints) -> Self {
        Self {
            meta,
            hints,
            stamp: None,
            storage: None,
        }
    }

    /// Whether `other` reads the same samples, which takes the same file opened the same way.
    fn same_file(&self, other: &Self) -> bool {
        self.meta.source == other.meta.source
            && self.stamp == other.stamp
            && self.hints == other.hints
            && self.meta.sample_type == other.meta.sample_type
            && self.meta.sample_rate == other.meta.sample_rate
    }

    fn file(&self) -> SourceFile {
        SourceFile {
            meta: self.meta.clone(),
            hints: self.hints.clone(),
            stamp: self.stamp,
        }
    }
}

/// Samples copied out of a capture, with the files they live in.
#[derive(Debug, Clone)]
pub struct Clipboard {
    sources: Vec<Source>,
    clip: Clip,
}

impl Clipboard {
    /// Take what a background check learned about a file the clipboard was copied from before it finished.
    pub fn describe(
        &mut self,
        opened: &Source,
        stamp: Option<SourceStamp>,
        storage: Option<Storage>,
    ) {
        for source in &mut self.sources {
            if source.stamp.is_none() && source.same_file(opened) {
                source.stamp = stamp;
                source.storage = storage;
            }
        }
    }
}

/// Where a paste goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placement {
    /// Before this sample, adding to the capture.
    At(u64),
    /// Instead of these samples.
    Replace(SampleSpan),
}

/// Why a paste was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PasteError {
    Unknown { name: String },
    Storage { name: String },
    Rate { name: String },
    Empty,
}

impl std::fmt::Display for PasteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unknown { name } => {
                write!(
                    f,
                    "{name} is still being checked, so it cannot be pasted yet"
                )
            }
            Self::Storage { name } => {
                write!(f, "{name} stores its samples differently from this capture")
            }
            Self::Rate { name } => write!(f, "{name} has another sample rate"),
            Self::Empty => f.write_str("there is nothing to paste"),
        }
    }
}

impl std::error::Error for PasteError {}

/// One document's sources, edit history and source envelopes.
pub struct Editing {
    sources: Vec<Source>,
    history: History<Option<SampleSpan>>,
    envelopes: Vec<Option<Arc<Snapshot>>>,
    /// Sources whose envelope is being built, so a second edit does not start another scan.
    building: Vec<bool>,
}

impl Editing {
    /// The file as it was opened, with nothing edited.
    pub fn new(meta: SignalMeta, hints: OpenHints) -> Self {
        let capture = Capture::whole(SourceId(0), meta.len_samples);
        Self {
            sources: vec![Source::new(meta, hints)],
            history: History::new(capture, None),
            envelopes: vec![None],
            building: vec![true],
        }
    }

    pub fn capture(&self) -> &Capture {
        self.history.capture()
    }

    pub fn len(&self) -> u64 {
        self.capture().len()
    }

    pub fn version(&self) -> u64 {
        self.history.version()
    }

    pub fn is_dirty(&self) -> bool {
        self.history.is_dirty()
    }

    pub fn can_undo(&self) -> bool {
        self.history.can_undo()
    }

    pub fn can_redo(&self) -> bool {
        self.history.can_redo()
    }

    /// The file the capture was opened from.
    pub fn file(&self) -> &Source {
        &self.sources[0]
    }

    /// Whether `source` is this capture's own file as it was opened, before its check finished.
    pub fn is_file(&self, source: &Source) -> bool {
        self.file().stamp.is_none() && self.file().same_file(source)
    }

    /// Record what was learned about the opened file off the window's thread.
    pub fn describe_file(&mut self, stamp: Option<SourceStamp>, storage: Option<Storage>) {
        self.sources[0].stamp = stamp;
        self.sources[0].storage = storage;
    }

    /// The selection the current version left, which undo and redo bring back.
    pub fn selection(&self) -> Option<SampleSpan> {
        *self.history.state()
    }

    /// Remove `span`, leaving nothing selected.
    pub fn delete(&mut self, span: SampleSpan) {
        let capture = self.capture().delete(span);
        self.history.apply(capture, None);
    }

    pub fn copy(&self, span: SampleSpan) -> Clipboard {
        Clipboard {
            sources: self.sources.clone(),
            clip: self.capture().copy(span),
        }
    }

    /// Paste `clipboard`, selecting what was pasted.
    pub fn paste(
        &mut self,
        clipboard: &Clipboard,
        placement: Placement,
    ) -> Result<SampleSpan, PasteError> {
        if clipboard.clip.is_empty() {
            return Err(PasteError::Empty);
        }
        let mut ids = Vec::with_capacity(clipboard.sources.len());
        let mut added = Vec::new();
        for source in &clipboard.sources {
            let id = match self.find(source) {
                Some(id) => id,
                None => {
                    self.check(source)?;
                    added.push(source.clone());
                    SourceId((self.sources.len() + added.len() - 1) as u32)
                }
            };
            ids.push(id);
        }
        let clip = clipboard.clip.remap(|id| ids[id.0 as usize]);
        let at = match placement {
            Placement::At(at) => at.min(self.len()),
            Placement::Replace(span) => span.start().min(self.len()),
        };
        let capture = match placement {
            Placement::At(_) => self.capture().insert(at, &clip),
            Placement::Replace(span) => self.capture().replace(span, &clip),
        };
        let pasted = SampleSpan::between(at, at + clip.len()).ok_or(PasteError::Empty)?;
        self.envelopes
            .resize(self.sources.len() + added.len(), None);
        self.building
            .resize(self.sources.len() + added.len(), false);
        self.sources.extend(added);
        self.history.apply(capture, Some(pasted));
        Ok(pasted)
    }

    /// The id a file already has in this capture, if it has one.
    fn find(&self, source: &Source) -> Option<SourceId> {
        self.sources
            .iter()
            .position(|known| known.same_file(source))
            .map(|index| SourceId(index as u32))
    }

    /// Refuse a file whose samples cannot sit beside this capture's in one file.
    fn check(&self, source: &Source) -> Result<(), PasteError> {
        let name = source
            .meta
            .source
            .file_name()
            .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
        let file = self.file();
        let (Some(ours), Some(theirs)) = (file.storage, source.storage) else {
            return Err(PasteError::Unknown { name });
        };
        if ours != theirs {
            return Err(PasteError::Storage { name });
        }
        if file.meta.sample_rate != source.meta.sample_rate {
            return Err(PasteError::Rate { name });
        }
        Ok(())
    }

    pub fn undo(&mut self) -> bool {
        self.history.undo()
    }

    pub fn redo(&mut self) -> bool {
        self.history.redo()
    }

    pub fn mark_saved(&mut self) {
        self.history.mark_saved();
    }

    pub fn mark_saved_version(&mut self, version: u64) {
        self.history.mark_saved_version(version);
    }

    /// Whether every file the current version reads has been checked, which a save needs.
    pub fn is_described(&self) -> bool {
        self.capture()
            .sources()
            .iter()
            .all(|id| self.sources[id.0 as usize].stamp.is_some())
    }

    /// What the analysis worker reads the current version from.
    pub fn edit_state(&self) -> EditState {
        EditState {
            version: self.version(),
            capture: self.capture().clone(),
            sources: self
                .sources
                .iter()
                .enumerate()
                .map(|(index, source)| {
                    (index > 0).then(|| (source.meta.clone(), source.hints.clone()))
                })
                .collect(),
        }
    }

    /// The sources a minimap envelope has to be built for now, each handed out once.
    pub fn missing_envelopes(&mut self) -> Vec<(SourceId, Source)> {
        let used = self.capture().sources();
        let mut missing = Vec::new();
        for id in used {
            let index = id.0 as usize;
            if !self.building[index] {
                self.building[index] = true;
                missing.push((id, self.sources[index].clone()));
            }
        }
        missing
    }

    pub fn set_envelope(&mut self, id: SourceId, snapshot: Arc<Snapshot>) {
        if let Some(slot) = self.envelopes.get_mut(id.0 as usize) {
            *slot = Some(snapshot);
        }
    }

    /// The minimap of the current version, put together from the source envelopes.
    pub fn minimap(&self) -> Option<Snapshot> {
        self.envelopes[0].as_ref()?;
        let meta = &self.file().meta;
        Some(crate::minimap::compose(
            self.capture(),
            &self.envelopes,
            meta.channels(),
            meta.sample_rate,
        ))
    }

    /// Whether the current version is the file exactly as it was opened.
    pub fn is_untouched(&self) -> bool {
        *self.capture() == Capture::whole(SourceId(0), self.file().meta.len_samples)
    }

    /// Save `span` of the current version, or all of it, to `target`, or nothing when it is empty.
    ///
    /// Only the files the samples come from are read, so one no longer used cannot stop the save,
    /// while the opened file stays protected even when none of it is left.
    pub fn save_request(
        &self,
        span: Option<SampleSpan>,
        target: std::path::PathBuf,
    ) -> Option<SaveRequest> {
        let pieces = match span {
            Some(span) => self.capture().segments(span),
            None => self.capture().pieces().to_vec(),
        };
        if pieces.is_empty() {
            return None;
        }
        let mut used: Vec<SourceId> = pieces.iter().map(|piece| piece.source).collect();
        used.sort_unstable();
        used.dedup();
        let file = self.file();
        Some(SaveRequest {
            meta: file.meta.clone(),
            sources: used
                .iter()
                .map(|id| self.sources[id.0 as usize].file())
                .collect(),
            segments: pieces
                .into_iter()
                .map(|piece| Segment {
                    source: used.binary_search(&piece.source).unwrap_or(0),
                    start: piece.start,
                    len: piece.len,
                })
                .collect(),
            target,
            protected: vec![argand_io::write::Protected {
                path: file.meta.source.clone(),
                stamp: file.stamp,
            }],
        })
    }
}

#[cfg(test)]
mod tests {
    include!("editing_tests.rs");
}
