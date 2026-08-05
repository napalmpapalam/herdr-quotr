//! A Claude Code session transcript, flattened into the lines the picker scrolls.

mod entry;
mod line;
mod path;
mod session;
mod transcript;

pub use crate::{
    line::{LineKind, SourceLine},
    path::find,
    session::SessionId,
    transcript::Transcript,
};
