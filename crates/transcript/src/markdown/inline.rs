//! Inline markers read and removed: `**bold**` becomes `bold`, `[a](b)` becomes `a (b)`.

use markup::{Emphasis, Span};

/// Strip `chars`, returning the clean text and the runs of it that carry emphasis.
pub(super) fn strip(chars: &[char]) -> (String, Vec<Span>) {
    let mut out = Strip { chars, text: String::new(), len: 0, spans: Vec::new() };
    out.scan(0, chars.len(), Emphasis::default());
    out.spans.retain(|span| !span.emphasis.is_plain());
    (out.text, out.spans)
}

/// Walks the source left to right, writing the stripped text and the spans over it.
struct Strip<'a> {
    chars: &'a [char],
    text: String,
    /// Characters written so far — where the next one lands in [`Self::text`].
    len: usize,
    spans: Vec<Span>,
}

impl Strip<'_> {
    fn at(&self, i: usize) -> Option<char> {
        self.chars.get(i).copied()
    }

    fn source(&self, from: usize, to: usize) -> String {
        self.chars.get(from..to).unwrap_or_default().iter().collect()
    }

    /// Write one character, extending the run before it when the emphasis has not changed.
    fn push(&mut self, c: char, emphasis: Emphasis) {
        self.text.push(c);
        match self.spans.last_mut() {
            Some(last) if last.emphasis == emphasis && last.to == self.len => last.to += 1,
            _ => self.spans.push(Span { from: self.len, to: self.len + 1, emphasis }),
        }
        self.len += 1;
    }

    fn push_str(&mut self, text: &str, emphasis: Emphasis) {
        for c in text.chars() {
            self.push(c, emphasis);
        }
    }

    /// Copy `from..to` through untouched — a code span's body, which is literal.
    fn copy(&mut self, from: usize, to: usize, emphasis: Emphasis) {
        for i in from..to {
            if let Some(c) = self.at(i) {
                self.push(c, emphasis);
            }
        }
    }

    /// Walk `from..to`, taking each construct whole. Emphasis recurses, so a code span inside
    /// bold comes out holding both; a marker whose partner is missing is written as text.
    fn scan(&mut self, from: usize, to: usize, outer: Emphasis) {
        let mut i = from;
        while i < to {
            let taken = match self.at(i) {
                Some('`') => self.code(i, to, outer),
                Some('[' | '!') => self.link(i, to, outer),
                Some('<') => self.autolink(i, to, outer),
                Some('*' | '_' | '~') => self.emphasis(i, to, outer),
                _ => None,
            };
            // Nothing taken means the character is not a marker after all, so it is text.
            if taken.is_none() {
                self.copy(i, i + 1, outer);
            }
            i += taken.unwrap_or(1);
        }
    }

    /// `` `code` `` — the body is literal, so nothing inside it is scanned.
    fn code(&mut self, i: usize, to: usize, outer: Emphasis) -> Option<usize> {
        let ticks = self.run_len(i, '`');
        let close = self.closer(i + ticks, to, '`', ticks)?;

        self.copy(i + ticks, close, Emphasis { code: true, ..outer });
        Some(close + ticks - i)
    }

    /// `[label](url)`, or `![alt](url)`. The destination is kept, dimmed, after the label —
    /// unless the two are the same, which reads better as a bare url.
    fn link(&mut self, i: usize, to: usize, outer: Emphasis) -> Option<usize> {
        let open = i + usize::from(self.at(i) == Some('!'));
        if self.at(open) != Some('[') {
            return None;
        }
        let close = self.find(open + 1, to, ']')?;
        if self.at(close + 1) != Some('(') {
            return None;
        }
        let end = self.find(close + 2, to, ')')?;

        let label = self.source(open + 1, close).trim().to_owned();
        let url = self.source(close + 2, end).trim().to_owned();
        let shown: &str = if label.is_empty() { &url } else { &label };

        self.push_str(shown, Emphasis { link: true, ..outer });
        if !url.is_empty() && shown != url.as_str() {
            self.push_str(&format!(" ({url})"), Emphasis { dim: true, ..outer });
        }
        Some(end + 1 - i)
    }

    /// `<https://x>` — the angle brackets go, the url stays. Anything else `<…>` wraps is
    /// prose or markup nobody wrote as a link.
    fn autolink(&mut self, i: usize, to: usize, outer: Emphasis) -> Option<usize> {
        let end = self.find(i + 1, to, '>')?;
        let url = self.source(i + 1, end);
        if url.chars().any(char::is_whitespace) || !(url.contains("://") || url.contains("mailto:"))
        {
            return None;
        }

        self.push_str(&url, Emphasis { link: true, ..outer });
        Some(end + 1 - i)
    }

    /// `**strong**`, `*emphasis*`, or `~~struck~~`, scanned again with the new emphasis folded
    /// in so a nested construct keeps both.
    fn emphasis(&mut self, i: usize, to: usize, outer: Emphasis) -> Option<usize> {
        let marker = self.at(i)?;
        let len = self.run_len(i, marker).min(2);
        if !self.opens_emphasis(i, marker, len) {
            return None;
        }
        let close = self.closer(i + len, to, marker, len)?;

        let inner = match (marker, len) {
            ('~', _) => Emphasis { struck: true, ..outer },
            (_, 2) => Emphasis { strong: true, ..outer },
            _ => Emphasis { italic: true, ..outer },
        };
        self.scan(i + len, close, inner);
        Some(close + len - i)
    }

    /// Whether a marker run at `i` can open emphasis at all.
    fn opens_emphasis(&self, i: usize, marker: char, len: usize) -> bool {
        // An underscore inside a word is not a marker — that keeps `snake_case` upright.
        let intraword =
            marker == '_' && i > 0 && self.at(i - 1).is_some_and(|c| c.is_ascii_alphanumeric());
        // A lone tilde is a home directory or a rough number.
        let half_tilde = marker == '~' && len < 2;
        // A marker followed by a space opens nothing — that is arithmetic or a stray bullet.
        let loose = self.at(i + len).is_some_and(char::is_whitespace);

        !intraword && !half_tilde && !loose
    }

    fn run_len(&self, i: usize, marker: char) -> usize {
        self.chars.iter().skip(i).take_while(|&&c| c == marker).count()
    }

    /// The first index in `from..to` holding exactly `len` of `marker` in a row.
    fn closer(&self, from: usize, to: usize, marker: char, len: usize) -> Option<usize> {
        (from..to.min(self.chars.len())).find(|&i| {
            self.at(i) == Some(marker)
                && self.chars.get(i.wrapping_sub(1)) != Some(&marker)
                && self.run_len(i, marker) == len
        })
    }

    fn find(&self, from: usize, to: usize, needle: char) -> Option<usize> {
        (from..to.min(self.chars.len())).find(|&i| self.at(i) == Some(needle))
    }
}

