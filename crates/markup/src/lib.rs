//! The vocabulary `transcript` writes and `ui` paints: what a line is, and how it reads.
//!
//! A leaf crate so neither side has to depend on the other, and neither has to keep its own
//! copy of these types.

mod block;
mod pos;
mod span;
mod tone;

pub use crate::{
    block::Block,
    pos::Pos,
    span::{Emphasis, Span},
    tone::Tone,
};
