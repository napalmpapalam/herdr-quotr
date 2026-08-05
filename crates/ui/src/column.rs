//! The reading measure — CSS `max-width` + `margin: 0 auto`, in a terminal.

use ratatui::layout::Rect;
use unicode_width::UnicodeWidthStr;

/// Reading measure for text, in columns.
pub const MAX_WIDTH: u16 = 100;

/// The centered, capped column text renders into.
///
/// An odd gutter's remainder goes right, so the column doesn't jitter on resize.
pub fn content_column(area: Rect) -> Rect {
    let width = area.width.min(MAX_WIDTH);
    Rect { x: area.x + (area.width - width) / 2, width, ..area }
}

/// Spaces filling the rest of the column, so a highlight covers the measure not the glyphs.
pub(crate) fn padding(row: &str, width: usize) -> String {
    " ".repeat(width.saturating_sub(row.width()))
}

#[cfg(test)]
mod tests {
    use ratatui::layout::Rect;

    use super::{MAX_WIDTH, content_column, padding};

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

    #[test]
    fn pads_by_display_width_not_byte_count() {
        assert_eq!(padding("ab", 5), "   ");
        assert_eq!(padding("日本", 5), " "); // two wide glyphs = four columns
        assert_eq!(padding("toolong", 3), "");
    }
}
