//! `argand`: the application binary.
//!
//! This milestone is the shell only -- a window that opens, is configured, and
//! remembers where it was. Signals, analysis and the real panels arrive with
//! the milestones after it.
//!
//! Two files back it, and each has exactly one writer. `argand.toml` is a
//! person's and is only ever read; `session.toml` is the program's and is
//! rewritten as the window moves. Both are read before the window exists, and
//! neither can stop it appearing: see [`config`] and [`session`].

mod config;
mod session;
mod shell;

use config::Config;
use session::Session;

fn main() {
    init_tracing();

    // Both files are read before the window is created, and nothing expensive
    // shares that path: what the window opens as depends on them.
    let config = Config::load(&Config::search_path());
    let state_path = Session::path();
    let saved = state_path
        .as_deref()
        .map_or_else(Session::default, Session::load);

    shell::run(config, saved, state_path);
}

/// The subscriber, set up as `aspec` sets its own up.
///
/// A GUI has no verbosity flag to answer to, so the default is the level a
/// person who has not asked for logs wants, and `RUST_LOG` overrides it for
/// anyone who has.
fn init_tracing() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("argand=info"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .try_init();
}
