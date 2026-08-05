//! Inline spans within a line: code, links, emphasis, and a table row's pipes.

use ratatui::style::{Modifier, Style};

use crate::{
    markdown::block,
    style::{LineStyle, Run},
    theme::Palette,
};

/// Style one line of prose: its block marker, then the inline spans inside it.
pub(super) fn line(text: &str, p: &Palette) -> LineStyle {
    let chars: Vec<char> = text.chars().collect();
    let mut scanner = Scanner::new(&chars, p, block::is_table_row(&chars));

    if let Some(start) = scanner.block_prefix() {
        scanner.scan(start, chars.len(), Style::new());
    }

    let style = LineStyle::new(scanner.runs);
    match block::quote_rule(&chars) {
        Some(shown) => style.showing(shown),
        None => style,
    }
}

/// The inline spans of `text` with no block marker handling — what a rendered table row
/// needs, since its `│` rules already stand where a marker would.
pub(crate) fn runs(text: &str, p: &Palette) -> Vec<Run> {
    let chars: Vec<char> = text.chars().collect();
    let mut scanner = Scanner::new(&chars, p, false);
    scanner.scan(0, chars.len(), Style::new());
    scanner.runs
}

/// Walks one line left to right, pushing style runs.
struct Scanner<'a> {
    chars: &'a [char],
    p: &'a Palette,
    /// Whether `|` is table scaffolding on this line rather than ordinary punctuation.
    pipes: bool,
    runs: Vec<Run>,
}

impl<'a> Scanner<'a> {
    fn new(chars: &'a [char], p: &'a Palette, pipes: bool) -> Self {
        Self { chars, p, pipes, runs: Vec::new() }
    }

    fn at(&self, i: usize) -> Option<char> {
        self.chars.get(i).copied()
    }

    fn dim(&self) -> Style {
        Style::new().fg(self.p.overlay0)
    }

    fn push(&mut self, start: usize, style: Style) {
        self.runs.push(Run::new(start, style));
    }

    /// Wrap `from..close` in dim markers `len` characters wide, with `inner` between them.
    fn wrap(&mut self, from: usize, close: usize, len: usize, inner: Style, outer: Style) {
        self.push(from, self.dim());
        self.push(from + len, inner);
        self.push(close, self.dim());
        self.push(close + len, outer);
    }

    /// Style the line's leading block marker, returning where inline scanning resumes — or
    /// `None` when the marker already claimed the whole line.
    fn block_prefix(&mut self) -> Option<usize> {
        let chars = self.chars;
        if block::indent(chars) >= 4 {
            return Some(0);
        }

        if block::is_table_rule(chars) || block::is_thematic_break(chars) {
            self.push(0, self.dim());
            return None;
        }

        if let Some(level) = block::heading_level(chars) {
            self.push(0, self.dim());
            self.push(block::indent(chars) + level, block::heading_style(level, self.p));
            return None;
        }

        let quote = block::quote_len(chars);
        if quote > 0 {
            self.push(0, self.dim());
            self.push(quote, Style::new());
            return Some(quote);
        }

        let bullet = block::bullet_len(chars);
        if bullet == 0 {
            return Some(0);
        }

        let at = block::indent(chars);
        self.push(at, Style::new().fg(self.p.code));
        self.push(at + bullet, Style::new());
        Some(at + bullet)
    }

    /// Walk `from..to` marking code spans, links, emphasis, and table pipes, resetting to
    /// `outer` between them. Emphasis recurses, so a code span inside bold is still colored.
    /// A marker whose partner is missing styles nothing, so raw text is never recolored.
    fn scan(&mut self, from: usize, to: usize, outer: Style) {
        let mut i = from;
        while i < to {
            let taken = match self.at(i) {
                Some('`') => self.code_span(i, to, outer),
                Some('[') => self.link(i, to, outer),
                Some('*' | '_' | '~') => self.emphasis(i, to, outer),
                Some('|') if self.pipes => {
                    self.push(i, self.dim());
                    self.push(i + 1, outer);
                    None
                }
                _ => None,
            };
            i += taken.unwrap_or(1);
        }
    }

