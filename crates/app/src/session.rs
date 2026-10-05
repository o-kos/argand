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
//! Several instances can run at once, so a write is a read-modify-write under
//! an advisory lock beside the file: [`Writer`] reads what is on disk, keeps
//! whatever another instance changed there and lays only its own changes over
//! it, see [`merge`]. The lock is tried without blocking, and a held one is
//! tried again at the next interval or, at exit, a few times over a bounded
//! wait. A filesystem without locks still gets the merge. All of it runs on a
//! thread of its own, see [`Saver`], so a stalled filesystem stalls the save
//! and not the window, and the exit waits for it at most [`Saver::EXIT_WAIT`].
//! [`VERSION`] guards a *downgrade* both at start-up and at every write: a file
//! from a newer layout is left alone.

use std::fs::{File, OpenOptions, TryLockError};
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
/// read. Version 2 added the recent list, version 3 the panel split, version 4 the analysis settings, and version 5 stopped persisting file-specific range, and version 6 added per-file time views, now ignored on load. Version 7 adds the time-ruler presentation; version 8 adds grid visibility; version 9 adds orientation; version 10 adds scale UI visibility; version 11 adds the interface theme chosen in the menu.
pub const VERSION: u32 = 11;

/// Every layout this program can read, oldest first.
///
/// An older file is read into the current shape and written back at
/// [`VERSION`]: missing fields have defaults; legacy dynamic range values are ignored.
const READABLE: [u32; 11] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, VERSION];

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
            level_scan_bytes: None,
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
/// The file name alone, which is what a person recognises. Two captures that
/// share one are told apart where the list has room for it, in the start page's
/// hint, and nowhere else: a list where every line is a path is a list nobody
/// reads, and a name long enough to fill the row is a name that ends in an
/// ellipsis rather than a directory.
pub fn recent_labels(recent: &[Recent]) -> Vec<String> {
    recent
        .iter()
        .map(|entry| {
            entry
                .path
                .file_name()
                .unwrap_or(entry.path.as_os_str())
                .to_string_lossy()
                .into_owned()
        })
        .collect()
}

/// Everything one run hands to the next.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Session {
    #[serde(default)]
    pub orientation: crate::orientation::Mode,
    #[serde(default = "default_grid_visibility")]
    pub show_grid: bool,
    /// Whether the ruler overlay controls ([+|-] corner buttons) are shown.
    #[serde(default = "default_scale_ui_visibility")]
    pub show_scale_ui: bool,
    #[serde(default)]
    pub time_ruler: crate::time_ruler::Mode,
    /// The interface theme chosen in the menu, or none to follow the configuration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme: Option<crate::config::Theme>,
    #[serde(default)]
    pub analysis_settings: Option<crate::settings::Settings>,
    /// Layout of this file, checked before anything in it is believed.
    pub version: u32,
    pub geometry: Option<Geometry>,
    pub window_state: WindowState,
    /// Most recently opened first, at most [`RECENT_LIMIT`] of them.
    #[serde(default)]
    pub recent: Vec<Recent>,
}

fn default_grid_visibility() -> bool {
    true
}

fn default_scale_ui_visibility() -> bool {
    true
}

impl Default for Session {
    fn default() -> Self {
        Self {
            version: VERSION,
            show_grid: true,
            show_scale_ui: true,
            orientation: crate::orientation::Mode::default(),
            time_ruler: crate::time_ruler::Mode::default(),
            theme: None,
            analysis_settings: None,
            geometry: None,
            window_state: WindowState::default(),
            recent: Vec::new(),
        }
    }
}

/// The file as one read of it found it.
enum OnDisk {
    /// A layout this binary reads, already in the current shape.
    Readable(Session),
    /// A layout from a newer binary, which must be left alone.
    Foreign,
    /// Missing, unreadable or corrupt, so nothing in it is worth keeping.
    Unusable,
}

/// What the file's own version says about this binary.
enum VersionGate {
    /// A layout this binary reads.
    Readable,
    /// A layout from a newer binary, which must be left alone.
    ForeignVersion,
    /// Not parseable at all; nothing worth keeping.
    Corrupt,
}

