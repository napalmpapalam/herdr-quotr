//! Frame around the prose: the footer, and the question box that floats over it.

use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};
use unicode_width::UnicodeWidthStr;

use crate::{column::content_column, view::View};

/// Reverse video, so a key cap reads as a physical key.
const KEY_CAP: Style = Style::new().add_modifier(Modifier::REVERSED);

/// Footer key caps and the hint each one carries, painted left to right.
const BROWSE_KEYS: [(&str, &str); 6] = [
    (" drag ", " select   "),
    (" v V ", " char line   "),
    (" C ", " ask   "),
    (" S ", " send   "),
    (" [ ] ", " turn   "),
    (" q ", " quit   "),
];
const ASK_KEYS: [(&str, &str); 2] = [(" enter ", " send   "), (" esc ", " back   ")];

const BOX_HEIGHT: u16 = 3;

/// The key hints and status line, full width — chrome, so it spans the pane.
pub(crate) fn render_footer(f: &mut Frame, area: Rect, view: &View) {
    let keys: &[(&str, &str)] = if view.question.is_some() { &ASK_KEYS } else { &BROWSE_KEYS };
    let spans = keys
        .iter()
        .flat_map(|&(cap, hint)| [Span::styled(cap, KEY_CAP), Span::raw(hint)])
        .chain([Span::raw(view.status).dim()]);
    f.render_widget(Paragraph::new(spans.collect::<Line>()), area);
}

/// The question editor, floating over the middle of the buffer inside the reading column.
pub(crate) fn render_question(f: &mut Frame, area: Rect, question: &str) {
    let box_area = Rect {
        y: area.y + area.height.saturating_sub(BOX_HEIGHT) / 2,
        height: BOX_HEIGHT.min(area.height),
        ..content_column(area)
    };
    let block = Block::default().borders(Borders::ALL).title(" question ");
    let inner = block.inner(box_area);
    f.render_widget(Clear, box_area);
    f.render_widget(block, box_area);
    f.render_widget(Paragraph::new(question), inner);

    let typed = u16::try_from(question.width()).unwrap_or(u16::MAX);
    f.set_cursor_position((inner.x + typed.min(inner.width.saturating_sub(1)), inner.y));
}
