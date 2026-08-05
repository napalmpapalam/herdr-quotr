//! Moving the caret, the viewport, and the selection.

use transcript::Pos;
use ui::Pos as Screen;

use crate::app::{App, Grain};

impl App {
    /// Put the caret on `pos`, pulled back onto text that exists.
    pub fn move_to(&mut self, pos: Pos) {
        self.cursor = self.transcript.clamp(pos);
        self.follow_cursor();
    }

    /// By character, stopping at the ends of the line rather than wrapping onto the next.
    pub fn move_by_char(&mut self, delta: isize) {
        self.move_to(Pos { col: self.cursor.col.saturating_add_signed(delta), ..self.cursor });
    }

    /// By line, keeping the column where the new line is long enough to hold it.
    pub fn move_by_line(&mut self, delta: isize) {
        self.move_to(Pos { line: self.cursor.line.saturating_add_signed(delta), ..self.cursor });
    }

    pub fn move_to_line(&mut self, line: usize) {
        self.move_to(Pos::line_start(line));
    }

    pub fn page_by(&mut self, pages: isize) {
        let page = isize::try_from(self.painted.page()).unwrap_or(1);
        self.move_by_line(pages.saturating_mul(page));
    }

    pub fn next_turn(&mut self) {
        if let Some(line) = self.transcript.next_turn(self.cursor.line) {
            self.move_to_line(line);
        }
    }

    pub fn prev_turn(&mut self) {
        if let Some(line) = self.transcript.prev_turn(self.cursor.line) {
            self.move_to_line(line);
        }
    }

    /// Wheel scroll: moves the viewport, leaves the caret where it is.
    pub fn scroll_by(&mut self, delta: isize) {
        let last = self.transcript.lines().len().saturating_sub(1);
        self.scroll = self.scroll.saturating_add_signed(delta).min(last);
    }

    /// A press starts a selection at the character under the pointer; releasing without a
    /// drag leaves it empty, so a bare click just places the caret.
    pub fn press(&mut self, cell: Option<Screen>) {
        let Some(pos) = cell.map(from_screen) else { return };
        self.anchor = Some(self.transcript.clamp(pos));
        self.grain = Grain::Char; // the pointer always means exactly what it crossed
        self.move_to(pos);
    }

    pub fn drag(&mut self, cell: Option<Screen>) {
        let Some(pos) = cell.map(from_screen) else { return };
        self.move_to(pos);
    }

    /// Keep the caret on screen, using what the last frame managed to fit.
    fn follow_cursor(&mut self) {
        if self.cursor.line < self.scroll {
            self.scroll = self.cursor.line;
            return;
        }
        let bottom = self.scroll + self.painted.page() - 1;
        if self.cursor.line > bottom {
            self.scroll += self.cursor.line - bottom;
        }
    }
}

fn from_screen(pos: Screen) -> Pos {
    Pos::new(pos.line, pos.col)
}
