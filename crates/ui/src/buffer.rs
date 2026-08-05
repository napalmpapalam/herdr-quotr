//! The scrolled transcript, painted into the reading column.

use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, Paragraph},
};

use crate::{
    column::{content_column, pad},
    view::{Painted, Tone, View},
    wrap::wrap,
};

/// Range background, and the brighter one marking the line the cursor is on.
const SELECT_BG: Color = Color::Indexed(238);
const CURSOR_BG: Color = Color::Indexed(241);

/// The framed reading area: border spans `area`, text and highlights sit in the column.
pub(crate) fn render(f: &mut Frame, area: Rect, view: &View) -> Painted {
    let block = Block::default().borders(Borders::ALL).title(" quotr ");
    let inner = block.inner(area);
    f.render_widget(block, area);

    let column = content_column(inner);
    let height = usize::from(column.height);
    let width = usize::from(column.width);

    let mut rows = Vec::with_capacity(height);
    let mut map = Vec::with_capacity(height);
    let mut lines = 0;
    for (i, source) in view.lines.iter().enumerate().skip(view.scroll) {
        let wrapped = wrap(source.text, width);
        // A half-fitting line waits for the next scroll — except the first, which must show.
        if !rows.is_empty() && rows.len() + wrapped.len() > height {
            break;
        }
        let style = style_for(view, i, source.tone);
        rows.extend(wrapped.into_iter().map(|row| Line::styled(pad(row, width), style)));
        map.resize(rows.len(), i);
        lines += 1;
        if rows.len() >= height {
            break;
        }
    }
    rows.truncate(height);
    map.truncate(height);
    f.render_widget(Paragraph::new(rows), column);
    Painted::new(map, lines, column)
}

fn style_for(view: &View, line: usize, tone: Tone) -> Style {
    let base = match tone {
        Tone::User => Style::new().dim(),
        Tone::Agent | Tone::Gap => Style::new(),
    };
    if line == view.cursor {
        return base.bg(CURSOR_BG);
    }
    let selected = view.selection.is_some_and(|(from, to)| (from..=to).contains(&line));
    if selected {
        return base.bg(SELECT_BG);
    }
    base
}
