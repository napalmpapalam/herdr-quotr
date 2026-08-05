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

/// Pad to the column so a highlight fills the measure rather than just the glyphs.
pub(crate) fn pad(mut row: String, width: usize) -> String {
    row.extend(std::iter::repeat_n(' ', width.saturating_sub(row.width())));
    row
}

#[cfg(test)]
mod tests {
    use ratatui::layout::Rect;

    use super::{MAX_WIDTH, content_column, pad};

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
        assert_eq!(pad("ab".to_owned(), 5), "ab   ");
        assert_eq!(pad("日本".to_owned(), 5), "日本 "); // two wide glyphs = four columns
        assert_eq!(pad("toolong".to_owned(), 3), "toolong");
    }
}
