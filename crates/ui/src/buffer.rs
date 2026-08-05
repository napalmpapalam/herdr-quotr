//! The scrolled transcript, painted into the reading column.

use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::{
    column::{content_column, padding},
    view::{Painted, PaintedRow, Pos, Tone, View},
    wrap::{Row, cut, wrap},
};

/// Background marking the selected characters.
const SELECT_BG: Color = Color::Indexed(238);

/// The reading area. No border of its own — herdr's popup already frames the pane.
pub(crate) fn render(f: &mut Frame, area: Rect, view: &View) -> Painted {
    let column = content_column(area);
    let height = usize::from(column.height);
    let width = usize::from(column.width);

    let mut painted = Vec::with_capacity(height);
    let mut rows = Vec::with_capacity(height);
    let mut lines = 0;
    for (index, source) in view.lines.iter().enumerate().skip(view.scroll) {
        let wrapped = wrap(source.text, width);
        // A half-fitting line waits for the next scroll — except the first, which must show.
        if !rows.is_empty() && rows.len() + wrapped.len() > height {
            break;
        }
        let base = base_style(source.tone);
        for row in wrapped {
            rows.push(row_line(&row, index, view, base, width));
            painted.push(PaintedRow { line: index, start: row.start, text: row.text });
        }
        lines += 1;
        if rows.len() >= height {
            break;
        }
    }
    rows.truncate(height);
    painted.truncate(height);
    f.render_widget(Paragraph::new(rows), column);

    let painted = Painted::new(painted, lines, column);
    if let Some(caret) = painted.caret(view.cursor) {
        f.set_cursor_position(caret);
    }
    painted
}

/// One display row, split into the run before the selection, the selected run, and the rest.
///
/// The row is padded to the column so a highlight fills the measure; the padding joins the
/// selection only when the range carries on past this row.
fn row_line(row: &Row, line: usize, view: &View, base: Style, width: usize) -> Line<'static> {
    let end = row.start + row.text.chars().count();
    let (start, stop) = selected_cols(view.selection, line).unwrap_or((end, end));
    let (from, to) =
        (start.clamp(row.start, end) - row.start, stop.clamp(row.start, end) - row.start);
    let tail = padding(&row.text, width);
    // Only a range that runs on past this row keeps its highlight across the padding.
    let tail_style = if stop > end && start <= end { base.bg(SELECT_BG) } else { base };
    Line::from(vec![
        Span::styled(cut(&row.text, 0, from).to_owned(), base),
        Span::styled(cut(&row.text, from, to).to_owned(), base.bg(SELECT_BG)),
        Span::styled(cut(&row.text, to, usize::MAX).to_owned(), base),
        Span::styled(tail, tail_style),
    ])
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
