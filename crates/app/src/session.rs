//! `session.toml`: what the application remembers about itself.
//!
//! This file belongs to the program. It is rewritten whenever the window moves
//! or resizes, which is exactly why it is not the file a person edits -- see
//! [`crate::config`] for that one.
//!
//! Two properties matter more than what it stores. It is written atomically, so
//! a crash mid-write leaves the previous file rather than half of the new one;
//! and it is only ever advisory, so a missing, unreadable, corrupt or
//! future-versioned file costs a log line and the defaults, never a start-up
//! failure.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

/// The name the application writes under, in the platform state directory.
pub const FILE_NAME: &str = "session.toml";

/// Largest window edge any restored geometry may ask for, in logical pixels.
///
/// Past an 8K display and well under the surface limit any desktop driver
/// offers. The bound has to be conservative because the failure it prevents is
/// not an error return: the graphics backend logs that the requested size is
/// outside the surface capabilities and then unwraps the swapchain it could not
/// create, so an impossible size panics inside the call that opens the window
/// rather than coming back as something to handle.
///
/// A window genuinely spanning more than this -- three 4K displays side by side
/// -- opens smaller once instead. That is a far better outcome than a crash,
/// and far rarer than a corrupt file.
const MAX_DIMENSION: f32 = 8_192.0;

/// Furthest a restored window may be placed from the origin, in logical pixels.
///
/// Enough for any arrangement of displays; a saved corner beyond it is a
/// corrupt file rather than somewhere a window has been.
const MAX_ORIGIN: f32 = 65_536.0;

/// The layout this program knows how to read.
///
/// A file from a future version is left alone rather than guessed at: the
/// defaults cost a window position, and a wrong guess costs whatever that
/// version was recording.
pub const VERSION: u32 = 1;

/// A window rectangle in logical pixels, as the platform reports them.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Geometry {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Geometry {
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    fn right(self) -> f32 {
        self.x + self.width
    }

    fn bottom(self) -> f32 {
        self.y + self.height
    }

    /// Whether this rectangle is worth handing back to a window system.
    ///
    /// A zero-width window cannot be shown and a NaN one cannot be compared.
    /// The upper bound is the one that matters for a file a person can edit or
    /// a disk can corrupt: where no display is known there is nothing to clamp
    /// against, and a width of `1e30` would otherwise reach the toolkit and ask
    /// it for a surface no GPU can allocate. It is a guard against nonsense,
    /// not a policy about window sizes -- a real display clamps far tighter
    /// than this, in [`fit`].
    fn is_usable(self) -> bool {
        (1.0..=MAX_DIMENSION).contains(&self.width)
            && (1.0..=MAX_DIMENSION).contains(&self.height)
            && self.x.abs() <= MAX_ORIGIN
            && self.y.abs() <= MAX_ORIGIN
    }

    /// Area shared with `other`.
    fn overlap(self, other: Self) -> f32 {
        let width = (self.right().min(other.right()) - self.x.max(other.x)).max(0.0);
        let height = (self.bottom().min(other.bottom()) - self.y.max(other.y)).max(0.0);
        width * height
    }
}

/// Whether the window was left maximized or fullscreen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WindowState {
    #[default]
    Normal,
    Maximized,
    Fullscreen,
}

/// Everything one run hands to the next.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Session {
    /// Layout of this file, checked before anything in it is believed.
    pub version: u32,
    pub geometry: Option<Geometry>,
    pub window_state: WindowState,
}

impl Default for Session {
    fn default() -> Self {
        Self {
            version: VERSION,
            geometry: None,
            window_state: WindowState::default(),
        }
    }
}

impl Session {
    /// Read the session, falling back to defaults for every failure.
    pub fn load(path: &Path) -> Self {
        let text = match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                tracing::debug!(path = %path.display(), "no session yet, starting fresh");
                return Self::default();
            }
            Err(error) => {
                tracing::warn!(path = %path.display(), %error, "cannot read session, starting fresh");
                return Self::default();
            }
        };

        let session: Self = match toml::from_str(&text) {
            Ok(session) => session,
            Err(error) => {
                tracing::warn!(path = %path.display(), %error, "corrupt session, starting fresh");
                return Self::default();
            }
        };

        if session.version != VERSION {
            tracing::warn!(
                path = %path.display(),
                found = session.version,
                expected = VERSION,
                "session written by another version, starting fresh"
            );
            return Self::default();
        }
        session
    }

    /// Write the session so that no reader ever sees a partial file.
    ///
    /// The content goes to a temporary file beside the target and is renamed
    /// over it, which is atomic within a directory on every platform this ships
    /// to. Killing the process mid-write therefore leaves either the previous
    /// session or the new one, and never half of either.
    ///
    /// A failure to save is reported rather than raised: losing a window
    /// position is not worth interrupting whatever the person was doing. It is
    /// still answered, because a caller that keeps track of what reached the
    /// disk has to know that this did not.
    pub fn save(&self, path: &Path) -> bool {
        match self.write_atomically(path) {
            Ok(()) => true,
            Err(error) => {
                tracing::warn!(path = %path.display(), %error, "cannot save session");
                false
            }
        }
    }

    fn write_atomically(&self, path: &Path) -> std::io::Result<()> {
        let text = toml::to_string_pretty(self)
            .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        // Named for this process, so two runs saving at once cannot land on
        // each other's temporary file.
        let staging = path.with_extension(format!("toml.{}.tmp", std::process::id()));
        std::fs::write(&staging, text)?;
        std::fs::rename(&staging, path)
    }

    /// Where `session.toml` lives.
    pub fn path() -> Option<PathBuf> {
        let dir = dirs::state_dir().or_else(dirs::data_local_dir)?;
        Some(dir.join("argand").join(FILE_NAME))
    }
}