#[cfg(test)]
mod tests {
    use super::strip;
    use markup::Emphasis;

    fn text(source: &str) -> String {
        strip(&source.chars().collect::<Vec<_>>()).0
    }

    /// The emphasis in force at each character of the stripped text.
    fn marks(source: &str) -> Vec<Emphasis> {
        let (text, spans) = strip(&source.chars().collect::<Vec<_>>());
        (0..text.chars().count())
            .map(|col| {
                spans
                    .iter()
                    .find(|s| (s.from..s.to).contains(&col))
                    .map_or_else(Emphasis::default, |s| s.emphasis)
            })
            .collect()
    }

    #[test]
    fn a_code_span_loses_its_backticks_and_keeps_its_body_literal() {
        assert_eq!(text("run `cargo **test**` now"), "run cargo **test** now");
        assert_eq!(
            marks("run `x` now").get(4),
            Some(&Emphasis { code: true, ..Emphasis::default() })
        );
    }

    #[test]
    fn strong_and_emphasis_lose_their_markers() {
        assert_eq!(text("a **bold** and *soft* and ~~gone~~"), "a bold and soft and gone");
        let marks = marks("a **b** c");
        assert!(marks.get(2).is_some_and(|e| e.strong));
        assert!(marks.get(4).is_some_and(|e| e.is_plain()));
    }

    #[test]
    fn a_code_span_inside_bold_carries_both() {
        assert_eq!(text("**see `x` now**"), "see x now");
        let marks = marks("**see `x` now**");
        assert_eq!(
            marks.get(4),
            Some(&Emphasis { strong: true, code: true, ..Emphasis::default() })
        );
        assert!(marks.first().is_some_and(|e| e.strong && !e.code));
    }

    #[test]
    fn an_unclosed_marker_is_left_as_text() {
        assert_eq!(text("a ` lone tick"), "a ` lone tick");
        assert_eq!(text("2 * 3 = 6"), "2 * 3 = 6");
        assert_eq!(text("some_snake_case_name"), "some_snake_case_name");
        assert_eq!(text("~/.config and ~5 items"), "~/.config and ~5 items");
        assert!(marks("2 * 3 = 6").iter().all(|e| e.is_plain()));
    }

    #[test]
    fn a_nested_marker_never_reaches_past_its_parent() {
        // The stray backtick belongs to no pair, so nothing after the bold is emphasized.
        assert_eq!(text("**a ` b** c"), "a ` b c");
        assert!(marks("**a ` b** c").last().is_some_and(|e| e.is_plain()));
    }

    #[test]
    fn a_link_keeps_its_label_and_trails_its_destination() {
        assert_eq!(text("see [docs](http://x) ok"), "see docs (http://x) ok");
        let marks = marks("see [docs](http://x) ok");
        assert!(marks.get(4).is_some_and(|e| e.link));
        assert!(marks.get(9).is_some_and(|e| e.dim));
    }

    #[test]
    fn a_link_whose_label_is_its_url_reads_as_one_url() {
        assert_eq!(text("[http://x](http://x)"), "http://x");
        assert_eq!(text("[](http://x)"), "http://x");
        assert_eq!(text("![a shot](pic.png)"), "a shot (pic.png)");
    }

    #[test]
    fn an_angle_bracket_url_loses_its_brackets_and_prose_keeps_them() {
        assert_eq!(text("see <https://x> now"), "see https://x now");
        assert_eq!(text("a <b> and 1 < 2 > 0"), "a <b> and 1 < 2 > 0");
    }
}
