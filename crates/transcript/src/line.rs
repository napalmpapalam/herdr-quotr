//! The unit the picker scrolls and quotes: one line of the transcript.

/// Who a line came from. `Gap` is the blank row between turns, dropped from any quote.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineKind {
    Agent,
    User,
    Gap,
}

/// One line of the transcript, exactly as written.
#[derive(Debug, Clone)]
pub struct SourceLine {
    pub text: String,
    pub kind: LineKind,
}
