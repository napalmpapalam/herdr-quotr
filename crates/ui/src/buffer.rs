//! The scrolled transcript, painted into the reading column.

use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::{
    card,
    column::{GUTTER, content_column, padding, text_area},
    view::{Banked, Painted, PaintedRow, Pos, Scroll, Tone, View},
    wrap::{Row, cut, wrap},
};

/// Background marking the selected characters.
const SELECT_BG: Color = Color::Indexed(238);

/// The reading area. No border of its own — herdr's popup already frames the pane.
pub(crate) fn render(f: &mut Frame, area: Rect, view: &View) -> Painted {
    let column = content_column(area);
    let text = text_area(column);
    let height = usize::from(column.height);
    let width = usize::from(text.width);

    let top = match view.scroll {
        Scroll::From(line) => line,
        Scroll::Bottom => bottom_start(view, width, height),
    };
    let mut painted: Vec<Option<PaintedRow>> = Vec::with_capacity(height);
    let mut rows: Vec<Line<'static>> = Vec::with_capacity(height);
    let mut lines = 0;
    for (index, source) in view.lines.iter().enumerate().skip(top) {
        let wrapped = wrap(source.text, width);
        let cards = cards_under(view.banked, index, width);
        // A half-fitting line waits for the next scroll — except the first, which must show.
        if !rows.is_empty() && rows.len() + wrapped.len() + cards.len() > height {
            break;
        }
        let base = base_style(source.tone);
        for row in wrapped {
            rows.push(row_line(&row, index, view, base, width));
            painted.push(Some(PaintedRow { line: index, start: row.start, text: row.text }));
        }
        for mut card in cards {
            card.spans.insert(0, gutter(mark(view.banked, index, false)));
            rows.push(card);
            painted.push(None); // a card takes height but has nothing to select
        }
        lines += 1;
        if rows.len() >= height {
            break;
        }
    }
    rows.truncate(height);
    painted.truncate(height);
    f.render_widget(Paragraph::new(rows), column);

    let painted = Painted::new(painted, lines, top, text);
    if let Some(caret) = painted.caret(view.cursor) {
        f.set_cursor_position(caret);
    }
    painted
}

/// The topmost source line that still leaves the last one on the bottom row. A line that
/// would only half fit is left above the viewport rather than clipped.
fn bottom_start(view: &View, width: usize, height: usize) -> usize {
    let mut rows = 0;
    for (index, source) in view.lines.iter().enumerate().rev() {
        rows += wrap(source.text, width).len() + cards_under(view.banked, index, width).len();
        if rows >= height {
            return index + usize::from(rows > height);
        }
    }
    0
}

/// Card rows for every pair whose range ends on `line` — reviewr's inline comment card.
fn cards_under(banked: &[Banked], line: usize, width: usize) -> Vec<Line<'static>> {
    banked
        .iter()
        .filter(|pair| pair.to == line)
        .flat_map(|pair| card::lines(pair.number, pair.question, width))
        .collect()
}

/// One display row, cut into the runs before, inside, and after the selection. The padding
/// out to the measure joins the highlight only when the range carries on past this row.
fn row_line(row: &Row, line: usize, view: &View, base: Style, width: usize) -> Line<'static> {
    let end = row.start + row.text.chars().count();
    let (start, stop) = selected_cols(view.selection, line).unwrap_or((end, end));
    let (from, to) =
        (start.clamp(row.start, end) - row.start, stop.clamp(row.start, end) - row.start);
    let tail = padding(&row.text, width);
    // Only a range that runs on past this row keeps its highlight across the padding.
    let tail_style = if stop > end && start <= end { base.bg(SELECT_BG) } else { base };
    Line::from(vec![
        gutter(mark(view.banked, line, row.start == 0)),
        Span::styled(cut(&row.text, 0, from).to_owned(), base),
        Span::styled(cut(&row.text, from, to).to_owned(), base.bg(SELECT_BG)),
        Span::styled(cut(&row.text, to, usize::MAX).to_owned(), base),
        Span::styled(tail, tail_style),
    ])
}

fn gutter(text: String) -> Span<'static> {
    Span::styled(text, Style::new().add_modifier(Modifier::DIM))
}

/// The gutter cell for a row: a banked pair's number where its range opens, a bar down the
/// rest of it, blank elsewhere. `numbered` is false on a continuation row and on a card.
fn mark(banked: &[Banked], line: usize, numbered: bool) -> String {
    let width = usize::from(GUTTER) - 1;
    let Some(pair) = banked.iter().find(|pair| (pair.from..=pair.to).contains(&line)) else {
        return " ".repeat(usize::from(GUTTER));
    };
    if numbered && line == pair.from {
        return format!("{:>width$}│", pair.number);
    }
    format!("{:>width$}│", "")
}

/// The columns of `line` a selection covers, unbounded at an end the range passes through.
fn selected_cols(selection: Option<(Pos, Pos)>, line: usize) -> Option<(usize, usize)> {
    let (from, to) = selection.filter(|(from, to)| (from.line..=to.line).contains(&line))?;
    let start = if line == from.line { from.col } else { 0 };
    let end = if line == to.line { to.col } else { usize::MAX };
    Some((start, end))
}

fn base_style(tone: Tone) -> Style {
    match tone {
        Tone::User => Style::new().dim(),
        Tone::Agent | Tone::Gap => Style::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::{Banked, mark};

    const BANKED: [Banked<'static>; 2] = [
        Banked { number: 1, from: 2, to: 4, question: "why?" },
        Banked { number: 2, from: 9, to: 9, question: "" },
    ];

    #[test]
    fn the_number_prints_once_where_the_range_opens() {
        assert_eq!(mark(&BANKED, 2, true), " 1│");
        assert_eq!(mark(&BANKED, 2, false), "  │"); // a wrapped continuation of the same line
        assert_eq!(mark(&BANKED, 3, true), "  │");
        assert_eq!(mark(&BANKED, 4, true), "  │");
    }

    #[test]
    fn an_unbanked_line_gets_blank_cells_the_same_width() {
        assert_eq!(mark(&BANKED, 5, true), "   ");
        assert_eq!(mark(&[], 0, true), "   ");
    }

    #[test]
    fn a_one_line_pair_is_marked_too() {
        assert_eq!(mark(&BANKED, 9, true), " 2│");
    }
}
