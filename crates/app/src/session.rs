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
//!
//! What it is not is serialized between processes. Each run reads the file once
//! and writes it whole, so two running at the same time keep whatever the last
//! one wrote and lose the other's -- including a recent entry and the only copy
//! of the hints that open the capture it names. [`VERSION`] guards a
//! *downgrade*, where a binary that cannot read the layout leaves the file
//! alone, and not a race: an older binary already running has read its own copy
//! and will rewrite it in its own layout whatever the number on disk says.
//! Issue #43 carries that, and until it is answered what this file remembers is
//! what the last instance to write it remembered.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use argand_io::OpenHints;
use serde::{Deserialize, Serialize};

/// The name the application writes under, in the platform state directory.
pub const FILE_NAME: &str = "session.toml";

/// Largest window edge to ask for when nothing better is known, in logical
/// pixels.
///
/// The failure this guards against is not an error return: the graphics backend
/// logs that the requested size is outside the surface capabilities and then
/// unwraps the swapchain it could not create, so an impossible size panics
/// inside the call that opens the window rather than coming back as something
/// to handle.
///
/// No logical size is provably safe, because what the driver compares against
/// is *device* pixels and the scale factor between them has no upper bound in
/// any of the protocols: Windows offers 500%, and Wayland requires only that a
/// scale be positive. So this is a floor on the damage rather than a proof:
/// even at 500% it asks for 10240 device pixels, inside what any desktop driver
/// offers.
///
/// It binds only where no display is known, because [`fit`] otherwise clamps to
/// a real display, whose own size that display's driver supports by
/// construction. And where no display is known -- Wayland's first window -- the
/// compositor settles the size anyway, so a window clamped here loses nothing
/// it would have kept.
const UNVERIFIED_MAX_DIMENSION: f32 = 2_048.0;

/// Largest edge or offset a saved rectangle may hold at all, in logical pixels.
///
/// This one is about nonsense rather than about surfaces: past it, a file has
/// been corrupted or hand-edited into something no window ever was, and there
/// is nothing to restore.
const ABSURD: f32 = 65_536.0;

/// Files kept in the recent list.
///
/// Long enough to reach back past a day's work, short enough that the menu is
/// still a list rather than a search.
pub const RECENT_LIMIT: usize = 10;

/// The layout this program writes.
///
/// A file from a version this one does not know is left alone rather than
/// guessed at: the defaults cost a window position, and a wrong guess costs
/// whatever that version was recording. The number goes up whenever the layout
/// gains something, so that an older binary sees a number it does not know and
/// leaves the file rather than quietly rewriting it without what it could not
/// read. Version 2 added the recent list; version 3 adds the panel split.
pub const VERSION: u32 = 3;

/// Every layout this program can read, oldest first.
///
/// An older file is read into the current shape and written back at
/// [`VERSION`]: each version so far only added fields, so what is missing has
/// a default and nothing has to be converted.
const READABLE: [u32; 3] = [1, 2, VERSION];

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
        (1.0..=ABSURD).contains(&self.width)
            && (1.0..=ABSURD).contains(&self.height)
            && self.x.abs() <= ABSURD
            && self.y.abs() <= ABSURD
    }

    /// The same rectangle with neither edge past `limit`.
    fn capped(self, limit: f32) -> Self {
        Self {
            width: self.width.min(limit),
            height: self.height.min(limit),
            ..self
        }
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

/// A session read from disk, and whether it may be written back over.
///
/// The two travel together because a file this version cannot read is also a
/// file it must not overwrite: reading it as defaults costs a window position,
/// and writing over it costs whatever the version that wrote it was recording.
#[derive(Debug, Clone, PartialEq)]
pub struct Restored {
    pub session: Session,
    pub writable: bool,
}

/// How a file was opened, written the way the command line spells it.
///
/// The strings are deliberate. `OpenHints` is `argand-io`'s and carries a
/// sample type, a raw layout and a normalize mode, none of which this file
/// needs to know the shape of -- only how a person writes them, which is what
/// `--raw`, `--sample-type` and `--normalize` already answer. Storing the spelling
/// means one grammar for the command line, the report and this file, and it
/// means a session written by a version that learned a new sample type is
/// still readable rather than merely unparseable.
///
/// A value that no longer parses is dropped with a log line, because a hint
/// that cannot be read is worth less than the rest of the entry it sits in.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Hints {
    /// `--raw`, as `<type>[@<rate>]`.
    pub raw: Option<String>,
    pub sample_type: Option<String>,
    pub sample_rate: Option<f64>,
    pub center_freq: f64,
    pub byte_offset: u64,
    pub normalize: Option<String>,
    pub gain_db: f32,
}

