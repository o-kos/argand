use crate::analysis::{Analyst, Delivery, Update};
use crate::document::{Document, Effect};
use crate::settings::Settings;

/// One open file's analysis-acceptance state: the document the deliveries
/// build, the worker gating which deliveries are current, and which settings
/// the picture on screen was produced with. The window-side parts of an open
/// file, such as the plot handle and the delivery tasks, stay in `Shell`'s own
/// `OpenFile`, because they name GPUI and cannot run in CI.
pub(crate) struct OpenFileState {
    pub(crate) document: Document,
    pub(crate) analyst: Analyst,
    displayed_settings: Option<Settings>,
}

impl OpenFileState {
    pub(crate) fn new(document: Document, analyst: Analyst) -> Self {
        Self {
            document,
            analyst,
            displayed_settings: None,
        }
    }

    /// The settings the picture on screen was produced with, if a picture has
    /// been accepted at all.
    pub(crate) fn displayed_settings(&self) -> Option<&Settings> {
        self.displayed_settings.as_ref()
    }

    /// Retake the recorded settings as these, when the picture on screen was
    /// produced with settings equivalent to them. The settings editor's apply
    /// uses this for a preview it has already seen and will not re-request,
    /// where the record must follow the applied settings without a picture.
    pub(crate) fn retake_equivalent(&mut self, settings: &Settings) -> bool {
        let matches = self
            .displayed_settings
            .as_ref()
            .is_some_and(|displayed| displayed.equivalent(*settings));
        if matches {
            self.displayed_settings = Some(*settings);
        }
        matches
    }

    /// Whether `delivery` would change the picture of a capture whose file changed on disk.
    ///
    /// The picture shown stays, whatever a pass already running reads; an opening or a failure
    /// still lands, so readers rebinding a saved file finish.
    pub(crate) fn holds_picture(&self, delivery: &Delivery) -> bool {
        self.document.is_changed()
            && matches!(
                delivery.update,
                Update::Progress { .. } | Update::Snapshot { .. } | Update::Ready { .. }
            )
    }

    /// Gate the delivery, apply it, and record the settings behind the picture
    /// when the delivery carried one. A superseded delivery is rejected with
    /// `None` and leaves every field untouched, so a failed analysis leaves the
    /// last good settings in place and a cancelled editor preview keeps them
    /// until the restored settings produce their own picture.
    pub(crate) fn accept(&mut self, delivery: Delivery, settings: &Settings) -> Option<Effect> {
        if !self.analyst.accepts(&delivery) || self.holds_picture(&delivery) {
            return None;
        }
        let effect = self.document.apply(delivery.update);
        if effect == Effect::Analysis {
            self.displayed_settings = Some(*settings);
        }
        Some(effect)
    }
}

#[cfg(test)]
#[path = "open_file_test.rs"]
mod tests;

#[cfg(test)]
include!("open_file_support_test.rs");
