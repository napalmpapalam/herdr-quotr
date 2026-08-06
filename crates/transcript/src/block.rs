//! Reading a line's block-level shape off the markers it opens with.

use markup::Block;

/// Past this many leading spaces a line is indented code, not a marked block.
pub(crate) const CODE_INDENT: usize = 4;

/// The block `chars` opens. Fenced lines are classified by [`crate::markdown`], which is the
/// only place that knows a fence is open.
pub(crate) fn classify(chars: &[char]) -> Block {
    if indent(chars) >= CODE_INDENT {
        return Block::Prose;
    }
    if is_table_rule(chars) || is_thematic_break(chars) {
        return Block::Rule;
    }
    if let Some(level) = heading_level(chars) {
        return Block::Heading(level);
    }
    match (quote_len(chars), bullet_len(chars), is_table_row(chars)) {
        (quote, _, _) if quote > 0 => Block::Quote(quote),
        (_, len, _) if len > 0 => Block::Bullet { at: indent(chars), len },
        (_, _, true) => Block::TableRow,
        _ => Block::Prose,
    }
}

/// Leading spaces, capped at [`CODE_INDENT`].
pub(crate) fn indent(chars: &[char]) -> usize {
    chars.iter().take(CODE_INDENT).take_while(|&&c| c == ' ').count()
}

/// The same, for a line still in its source form.
pub(crate) fn indent_of(text: &str) -> usize {
    text.chars().take(CODE_INDENT).take_while(|&c| c == ' ').count()
}

/// A pipe-table row: starts with `|` once indented past any block marker.
fn is_table_row(chars: &[char]) -> bool {
    chars.get(indent(chars)) == Some(&'|')
}

/// A `|---|:--:|` delimiter row, which carries no content at all.
fn is_table_rule(chars: &[char]) -> bool {
    is_table_row(chars) && chars.iter().all(|&c| matches!(c, '|' | '-' | ':' | ' '))
}

/// Three or more of the same `-`, `*`, or `_`, spaces allowed between and nothing else.
fn is_thematic_break(chars: &[char]) -> bool {
    let at = indent(chars);
    let Some(&marker) = chars.get(at).filter(|c| matches!(c, '-' | '*' | '_')) else {
        return false;
    };

    let rest = || chars.iter().skip(at);
    rest().filter(|&&c| c == marker).count() >= 3 && rest().all(|&c| c == marker || c == ' ')
}

/// `#` count when the line is an ATX heading, else `None`.
fn heading_level(chars: &[char]) -> Option<usize> {
    let at = indent(chars);
    let hashes = chars.iter().skip(at).take_while(|&&c| c == '#').count();
    let followed = matches!(chars.get(at + hashes), None | Some(' '));

    ((1..=6).contains(&hashes) && followed).then_some(hashes)
}

/// Characters a blockquote's marker takes, or 0 when the line is not one. A whole nested
/// prefix counts as one: `> > ` reads as one rule, not two.
fn quote_len(chars: &[char]) -> usize {
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
fn bullet_len(chars: &[char]) -> usize {
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

#[cfg(test)]
mod tests {
    use super::{Block, classify};

    fn block(text: &str) -> Block {
        classify(&text.chars().collect::<Vec<_>>())
    }

    #[test]
    fn a_heading_is_one_to_six_hashes_before_a_space() {
        assert_eq!(block("## Title"), Block::Heading(2));
        assert_eq!(block("####### no"), Block::Prose);
        assert_eq!(block("#no-space"), Block::Prose);
    }

    #[test]
    fn a_bullet_needs_a_space_after_its_marker() {
        assert_eq!(block("- item"), Block::Bullet { at: 0, len: 1 });
        assert_eq!(block("  12. item"), Block::Bullet { at: 2, len: 3 });
        assert_eq!(block("well-known"), Block::Prose);
    }

    #[test]
    fn a_break_is_three_of_one_marker_and_nothing_else() {
        assert_eq!(block("---"), Block::Rule);
        assert_eq!(block("* * *"), Block::Rule);
        assert_eq!(block("- item"), Block::Bullet { at: 0, len: 1 });
    }

    #[test]
    fn only_a_delimiter_row_is_all_scaffolding() {
        assert_eq!(block("|---|:--|"), Block::Rule);
        assert_eq!(block("| a | b |"), Block::TableRow);
        assert_eq!(block("a | b"), Block::Prose);
    }

    #[test]
    fn a_quote_prefix_counts_its_whole_nesting() {
        assert_eq!(block("> quoted"), Block::Quote(2));
        assert_eq!(block("> > nested"), Block::Quote(4));
    }

    #[test]
    fn four_spaces_in_is_indented_code() {
        assert_eq!(block("    # not a heading"), Block::Prose);
    }
}
