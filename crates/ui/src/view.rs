//! What the app hands the paint layer, and what the paint layer hands back.

use ratatui::layout::Rect;
use unicode_width::UnicodeWidthChar;

use crate::{style::LineStyle, theme::Palette};

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

/// A banked quote+question pair, in the shape the gutter and its card need.
#[derive(Debug, Clone, Copy)]
pub struct Banked<'a> {
    /// What the gutter prints, counting from 1.
    pub number: usize,
    /// First and last source line the pair covers — the marked range.
    pub from: usize,
    pub to: usize,
    /// Empty for a bare quote, which gets a gutter mark but no card.
    pub question: &'a str,
}

/// Where the buffer starts painting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scroll {
    /// From this source line, downward.
    From(usize),
    /// Far enough back that the last line lands on the bottom row — how the picker opens.
    /// Only the paint layer can resolve it: it depends on how each line wraps.
    Bottom,
}

/// Everything the picker paints, filled by the app each frame.
#[derive(Debug)]
pub struct View<'a> {
    pub lines: &'a [SourceLine<'a>],
    /// Markdown styling for each line of `lines`, from [`crate::analyze`]. A line past the
    /// end of this slice paints in its base style.
    pub styles: &'a [LineStyle],
    /// Colors this frame paints in.
    pub palette: Palette,
    /// First source line of each turn — where the gutter draws its turn marker.
    pub turns: &'a [usize],
    /// Where the caret sits — painted as the terminal's own cursor.
    pub cursor: Pos,
    /// Inclusive range, when a selection is live.
    pub selection: Option<(Pos, Pos)>,
    /// Pairs waiting to go out together, in bank order.
    pub banked: &'a [Banked<'a>],
    pub scroll: Scroll,
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
    /// A rendered table row: what was painted is padded, so a column offset into it names no
    /// source character and the whole line is the smallest thing that can be selected.
    pub(crate) linewise: bool,
}

/// What the last frame put on screen — how the app scrolls and hit-tests the mouse.
///
/// A `None` row is a card: it takes height but carries no text to select.
#[derive(Debug, Default)]
pub struct Painted {
    rows: Vec<Option<PaintedRow>>,
    /// Source lines that fit whole.
    lines: usize,
    /// The source line the frame started at — the app adopts it after a [`Scroll::Bottom`].
    top: usize,
    /// The text area the rows were painted into, past the gutter.
    area: Rect,
}

impl Painted {
    pub(crate) fn new(rows: Vec<Option<PaintedRow>>, lines: usize, top: usize, area: Rect) -> Self {
        Self { rows, lines, top, area }
    }

    /// How many source lines a page holds — at least one, so paging always moves.
    pub fn page(&self) -> usize {
        self.lines.max(1)
    }

    /// First source line this frame painted.
    pub fn top(&self) -> usize {
        self.top
    }

    /// The character under a screen cell, and whether its line only selects whole.
    pub fn hit(&self, x: u16, y: u16) -> Option<Hit> {
        let index = usize::from(y.checked_sub(self.area.y)?);
        let row = self.rows.get(index)?.as_ref()?;
        let col = col_at(&row.text, x.saturating_sub(self.area.x));
        Some(Hit { pos: Pos { line: row.line, col: row.start + col }, linewise: row.linewise })
    }

    /// Screen cell a position sits on, for placing the terminal cursor.
    pub(crate) fn caret(&self, pos: Pos) -> Option<(u16, u16)> {
        let (index, row) = self
            .rows
            .iter()
            .enumerate()
            .rev()
            .filter_map(|(index, row)| Some((index, row.as_ref()?)))
            .find(|(_, row)| row.line == pos.line && row.start <= pos.col)?;
        // A rendered row's columns are padding, so the caret rests at its start instead.
        let cells: usize = if row.linewise {
            0
        } else {
            row.text.chars().take(pos.col - row.start).map(|ch| ch.width().unwrap_or(0)).sum()
        };
        let x = self.area.x.saturating_add(u16::try_from(cells).unwrap_or(u16::MAX));
        Some((x.min(self.area.right().saturating_sub(1)), self.area.y + u16::try_from(index).ok()?))
    }
}

/// Where a mouse cell landed: the character under it, and whether that line selects whole.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hit {
    pub pos: Pos,
    pub linewise: bool,
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
        PaintedRow { line, start, text: text.to_owned(), linewise: false }
    }

    /// Line 7 wrapped to two rows, line 8 to one, painted at column x=3, y=5.
    fn painted() -> Painted {
        let rows =
            vec![row(7, 0, "hello "), row(7, 6, "world"), row(8, 0, "日本")].into_iter().map(Some);
        Painted::new(rows.collect(), 2, 7, Rect::new(3, 5, 6, 3))
    }

    /// The same, with a card spliced under line 7's second row.
    fn with_card() -> Painted {
        let rows = vec![
            Some(row(7, 0, "hello ")),
            Some(row(7, 6, "world")),
            None,
            Some(row(8, 0, "日本")),
        ];
        Painted::new(rows, 2, 7, Rect::new(3, 5, 6, 4))
    }

    #[test]
    fn hit_maps_a_screen_cell_to_the_character_under_it() {
        let painted = painted();
        assert_eq!(painted.hit(3, 4), None);
        assert_eq!(painted.hit(4, 5).map(|h| h.pos), Some(Pos { line: 7, col: 1 }));
        assert_eq!(painted.hit(5, 6).map(|h| h.pos), Some(Pos { line: 7, col: 8 }));
        assert_eq!(painted.hit(3, 8), None);
    }

    #[test]
    fn hit_past_the_end_of_a_row_lands_on_its_last_boundary() {
        assert_eq!(painted().hit(99, 6).map(|h| h.pos), Some(Pos { line: 7, col: 11 }));
    }

    #[test]
    fn a_wide_glyph_takes_two_cells() {
        let painted = painted();
        assert_eq!(painted.hit(4, 7).map(|h| h.pos), Some(Pos { line: 8, col: 0 }));
        assert_eq!(painted.hit(5, 7).map(|h| h.pos), Some(Pos { line: 8, col: 1 }));
    }

    #[test]
    fn caret_finds_the_wrapped_row_a_position_fell_on() {
        let painted = painted();
        assert_eq!(painted.caret(Pos { line: 7, col: 8 }), Some((5, 6)));
        assert_eq!(painted.caret(Pos { line: 8, col: 1 }), Some((5, 7)));
        assert_eq!(painted.caret(Pos { line: 9, col: 0 }), None);
    }

    #[test]
    fn a_card_row_selects_nothing_and_still_shifts_what_is_below_it() {
        let painted = with_card();
        assert_eq!(painted.hit(4, 7), None); // the card row itself
        assert_eq!(painted.hit(4, 8).map(|h| h.pos), Some(Pos { line: 8, col: 0 }));
        assert_eq!(painted.caret(Pos { line: 8, col: 0 }), Some((3, 8)));
    }
}