impl From<&OpenHints> for Hints {
    fn from(hints: &OpenHints) -> Self {
        Self {
            raw: hints.raw.map(|spec| spec.to_string()),
            sample_type: hints.sample_type.map(|kind| kind.to_string()),
            sample_rate: hints.sample_rate,
            center_freq: hints.center_freq,
            byte_offset: hints.byte_offset,
            normalize: hints.normalize.map(|mode| mode.to_string()),
            gain_db: hints.gain_db,
        }
    }
}

impl Hints {
    /// Read the spellings back, dropping any this version cannot parse.
    pub fn to_open_hints(&self) -> OpenHints {
        OpenHints {
            raw: parsed("raw", &self.raw),
            sample_type: parsed("sample_type", &self.sample_type),
            sample_rate: self.sample_rate,
            center_freq: self.center_freq,
            byte_offset: self.byte_offset,
            normalize: parsed("normalize", &self.normalize),
            gain_db: self.gain_db,
        }
    }
}

/// One stored spelling, read by the parser that reads the command line.
///
/// A hint that will not parse is dropped rather than raised: it costs the flag
/// it stood for, where the alternative is a recent list that refuses to open a
/// file over a single word it does not recognise.
fn parsed<T: std::str::FromStr>(field: &'static str, text: &Option<String>) -> Option<T> {
    let text = text.as_deref()?;
    match text.parse() {
        Ok(value) => Some(value),
        Err(_) => {
            tracing::warn!(field, value = text, "unreadable hint in the recent list");
            None
        }
    }
}

/// A file that was opened, and what it took to open it.
///
/// The hints are the point. A headerless capture is not openable at all
/// without the layout it was first opened with, so a recent list that stored
/// only paths would offer entries that fail every time they are chosen.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Recent {
    pub path: PathBuf,
    #[serde(default)]
    pub hints: Hints,
}

/// What a menu calls each entry of the recent list.
///
/// The file name, which is what a person recognises, unless two entries share
/// one -- captures are named by frequency and timestamp, so a directory full
/// of them and its copy elsewhere collide easily. Where they do, the directory
/// holding the file is what tells them apart, and only those entries carry it:
/// a list where every line is a path is a list nobody reads.
pub fn recent_labels(recent: &[Recent]) -> Vec<String> {
    let name = |entry: &Recent| {
        entry
            .path
            .file_name()
            .unwrap_or(entry.path.as_os_str())
            .to_string_lossy()
            .into_owned()
    };
    let names: Vec<String> = recent.iter().map(name).collect();
    names
        .iter()
        .enumerate()
        .map(|(i, label)| {
            let unique = names
                .iter()
                .enumerate()
                .all(|(j, other)| j == i || other != label);
            if unique {
                return label.clone();
            }
            match recent[i]
                .path
                .parent()
                .filter(|dir| !dir.as_os_str().is_empty())
            {
                Some(dir) => format!("{label} - {}", dir.display()),
                None => label.clone(),
            }
        })
        .collect()
}

/// Everything one run hands to the next.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Session {
    /// User-adjusted waveform share; absence preserves the 3-rem default.
    #[serde(default)]
    pub waveform_fraction: Option<f32>,
    /// Layout of this file, checked before anything in it is believed.
    pub version: u32,
    pub geometry: Option<Geometry>,
    pub window_state: WindowState,
    /// Most recently opened first, at most [`RECENT_LIMIT`] of them.
    #[serde(default)]
    pub recent: Vec<Recent>,
}

impl Default for Session {
    fn default() -> Self {
        Self {
            version: VERSION,
            waveform_fraction: None,
            geometry: None,
            window_state: WindowState::default(),
            recent: Vec::new(),
        }
    }
}

