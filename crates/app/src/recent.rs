//! Shared recent-file availability, checked without blocking the window.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::session::{RECENT_LIMIT, Recent, normalize_recent_path};

type Availability = (PathBuf, bool);
type Probe = Arc<dyn Fn(&Path) -> bool + Send + Sync>;

pub struct RecentFiles {
    entries: Vec<Recent>,
    current: Option<PathBuf>,
    available: HashSet<PathBuf>,
    pending: HashSet<PathBuf>,
    sender: async_channel::Sender<Availability>,
    receiver: async_channel::Receiver<Availability>,
    probe: Probe,
}

impl RecentFiles {
    pub fn new(entries: &[Recent]) -> Self {
        let (sender, receiver) = async_channel::unbounded();
        Self {
            entries: entries.iter().take(RECENT_LIMIT).cloned().collect(),
            current: None,
            available: HashSet::new(),
            pending: HashSet::new(),
            sender,
            receiver,
            probe: Arc::new(|path| path.is_file()),
        }
    }

    pub fn updates(&self) -> async_channel::Receiver<Availability> {
        self.receiver.clone()
    }

    pub fn set_current(&mut self, path: &Path) {
        self.current = Some(normalize_recent_path(path));
    }

    pub fn clear_current(&mut self) {
        self.current = None;
    }

    pub fn refresh(&mut self, entries: &[Recent]) {
        self.entries = entries.iter().take(RECENT_LIMIT).cloned().collect();
        self.available
            .retain(|path| self.entries.iter().any(|entry| entry.path == *path));
        for path in self
            .entries
            .iter()
            .map(|entry| entry.path.clone())
            .collect::<Vec<_>>()
        {
            self.start_check(path);
        }
    }

    pub fn apply(&mut self, path: PathBuf, exists: bool) {
        self.pending.remove(&path);
        if exists && self.entries.iter().any(|entry| entry.path == path) {
            self.available.insert(path);
        } else {
            self.available.remove(&path);
        }
    }

    fn start_check(&mut self, path: PathBuf) {
        if !self.pending.insert(path.clone()) {
            return;
        }
        let sender = self.sender.clone();
        let probe = self.probe.clone();
        let checked = path.clone();
        // One outstanding probe per path, even if history changes. A stuck old
        // path must not prevent a newly opened local file from being checked.
        if let Err(error) = std::thread::Builder::new()
            .name("argand-recent".to_owned())
            .spawn(move || {
                let exists = probe(&checked);
                let _ = sender.send_blocking((checked, exists));
            })
        {
            self.pending.remove(&path);
            self.available.remove(&path);
            tracing::warn!(%error, "cannot check a recent file");
        }
    }

    pub fn shortcut(&self, index: usize) -> Option<Recent> {
        crate::app_menu::digit(index)?;
        self.visible().get(index).cloned()
    }

    pub fn visible(&self) -> Vec<Recent> {
        self.entries
            .iter()
            .filter(|entry| self.available.contains(&entry.path))
            .filter(|entry| self.current.as_ref() != Some(&entry.path))
            .cloned()
            .collect()
    }
}

/// What the start page and File call each shown recent file.
///
/// The file name alone where no other shown file has it. Files sharing a name
/// add their parent folders, nearest first, up to the first that tells each one
/// apart from the others. Folders every one of them has at that depth are
/// elided with `…`, and so are those beyond the telling one. A file whose whole
/// path is needed starts from its root.
///
/// A long file name or folder keeps its start and end around `…`, both as
/// short as still tells it from the other names, or folders at its depth.
pub fn labels(recent: &[Recent]) -> Vec<String> {
    let names: Vec<String> = recent.iter().map(|entry| file_name(&entry.path)).collect();
    let folders: Vec<Vec<String>> = recent.iter().map(|entry| folders(&entry.path)).collect();
    let name_side = side(names.iter());
    (0..recent.len())
        .map(|index| {
            let name = shortened(&names[index], name_side);
            let group: Vec<usize> = (0..recent.len())
                .filter(|&other| names[other] == names[index])
                .collect();
            if group.len() == 1 {
                return name;
            }
            let others: Vec<&[String]> = group
                .iter()
                .filter(|&&other| other != index)
                .map(|&other| folders[other].as_slice())
                .collect();
            let folder = |at: usize, folder: &str| {
                let side = side(group.iter().filter_map(|&member| folders[member].get(at)));
                shortened(folder, side)
            };
            told_apart(&recent[index].path, &name, &folders[index], &others, folder)
        })
        .collect()
}

