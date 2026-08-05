//! Pipe tables drawn as a grid, the one construct the picker renders rather than styles.
//!
//! A grid pads its cells and adds a border above and below, so display characters stop
//! matching source characters. Selection inside a rendered table is therefore linewise —
//! whole rows — and the quote still comes from the transcript, so it goes out as the original
//! `| a | b |` markdown.

use ratatui::style::Style;
use unicode_width::UnicodeWidthStr;

use crate::{
    markdown,
    style::{LineStyle, Run},
    theme::Palette,
};

/// Where a cell's content sits, from the `:---:` delimiter row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Align {
    Left,
    Center,
    Right,
}

/// A table found in the source: its lines, and the cells parsed out of them.
#[derive(Debug)]
pub(crate) struct Table {
    /// Rendered text for each source line the table covers, in order.
    rows: Vec<String>,
    /// The border above the first row and below the last — chrome with no source line.
    top: String,
    bottom: String,
    /// Column boundaries in the rendered text, for painting the rules dim.
    rules: Vec<usize>,
}

impl Table {
    /// The rendered table starting at `lines[0]`, or `None` when this is not a table or the
    /// grid would not fit `width` — a table too wide to draw stays raw markdown, which wraps.
    pub(crate) fn parse(lines: &[&str], width: usize) -> Option<Self> {
        let header = cells(lines.first()?)?;
        let aligns = alignments(lines.get(1)?)?;
        let columns = header.len().max(aligns.len());
        let body: Vec<Vec<String>> = lines.iter().skip(2).map_while(|line| cells(line)).collect();

        let mut widths = vec![0; columns];
        for row in std::iter::once(&header).chain(&body) {
            for (i, cell) in row.iter().enumerate() {
                if let Some(slot) = widths.get_mut(i) {
                    *slot = (*slot).max(cell.width());
                }
            }
        }
        // Two spaces of padding per column, plus one rule per boundary.
        let drawn: usize = widths.iter().map(|w| w + 2).sum::<usize>() + columns + 1;
        if drawn > width {
            return None;
        }

        let mut rows = vec![row_text(&header, &widths, &aligns), rule(&widths, '├', '┼', '┤')];
        rows.extend(body.iter().map(|cells| row_text(cells, &widths, &aligns)));
        Some(Self {
            rows,
            top: rule(&widths, '┌', '┬', '┐'),
            bottom: rule(&widths, '└', '┴', '┘'),
            rules: boundaries(&widths),
        })
    }

    /// How many source lines the table covers.
    pub(crate) fn len(&self) -> usize {
        self.rows.len()
    }

    /// The style for each covered line, in order: the grid text, its dim rules, and the
    /// borders hung above the first line and below the last.
    pub(crate) fn styles(&self, p: &Palette) -> Vec<LineStyle> {
        let dim = Style::new().fg(p.overlay0);
        let mut rules = Vec::with_capacity(self.rules.len() * 2);
        for &at in &self.rules {
            rules.push(Run::new(at, dim));
            rules.push(Run::new(at + 1, Style::new()));
        }
        let last = self.rows.len().saturating_sub(1);
        self.rows
            .iter()
            .enumerate()
            .map(|(index, text)| {
                // A delimiter row is all rule; every other row styles its cells like prose,
                // against the rendered text rather than the source it was padded from.
                let line = if index == 1 {
                    LineStyle::new(vec![Run::new(0, dim)])
                } else {
                    let mut runs = rules.clone();
                    // Pushed last, so an inline span wins a rule at the same offset.
                    runs.extend(markdown::inline_runs(text, p));
                    runs.sort_by_key(Run::start);
                    LineStyle::new(runs)
                };
                let line = line.showing(text.clone()).linewise();
                let line = if index == 0 { line.above(self.top.clone(), dim) } else { line };
                if index == last { line.below(self.bottom.clone(), dim) } else { line }
            })
            .collect()
    }
}

/// A row's cells, or `None` when the line is not a pipe-table row.
fn cells(line: &str) -> Option<Vec<String>> {
    let trimmed = line.trim();
    let inner = trimmed.strip_prefix('|')?;
    let inner = inner.strip_suffix('|').unwrap_or(inner);
    Some(inner.split('|').map(|cell| cell.trim().to_owned()).collect())
}

/// Per-column alignment from a `|:--|:-:|--:|` row, or `None` when it is not one.
fn alignments(line: &str) -> Option<Vec<Align>> {
    let cells = cells(line)?;
    let scaffolding = line.chars().all(|c| matches!(c, '|' | '-' | ':' | ' '));
    if cells.is_empty() || !scaffolding {
        return None;
    }

    cells.iter().map(|cell| align_of(cell)).collect()
}

/// One delimiter cell's alignment: `:` marks the side content sits against. `None` when the
/// cell is not a delimiter at all.
fn align_of(cell: &str) -> Option<Align> {
    let dashes = cell.trim_matches(':');
    if cell.len() < 2 || dashes.is_empty() || !dashes.chars().all(|c| c == '-') {
        return None;
    }

    Some(match (cell.starts_with(':'), cell.ends_with(':')) {
        (true, true) => Align::Center,
        (false, true) => Align::Right,
        _ => Align::Left,
    })
}