impl Session {
    /// Read the session, falling back to defaults for every failure.
    ///
    /// Every failure but one leaves the file writable: a missing, unreadable or
    /// corrupt session has nothing worth keeping. A session from a newer
    /// version does, so that one is read as defaults *and* closed to writing.
    pub fn load(path: &Path) -> Restored {
        match Self::read(path) {
            OnDisk::Readable(session) => Restored {
                session,
                writable: true,
            },
            OnDisk::Foreign => Restored {
                session: Self::default(),
                writable: false,
            },
            OnDisk::Unusable => Restored {
                session: Self::default(),
                writable: true,
            },
        }
    }

    /// What the file holds now, as start-up and every write see it.
    fn read(path: &Path) -> OnDisk {
        // The version is read on its own, before anything else is asked of the
        // text. A file from a newer layout is exactly the file whose *other*
        // fields this version cannot parse, so a single parse would fail on
        // them and report a corrupt session -- and then overwrite it, which is
        // the one thing that must not happen to a file another version wrote.
        let Some(text) = Self::read_text(path) else {
            return OnDisk::Unusable;
        };
        match Self::version_gate(path, &text) {
            VersionGate::Readable => {}
            VersionGate::ForeignVersion => return OnDisk::Foreign,
            VersionGate::Corrupt => return OnDisk::Unusable,
        }
        Self::parse_payload(path, &text).map_or(OnDisk::Unusable, OnDisk::Readable)
    }

