//! The reading measure — CSS `max-width` + `margin: 0 auto`, in a terminal.

use ratatui::layout::Rect;
use unicode_width::UnicodeWidthStr;

/// Reading measure for text, in columns, when the config file names none.
pub const DEFAULT_MEASURE: u16 = 100;

/// Narrowest measure worth setting: the gutter plus room for a word.
pub const MIN_MEASURE: u16 = 20;

/// Widest measure worth setting. Past this the column stops being a measure.
pub const MAX_MEASURE: u16 = 400;

/// Cells the bank marks take at the left of the column. Always reserved, so banking the
/// first pair doesn't reflow the buffer under the reader.
pub(crate) const GUTTER: u16 = 3;

/// The centered, capped column text renders into.
///
/// An odd gutter's remainder goes right, so the column doesn't jitter on resize.
pub fn content_column(area: Rect, measure: u16) -> Rect {
    let width = area.width.min(measure);
    Rect { x: area.x + (area.width - width) / 2, width, ..area }
}

/// The part of the column text lands in: everything past the bank gutter.
pub(crate) fn text_area(column: Rect) -> Rect {
    Rect {
        x: column.x.saturating_add(GUTTER),
        width: column.width.saturating_sub(GUTTER),
        ..column
    }
}

/// Spaces filling the rest of the column, so a highlight covers the measure not the glyphs.
pub(crate) fn padding(row: &str, width: usize) -> String {
    " ".repeat(width.saturating_sub(row.width()))
}

#[cfg(test)]
mod tests {
    use ratatui::layout::Rect;

    use super::{DEFAULT_MEASURE, GUTTER, content_column, padding, text_area};

    #[test]
    fn caps_and_centers_on_a_wide_pane() {
        let col = content_column(Rect::new(1, 2, 141, 40), DEFAULT_MEASURE);
        assert_eq!(col.width, DEFAULT_MEASURE);
        assert_eq!(col.x, 1 + 20); // odd remainder goes right: 41 -> 20 left, 21 right
        assert_eq!((col.y, col.height), (2, 40));
    }

    #[test]
    fn takes_full_width_below_the_cap() {
        let area = Rect::new(3, 0, 60, 10);
        assert_eq!(content_column(area, DEFAULT_MEASURE), area);
    }

    #[test]
    fn a_configured_measure_replaces_the_default_cap() {
        assert_eq!(content_column(Rect::new(0, 0, 141, 40), 72).width, 72);
    }

    #[test]
    fn the_gutter_comes_off_the_left_of_the_column() {
        let column = Rect::new(20, 0, 100, 10);
        let text = text_area(column);
        assert_eq!(text.x, 20 + GUTTER);
        assert_eq!(text.width, 100 - GUTTER);
    }

    #[test]
    fn pads_by_display_width_not_byte_count() {
        assert_eq!(padding("ab", 5), "   ");
        assert_eq!(padding("日本", 5), " "); // two wide glyphs = four columns
        assert_eq!(padding("toolong", 3), "");
    }
}