/// One grid row: `│ cell │ cell │`, each cell padded to its column and aligned.
fn row_text(cells: &[String], widths: &[usize], aligns: &[Align]) -> String {
    let mut out = String::from("│");
    for (index, &width) in widths.iter().enumerate() {
        let cell = cells.get(index).map_or("", String::as_str);
        let pad = width.saturating_sub(cell.width());
        let align = aligns.get(index).copied().unwrap_or(Align::Left);
        let (left, right) = match align {
            Align::Left => (0, pad),
            Align::Right => (pad, 0),
            Align::Center => (pad / 2, pad - pad / 2),
        };
        out.push(' ');
        out.push_str(&" ".repeat(left));
        out.push_str(cell);
        out.push_str(&" ".repeat(right));
        out.push(' ');
        out.push('│');
    }
    out
}

/// A horizontal rule with the given corner and junction characters.
fn rule(widths: &[usize], start: char, join: char, end: char) -> String {
    let mut out = String::new();
    out.push(start);
    for (index, &width) in widths.iter().enumerate() {
        out.push_str(&"─".repeat(width + 2));
        out.push(if index + 1 == widths.len() { end } else { join });
    }
    out
}

/// Character offsets of every `│` in a rendered row.
fn boundaries(widths: &[usize]) -> Vec<usize> {
    let mut at = 0;
    let mut out = vec![0];
    for &width in widths {
        at += width + 3;
        out.push(at);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{Align, Table, alignments, cells};
    use crate::theme;

    const SRC: [&str; 4] =
        ["| slot | hue |", "|---|:-:|", "| `code` | orange |", "| strong | yellow |"];

    #[test]
    fn a_row_splits_into_trimmed_cells() {
        assert_eq!(cells("| a | bb |"), Some(vec!["a".to_owned(), "bb".to_owned()]));
        assert_eq!(cells("no pipes"), None);
    }

    #[test]
    fn the_delimiter_row_carries_the_alignments() {
        assert_eq!(
            alignments("|---|:-:|--:|"),
            Some(vec![Align::Left, Align::Center, Align::Right])
        );
        assert_eq!(alignments("| a | b |"), None);
    }

    /// The shipped fixture, rendered. Empty when it failed to parse, which every test that
    /// uses it then fails on.
    fn drawn_styles() -> Vec<crate::style::LineStyle> {
        Table::parse(&SRC, 80)
            .map(|t| t.styles(&theme::default_theme().palette))
            .unwrap_or_default()
    }

    #[test]
    fn a_table_draws_a_grid_the_width_of_its_widest_cell() {
        assert_eq!(Table::parse(&SRC, 80).map(|t| t.len()), Some(SRC.len()));
        let styles = drawn_styles();
        let drawn: Vec<&str> = styles.iter().filter_map(|s| s.display()).collect();
        assert_eq!(drawn.first(), Some(&"│ slot   │  hue   │"));
        assert_eq!(drawn.get(1), Some(&"├────────┼────────┤"));
        assert_eq!(drawn.get(2), Some(&"│ `code` │ orange │"));
        // Every row is the same width, so the grid lines up.
        assert!(drawn.iter().all(|row| row.chars().count() == 19));
    }

    #[test]
    fn the_borders_hang_off_the_first_and_last_rows() {
        let styles = drawn_styles();
        assert_eq!(
            styles.first().and_then(|s| s.above_row()).map(|(t, _)| t),
            Some("┌────────┬────────┐"),
        );
        assert_eq!(
            styles.last().and_then(|s| s.below_row()).map(|(t, _)| t),
            Some("└────────┴────────┘")
        );
        assert!(styles.first().and_then(|s| s.below_row()).is_none());
    }

    #[test]
    fn every_row_of_a_rendered_table_selects_linewise() {
        let styles = drawn_styles();
        assert!(!styles.is_empty());
        assert!(styles.iter().all(crate::style::LineStyle::is_linewise));
    }

    #[test]
    fn cell_contents_are_styled_like_prose() {
        let p = theme::default_theme().palette;
        let styles = drawn_styles();
        // "│ `code` │ orange │" — the backtick dims and the word inside takes the code accent.
        let at = |col: usize| styles.get(2).map(|row| row.at(col).fg);
        assert_eq!(at(0), Some(Some(p.overlay0))); // the grid rule
        assert_eq!(at(2), Some(Some(p.overlay0))); // the backtick
        assert_eq!(at(3), Some(Some(p.code))); // the code inside it
    }

    #[test]
    fn a_grid_too_wide_to_draw_stays_raw_markdown() {
        assert!(Table::parse(&SRC, 12).is_none());
    }

    #[test]
    fn a_row_without_a_delimiter_under_it_is_not_a_table() {
        assert!(Table::parse(&["| a | b |", "just prose"], 80).is_none());
    }
}
