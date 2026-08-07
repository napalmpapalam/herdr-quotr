//! Frame around the prose: the footer, and the question box that floats over it.

use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
};
use unicode_width::UnicodeWidthStr;

use crate::{
    column::content_column,
    view::{Painted, View},
    wrap::wrap,
};

/// Footer key caps and the hint each one carries, painted left to right.
const BROWSE_KEYS: [(&str, &str); 8] = [
    ("drag", "select"),
    ("v V", "char line"),
    ("c", "ask"),
    ("e", "edit"),
    ("d", "del"),
    ("s", "send"),
    ("[ ]", "turn"),
    ("q", "quit"),
];
const ASK_KEYS: [(&str, &str); 2] = [("enter", "bank"), ("esc", "back")];

/// Rows the box spends on its own frame, above and below the question.
const BORDERS: u16 = 2;

/// What an empty question box shows in place of the text not typed yet.
const PLACEHOLDER: &str = "what do you want to ask about this quote?";

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
    let bar = Style::new().bg(p.bar);
    f.render_widget(Paragraph::new(spans.collect::<Line>()).style(bar), area);
}

/// The question editor, floating just under the quote it belongs to. It wraps at the measure
/// and grows a row at a time, so a long question is written in the open rather than blind.
pub(crate) fn render_question(
    f: &mut Frame,
    area: Rect,
    view: &View,
    painted: &Painted,
    question: &str,
) {
    let p = &view.palette;
    let column = content_column(area, view.measure);
    let width = usize::from(column.width.saturating_sub(BORDERS)).max(1);
    let typed = rows(question, width);
    let grown = u16::try_from(typed.len()).unwrap_or(u16::MAX).saturating_add(BORDERS);
    let height = grown.min(area.height);
    let box_area = Rect { y: box_y(area, view, painted, height), height, ..column };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(p.code))
        .title(Span::styled(" question ", Style::new().fg(p.code).add_modifier(Modifier::BOLD)));
    let inner = block.inner(box_area);

    // Past the height of the pane the box stops growing and shows its tail — the caret is at
    // the end of the text, which is the row the typist is watching.
    let shown = typed.get(typed.len().saturating_sub(usize::from(inner.height))..).unwrap_or(&[]);
    // An empty box says what it wants; the caret sits on the first character of the hint.
    let hint = Style::new().fg(p.overlay0).add_modifier(Modifier::ITALIC);
    let body: Vec<Line<'_>> = if question.is_empty() {
        vec![Line::from(Span::styled(PLACEHOLDER, hint))]
    } else {
        shown
            .iter()
            .map(|row| Line::from(Span::styled(row.as_str(), Style::new().fg(p.text))))
            .collect()
    };
    f.render_widget(Clear, box_area);
    f.render_widget(block, box_area);
    f.render_widget(Paragraph::new(body), inner);

    let last = shown.last().map_or(0, |row| row.width());
    let x = u16::try_from(last).unwrap_or(u16::MAX).min(inner.width.saturating_sub(1));
    let y = u16::try_from(shown.len().saturating_sub(1)).unwrap_or(0);
    f.set_cursor_position((inner.x + x, inner.y + y));
}

/// The question wrapped to the box, with an empty row waiting whenever the last one is full —
/// so the box grows before the character that needs the space, not after it.
fn rows(question: &str, width: usize) -> Vec<String> {
    let mut rows: Vec<String> = wrap(question, width).into_iter().map(|row| row.text).collect();
    if rows.last().is_some_and(|row| row.width() >= width) {
        rows.push(String::new());
    }
    rows
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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use markup::{Pos, Tone};
    use ratatui::{Terminal, backend::TestBackend, buffer::Buffer};

    use super::{BORDERS, rows};
    use crate::{Scroll, SourceLine, View, render, theme};

    /// Measure the box is painted into by [`frame`], and the width its text wraps at.
    const MEASURE: u16 = 40;
    const WIDTH: usize = MEASURE as usize - BORDERS as usize;

    /// A question long enough to wrap at [`MEASURE`].
    const LONG: &str = "why does this branch exist at all, and what breaks if I delete it now";

    /// One rendered frame with the question box open, and where the caret landed.
    fn frame(question: &str, height: u16) -> (Buffer, (u16, u16)) {
        let lines = [SourceLine { text: "a line to quote", tone: Tone::Agent }];
        let view = View {
            lines: &lines,
            styles: &[],
            palette: theme::default_theme().palette,
            measure: MEASURE,
            turns: &[0],
            cursor: Pos { line: 0, col: 0 },
            selection: Some((Pos { line: 0, col: 0 }, Pos { line: 0, col: 4 })),
            banked: &[],
            scroll: Scroll::From(0),
            question: Some(question),
            status: "",
        };
        let mut terminal = Terminal::new(TestBackend::new(60, height)).unwrap();
        terminal.draw(|f| drop(render(f, &view))).unwrap();
        let caret = terminal.get_cursor_position().unwrap();
        (terminal.backend().buffer().clone(), (caret.x, caret.y))
    }

    /// Rows of the frame the box's border reaches, top to bottom.
    fn box_rows(buffer: &Buffer) -> Vec<u16> {
        (0..buffer.area.height)
            .filter(|&y| {
                (0..buffer.area.width).any(|x| {
                    buffer
                        .cell((x, y))
                        .is_some_and(|c| ["\u{256d}", "\u{2502}", "\u{2570}"].contains(&c.symbol()))
                })
            })
            .collect()
    }

    #[test]
    fn the_box_grows_with_the_question_instead_of_clipping_it() {
        let wrapped = rows(LONG, WIDTH).len();
        assert!(wrapped > 1, "the fixture should wrap");
        let (painted, _) = frame(LONG, 24);
        assert_eq!(box_rows(&painted).len(), wrapped + usize::from(BORDERS));
    }

    #[test]
    fn the_caret_follows_the_text_onto_the_last_row() {
        let (painted, (_, caret)) = frame(LONG, 24);
        let bottom = box_rows(&painted).last().copied().unwrap();
        assert_eq!(caret, bottom - 1);
    }

    #[test]
    fn a_question_taller_than_the_pane_shows_its_tail() {
        let long = LONG.repeat(8);
        let (painted, (_, caret)) = frame(&long, 12);
        let box_rows = box_rows(&painted);
        assert!(box_rows.len() < rows(&long, WIDTH).len(), "the box should stop growing");
        assert_eq!(caret, box_rows.last().copied().unwrap() - 1, "the caret stays in view");
    }

    #[test]
    fn an_empty_box_keeps_the_caret_at_the_start_of_the_hint() {
        let (painted, (caret_x, caret_y)) = frame("", 24);
        assert_eq!(box_rows(&painted).len(), 1 + usize::from(BORDERS));
        let cell = painted.cell((caret_x, caret_y)).unwrap();
        assert_eq!(cell.symbol(), "w", "the caret sits on the first character of the hint");
    }

    #[test]
    fn a_question_that_fits_is_one_row() {
        assert_eq!(rows("why?", 40), ["why?"]);
    }

    #[test]
    fn a_long_question_wraps_instead_of_running_off_the_box() {
        assert_eq!(rows("the quick brown fox", 10), ["the quick ", "brown fox"]);
    }

    #[test]
    fn an_empty_question_still_takes_a_row() {
        assert_eq!(rows("", 40), [""]);
    }

    #[test]
    fn a_full_last_row_opens_the_next_one() {
        assert_eq!(rows("abcd", 4), ["abcd", ""]);
    }
}
