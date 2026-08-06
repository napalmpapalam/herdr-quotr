//! How a run of characters reads, once the markers that said so are gone.

/// Emphasis carried by a run of text. Flat, not nested: a code span inside bold arrives as
/// one run holding both, so the painter never has to reconstruct a tree.
#[expect(clippy::struct_excessive_bools, reason = "one flag per construct, all independent")]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Emphasis {
    pub code: bool,
    pub strong: bool,
    pub italic: bool,
    pub struck: bool,
    pub link: bool,
    /// A link's destination, kept for the reader but stepped back from its label.
    pub dim: bool,
}

impl Emphasis {
    /// Whether this run reads as ordinary prose.
    pub fn is_plain(self) -> bool {
        self == Self::default()
    }
}

/// Characters `from..to` of a stripped line, and how they read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub from: usize,
    pub to: usize,
    pub emphasis: Emphasis,
}

impl Span {
    /// The part of this span inside `from..to`, moved to start at `at`.
    #[must_use]
    pub fn shift(self, from: usize, to: usize, at: usize) -> Option<Self> {
        let start = self.from.max(from);
        let end = self.to.min(to);

        // Lazy: a span that misses this window entirely would underflow the arithmetic.
        (start < end).then(|| Self {
            from: start - from + at,
            to: end - from + at,
            emphasis: self.emphasis,
        })
    }
}