impl Session {
    /// Read the session, falling back to defaults for every failure.
    ///
    /// Every failure but one leaves the file writable: a missing, unreadable or
    /// corrupt session has nothing worth keeping. A session from a newer
    /// version does, so that one is read as defaults *and* closed to writing.
    pub fn load(path: &Path) -> Restored {
        let fresh = Restored {
            session: Self::default(),
            writable: true,
        };

        let text = match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                tracing::debug!(path = %path.display(), "no session yet, starting fresh");
                return fresh;
            }
            Err(error) => {
                tracing::warn!(path = %path.display(), %error, "cannot read session, starting fresh");
                return fresh;
            }
        };

        // The version is read on its own, before anything else is asked of the
        // text. A file from a newer layout is exactly the file whose *other*
        // fields this version cannot parse, so a single parse would fail on
        // them and report a corrupt session -- and then overwrite it, which is
        // the one thing that must not happen to a file another version wrote.
        #[derive(Deserialize)]
        struct Versioned {
            version: u32,
        }

        match toml::from_str::<Versioned>(&text) {
            Ok(Versioned { version }) if READABLE.contains(&version) => {}
            Ok(Versioned { version }) => {
                tracing::warn!(
                    path = %path.display(),
                    found = version,
                    expected = VERSION,
                    "session written by another version; starting fresh and leaving it alone"
                );
                return Restored {
                    session: Self::default(),
                    writable: false,
                };
            }
            Err(error) => {
                tracing::warn!(path = %path.display(), %error, "corrupt session, starting fresh");
                return fresh;
            }
        }

        match toml::from_str::<Self>(&text) {
            // Read at whatever version wrote it, written back at this one:
            // anything an older layout did not have is at its default, which
            // is what an absent field means.
            Ok(session) => Restored {
                session: Self {
                    version: VERSION,
                    waveform_fraction: session
                        .waveform_fraction
                        .filter(|value| value.is_finite() && (0.0..=1.0).contains(value)),
                    ..session
                },
                writable: true,
            },
            Err(error) => {
                tracing::warn!(path = %path.display(), %error, "corrupt session, starting fresh");
                fresh
            }
        }
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

    /// Put a file at the head of the recent list.
    ///
    /// The same path written the same way moves to the head rather than
    /// appearing twice, and it moves with the hints it was opened with this
    /// time: a capture reopened with a corrected sample rate should come back
    /// with the corrected one.
    ///
    /// Written the same way, not the same file: the comparison is between the
    /// absolute paths, and a link and its target are two of those. How much
    /// else two spellings share depends on the platform -- `std::path::absolute`
    /// drops a `.` everywhere and folds a `..` on Windows -- so this merges
    /// some pairs and not others, and does not try to say which.
    ///
    /// It does not ask the filesystem what a path points at. This is a list of
    /// the names a person opened things by, and two names for one capture are
    /// two names; [`recent_labels`] already tells apart the ones that would
    /// read alike. A list of files instead would mean a filesystem call for
    /// every stored entry on every open, and would still be wrong for one that
    /// has since moved.
    pub fn remember(&mut self, path: &Path, hints: &OpenHints) {
        // Absolute, because the list outlives the directory the application
        // was started in. `argand dump.bin` stored literally would, from
        // somewhere else, either fail to open or -- worse -- open a different
        // `dump.bin` with the first one's layout hints.
        //
        // Made absolute rather than canonical: a link is a name a person chose
        // and expects to see again, and resolving it would also require the
        // file still to be there, which is not a condition for remembering
        // where it was.
        let path = std::path::absolute(path).unwrap_or_else(|error| {
            tracing::warn!(path = %path.display(), %error, "cannot resolve the path; remembering it as given");
            path.to_owned()
        });
        // TOML is UTF-8 by definition and a filename on Linux is any bytes, so
        // a path that is not one cannot be written. Refusing it here costs the
        // entry; letting it into the list would cost every later save,
        // including the window's own geometry, for as long as it stayed there.
        if path.to_str().is_none() {
            tracing::warn!(
                path = %path.display(),
                "not a UTF-8 path; it cannot be written to the session and is not remembered"
            );
            return;
        }
        self.recent.retain(|entry| entry.path != path);
        self.recent.insert(
            0,
            Recent {
                path,
                hints: Hints::from(hints),
            },
        );
        self.recent.truncate(RECENT_LIMIT);
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
        return Some(saved.capped(UNVERIFIED_MAX_DIMENSION));
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

/// The rectangle to come back to, given what the window reports now.
///
/// `None` where this report says nothing about it -- a maximized or fullscreen
/// window reports the screen it covers, not the size it would return to -- and
/// the caller keeps whatever it had.
///
/// A backend that misreports the state defeats this, and two do: X11 answers
/// `is_maximized()` with false while a maximized window is minimized, and macOS
/// calls a window maximized only once its size matches the visible frame
/// exactly, so the frames of a maximize animation arrive as ordinary. gpui
/// exposes neither a minimized nor a transitional predicate, and filtering on
/// geometry instead was tried and withdrawn: a window merely the size of some
/// other display is not a maximized one, and refusing it loses an ordinary
/// resize outright. Issue #38 carries the analysis and what would work.
pub fn restore_rectangle(reported: Geometry, state: WindowState) -> Option<Geometry> {
    (state == WindowState::Normal).then_some(reported)
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
