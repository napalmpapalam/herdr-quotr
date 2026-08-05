//! Frame around the prose: the footer, and the question box that floats over it.

use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};
use unicode_width::UnicodeWidthStr;

use crate::{
    column::content_column,
    view::{Painted, View},
};

/// Reverse video, so a key cap reads as a physical key.
const KEY_CAP: Style = Style::new().add_modifier(Modifier::REVERSED);

/// Footer key caps and the hint each one carries, painted left to right.
const BROWSE_KEYS: [(&str, &str); 7] = [
    (" drag ", " select   "),
    (" v V ", " char line   "),
    (" C ", " ask   "),
    (" e d ", " edit del   "),
    (" S ", " send   "),
    (" [ ] ", " turn   "),
    (" q ", " quit   "),
];
const ASK_KEYS: [(&str, &str); 2] = [(" enter ", " bank   "), (" esc ", " back   ")];

const BOX_HEIGHT: u16 = 3;

/// The key hints, the bank count, and the status line — chrome, so it spans the pane.
pub(crate) fn render_footer(f: &mut Frame, area: Rect, view: &View) {
    let keys: &[(&str, &str)] = if view.question.is_some() { &ASK_KEYS } else { &BROWSE_KEYS };
    let banked = match view.banked.len() {
        0 => String::new(),
        n => format!("{n} banked   "),
    };
    let spans = keys
        .iter()
        .flat_map(|&(cap, hint)| [Span::styled(cap, KEY_CAP), Span::raw(hint)])
        .chain([Span::raw(banked).bold(), Span::raw(view.status).dim()]);
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
    let block = Block::default().borders(Borders::ALL).title(" question ");
    let inner = block.inner(box_area);
    f.render_widget(Clear, box_area);
    f.render_widget(block, box_area);
    f.render_widget(Paragraph::new(question), inner);

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
