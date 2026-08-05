//! A Claude Code session transcript, flattened into the lines the picker scrolls.

mod command;
mod entry;
mod line;
mod path;
mod pos;
mod session;
mod transcript;

pub use crate::{
    line::{LineKind, SourceLine},
    path::find,
    pos::Pos,
    session::SessionId,
    transcript::Transcript,
};
