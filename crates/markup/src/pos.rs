//! Where a selection starts and ends.

/// A source line and a character offset into it. Ordering is reading order.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Pos {
    pub line: usize,
    pub col: usize,
}

impl Pos {
    pub fn new(line: usize, col: usize) -> Self {
        Self { line, col }
    }

    /// Start of `line` — where a linewise selection and every vertical move land.
    pub fn line_start(line: usize) -> Self {
        Self { line, col: 0 }
    }
}
