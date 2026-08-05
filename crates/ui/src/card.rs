//! The box that shows a banked pair's question under the quote it belongs to.

use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};
use unicode_width::UnicodeWidthStr;

use crate::{theme::Palette, wrap::wrap};

/// Left inset, so a card reads as attached to the quote above it.
const INDENT: usize = 2;

/// The card's rows for a pair, at `width` columns of text area. Empty without a question:
/// the gutter mark already says everything a bare quote's card would.
pub(crate) fn lines(
    number: usize,
    question: &str,
    width: usize,
    p: &Palette,
) -> Vec<Line<'static>> {
    if question.is_empty() {
        return Vec::new();
    }
    let box_width = width.saturating_sub(INDENT).max(8);
    let text_width = box_width.saturating_sub(4).max(1); // inside "│ " … " │"
    let border = Style::new().fg(p.overlay0);
    let body = Style::new().fg(p.text);
    let pad = || Span::raw(" ".repeat(INDENT));

    let label = format!(" {number} ");
    let fill = box_width.saturating_sub(3 + label.width());
    let mut rows = vec![Line::from(vec![
        pad(),
        Span::styled("╭─", border),
        Span::styled(label, body.add_modifier(Modifier::BOLD)),
        Span::styled(format!("{}╮", "─".repeat(fill)), border),
    ])];
    for row in wrap(question, text_width) {
        let gap = " ".repeat(text_width.saturating_sub(row.text.width()));
        rows.push(Line::from(vec![
            pad(),
            Span::styled("│ ", border),
            Span::styled(row.text, body),
            Span::styled(format!("{gap} │"), border),
        ]));
    }
    rows.push(Line::from(vec![
        pad(),
        Span::styled(format!("╰{}╯", "─".repeat(box_width.saturating_sub(2))), border),
    ]));
    rows
}

#[cfg(test)]
mod tests {
    use ratatui::text::Line;

    use super::lines;
    use crate::theme;

    #[test]
    fn a_bare_quote_gets_no_card() {
        assert!(lines(1, "", 40, &theme::default_theme().palette).is_empty());
    }

    #[test]
    fn every_row_is_the_same_width() {
        let rows =
            lines(2, "why does this branch exist at all", 30, &theme::default_theme().palette);
        let widths: Vec<_> = rows.iter().map(Line::width).collect();
        assert_eq!(widths, vec![30; rows.len()]);
    }

    #[test]
    fn a_question_that_fits_is_one_body_row_between_the_borders() {
        assert_eq!(lines(1, "why?", 40, &theme::default_theme().palette).len(), 3);
    }

    #[test]
    fn a_long_question_grows_the_box_instead_of_being_cut() {
        let rows = lines(1, "one two three four five six", 16, &theme::default_theme().palette);
        assert!(rows.len() > 3, "expected wrapped body rows, got {}", rows.len());
    }
}
