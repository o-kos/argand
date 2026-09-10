//! `argand`: the application binary.
//!
//! The window opens a signal file, analyses it on a thread of its own and
//! shows the spectrogram and waveform that come back. Editing and selections
//! arrive with the milestones after it.
//!
//! Two files back it, and each has exactly one writer. `argand.toml` is a
//! person's and is only ever read; `session.toml` is the program's and is
//! rewritten as the window moves. Both are read before the window exists, and
//! neither can stop it appearing: see [`config`] and [`session`].

mod analysis;
mod axes;
mod chrome;
mod cli;
mod config;
mod cpu;
mod document;
mod execution;
mod minimap;
mod navigation;
mod panels;
mod profiling;
mod recent;
mod session;
mod settings;
mod shell;
mod spectrogram;
mod waveform;

use clap::Parser;

use cli::Args;
use config::Config;
use session::{Session, Writer};

fn main() {
    let args = Args::parse();
    init_tracing();

    // Both files are read before the window is created, and nothing expensive
    // shares that path: what the window opens as depends on them.
    let config = Config::load(&Config::search_path());

    // A session is only written back when there is somewhere to write and the
    // file there is not from a version this one would be overwriting.
    let state_path = Session::path();
    let restored = state_path.as_deref().map_or_else(
        || session::Restored {
            session: Session::default(),
            writable: true,
        },
        Session::load,
    );
    let writer = state_path
        .filter(|_| restored.writable)
        .map(|path| Writer::new(path, restored.session.clone()));

    shell::run(config, restored.session, writer, args.origin());
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