/// Turns a stream of window positions into occasional writes.
///
/// The window's geometry is read on every frame it is drawn, so a drag offers
/// hundreds of positions a second. Writing each one would rewrite the file per
/// frame for a value only the last of which matters, so an offer is recorded
/// and written at most once per [`Self::INTERVAL`]. Whatever the last offer
/// left unwritten is flushed when the window closes.
///
/// Time arrives as a parameter rather than being read here, so the schedule can
/// be tested without waiting for it.
pub struct Writer {
    path: PathBuf,
    /// What the file holds, as far as this knows.
    stored: Session,
    /// An offer newer than `stored` that has not been written yet.
    pending: Option<Session>,
    /// When a write was last *attempted*, which is what the interval runs from.
    ///
    /// Timing the successes instead would turn a failure that persists -- a
    /// permission that is not coming back, a disk that stays full -- into an
    /// attempt and a warning on every frame, because the last success would
    /// never move.
    last_attempt: Option<Instant>,
}

impl Writer {
    /// Shortest gap between two writes.
    ///
    /// Long enough that a drag writes a handful of times rather than hundreds,
    /// short enough that a session killed without closing loses almost nothing.
    pub const INTERVAL: Duration = Duration::from_millis(500);

    pub fn new(path: PathBuf, stored: Session) -> Self {
        Self {
            path,
            stored,
            pending: None,
            last_attempt: None,
        }
    }

    /// Record where the window is now, and write if enough time has passed.
    pub fn offer(&mut self, session: Session, now: Instant) {
        // A window that has come back to where the file already has it leaves
        // nothing to write -- including anything offered in between, which the
        // window has since moved off.
        if session == self.stored {
            self.pending = None;
            return;
        }
        self.pending = Some(session);

        let due = self
            .last_attempt
            .is_none_or(|last| now.duration_since(last) >= Self::INTERVAL);
        if due {
            self.write(now);
        }
    }

    /// Write whatever is still pending, whatever the schedule says.
    pub fn flush(&mut self, now: Instant) {
        if self.pending.is_some() {
            self.write(now);
        }
    }

    fn write(&mut self, now: Instant) {
        let Some(session) = self.pending.clone() else {
            return;
        };
        // The attempt counts whatever came of it, so a failure waits its turn
        // like a success does.
        self.last_attempt = Some(now);
        // A write that did not happen is not a write. Keeping it pending is
        // what gives a transient failure -- a full disk, a permission that
        // comes back -- another chance at the next interval or at the flush.
        if !session.save(&self.path) {
            return;
        }
        self.pending = None;
        self.stored = session;
    }
}

/// Where the window should open, given what was saved and what displays exist.
///
/// A saved rectangle is not a promise: the display it was on may be gone, may
/// have moved, or may have changed resolution since. `None` hands the placement
/// back to the platform, which is the right answer for a first run and for a
/// rectangle that has nowhere to go.
///
/// The displays arrive as plain rectangles rather than as anything the toolkit
/// owns, which is what lets every rule here be tested without a window.
pub fn place(saved: Option<Geometry>, displays: &[Geometry]) -> Option<Geometry> {
    let saved = saved.filter(|g| g.is_usable())?;
    if displays.is_empty() {
        // Nothing to clamp against, which is what the first window sees on
        // Wayland: gpui learns the outputs from the display globals, and this
        // runs before it has processed them. Discarding the rectangle here
        // would throw away the size along with the position, and the size is
        // the half that can still be honoured -- placement is the compositor's
        // business on that platform, and it puts the window somewhere visible
        // by construction.
        tracing::debug!(
            "no displays known yet; restoring the size and leaving placement to the platform"
        );
        return Some(saved);
    }

    // The display it belongs to is the one it covers most of. Overlap rather
    // than the nearest centre: a window dragged half off a screen belongs to
    // the screen holding the other half, whatever its midpoint says.
    let home = displays
        .iter()
        .copied()
        .max_by(|a, b| saved.overlap(*a).total_cmp(&saved.overlap(*b)))?;

    if saved.overlap(home) > 0.0 {
        return Some(fit(saved, home));
    }

    // Nothing overlaps: the display it was saved on is not here any more. The
    // primary display is the first one, and it is where a window with nowhere
    // to return to should appear.
    let fallback = *displays.first()?;
    tracing::info!("the display this window was left on is gone, opening on the primary one");
    Some(fit(saved, fallback))
}

/// Shrink and shift a rectangle until it lies inside `display`.
///
/// Size is clamped before position, because a window wider than the screen has
/// no position that would bring it inside.
fn fit(window: Geometry, display: Geometry) -> Geometry {
    let width = window.width.min(display.width);
    let height = window.height.min(display.height);
    let x = window.x.clamp(display.x, display.right() - width);
    let y = window.y.clamp(display.y, display.bottom() - height);
    Geometry::new(x, y, width, height)
}

#[cfg(test)]
mod tests {
    include!("session_tests.rs");
}