    /// `` `code` `` — the body is literal, so nothing inside it is scanned.
    fn code_span(&mut self, i: usize, to: usize, outer: Style) -> Option<usize> {
        let ticks = self.run_len(i, '`');
        let close = self.closer(i + ticks, to, '`', ticks)?;

        self.wrap(i, close, ticks, outer.fg(self.p.code), outer);
        Some(close + ticks - i)
    }

    /// `[text](url)` — the text in the link accent, the brackets and destination dim.
    fn link(&mut self, i: usize, to: usize, outer: Style) -> Option<usize> {
        let close = self.find(i + 1, to, ']')?;
        if self.at(close + 1) != Some('(') {
            return None;
        }
        let end = self.find(close + 2, to, ')')?;

        self.push(i, self.dim());
        self.push(i + 1, outer.fg(self.p.link).add_modifier(Modifier::UNDERLINED));
        self.push(close, self.dim());
        self.push(end + 1, outer);
        Some(end + 1 - i)
    }

    /// `**strong**`, `*emphasis*`, or `~~struck~~`. Only strong takes a color; the content is
    /// scanned again with the new style folded in.
    fn emphasis(&mut self, i: usize, to: usize, outer: Style) -> Option<usize> {
        let marker = self.at(i)?;
        let len = self.run_len(i, marker).min(2);
        if !self.opens_emphasis(i, marker, len) {
            return None;
        }
        let close = self.closer(i + len, to, marker, len)?;

        let inner = match (marker, len) {
            ('~', _) => outer.add_modifier(Modifier::CROSSED_OUT),
            (_, 2) => outer.fg(self.p.strong).add_modifier(Modifier::BOLD),
            _ => outer.add_modifier(Modifier::ITALIC),
        };
        self.push(i, self.dim());
        self.push(i + len, inner);
        self.scan(i + len, close, inner);
        self.push(close, self.dim());
        self.push(close + len, outer);
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
    use ratatui::style::{Modifier, Style};

    use super::line;
    use crate::theme::{self, Palette};

    fn palette() -> Palette {
        theme::default_theme().palette
    }

    /// The styles a line paints, character by character.
    fn styles(text: &str) -> Vec<Style> {
        let style = line(text, &palette());
        (0..text.chars().count()).map(|col| style.at(col)).collect()
    }

    #[test]
    fn a_heading_dims_its_hashes_and_accents_the_rest() {
        let p = palette();
        let styles = styles("## Title");
        assert_eq!(styles.first().map(|s| s.fg), Some(Some(p.overlay0)));
        assert_eq!(styles.get(3).map(|s| s.fg), Some(Some(p.heading)));
        assert!(styles.get(3).is_some_and(|s| s.add_modifier.contains(Modifier::BOLD)));
    }

    #[test]
    fn deeper_headings_step_back() {
        let p = palette();
        assert_eq!(styles("### Sub").get(4).map(|s| s.fg), Some(Some(p.heading_deep)));
        assert_eq!(styles("##### Deep").get(6).map(|s| s.fg), Some(Some(p.subtext0)));
        assert_eq!(styles("####### no").first().map(|s| s.fg), Some(None));
    }

    #[test]
    fn a_bullet_is_accented_and_its_text_is_not() {
        let p = palette();
        let bullet = styles("- item");
        assert_eq!(bullet.first().map(|s| s.fg), Some(Some(p.code)));
        assert_eq!(bullet.get(2).map(|s| s.fg), Some(None));
        assert_eq!(styles("12. item").first().map(|s| s.fg), Some(Some(p.code)));
        assert_eq!(styles("well-known").first().map(|s| s.fg), Some(None));
    }

    #[test]
    fn a_blockquote_marker_dims_and_its_text_carries_on() {
        let p = palette();
        let styles = styles("> quoted `code`");
        assert_eq!(styles.first().map(|s| s.fg), Some(Some(p.overlay0)));
        assert_eq!(styles.get(2).map(|s| s.fg), Some(None));
        assert_eq!(styles.get(10).map(|s| s.fg), Some(Some(p.code)));
    }

    #[test]
    fn a_table_dims_its_pipes_and_its_whole_delimiter_row() {
        let p = palette();
        let row = styles("| `a` | b |");
        assert_eq!(row.first().map(|s| s.fg), Some(Some(p.overlay0)));
        assert_eq!(row.get(3).map(|s| s.fg), Some(Some(p.code)));
        assert_eq!(row.get(6).map(|s| s.fg), Some(Some(p.overlay0)));
        assert_eq!(row.get(8).map(|s| s.fg), Some(None));
        assert!(styles("|---|:--|").iter().all(|s| s.fg == Some(p.overlay0)));
        // A pipe in ordinary prose is punctuation, not scaffolding.
        assert!(styles("a | b").iter().all(|s| s.fg.is_none()));
    }

    #[test]
    fn a_thematic_break_is_all_marker() {
        let p = palette();
        assert!(styles("---").iter().all(|s| s.fg == Some(p.overlay0)));
        assert!(styles("* * *").iter().all(|s| s.fg == Some(p.overlay0)));
    }

    #[test]
    fn a_code_span_colors_between_dim_backticks() {
        let p = palette();
        let styles = styles("run `cargo test` now");
        assert_eq!(styles.get(4).map(|s| s.fg), Some(Some(p.overlay0)));
        assert_eq!(styles.get(5).map(|s| s.fg), Some(Some(p.code)));
        assert_eq!(styles.get(15).map(|s| s.fg), Some(Some(p.overlay0)));
        assert_eq!(styles.get(17).map(|s| s.fg), Some(None));
    }

    #[test]
    fn an_unclosed_marker_styles_nothing() {
        assert!(styles("a ` lone tick").iter().all(|s| *s == Style::new()));
        assert!(styles("2 * 3 = 6").iter().all(|s| *s == Style::new()));
        assert!(styles("some_snake_case_name").iter().all(|s| *s == Style::new()));
        assert!(styles("~/.config and ~5 items").iter().all(|s| *s == Style::new()));
    }

    #[test]
    fn strong_takes_the_bold_accent_and_emphasis_only_leans() {
        let p = palette();
        let styles = styles("a **bold** and *soft*");
        assert_eq!(styles.get(2).map(|s| s.fg), Some(Some(p.overlay0)));
        assert_eq!(styles.get(4).map(|s| s.fg), Some(Some(p.strong)));
        assert_eq!(styles.get(8).map(|s| s.fg), Some(Some(p.overlay0)));
        assert_eq!(styles.get(10).map(|s| (s.fg, s.add_modifier)), Some((None, Modifier::empty())));
        assert_eq!(styles.get(16).map(|s| s.fg), Some(None));
        assert!(styles.get(16).is_some_and(|s| s.add_modifier.contains(Modifier::ITALIC)));
    }

    #[test]
    fn strikethrough_crosses_out_between_dim_markers() {
        let p = palette();
        let styles = styles("a ~~gone~~ b");
        assert_eq!(styles.get(2).map(|s| s.fg), Some(Some(p.overlay0)));
        assert!(styles.get(4).is_some_and(|s| s.add_modifier.contains(Modifier::CROSSED_OUT)));
        assert_eq!(styles.get(11).map(|s| s.add_modifier), Some(Modifier::empty()));
    }

    #[test]
    fn a_link_accents_its_text_and_dims_its_destination() {
        let p = palette();
        let styles = styles("see [docs](http://x) ok");
        assert_eq!(styles.get(5).map(|s| s.fg), Some(Some(p.link)));
        assert!(styles.get(5).is_some_and(|s| s.add_modifier.contains(Modifier::UNDERLINED)));
        assert_eq!(styles.get(9).map(|s| s.fg), Some(Some(p.overlay0)));
        assert_eq!(styles.get(21).map(|s| s.fg), Some(None));
    }

    #[test]
    fn a_code_span_inside_bold_is_still_colored_and_still_bold() {
        let p = palette();
        let styles = styles("**see `x` now**");
        assert_eq!(styles.get(2).map(|s| s.fg), Some(Some(p.strong)));
        assert_eq!(styles.get(6).map(|s| s.fg), Some(Some(p.overlay0)));
        assert_eq!(styles.get(7).map(|s| s.fg), Some(Some(p.code)));
        assert!(styles.get(7).is_some_and(|s| s.add_modifier.contains(Modifier::BOLD)));
        assert_eq!(styles.get(10).map(|s| s.fg), Some(Some(p.strong)));
    }

    #[test]
    fn a_nested_marker_never_reaches_past_its_parent() {
        // The stray backtick belongs to no pair, so nothing after the bold is recolored.
        assert_eq!(styles("**a ` b** c").last().map(|s| s.fg), Some(None));
    }
}
