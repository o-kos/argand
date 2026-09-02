use super::*;

/// A scratch directory removed when it goes out of scope.
struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new(label: &str) -> Self {
        use std::sync::atomic::{AtomicU32, Ordering};
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "argand-app-{label}-{}-{unique}",
            std::process::id()
        ));
        std::fs::create_dir_all(&path).expect("create scratch dir");
        Self { path }
    }

    fn write(&self, name: &str, text: &str) -> PathBuf {
        let path = self.path.join(name);
        std::fs::write(&path, text).expect("write fixture");
        path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

#[test]
fn a_file_that_sets_one_value_is_a_complete_file() {
    let dir = TempDir::new("partial");
    let path = dir.write("argand.toml", "theme = \"light\"\n");

    let config = Config::load(&[path]);
    assert_eq!(config.theme, Theme::Light);
    // Everything it did not mention keeps the shipped value.
    assert_eq!(config.panels, Panels::default());
}

#[test]
fn nothing_in_the_file_can_stop_the_application_starting() {
    let dir = TempDir::new("broken");
    // Each of these is a different way for a person to get it wrong.
    for (name, text) in [
        ("truncated.toml", "theme = "),
        ("wrong-type.toml", "theme = 42"),
        ("unknown-key.toml", "colour_scheme = \"oceanic\""),
        ("not-toml.toml", "<html></html>"),
    ] {
        let path = dir.write(name, text);
        assert_eq!(
            Config::load(&[path]),
            Config::default(),
            "{name} did not fall back to defaults"
        );
    }
}

#[test]
fn a_missing_file_is_not_a_failure_and_does_not_stop_the_search() {
    let dir = TempDir::new("search");
    let missing = dir.path.join("absent.toml");
    let present = dir.write("argand.toml", "theme = \"light\"\n");

    // The first candidate is simply not there, so the second one answers.
    assert_eq!(
        Config::load(&[missing.clone(), present]).theme,
        Theme::Light
    );
    // And no candidate at all is still a working configuration.
    assert_eq!(Config::load(&[missing]), Config::default());
}

#[test]
fn the_copy_beside_the_binary_wins_over_the_one_on_the_host() {
    let dir = TempDir::new("order");
    let beside = dir.write("beside.toml", "theme = \"light\"\n");
    let host = dir.write("host.toml", "theme = \"dark\"\n");

    // Whichever comes first in the search path decides, and `search_path`
    // puts the executable's own directory there.
    assert_eq!(Config::load(&[beside, host]).theme, Theme::Light);
    let ordered = Config::search_path();
    assert!(!ordered.is_empty(), "nothing to read a configuration from");
}

#[test]
fn the_application_never_writes_the_file_a_person_owns() {
    let dir = TempDir::new("readonly");
    let text = "# a comment someone wrote\ntheme = \"light\"\n";
    let path = dir.write("argand.toml", text);

    let _ = Config::load(std::slice::from_ref(&path));
    assert_eq!(
        std::fs::read_to_string(&path).expect("the file is still there"),
        text,
        "loading the configuration rewrote it"
    );
}
