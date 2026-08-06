//! The unit the picker scrolls and quotes: one line of the transcript.

use markup::{Block, Span, Tone};

/// One line of the transcript with its markdown markers read off it.
///
/// `text` is what the picker shows *and* what a quote sends — the two cannot drift, because
/// there is only one string.
#[derive(Debug, Clone)]
pub struct SourceLine {
    pub text: String,
    pub tone: Tone,
    pub block: Block,
    /// Emphasis over `text`, in order and non-overlapping.
    pub spans: Vec<Span>,
}

impl SourceLine {
    /// The blank row between two turns.
    pub(crate) fn gap() -> Self {
        Self { text: String::new(), tone: Tone::Gap, block: Block::Prose, spans: Vec::new() }
    }

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
    use super::{Block, SourceLine, Tone};

    fn line(text: &str) -> SourceLine {
        SourceLine {
            text: text.to_owned(),
            tone: Tone::Agent,
            block: Block::Prose,
            spans: Vec::new(),
        }
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
