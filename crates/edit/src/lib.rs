//! An edited capture, kept as a piece table over the files it came from.
//!
//! Nothing here copies samples. A [`Capture`] is a list of [`Piece`]s, each a
//! run of samples in one source, so deleting or pasting an hour of a
//! multi-gigabyte recording costs a few list entries. Captures are immutable:
//! an edit builds a new one, which is what makes undo a matter of keeping the
//! old one ([`History`]) and lets a worker go on reading a version the window
//! has already moved past.
//!
//! A sample here is what it is everywhere in argand: one value of a real
//! signal or one I/Q pair, so no edit can split I from Q.

mod capture;
mod history;
mod source;

pub use capture::{Capture, Clip, Piece, SourceId};
pub use history::History;
pub use source::EditedSource;
