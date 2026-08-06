//! Markdown markers read and removed, so what the picker shows is what the agent gets sent.
//!
//! Fence rails stay: they carry the language and the block boundary, and an unfenced block
//! would reach the agent looking like prose.

mod inline;

use markup::{Block, Span};

use crate::block::{self, CODE_INDENT};

/// One line with its markers gone: the text to show, what the line is, and how it reads.
#[derive(Debug)]
pub(crate) struct Stripped {
    pub(crate) text: String,
    pub(crate) block: Block,
    pub(crate) spans: Vec<Span>,
}

/// Read one turn's body. A fenced block stays literal, and a fence never outlives its turn.
pub(crate) fn strip(body: &str) -> Vec<Stripped> {
    let mut open: Option<Fence> = None;

    body.lines()
        .map(|text| match open.take() {
            Some(fence) => inside(&mut open, fence, text),
            None => outside(&mut open, text),
        })
        .collect()
}

/// A line inside a fenced block: the closing rail, or one more literal body line.
fn inside(open: &mut Option<Fence>, fence: Fence, text: &str) -> Stripped {
    if fence.closed_by(text) {
        return literal(text, Block::Rail);
    }

    *open = Some(fence);
    literal(text, Block::Code)
}

/// A line outside one: it opens a fence, or it is prose with markers to read.
fn outside(open: &mut Option<Fence>, text: &str) -> Stripped {
    let Some(fence) = Fence::opened_by(text) else { return prose(text) };

    *open = Some(fence);
    literal(text, Block::Rail)
}

/// A line whose markers are read: the block prefix that survives, then the stripped rest.
fn prose(text: &str) -> Stripped {
    let chars: Vec<char> = text.chars().collect();
    let block = block::classify(&chars);
    if block == Block::Rule {
        return literal(text, block);
    }

    let (keep, skip) = prefix(block, &chars);
    let head: String = chars.get(..keep).unwrap_or_default().iter().collect();
    let (tail, spans) = inline::strip(chars.get(keep + skip..).unwrap_or_default());

    Stripped {
        text: head + &tail,
        block,
        spans: spans
            .into_iter()
            .map(|span| Span { from: span.from + keep, to: span.to + keep, ..span })
            .collect(),
    }
}

/// Characters of the block marker to keep, then how many to drop. Only a heading's `#`s go:
/// a bullet and a quote's rule still read as themselves in plain text.
fn prefix(block: Block, chars: &[char]) -> (usize, usize) {
    match block {
        Block::Heading(level) => {
            let at = block::indent(chars) + level;
            (0, at + usize::from(chars.get(at) == Some(&' ')))
        }
        Block::Quote(len) => (len, 0),
        Block::Bullet { at, len } => (at + len, 0),
        _ => (0, 0),
    }
}

fn literal(text: &str, block: Block) -> Stripped {
    Stripped { text: text.to_owned(), block, spans: Vec::new() }
}

/// An open fenced block: only a run of the same marker, at least as long, closes it.
struct Fence {
    marker: char,
    len: usize,
}

impl Fence {
    /// The fence `text` opens, if it opens one: up to three spaces, then three or more
    /// backticks or tildes, then an info string.
    fn opened_by(text: &str) -> Option<Self> {
        if block::indent_of(text) >= CODE_INDENT {
            return None;
        }

        let rest = text.trim_start_matches(' ');
        let marker = rest.chars().next().filter(|&c| c == '`' || c == '~')?;
        let len = rest.chars().take_while(|&c| c == marker).count();
        // A backtick fence's info string may not hold a backtick — that is inline code.
        if len < 3 || (marker == '`' && rest.chars().skip(len).any(|c| c == '`')) {
            return None;
        }

        Some(Self { marker, len })
    }

    /// Whether `text` is a closing rail: the same character, at least as long, alone.
    fn closed_by(&self, text: &str) -> bool {
        let rest = text.trim_start_matches(' ');

        block::indent_of(text) < CODE_INDENT
            && rest.chars().take_while(|&c| c == self.marker).count() >= self.len
            && rest.trim_end_matches(self.marker).trim().is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::{Block, strip};

    fn lines(body: &str) -> Vec<(String, Block)> {
        strip(body).into_iter().map(|line| (line.text, line.block)).collect()
    }

    fn text(body: &str) -> Vec<String> {
        strip(body).into_iter().map(|line| line.text).collect()
    }

    #[test]
    fn a_heading_loses_its_hashes_and_keeps_its_depth() {
        assert_eq!(lines("## Title"), [("Title".to_owned(), Block::Heading(2))]);
        assert_eq!(text("###### **Deep**"), ["Deep"]);
    }

    #[test]
    fn a_bullet_and_a_quote_keep_their_markers() {
        assert_eq!(text("- an *item*"), ["- an item"]);
        assert_eq!(text("  12. `x`"), ["  12. x"]);
        assert_eq!(text("> a **quote**"), ["> a quote"]);
    }

    #[test]
    fn a_spans_offset_counts_from_the_start_of_the_line() {
        let quoted = strip("> a **b**");
        // "> a b" — the bold sits on the last character, past the two-character prefix.
        assert_eq!(quoted.first().map(|l| l.text.as_str()), Some("> a b"));
        let span = quoted.first().and_then(|l| l.spans.first()).copied();
        assert_eq!(span.map(|s| (s.from, s.to, s.emphasis.strong)), Some((4, 5, true)));
    }

    #[test]
    fn a_fenced_block_keeps_its_rails_and_its_body_untouched() {
        let fenced = lines("```rust\nlet x = **1**;\n```\nafter");
        assert_eq!(fenced.first(), Some(&("```rust".to_owned(), Block::Rail)));
        assert_eq!(fenced.get(1), Some(&("let x = **1**;".to_owned(), Block::Code)));
        assert_eq!(fenced.get(2), Some(&("```".to_owned(), Block::Rail)));
        assert_eq!(fenced.get(3), Some(&("after".to_owned(), Block::Prose)));
    }

    #[test]
    fn an_unclosed_fence_runs_to_the_end_of_the_turn() {
        let fenced = lines("````js\n```\nstill code");
        assert_eq!(fenced.get(1).map(|l| l.1), Some(Block::Code));
        assert_eq!(fenced.get(2).map(|l| l.1), Some(Block::Code));
    }

    #[test]
    fn a_rule_is_left_exactly_as_it_was() {
        assert_eq!(lines("---"), [("---".to_owned(), Block::Rule)]);
        assert_eq!(lines("|---|:--|"), [("|---|:--|".to_owned(), Block::Rule)]);
    }

    #[test]
    fn a_table_row_keeps_its_pipes_and_loses_the_markers_between_them() {
        assert_eq!(lines("| `a` | **b** |"), [("| a | b |".to_owned(), Block::TableRow)]);
    }
}