/// Names no longer than this are shown whole.
const LONGEST_WHOLE: usize = 20;
/// The characters a shortened name keeps on each side at least.
const SHORTEST_SIDE: usize = 8;

/// `text` as its first and last `side` characters around `…`, or whole when that is not shorter.
fn shortened(text: &str, side: usize) -> String {
    let count = text.chars().count();
    if count <= LONGEST_WHOLE || count <= 2 * side + 1 {
        return text.to_owned();
    }
    let head: String = text.chars().take(side).collect();
    let tail: String = text.chars().skip(count - side).collect();
    format!("{head}…{tail}")
}

/// The shortest side that keeps every two different `texts` different once shortened.
fn side<'a>(texts: impl Iterator<Item = &'a String>) -> usize {
    let mut distinct: Vec<&String> = texts.collect();
    distinct.sort_unstable();
    distinct.dedup();
    let longest = distinct
        .iter()
        .map(|text| text.chars().count())
        .max()
        .unwrap_or(0);
    (SHORTEST_SIDE..=longest.max(SHORTEST_SIDE))
        .find(|&side| {
            let mut short: Vec<String> =
                distinct.iter().map(|text| shortened(text, side)).collect();
            short.sort_unstable();
            short.dedup();
            short.len() == distinct.len()
        })
        .unwrap_or(longest)
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .unwrap_or(path.as_os_str())
        .to_string_lossy()
        .into_owned()
}

/// The folders holding `path`, nearest first.
fn folders(path: &Path) -> Vec<String> {
    path.parent()
        .into_iter()
        .flat_map(Path::components)
        .filter_map(|component| match component {
            std::path::Component::Normal(name) => Some(name.to_string_lossy().into_owned()),
            _ => None,
        })
        .rev()
        .collect()
}

/// `name` with the folders of `own` that tell it apart from every one of `others`, each as `folder` shows it.
fn told_apart(
    path: &Path,
    name: &str,
    own: &[String],
    others: &[&[String]],
    folder: impl Fn(usize, &str) -> String,
) -> String {
    let differs = |depth: usize| {
        others
            .iter()
            .all(|other| own[..depth] != other[..depth.min(other.len())])
    };
    let depth = (1..=own.len()).find(|&depth| differs(depth));
    let (depth, whole) = match depth {
        Some(depth) => (depth, depth == own.len()),
        None => (own.len(), true),
    };
    let ellipsis = "…".to_string();
    let mut segments: Vec<String> = Vec::new();
    if !whole {
        segments.push(ellipsis.clone());
    }
    for at in (0..depth).rev() {
        // A path shown to its root is told apart by where it starts, so nothing of it is elided.
        let shared = !whole && others.iter().all(|other| other.get(at) == Some(&own[at]));
        if !shared {
            segments.push(folder(at, &own[at]));
        } else if segments.last() != Some(&ellipsis) {
            segments.push(ellipsis.clone());
        }
    }
    segments.push(name.to_owned());
    let separator = std::path::MAIN_SEPARATOR.to_string();
    let root = match whole {
        true => root(path),
        false => String::new(),
    };
    root + &segments.join(&separator)
}

/// The prefix and root `path` starts from, empty for a relative one.
fn root(path: &Path) -> String {
    path.components()
        .take_while(|component| {
            matches!(
                component,
                std::path::Component::Prefix(_) | std::path::Component::RootDir
            )
        })
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect()
}

#[cfg(test)]
mod tests {
    include!("recent_tests.rs");
}
