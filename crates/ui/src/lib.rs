//! Rendering. Slice 1 paints a placeholder body — the transcript lands in slice 2.
//!
//! Takes plain data, never the app state, so the paint layer depends on nothing but
//! ratatui and the binary is free to own its own types.

use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

/// Reading measure for text, in columns — CSS `max-width`. Configurable in slice 4.
pub const MAX_WIDTH: u16 = 100;

/// The centered, capped column text renders into — CSS `max-width` + `margin: 0 auto`.
///
/// Below the cap the column takes the full width, and the remainder of an odd gutter goes
/// right, so the column never jitters by one as the pane resizes. Selection highlight and
/// line marks belong inside this rect; chrome (border, footer) spans the pane.
pub fn content_column(area: Rect) -> Rect {
    let width = area.width.min(MAX_WIDTH);
    Rect { x: area.x + (area.width - width) / 2, width, ..area }
}

/// Paint the whole picker: framed body above a one-row footer.
pub fn render(f: &mut Frame, status: &str) {
    let [body, footer] =
        Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).areas(f.area());
    render_body(f, body);
    render_footer(f, status, footer);
}

/// The framed reading area. The border is chrome and spans `area`; the text sits in the
/// centered column inside it.
fn render_body(f: &mut Frame, area: Rect) {
    let block = Block::default().borders(Borders::ALL).title(" quotr ");
    let inner = block.inner(area);
    f.render_widget(block, area);

    let lines = vec![
        Line::from("quotr — pick lines from the agent's own answer and quote them back."),
        Line::from(""),
        Line::from(
            "Slice 1: the transcript is not wired up yet. `S` sends a hardcoded block.".dim(),
        ),
    ];
    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), content_column(inner));
}

/// Footer key caps and the hint each one carries, painted left to right.
const FOOTER_KEYS: [(&str, &str); 2] = [(" S ", " send   "), (" q ", " quit   ")];

/// Reverse video, so a key cap reads as a physical key.
const KEY_CAP: Style = Style::new().add_modifier(Modifier::REVERSED);

/// The key hints and status line, full width.
fn render_footer(f: &mut Frame, status: &str, area: Rect) {
    let spans = FOOTER_KEYS
        .iter()
        .flat_map(|&(cap, hint)| [Span::styled(cap, KEY_CAP), Span::raw(hint)])
        .chain([Span::raw(status).dim()]);
    f.render_widget(Paragraph::new(spans.collect::<Line>()), area);
}

#[cfg(test)]
mod tests {
    use ratatui::layout::Rect;

    use super::{MAX_WIDTH, content_column};

    #[test]
    fn caps_and_centers_on_a_wide_pane() {
        let col = content_column(Rect::new(1, 2, 141, 40));
        assert_eq!(col.width, MAX_WIDTH);
        assert_eq!(col.x, 1 + 20); // odd remainder goes right: 41 -> 20 left, 21 right
        assert_eq!((col.y, col.height), (2, 40));
    }

    #[test]
    fn takes_full_width_below_the_cap() {
        let area = Rect::new(3, 0, 60, 10);
        assert_eq!(content_column(area), area);
    }
}