    /// The file's text, or nothing when there is no session to read.
    ///
    /// A missing file is the ordinary first run and stays quiet at debug
    /// level; anything else is warned about. Both leave the session writable.
    fn read_text(path: &Path) -> Option<String> {
        match std::fs::read_to_string(path) {
            Ok(text) => Some(text),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                tracing::debug!(path = %path.display(), "no session yet");
                None
            }
            Err(error) => {
                tracing::warn!(path = %path.display(), %error, "cannot read session, ignoring it");
                None
            }
        }
    }

    /// Probe the file's version before anything else is parsed from it.
    fn version_gate(path: &Path, text: &str) -> VersionGate {
        #[derive(Deserialize)]
        struct Versioned {
            version: u32,
        }

        match toml::from_str::<Versioned>(text) {
            Ok(Versioned { version }) if READABLE.contains(&version) => VersionGate::Readable,
            Ok(Versioned { version }) => {
                tracing::warn!(
                    path = %path.display(),
                    found = version,
                    expected = VERSION,
                    "session written by another version; leaving it alone"
                );
                VersionGate::ForeignVersion
            }
            Err(error) => {
                tracing::warn!(path = %path.display(), %error, "corrupt session, ignoring it");
                VersionGate::Corrupt
            }
        }
    }

    /// The payload parse: whatever an older layout did not have lands at its
    /// default, which is what an absent field means, and the result is
    /// written back at this binary's version.
    fn parse_payload(path: &Path, text: &str) -> Option<Session> {
        match toml::from_str::<Self>(text) {
            Ok(mut session) => {
                session.version = VERSION;
                Some(session)
            }
            Err(error) => {
                tracing::warn!(path = %path.display(), %error, "corrupt session, ignoring it");
                None
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
    /// two names. A list of files instead would mean a filesystem call for
    /// every stored entry on every open, and would still be wrong for one that
    /// has since moved.
    pub fn remember(&mut self, path: &Path, hints: &OpenHints) {
        let path = normalize_recent_path(path);
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

pub fn normalize_recent_path(path: &Path) -> PathBuf {
    // History outlives the working directory and preserves chosen symlink names.
    std::path::absolute(path).unwrap_or_else(|error| {
        tracing::warn!(path = %path.display(), %error, "cannot resolve the path; remembering it as given");
        path.to_owned()
    })
}

/// What another instance left in the file, with this instance's changes on top.
///
/// `base` is the session this instance last read or wrote, `mine` is what it
/// holds now and `theirs` is the file as it stands. A setting comes from `mine`
/// only where `mine` changed it since `base`, so a setting another instance
/// changed meanwhile survives unless this one changed it too. The window
/// rectangle and its state travel as one, because a rectangle belongs to the
/// state it was measured in.
///
/// The recent list is merged by entry rather than taken whole: the files this
/// instance opened since `base` go to the head of the file's list, in the order
/// it opened them, and the rest of that list follows.
pub fn merge(base: &Session, mine: &Session, theirs: &Session) -> Session {
    let window = pick(
        &(base.geometry, base.window_state),
        &(mine.geometry, mine.window_state),
        &(theirs.geometry, theirs.window_state),
    );
    let opened = &mine.recent[..opened_since(&base.recent, &mine.recent)];
    let mut recent = opened.to_vec();
    recent.extend(
        theirs
            .recent
            .iter()
            .filter(|entry| !opened.iter().any(|head| head.path == entry.path))
            .cloned(),
    );
    recent.truncate(RECENT_LIMIT);
    Session {
        orientation: pick(&base.orientation, &mine.orientation, &theirs.orientation),
        show_grid: pick(&base.show_grid, &mine.show_grid, &theirs.show_grid),
        show_scale_ui: pick(
            &base.show_scale_ui,
            &mine.show_scale_ui,
            &theirs.show_scale_ui,
        ),
        time_ruler: pick(&base.time_ruler, &mine.time_ruler, &theirs.time_ruler),
        theme: pick(&base.theme, &mine.theme, &theirs.theme),
        analysis_settings: pick(&persisted(base), &persisted(mine), &persisted(theirs)),
        version: VERSION,
        geometry: window.0,
        window_state: window.1,
        recent,
    }
}

/// The analysis settings as the file holds them, so a change it never stores is no change.
fn persisted(session: &Session) -> Option<crate::settings::Settings> {
    session
        .analysis_settings
        .map(crate::settings::Settings::persisted)
}

fn pick<T: Clone + PartialEq>(base: &T, mine: &T, theirs: &T) -> T {
    if mine == base {
        theirs.clone()
    } else {
        mine.clone()
    }
}

/// How many entries at the head of `mine` were put there since `base`.
///
/// [`Session::remember`] moves an entry to the head and drops its path from
/// the rest, so `mine` is that head followed by what is left of `base`. The
/// shortest head that explains `mine` this way is the answer. A list no head
/// explains counts as opened whole, which merges it as a union rather than
/// losing any of it.
fn opened_since(base: &[Recent], mine: &[Recent]) -> usize {
    if mine == base {
        return 0;
    }
    (0..mine.len())
        .find(|&count| {
            let head = &mine[..count];
            let rest = base
                .iter()
                .filter(|entry| !head.iter().any(|opened| opened.path == entry.path))
                .take(RECENT_LIMIT.saturating_sub(count));
            rest.eq(mine[count..].iter())
        })
        .unwrap_or(mine.len())
}

/// What one attempt to write the session came to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Outcome {
    Written,
    /// The write failed and stays pending for another attempt.
    Failed,
    /// Another instance holds the lock and the write stays pending.
    Busy,
    /// A newer version wrote the file, so this run no longer writes it.
    Foreign,
}

/// The advisory lock around one read-modify-write.
enum Lock {
    /// Released when the file closes.
    Held(File),
    Busy,
    /// The filesystem cannot lock, so the write goes ahead without one.
    Unavailable,
}

/// Turns a stream of window positions into occasional writes.
///
/// The window's geometry is read on every frame it is drawn, so a drag offers
/// hundreds of positions a second. Writing each one would rewrite the file per
/// frame for a value only the last of which matters, so an offer is recorded
/// and written at most once per [`Self::INTERVAL`]. Whatever the last offer
/// left unwritten is flushed when the window closes.
///
/// Every write merges with what another instance may have written since, see
/// [`merge`], under a lock beside the file that is tried rather than waited
/// for: an instance that finds it held keeps its offer for the next interval.
///
/// Time arrives as a parameter rather than being read here, so the schedule can
/// be tested without waiting for it.
pub struct Writer {
    path: PathBuf,
    /// What this instance last read from or wrote to the file, as it saw it.
    ///
    /// It is the base of every [`merge`], and it is this instance's own view
    /// rather than the merged file, so another instance's changes never read
    /// as this one's.
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
    /// Set once a newer version is found to have written the file.
    closed: bool,
    /// Whether writing without a lock has been reported yet.
    reported_unlocked: bool,
}

impl Writer {
    /// Shortest gap between two writes.
    ///
    /// Long enough that a drag writes a handful of times rather than hundreds,
    /// short enough that a session killed without closing loses almost nothing.
    pub const INTERVAL: Duration = Duration::from_millis(500);

    /// Attempts the final flush makes while another instance holds the lock.
    const FLUSH_ATTEMPTS: u32 = 10;

    /// Pause between those attempts, which bounds the wait at exit.
    const FLUSH_PAUSE: Duration = Duration::from_millis(20);

    pub fn new(path: PathBuf, stored: Session) -> Self {
        Self {
            path,
            stored,
            pending: None,
            last_attempt: None,
            closed: false,
            reported_unlocked: false,
        }
    }

    /// Record where the window is now, and write if enough time has passed.
    pub fn offer(&mut self, session: Session, now: Instant) {
        if self.closed {
            return;
        }
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

    /// When the pending offer may be written, if there is one to write.
    pub fn due_at(&self) -> Option<Instant> {
        self.pending.as_ref()?;
        Some(
            self.last_attempt
                .map_or_else(Instant::now, |last| last + Self::INTERVAL),
        )
    }

    /// Write the pending offer if its interval has ended, without a new offer.
    pub fn tick(&mut self, now: Instant) {
        if self.due_at().is_some_and(|due| now >= due) {
            self.write(now);
        }
    }

    /// Write whatever is still pending, whatever the schedule says.
    ///
    /// Another instance holding the lock is waited for, briefly and boundedly,
    /// because nothing comes after this to try again.
    pub fn flush(&mut self, now: Instant) {
        for attempt in 0..Self::FLUSH_ATTEMPTS {
            if attempt > 0 {
                std::thread::sleep(Self::FLUSH_PAUSE);
            }
            if self.pending.is_none() || self.write(now) != Outcome::Busy {
                return;
            }
        }
        tracing::warn!(
            path = %self.path.display(),
            "another instance kept the session locked; this run's last changes are not saved"
        );
    }

    fn write(&mut self, now: Instant) -> Outcome {
        let Some(session) = self.pending.clone() else {
            return Outcome::Written;
        };
        // The attempt counts whatever came of it, so a failure waits its turn
        // like a success does.
        self.last_attempt = Some(now);
        let outcome = self.merge_and_save(&session);
        match outcome {
            Outcome::Written => {
                self.pending = None;
                self.stored = session;
            }
            Outcome::Foreign => {
                tracing::warn!(
                    path = %self.path.display(),
                    "a newer version has written the session; this run no longer saves it"
                );
                self.pending = None;
                self.closed = true;
            }
            // Kept pending for another chance at the next interval or at the flush
            Outcome::Failed | Outcome::Busy => {}
        }
        outcome
    }

    fn merge_and_save(&mut self, mine: &Session) -> Outcome {
        if let Some(parent) = self.path.parent()
            && let Err(error) = std::fs::create_dir_all(parent)
        {
            tracing::warn!(path = %self.path.display(), %error, "cannot save session");
            return Outcome::Failed;
        }
        let _held = match self.lock() {
            Lock::Held(file) => Some(file),
            Lock::Unavailable => None,
            Lock::Busy => return Outcome::Busy,
        };
        let merged = match Session::read(&self.path) {
            OnDisk::Readable(theirs) => merge(&self.stored, mine, &theirs),
            OnDisk::Foreign => return Outcome::Foreign,
            OnDisk::Unusable => mine.clone(),
        };
        if merged.save(&self.path) {
            Outcome::Written
        } else {
            Outcome::Failed
        }
    }

    /// Try the lock beside the session, without waiting for it.
    fn lock(&mut self) -> Lock {
        let opened = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(false)
            .open(self.path.with_extension("toml.lock"));
        let error = match opened.map(|file| (file.try_lock(), file)) {
            Ok((Ok(()), file)) => return Lock::Held(file),
            Ok((Err(TryLockError::WouldBlock), _)) => return Lock::Busy,
            Ok((Err(TryLockError::Error(error)), _)) | Err(error) => error,
        };
        if !self.reported_unlocked {
            self.reported_unlocked = true;
            tracing::warn!(
                path = %self.path.display(),
                %error,
                "cannot lock the session; saving it without a lock"
            );
        }
        Lock::Unavailable
    }
}

/// Writes the session on a thread of its own, so a stalled filesystem stalls
/// the save rather than the window.
///
/// The shell hands over the session as it stands and goes on. Offers pass
/// through a mailbox of one: a newer offer replaces one the thread has not
/// taken yet, so however long a write hangs, the thread is owed one session
/// and not a queue of them. The thread runs a [`Writer`], which keeps its
/// schedule, and writes a throttled offer once its interval ends.
///
/// Dropping it asks for a final flush and waits for that at most
/// [`Self::EXIT_WAIT`]. A thread still writing after that is left to the end
/// of the process, and the atomic rename keeps the file whole whatever moment
/// that is.
pub struct Saver {
    shared: std::sync::Arc<Mailbox>,
}

/// What the shell and the writing thread share.
struct Mailbox {
    slot: std::sync::Mutex<Slot>,
    /// Signalled when an offer or the closing request arrives.
    arrived: std::sync::Condvar,
    /// Signalled once the final flush is done.
    finished: std::sync::Condvar,
}

#[derive(Default)]
struct Slot {
    offer: Option<Session>,
    closing: bool,
    finished: bool,
    /// Set when the exit stopped waiting before the final flush finished.
    abandoned: bool,
}

impl Saver {
    /// Longest the exit waits for the final write.
    pub const EXIT_WAIT: Duration = Duration::from_secs(1);

    /// Start the thread, or answer nothing when the platform will not give one.
    pub fn spawn(writer: Writer) -> Option<Self> {
        let shared = std::sync::Arc::new(Mailbox {
            slot: std::sync::Mutex::new(Slot::default()),
            arrived: std::sync::Condvar::new(),
            finished: std::sync::Condvar::new(),
        });
        let thread = std::sync::Arc::clone(&shared);
        let spawned = std::thread::Builder::new()
            .name("argand-session".to_owned())
            .spawn(move || thread.run(writer));
        match spawned {
            Ok(_) => Some(Self { shared }),
            Err(error) => {
                tracing::warn!(%error, "cannot start the session writer; this run is not remembered");
                None
            }
        }
    }

    /// Hand over the session as it now stands, without waiting for any write.
    pub fn offer(&self, session: Session) {
        let mut slot = self.shared.lock();
        slot.offer = Some(session);
        self.shared.arrived.notify_one();
    }

    /// Ask for the final flush and wait for it at most `wait`.
    ///
    /// Answers whether the flush finished in time.
    fn close(&self, wait: Duration) -> bool {
        let mut slot = self.shared.lock();
        slot.closing = true;
        self.shared.arrived.notify_one();
        let deadline = Instant::now() + wait;
        while !slot.finished {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                slot.abandoned = true;
                return false;
            }
            slot = match self.shared.finished.wait_timeout(slot, left) {
                Ok((slot, _)) => slot,
                Err(poisoned) => poisoned.into_inner().0,
            };
        }
        true
    }
}

impl Drop for Saver {
    /// Nothing is logged here, because a blocked log would hold up the exit past its bound.
    fn drop(&mut self) {
        self.close(Self::EXIT_WAIT);
    }
}

impl Mailbox {
    /// The slot, even after a panic elsewhere left the lock poisoned.
    fn lock(&self) -> std::sync::MutexGuard<'_, Slot> {
        self.slot
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn run(&self, mut writer: Writer) {
        loop {
            let (offer, closing) = self.next(writer.due_at());
            if let Some(session) = offer {
                writer.offer(session, Instant::now());
            }
            if closing {
                writer.flush(Instant::now());
                let mut slot = self.lock();
                slot.finished = true;
                self.finished.notify_all();
                if slot.abandoned {
                    tracing::warn!(
                        "the session was still being written when the exit stopped waiting for it"
                    );
                }
                return;
            }
            writer.tick(Instant::now());
        }
    }

    /// Wait for an offer, the closing request or `due`, whichever comes first.
    fn next(&self, due: Option<Instant>) -> (Option<Session>, bool) {
        let mut slot = self.lock();
        loop {
            if slot.offer.is_some() || slot.closing {
                return (slot.offer.take(), slot.closing);
            }
            slot = match due {
                None => self
                    .arrived
                    .wait(slot)
                    .unwrap_or_else(std::sync::PoisonError::into_inner),
                Some(due) => {
                    let left = due.saturating_duration_since(Instant::now());
                    if left.is_zero() {
                        return (None, false);
                    }
                    match self.arrived.wait_timeout(slot, left) {
                        Ok((slot, _)) => slot,
                        Err(poisoned) => poisoned.into_inner().0,
                    }
                }
            };
        }
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
