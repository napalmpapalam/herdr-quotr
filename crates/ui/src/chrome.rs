//! Frame around the prose: the footer, and the question box that floats over it.

use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};
use unicode_width::UnicodeWidthStr;

use crate::{
    column::content_column,
    view::{Painted, View},
};

/// Footer key caps and the hint each one carries, painted left to right.
const BROWSE_KEYS: [(&str, &str); 8] = [
    ("drag", "select"),
    ("v V", "char line"),
    ("C", "ask"),
    ("e", "edit"),
    ("d", "del"),
    ("S", "send"),
    ("[ ]", "turn"),
    ("q", "quit"),
];
const ASK_KEYS: [(&str, &str); 2] = [("enter", "bank"), ("esc", "back")];

const BOX_HEIGHT: u16 = 3;

/// The key hints, the bank count, and the status line — chrome, so it spans the pane.
pub(crate) fn render_footer(f: &mut Frame, area: Rect, view: &View) {
    let keys: &[(&str, &str)] = if view.question.is_some() { &ASK_KEYS } else { &BROWSE_KEYS };
    let banked = match view.banked.len() {
        0 => String::new(),
        n => format!("{n} banked   "),
    };
    let p = &view.palette;
    let bracket = Style::new().fg(p.overlay0);
    let spans = keys
        .iter()
        .flat_map(|&(cap, hint)| {
            // The turn keys *are* brackets; wrapping them again reads as `[[ ]]`.
            let bare = cap.contains(['[', ']']);
            [
                Span::styled(if bare { "" } else { "[" }, bracket),
                Span::styled(cap, Style::new().fg(p.code)),
                Span::styled(if bare { " " } else { "] " }, bracket),
                Span::styled(format!("{hint}   "), Style::new().fg(p.subtext0)),
            ]
        })
        .chain([
            Span::styled(banked, Style::new().fg(p.text).add_modifier(Modifier::BOLD)),
            Span::styled(view.status, Style::new().fg(p.overlay0)),
        ]);
    f.render_widget(Paragraph::new(spans.collect::<Line>()), area);
}

/// The question editor, floating just under the quote it belongs to.
pub(crate) fn render_question(
    f: &mut Frame,
    area: Rect,
    view: &View,
    painted: &Painted,
    question: &str,
) {
    let height = BOX_HEIGHT.min(area.height);
    let box_area = Rect { y: box_y(area, view, painted, height), height, ..content_column(area) };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::new().fg(view.palette.overlay0))
        .title(Span::styled(" question ", Style::new().fg(view.palette.text)));
    let inner = block.inner(box_area);
    f.render_widget(Clear, box_area);
    f.render_widget(block, box_area);
    f.render_widget(Paragraph::new(question).style(Style::new().fg(view.palette.text)), inner);

    let typed = u16::try_from(question.width()).unwrap_or(u16::MAX);
    f.set_cursor_position((inner.x + typed.min(inner.width.saturating_sub(1)), inner.y));
}

/// The row below the quote, flipped above when the box would hang off the bottom, centered
/// when the quote is off screen. Editing a banked pair has no selection, so it uses the
/// caret — inside the pair either way.
fn box_y(area: Rect, view: &View, painted: &Painted, height: u16) -> u16 {
    let anchor = view.selection.map_or(view.cursor, |(_, to)| to);
    let Some((_, row)) = painted.caret(anchor) else {
        return area.y + area.height.saturating_sub(height) / 2;
    };
    let last = area.bottom().saturating_sub(height);
    if row + 1 > last {
        return row.saturating_sub(height).max(area.y);
    }
    row + 1
}
