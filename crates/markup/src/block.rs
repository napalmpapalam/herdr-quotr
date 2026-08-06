//! What a line is: its block-level shape, once its markers have been read.

/// A line's block shape. Markers that survive stripping — a bullet, a quote's `>`, a table's
/// pipes — are still in the text and carry an offset; a heading's `#`s are gone.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Block {
    #[default]
    Prose,
    /// ATX heading, one to six deep.
    Heading(usize),
    /// Blockquote; the payload is how many characters its `> ` prefix takes.
    Quote(usize),
    /// List item, its marker at `at` and `len` characters long.
    Bullet {
        at: usize,
        len: usize,
    },
    /// All marker and no content: a thematic break or a table's `|---|` delimiter row.
    Rule,
    TableRow,
    /// A fenced block's opening or closing rail.
    Rail,
    /// A line inside a fenced block. Literal, so nothing in it was stripped.
    Code,
}
