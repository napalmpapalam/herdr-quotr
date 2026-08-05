//! Greedy word wrap for the reading column.

use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// Wrap `text` to `width` columns, breaking inside a word only when the word is longer.
///
/// Rows concatenate back to `text`, so a display break never leaks into a quote.
pub(crate) fn wrap(text: &str, width: usize) -> Vec<String> {
    if width == 0 || text.is_empty() {
        return vec![String::new()];
    }
    let mut rows = Vec::new();
    let mut row = String::new();
    let mut used = 0;
    let mut after_space: Option<usize> = None;
    for ch in text.chars() {
        let w = ch.width().unwrap_or(0);
        if used + w > width && !row.is_empty() {
            let carry = after_space
                .filter(|&at| at < row.len())
                .map_or_else(String::new, |at| row.split_off(at));
            rows.push(std::mem::replace(&mut row, carry));
            used = row.width();
            after_space = None;
        }
        row.push(ch);
        used += w;
        if ch == ' ' {
            after_space = Some(row.len());
        }
    }
    rows.push(row);
    rows
}

#[cfg(test)]
mod tests {
    use super::wrap;

    #[test]
    fn breaks_at_the_last_space_that_fits() {
        assert_eq!(wrap("the quick brown fox", 10), ["the quick ", "brown fox"]);
    }

    #[test]
    fn hard_breaks_a_word_longer_than_the_column() {
        assert_eq!(wrap("supercalifragilistic", 8), ["supercal", "ifragili", "stic"]);
    }

    #[test]
    fn keeps_a_blank_line_as_one_row() {
        assert_eq!(wrap("", 40), [""]);
    }

    #[test]
    fn rows_rejoin_to_the_source_line() {
        let text = "  indented code that runs past the column and keeps going";
        assert_eq!(wrap(text, 12).concat(), text);
    }
}
