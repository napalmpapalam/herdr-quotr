//! Moving the cursor, the viewport, and the selection.

use crate::app::App;

impl App {
    pub fn move_to(&mut self, line: usize) {
        self.cursor = line.min(self.transcript.lines().len().saturating_sub(1));
        self.follow_cursor();
    }

    pub fn move_by(&mut self, delta: isize) {
        self.move_to(self.cursor.saturating_add_signed(delta));
    }

    pub fn page_by(&mut self, pages: isize) {
        let page = isize::try_from(self.painted.page()).unwrap_or(1);
        self.move_by(pages.saturating_mul(page));
    }

    pub fn next_turn(&mut self) {
        if let Some(line) = self.transcript.next_turn(self.cursor) {
            self.move_to(line);
        }
    }

    pub fn prev_turn(&mut self) {
        if let Some(line) = self.transcript.prev_turn(self.cursor) {
            self.move_to(line);
        }
    }

    /// Wheel scroll: moves the viewport, leaves the cursor where it is.
    pub fn scroll_by(&mut self, delta: isize) {
        let last = self.transcript.lines().len().saturating_sub(1);
        self.scroll = self.scroll.saturating_add_signed(delta).min(last);
    }

    pub fn toggle_range(&mut self) {
        self.anchor = self.anchor.is_none().then_some(self.cursor);
    }

    pub fn clear_range(&mut self) {
        self.anchor = None;
    }

    /// Anchor and cursor both land here, so a press without a drag selects one line.
    pub fn press(&mut self, row: u16) {
        let Some(line) = self.painted.hit(row) else { return };
        self.anchor = Some(line);
        self.move_to(line);
    }

    pub fn drag(&mut self, row: u16) {
        let Some(line) = self.painted.hit(row) else { return };
        self.move_to(line);
    }

    /// Keep the cursor on screen, using what the last frame managed to fit.
    fn follow_cursor(&mut self) {
        if self.cursor < self.scroll {
            self.scroll = self.cursor;
            return;
        }
        let bottom = self.scroll + self.painted.page() - 1;
        if self.cursor > bottom {
            self.scroll += self.cursor - bottom;
        }
    }
}
