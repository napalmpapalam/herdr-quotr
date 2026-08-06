//! Who wrote a line.

/// `Gap` is the blank row between turns, dropped from any quote.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Agent,
    User,
    #[default]
    Gap,
}
