//! What the app hands the paint layer, and what the paint layer hands back.

use ratatui::layout::Rect;
use unicode_width::UnicodeWidthChar;

/// Who wrote a line, as far as painting is concerned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Agent,
    User,
    Gap,
}

/// One source line, as the paint layer sees it.
#[derive(Debug, Clone, Copy)]
pub struct SourceLine<'a> {
    pub text: &'a str,
    pub tone: Tone,
}

/// A source line and a character offset into it. Ordering is reading order.
///
/// Mirrors `transcript::Pos`; the app maps between the two each frame, so the paint layer
/// stays free of the transcript.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Pos {
    pub line: usize,
    pub col: usize,
}

/// Everything the picker paints, filled by the app each frame.
#[derive(Debug)]
pub struct View<'a> {
    pub lines: &'a [SourceLine<'a>],
    /// Where the caret sits — painted as the terminal's own cursor.
    pub cursor: Pos,
    /// Inclusive range, when a selection is live.
    pub selection: Option<(Pos, Pos)>,
    /// First source line painted.
    pub scroll: usize,
    /// The question being typed, when the box is open.
    pub question: Option<&'a str>,
    pub status: &'a str,
}

/// One painted display row, and where its characters came from.
#[derive(Debug)]
pub(crate) struct PaintedRow {
    pub(crate) line: usize,
    /// Character offset of the row's first character within its source line.
    pub(crate) start: usize,
    pub(crate) text: String,
}

/// What the last frame put on screen — how the app scrolls and hit-tests the mouse.
#[derive(Debug, Default)]
pub struct Painted {
    rows: Vec<PaintedRow>,
    /// Source lines that fit whole.
    lines: usize,
    /// The column the rows were painted into.
    area: Rect,
}

impl Painted {
    pub(crate) fn new(rows: Vec<PaintedRow>, lines: usize, area: Rect) -> Self {
        Self { rows, lines, area }
    }

    /// How many source lines a page holds — at least one, so paging always moves.
    pub fn page(&self) -> usize {
        self.lines.max(1)
    }

    /// The character under a screen cell.
    pub fn hit(&self, x: u16, y: u16) -> Option<Pos> {
        let index = usize::from(y.checked_sub(self.area.y)?);
        let row = self.rows.get(index)?;
        let col = col_at(&row.text, x.saturating_sub(self.area.x));
        Some(Pos { line: row.line, col: row.start + col })
    }

    /// Screen cell a position sits on, for placing the terminal cursor.
    pub(crate) fn caret(&self, pos: Pos) -> Option<(u16, u16)> {
        let (index, row) = self
            .rows
            .iter()
            .enumerate()
            .rev()
            .find(|(_, row)| row.line == pos.line && row.start <= pos.col)?;
        let cells: usize =
            row.text.chars().take(pos.col - row.start).map(|ch| ch.width().unwrap_or(0)).sum();
        let x = self.area.x.saturating_add(u16::try_from(cells).unwrap_or(u16::MAX));
        Some((x.min(self.area.right().saturating_sub(1)), self.area.y + u16::try_from(index).ok()?))
    }
}

/// Character offset of the cell `dx` columns into `text`, or its end when `dx` runs past.
fn col_at(text: &str, dx: u16) -> usize {
    let mut used = 0;
    for (col, ch) in text.chars().enumerate() {
        used += ch.width().unwrap_or(0);
        if used > usize::from(dx) {
            return col;
        }
    }
    text.chars().count()
}

#[cfg(test)]
mod tests {
    use ratatui::layout::Rect;

    use super::{Painted, PaintedRow, Pos};

    fn row(line: usize, start: usize, text: &str) -> PaintedRow {
        PaintedRow { line, start, text: text.to_owned() }
    }

    /// Line 7 wrapped to two rows, line 8 to one, painted at column x=3, y=5.
    fn painted() -> Painted {
        let rows = vec![row(7, 0, "hello "), row(7, 6, "world"), row(8, 0, "日本")];
        Painted::new(rows, 2, Rect::new(3, 5, 6, 3))
    }

    #[test]
    fn hit_maps_a_screen_cell_to_the_character_under_it() {
        let painted = painted();
        assert_eq!(painted.hit(3, 4), None);
        assert_eq!(painted.hit(4, 5), Some(Pos { line: 7, col: 1 }));
        assert_eq!(painted.hit(5, 6), Some(Pos { line: 7, col: 8 }));
        assert_eq!(painted.hit(3, 8), None);
    }

    #[test]
    fn hit_past_the_end_of_a_row_lands_on_its_last_boundary() {
        assert_eq!(painted().hit(99, 6), Some(Pos { line: 7, col: 11 }));
    }

    #[test]
    fn a_wide_glyph_takes_two_cells() {
        let painted = painted();
        assert_eq!(painted.hit(4, 7), Some(Pos { line: 8, col: 0 }));
        assert_eq!(painted.hit(5, 7), Some(Pos { line: 8, col: 1 }));
    }

    #[test]
    fn caret_finds_the_wrapped_row_a_position_fell_on() {
        let painted = painted();
        assert_eq!(painted.caret(Pos { line: 7, col: 8 }), Some((5, 6)));
        assert_eq!(painted.caret(Pos { line: 8, col: 1 }), Some((5, 7)));
        assert_eq!(painted.caret(Pos { line: 9, col: 0 }), None);
    }
}
