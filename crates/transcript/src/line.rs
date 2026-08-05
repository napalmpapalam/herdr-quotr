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

impl SourceLine {
    /// Characters in the line — the unit a selection column counts, not bytes.
    pub fn len(&self) -> usize {
        self.text.chars().count()
    }

    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    /// Characters `from..to`, clamped to the line.
    pub fn slice(&self, from: usize, to: usize) -> &str {
        let start = byte_at(&self.text, from);
        let end = byte_at(&self.text, to.max(from));
        self.text.get(start..end).unwrap_or_default()
    }
}

/// Byte offset of character `col`, or the end of the string when it runs past.
fn byte_at(text: &str, col: usize) -> usize {
    text.char_indices().nth(col).map_or(text.len(), |(at, _)| at)
}

#[cfg(test)]
mod tests {
    use super::{LineKind, SourceLine};

    fn line(text: &str) -> SourceLine {
        SourceLine { text: text.to_owned(), kind: LineKind::Agent }
    }

    #[test]
    fn slices_by_character_not_byte() {
        let line = line("日本語abc");
        assert_eq!(line.len(), 6);
        assert_eq!(line.slice(1, 4), "本語a");
    }

    #[test]
    fn clamps_a_range_that_runs_past_the_line() {
        assert_eq!(line("abc").slice(1, 99), "bc");
        assert_eq!(line("abc").slice(9, 99), "");
        assert_eq!(line("abc").slice(2, 1), "");
    }
}
