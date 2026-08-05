//! Per-character styling of a source line, as runs the buffer cuts rows against.

use ratatui::style::Style;

/// A style that takes effect at a character offset and holds until the next run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Run {
    start: usize,
    style: Style,
}

impl Run {
    pub(crate) fn new(start: usize, style: Style) -> Self {
        Self { start, style }
    }

    pub(crate) fn start(&self) -> usize {
        self.start
    }
}

/// One source line's styling: runs in increasing order, each holding to the next.
///
/// Empty means the line paints entirely in the caller's base style — an unstyled paragraph,
/// a user turn, or a fenced block in a language `syntect` does not know.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct LineStyle {
    runs: Vec<Run>,
    display: Option<String>,
    /// Chrome rows hung above and below this line — a rendered table's borders. They take
    /// height and select nothing, like a banked pair's card.
    above: Option<String>,
    below: Option<String>,
    chrome: Style,
    linewise: bool,
}

impl LineStyle {
    pub(crate) fn new(runs: Vec<Run>) -> Self {
        Self {
            runs,
            display: None,
            above: None,
            below: None,
            chrome: Style::new(),
            linewise: false,
        }
    }

    /// Paint `display` in place of the source line. Every substitution is one character wide
    /// for one character wide, so offsets, hit testing, and the quoted output are untouched —
    /// only the glyph on screen differs.
    pub(crate) fn showing(self, display: String) -> Self {
        Self { display: Some(display), ..self }
    }

    /// What to paint for this line, or `None` to paint the source as it is.
    pub(crate) fn display(&self) -> Option<&str> {
        self.display.as_deref()
    }

    pub(crate) fn above(self, text: String, chrome: Style) -> Self {
        Self { above: Some(text), chrome, ..self }
    }

    pub(crate) fn below(self, text: String, chrome: Style) -> Self {
        Self { below: Some(text), chrome, ..self }
    }

    pub(crate) fn above_row(&self) -> Option<(&str, Style)> {
        Some((self.above.as_deref()?, self.chrome))
    }

    pub(crate) fn below_row(&self) -> Option<(&str, Style)> {
        Some((self.below.as_deref()?, self.chrome))
    }

    /// Select this line whole. A rendered grid pads its cells, so a character offset into
    /// what was painted no longer names a character in the source.
    pub(crate) fn linewise(self) -> Self {
        Self { linewise: true, ..self }
    }

    pub fn is_linewise(&self) -> bool {
        self.linewise
    }

    /// The style in force at character offset `col`.
    pub(crate) fn at(&self, col: usize) -> Style {
        self.runs.iter().rev().find(|run| run.start <= col).map_or_else(Style::new, |run| run.style)
    }

    /// Offsets inside `from..to` where the style changes.
    pub(crate) fn breaks(&self, from: usize, to: usize) -> impl Iterator<Item = usize> + '_ {
        self.runs.iter().map(|run| run.start).filter(move |&at| (from..to).contains(&at))
    }
}

#[cfg(test)]
mod tests {
    use ratatui::style::{Color, Style};

    use super::{LineStyle, Run};

    fn styled() -> LineStyle {
        LineStyle::new(vec![
            Run::new(2, Style::new().fg(Color::Red)),
            Run::new(5, Style::new()),
            Run::new(9, Style::new().fg(Color::Blue)),
        ])
    }

    #[test]
    fn a_run_holds_until_the_next_one_starts() {
        let style = styled();
        assert_eq!(style.at(0), Style::new());
        assert_eq!(style.at(2), Style::new().fg(Color::Red));
        assert_eq!(style.at(4), Style::new().fg(Color::Red));
        assert_eq!(style.at(5), Style::new());
        assert_eq!(style.at(99), Style::new().fg(Color::Blue));
    }

    #[test]
    fn an_empty_style_is_the_base_everywhere() {
        assert_eq!(LineStyle::default().at(7), Style::new());
        assert_eq!(LineStyle::default().breaks(0, 100).count(), 0);
    }

    #[test]
    fn breaks_are_the_run_starts_inside_the_window() {
        assert_eq!(styled().breaks(2, 9).collect::<Vec<_>>(), [2, 5]);
        assert_eq!(styled().breaks(6, 8).count(), 0);
    }
}
