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
    view::{Painted, Question, View},
    wrap::{Row, cut, wrap},
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
    question: Question<'_>,
) {
    let p = &view.palette;
    let column = content_column(area, view.measure);
    let width = usize::from(column.width.saturating_sub(BORDERS)).max(1);
    let typed = rows(question.text, width);
    let grown = u16::try_from(typed.len()).unwrap_or(u16::MAX).saturating_add(BORDERS);
    let height = grown.min(area.height);
    let box_area = Rect { y: box_y(area, view, painted, height), height, ..column };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(p.code))
        .title(Span::styled(" question ", Style::new().fg(p.code).add_modifier(Modifier::BOLD)));
    let inner = block.inner(box_area);

    // Past the height of the pane the box stops growing and scrolls to the row the caret is on,
    // which is the row the typist is watching.
    let caret_row = row_at(&typed, question.caret);
    let visible = usize::from(inner.height).max(1);
    let first = caret_row.saturating_sub(visible - 1);
    let shown = typed.get(first..(first + visible).min(typed.len())).unwrap_or(&[]);
    // An empty box says what it wants; the caret sits on the first character of the hint.
    let hint = Style::new().fg(p.overlay0).add_modifier(Modifier::ITALIC);
    let body: Vec<Line<'_>> = if question.text.is_empty() {
        vec![Line::from(Span::styled(PLACEHOLDER, hint))]
    } else {
        shown
            .iter()
            .map(|row| Line::from(Span::styled(row.text.as_str(), Style::new().fg(p.text))))
            .collect()
    };
    f.render_widget(Clear, box_area);
    f.render_widget(block, box_area);
    f.render_widget(Paragraph::new(body), inner);

    let before = typed
        .get(caret_row)
        .map_or(0, |row| cut(&row.text, 0, question.caret.saturating_sub(row.start)).width());
    let x = u16::try_from(before).unwrap_or(u16::MAX).min(inner.width.saturating_sub(1));
    let y = u16::try_from(caret_row - first).unwrap_or(0);
    f.set_cursor_position((inner.x + x, inner.y + y));
}

/// The question wrapped to the box, with an empty row waiting whenever the last one is full —
/// so the box grows before the character that needs the space, not after it.
fn rows(question: &str, width: usize) -> Vec<Row> {
    let mut rows = wrap(question, width);
    let full = rows.last().is_some_and(|row| row.text.width() >= width);
    if let Some(last) = rows.last().filter(|_| full) {
        let start = last.start + last.text.chars().count();
        rows.push(Row { start, text: String::new() });
    }
    rows
}

/// The wrapped row the caret sits on: the last one starting at or before it, so a caret at a
/// row boundary lands where the next character will be typed.
fn row_at(rows: &[Row], caret: usize) -> usize {
    rows.iter().rposition(|row| row.start <= caret).unwrap_or(0)
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

    use super::{BORDERS, Row, rows};
    use crate::{Question, Scroll, SourceLine, View, render, theme};

    /// Measure the box is painted into by [`frame`], and the width its text wraps at.
    const MEASURE: u16 = 40;
    const WIDTH: usize = MEASURE as usize - BORDERS as usize;

    /// A question long enough to wrap at [`MEASURE`].
    const LONG: &str = "why does this branch exist at all, and what breaks if I delete it now";

    /// The text of each wrapped row, for asserting on the wrap alone.
    fn texts(rows: &[Row]) -> Vec<&str> {
        rows.iter().map(|row| row.text.as_str()).collect()
    }

    /// One rendered frame with the question box open, and where the caret landed.
    fn frame(question: &str, caret: usize, height: u16) -> (Buffer, (u16, u16)) {
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
            question: Some(Question { text: question, caret }),
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
        let (painted, _) = frame(LONG, LONG.chars().count(), 24);
        assert_eq!(box_rows(&painted).len(), wrapped + usize::from(BORDERS));
    }

    #[test]
    fn the_caret_follows_the_text_onto_the_last_row() {
        let (painted, (_, caret)) = frame(LONG, LONG.chars().count(), 24);
        let bottom = box_rows(&painted).last().copied().unwrap();
        assert_eq!(caret, bottom - 1);
    }

    #[test]
    fn a_question_taller_than_the_pane_shows_its_tail() {
        let long = LONG.repeat(8);
        let (painted, (_, caret)) = frame(&long, long.chars().count(), 12);
        let box_rows = box_rows(&painted);
        assert!(box_rows.len() < rows(&long, WIDTH).len(), "the box should stop growing");
        assert_eq!(caret, box_rows.last().copied().unwrap() - 1, "the caret stays in view");
    }

    #[test]
    fn an_empty_box_keeps_the_caret_at_the_start_of_the_hint() {
        let (painted, (caret_x, caret_y)) = frame("", 0, 24);
        assert_eq!(box_rows(&painted).len(), 1 + usize::from(BORDERS));
        let cell = painted.cell((caret_x, caret_y)).unwrap();
        assert_eq!(cell.symbol(), "w", "the caret sits on the first character of the hint");
    }

    #[test]
    fn the_caret_lands_on_the_character_it_sits_on() {
        let (painted, (x, y)) = frame(LONG, 5, 24);
        assert_eq!(painted.cell((x, y)).unwrap().symbol(), "o", "'why d|oes' — the sixth char");
    }

    #[test]
    fn the_caret_lands_on_the_wrapped_row_that_holds_it() {
        let wrapped = rows(LONG, WIDTH);
        let second = wrapped.get(1).unwrap();
        let (painted, (x, y)) = frame(LONG, second.start + 2, 24);
        let top = box_rows(&painted).first().copied().unwrap();
        assert_eq!(y, top + 2, "the second text row of the box");
        let expected = second.text.chars().nth(2).unwrap().to_string();
        assert_eq!(painted.cell((x, y)).unwrap().symbol(), expected);
    }

    #[test]
    fn a_caret_moved_back_scrolls_the_box_to_it() {
        let long = LONG.repeat(8);
        let (painted, (x, y)) = frame(&long, 0, 12);
        let top = box_rows(&painted).first().copied().unwrap();
        assert_eq!(y, top + 1, "the head of the question, not its tail");
        let first = long.chars().next().unwrap().to_string();
        assert_eq!(painted.cell((x, y)).unwrap().symbol(), first);
    }

    #[test]
    fn a_question_that_fits_is_one_row() {
        assert_eq!(texts(&rows("why?", 40)), ["why?"]);
    }

    #[test]
    fn a_long_question_wraps_instead_of_running_off_the_box() {
        assert_eq!(texts(&rows("the quick brown fox", 10)), ["the quick ", "brown fox"]);
    }

    #[test]
    fn an_empty_question_still_takes_a_row() {
        assert_eq!(texts(&rows("", 40)), [""]);
    }

    #[test]
    fn a_full_last_row_opens_the_next_one() {
        let rows = rows("abcd", 4);
        assert_eq!(texts(&rows), ["abcd", ""]);
        assert_eq!(rows.get(1).unwrap().start, 4, "the empty row starts past the text");
    }
}
