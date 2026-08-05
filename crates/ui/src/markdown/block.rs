//! Line-leading constructs: headings, quotes, bullets, breaks, and table rows.

use ratatui::style::{Modifier, Style};

use crate::theme::Palette;

/// Past this many leading spaces a line is indented code, not a marked block.
const CODE_INDENT: usize = 4;

/// Leading spaces, capped at [`CODE_INDENT`].
pub(super) fn indent(chars: &[char]) -> usize {
    chars.iter().take(CODE_INDENT).take_while(|&&c| c == ' ').count()
}

/// The same, for a line still in its source form.
pub(super) fn indent_of(text: &str) -> usize {
    text.chars().take(CODE_INDENT).take_while(|&c| c == ' ').count()
}

/// A pipe-table row: starts with `|` once indented past any block marker.
pub(super) fn is_table_row(chars: &[char]) -> bool {
    indent(chars) < CODE_INDENT && chars.get(indent(chars)) == Some(&'|')
}

/// A `|---|:--:|` delimiter row, which carries no content at all.
pub(super) fn is_table_rule(chars: &[char]) -> bool {
    is_table_row(chars) && chars.iter().all(|&c| matches!(c, '|' | '-' | ':' | ' '))
}

/// Three or more of the same `-`, `*`, or `_`, spaces allowed between and nothing else.
pub(super) fn is_thematic_break(chars: &[char]) -> bool {
    let at = indent(chars);
    let Some(&marker) = chars.get(at).filter(|c| matches!(c, '-' | '*' | '_')) else {
        return false;
    };

    let rest = || chars.iter().skip(at);
    rest().filter(|&&c| c == marker).count() >= 3 && rest().all(|&c| c == marker || c == ' ')
}

/// `#` count when the line is an ATX heading, else `None`.
pub(super) fn heading_level(chars: &[char]) -> Option<usize> {
    let at = indent(chars);
    let hashes = chars.iter().skip(at).take_while(|&&c| c == '#').count();
    let followed = matches!(chars.get(at + hashes), None | Some(' '));

    ((1..=6).contains(&hashes) && followed).then_some(hashes)
}

/// Heading color by level: the top two carry the accent, deeper ones step back.
pub(super) fn heading_style(level: usize, p: &Palette) -> Style {
    let fg = match level {
        1 | 2 => p.heading,
        3 => p.heading_deep,
        _ => p.subtext0,
    };

    Style::new().fg(fg).add_modifier(Modifier::BOLD)
}

/// Characters a blockquote's marker takes, or 0 when the line is not one. A whole nested
/// prefix counts as one: `> > ` reads as one rule, not two.
pub(super) fn quote_len(chars: &[char]) -> usize {
    let at = indent(chars);
    if chars.get(at) != Some(&'>') {
        return 0;
    }

    (at..chars.len())
        .take_while(|&i| matches!(chars.get(i), Some('>' | ' ')))
        .last()
        .map_or(at, |i| i + 1)
}

/// Characters the list bullet takes, or 0 when the line has none.
pub(super) fn bullet_len(chars: &[char]) -> usize {
    let at = |i: usize| chars.get(i).copied();
    let start = indent(chars);
    if matches!(at(start), Some('-' | '*' | '+')) && matches!(at(start + 1), None | Some(' ')) {
        return 1;
    }

    let digits = chars.iter().skip(start).take(9).take_while(|c| c.is_ascii_digit()).count();
    let ordered = digits > 0
        && matches!(at(start + digits), Some('.' | ')'))
        && matches!(at(start + digits + 1), None | Some(' '));

    usize::from(ordered) * (digits + 1)
}

/// A blockquote's `>` markers painted as `│`, or `None` when the line has none.
///
/// One-for-one and the same width, and the quote is built from the transcript rather than
/// from what was painted, so a quoted blockquote still goes out holding `>`.
pub(super) fn quote_rule(chars: &[char]) -> Option<String> {
    let end = match quote_len(chars) {
        0 => return None,
        end => end,
    };

    Some(
        chars.iter().enumerate().map(|(i, &c)| if i < end && c == '>' { '│' } else { c }).collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::{bullet_len, heading_level, is_table_rule, is_thematic_break, quote_rule};

    fn chars(text: &str) -> Vec<char> {
        text.chars().collect()
    }

    #[test]
    fn a_heading_is_one_to_six_hashes_before_a_space() {
        assert_eq!(heading_level(&chars("## Title")), Some(2));
        assert_eq!(heading_level(&chars("####### no")), None);
        assert_eq!(heading_level(&chars("#no-space")), None);
    }

    #[test]
    fn a_bullet_needs_a_space_after_its_marker() {
        assert_eq!(bullet_len(&chars("- item")), 1);
        assert_eq!(bullet_len(&chars("12. item")), 3);
        assert_eq!(bullet_len(&chars("well-known")), 0);
    }

    #[test]
    fn a_break_is_three_of_one_marker_and_nothing_else() {
        assert!(is_thematic_break(&chars("---")));
        assert!(is_thematic_break(&chars("* * *")));
        assert!(!is_thematic_break(&chars("- item")));
    }

    #[test]
    fn only_a_delimiter_row_is_all_scaffolding() {
        assert!(is_table_rule(&chars("|---|:--|")));
        assert!(!is_table_rule(&chars("| a | b |")));
    }

    #[test]
    fn a_quote_marker_paints_as_a_rule_without_touching_the_prose() {
        assert_eq!(quote_rule(&chars("> quoted")).as_deref(), Some("│ quoted"));
        assert_eq!(quote_rule(&chars("> > nested")).as_deref(), Some("│ │ nested"));
        assert_eq!(quote_rule(&chars("> a > b")).as_deref(), Some("│ a > b"));
        assert_eq!(quote_rule(&chars("no quote here")), None);
    }
}
