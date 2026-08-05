//! Fenced code blocks, collected whole so `syntect` can highlight them in one pass.

use crate::{highlight::Highlighter, markdown::block, style::LineStyle};

/// An open fenced block, collecting its body until the closing marker.
#[derive(Debug)]
pub(super) struct Fence {
    /// The marker and how many of it opened the block — only a run at least this long closes.
    marker: char,
    len: usize,
    /// The fence's info string, matched as a language by the highlighter.
    lang: String,
    /// Source line the body starts on.
    start: usize,
    body: String,
}

impl Fence {
    /// The fence `text` opens, if it opens one: up to three spaces, then three or more
    /// backticks or tildes, then an info string. `start` is the line its body begins on.
    pub(super) fn opened_by(text: &str, start: usize) -> Option<Self> {
        if block::indent_of(text) >= 4 {
            return None;
        }

        let rest = text.trim_start_matches(' ');
        let marker = rest.chars().next().filter(|&c| c == '`' || c == '~')?;
        let len = rest.chars().take_while(|&c| c == marker).count();
        if len < 3 {
            return None;
        }

        let info: String = rest.chars().skip(len).collect();
        // A backtick fence's info string may not hold a backtick — that is inline code.
        if marker == '`' && info.contains('`') {
            return None;
        }

        let lang = info.split_whitespace().next().unwrap_or_default().to_owned();
        Some(Self { marker, len, lang, start, body: String::new() })
    }

    /// Whether `text` is a closing marker: the same character, at least as long, alone.
    pub(super) fn closed_by(&self, text: &str) -> bool {
        let rest = text.trim_start_matches(' ');

        block::indent_of(text) < 4
            && rest.chars().take_while(|&c| c == self.marker).count() >= self.len
            && rest.trim_end_matches(self.marker).trim().is_empty()
    }

    pub(super) fn push(&mut self, text: &str) {
        self.body.push_str(text);
        self.body.push('\n');
    }

    /// Highlight the collected body, paired with the source line each result belongs to.
    pub(super) fn highlight(&self, highlighter: &Highlighter) -> Vec<(usize, LineStyle)> {
        highlighter
            .runs(&self.body, &self.lang)
            .into_iter()
            .enumerate()
            .map(|(offset, runs)| (self.start + offset, LineStyle::new(runs)))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::Fence;

    #[test]
    fn only_a_long_enough_run_of_the_same_marker_closes_a_fence() {
        let fence = Fence::opened_by("````js", 1);
        assert_eq!(fence.as_ref().map(|f| f.lang.as_str()), Some("js"));
        assert_eq!(fence.as_ref().map(|f| f.closed_by("```")), Some(false));
        assert_eq!(fence.as_ref().map(|f| f.closed_by("````")), Some(true));
        assert_eq!(fence.as_ref().map(|f| f.closed_by("~~~~")), Some(false));
    }

    #[test]
    fn three_markers_are_the_minimum_and_inline_code_is_not_one() {
        assert!(Fence::opened_by("``", 1).is_none());
        assert!(Fence::opened_by("`inline`", 1).is_none());
        assert!(Fence::opened_by("```", 1).is_some());
    }
}
