//! Greedy word wrap for the reading column.

use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// One display row of a wrapped source line.
#[derive(Debug)]
pub(crate) struct Row {
    /// Character offset of this row's first character within the source line.
    pub(crate) start: usize,
    pub(crate) text: String,
}

/// Wrap `text` to `width` columns, breaking inside a word only when the word is longer.
///
/// Rows concatenate back to `text`, so a display break never leaks into a quote.
pub(crate) fn wrap(text: &str, width: usize) -> Vec<Row> {
    if width == 0 || text.is_empty() {
        return vec![Row { start: 0, text: String::new() }];
    }
    let mut rows = Vec::new();
    let mut row = String::new();
    let mut start = 0;
    let mut used = 0;
    let mut after_space: Option<usize> = None;
    for ch in text.chars() {
        let w = ch.width().unwrap_or(0);
        if used + w > width && !row.is_empty() {
            let carry = after_space
                .filter(|&at| at < row.len())
                .map_or_else(String::new, |at| row.split_off(at));
            let done = std::mem::replace(&mut row, carry);
            let taken = done.chars().count();
            rows.push(Row { start, text: done });
            start += taken;
            used = row.width();
            after_space = None;
        }
        row.push(ch);
        used += w;
        if ch == ' ' {
            after_space = Some(row.len());
        }
    }
    rows.push(Row { start, text: row });
    rows
}

/// Characters `from..to` of `text`, clamped — the display-side twin of slicing a source line.
pub(crate) fn cut(text: &str, from: usize, to: usize) -> &str {
    let byte_at = |col: usize| text.char_indices().nth(col).map_or(text.len(), |(at, _)| at);
    text.get(byte_at(from)..byte_at(to.max(from))).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{Row, cut, wrap};

    fn texts(rows: &[Row]) -> Vec<&str> {
        rows.iter().map(|row| row.text.as_str()).collect()
    }

    #[test]
    fn breaks_at_the_last_space_that_fits() {
        assert_eq!(texts(&wrap("the quick brown fox", 10)), ["the quick ", "brown fox"]);
    }

    #[test]
    fn hard_breaks_a_word_longer_than_the_column() {
        assert_eq!(texts(&wrap("supercalifragilistic", 8)), ["supercal", "ifragili", "stic"]);
    }

    #[test]
    fn keeps_a_blank_line_as_one_row() {
        assert_eq!(texts(&wrap("", 40)), [""]);
    }

    #[test]
    fn rows_rejoin_to_the_source_line() {
        let text = "  indented code that runs past the column and keeps going";
        assert_eq!(texts(&wrap(text, 12)).concat(), text);
    }

    #[test]
    fn each_row_knows_where_it_starts_in_the_source_line() {
        // Counted in characters, not bytes: the wide glyphs are two columns each.
        let rows = wrap("日本語 tail", 6);
        assert_eq!(texts(&rows), ["日本語", " tail"]);
        assert_eq!(rows.iter().map(|row| row.start).collect::<Vec<_>>(), [0, 3]);
    }

    #[test]
    fn cuts_by_character_and_clamps() {
        assert_eq!(cut("日本語abc", 1, 4), "本語a");
        assert_eq!(cut("abc", 2, 99), "c");
        assert_eq!(cut("abc", 3, 1), "");
    }
}
