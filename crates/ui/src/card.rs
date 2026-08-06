//! The box that shows a banked pair's question under the quote it belongs to.

use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};
use unicode_width::UnicodeWidthStr;

use crate::{theme::Palette, wrap::wrap};

/// Left inset, so a card reads as attached to the quote above it.
const INDENT: usize = 2;

/// Characters of the quote a card's title shows before it trails off.
const TITLE: usize = 48;

/// The card's rows for a pair, at `width` columns of text area. Empty without a question:
/// the gutter mark already says everything a bare quote's card would.
pub(crate) fn lines(
    number: usize,
    question: &str,
    quote: &str,
    width: usize,
    p: &Palette,
) -> Vec<Line<'static>> {
    if question.is_empty() {
        return Vec::new();
    }
    let label = title(number, quote);
    // The card takes the measure, so it sits exactly where the question box the reader typed
    // into was — same footprint, banked instead of open.
    let box_width = width.saturating_sub(INDENT).max(label.width() + 4);
    let text_width = box_width.saturating_sub(4).max(1); // inside "│ " … " │"
    let wrapped: Vec<String> = wrap(question, text_width).into_iter().map(|row| row.text).collect();

    let border = Style::new().fg(p.overlay0);
    let body = Style::new().fg(p.text).add_modifier(Modifier::ITALIC);
    let pad = || Span::raw(" ".repeat(INDENT));

    let fill = box_width.saturating_sub(3 + label.width());
    let mut rows = vec![Line::from(vec![
        pad(),
        Span::styled("╭─", border),
        Span::styled(label, Style::new().fg(p.code).add_modifier(Modifier::BOLD)),
        Span::styled(format!("{}╮", "─".repeat(fill)), border),
    ])];
    for row in wrapped {
        let gap = " ".repeat(text_width.saturating_sub(row.width()));
        rows.push(Line::from(vec![
            pad(),
            Span::styled("│ ", border),
            Span::styled(row, body),
            Span::styled(format!("{gap} │"), border),
        ]));
    }
    rows.push(Line::from(vec![
        pad(),
        Span::styled(format!("╰{}╯", "─".repeat(box_width.saturating_sub(2))), border),
    ]));
    rows
}

/// How many rows a pair's card takes — the two borders plus its wrapped question.
pub fn rows(question: &str, width: usize) -> usize {
    let question = question.trim();
    if question.is_empty() {
        return 0;
    }

    2 + wrap(question, text_width(width)).len()
}

/// Columns inside the frame, after the inset and the `│ ` … ` │` it draws.
fn text_width(width: usize) -> usize {
    width.saturating_sub(INDENT).saturating_sub(4).max(1)
}

/// The card's title: its number, then as much of the quote as fits before it trails off.
fn title(number: usize, quote: &str) -> String {
    let quote = quote.trim();
    let head: String = quote.chars().take(TITLE).collect();
    let head = head.trim_end();
    let trailed = if quote.chars().count() > TITLE { "…" } else { "" };

    if head.is_empty() {
        return format!(" {number}. ");
    }

    format!(" {number}. \"{head}{trailed}\" ")
}

#[cfg(test)]
mod tests {
    use ratatui::text::Line;

    use super::{lines, rows, title};
    use crate::theme;

    #[test]
    fn the_title_carries_the_number_and_the_start_of_the_quote() {
        assert_eq!(title(3, "  short one  "), " 3. \"short one\" ");
        assert_eq!(
            title(1, "a quote that is quite a lot longer than the title has any room for"),
            " 1. \"a quote that is quite a lot longer than the titl…\" "
        );
        assert_eq!(title(2, "   "), " 2. ");
    }

    #[test]
    fn a_bare_quote_gets_no_card() {
        assert!(lines(1, "", "q", 40, &theme::default_theme().palette).is_empty());
    }

    #[test]
    fn every_row_lines_up_and_none_runs_past_the_measure() {
        let rows =
            lines(2, "why does this branch exist at all", "", 30, &theme::default_theme().palette);
        let widths: Vec<_> = rows.iter().map(Line::width).collect();
        assert!(widths.iter().all(|&w| Some(&w) == widths.first()), "ragged box: {widths:?}");
        assert!(widths.first().is_some_and(|&w| w <= 30), "wider than the measure: {widths:?}");
    }

    #[test]
    fn a_short_question_still_takes_the_whole_measure() {
        let drawn = lines(1, "why?", "", 40, &theme::default_theme().palette);
        assert_eq!(drawn.iter().map(Line::width).collect::<Vec<_>>(), vec![40; 3]);
    }

    #[test]
    fn the_row_count_matches_what_is_drawn() {
        let question = "one two three four five six";
        assert_eq!(
            rows(question, 16),
            lines(1, question, "", 16, &theme::default_theme().palette).len()
        );
        assert_eq!(rows("  ", 40), 0);
    }

    #[test]
    fn a_question_that_fits_is_one_body_row_between_the_borders() {
        assert_eq!(lines(1, "why?", "", 40, &theme::default_theme().palette).len(), 3);
    }

    #[test]
    fn a_long_question_grows_the_box_instead_of_being_cut() {
        let rows = lines(1, "one two three four five six", "", 16, &theme::default_theme().palette);
        assert!(rows.len() > 3, "expected wrapped body rows, got {}", rows.len());
    }
}
