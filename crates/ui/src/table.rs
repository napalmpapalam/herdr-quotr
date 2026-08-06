//! Pipe tables drawn as a grid, the one construct the picker renders rather than styles.
//!
//! A grid pads its cells and adds a border above and below, so display characters stop
//! matching source characters. Selection inside a rendered table is therefore linewise —
//! whole rows — and the quote still comes from the transcript, so it goes out as the original
//! `| a | b |` markdown.

use ratatui::style::Style;
use unicode_width::UnicodeWidthStr;

use markup::Span;

use crate::{
    markdown,
    style::{LineStyle, Run},
    theme::Palette,
    view::Markup,
};

/// Where a cell's content sits, from the `:---:` delimiter row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Align {
    Left,
    Center,
    Right,
}

/// One cell's text and where it started in the source row, which is what carries the row's
/// emphasis spans across the padding into the grid.
#[derive(Debug)]
struct Cell {
    at: usize,
    text: String,
}

/// A table found in the source: its lines, and the cells parsed out of them.
#[derive(Debug)]
pub(crate) struct Table {
    /// Rendered text for each source line the table covers, in order.
    rows: Vec<String>,
    /// Each row's emphasis, moved into rendered coordinates.
    spans: Vec<Vec<Span>>,
    /// The border above the first row and below the last — chrome with no source line.
    top: String,
    bottom: String,
    /// Column boundaries in the rendered text, for painting the rules dim.
    rules: Vec<usize>,
}

impl Table {
    /// The rendered table starting at `lines[0]`, or `None` when this is not a table or the
    /// grid would not fit `width` — a table too wide to draw stays raw markdown, which wraps.
    pub(crate) fn parse(lines: &[Markup<'_>], width: usize) -> Option<Self> {
        let header = cells(lines.first()?.text)?;
        let aligns = alignments(lines.get(1)?.text)?;
        let columns = header.len().max(aligns.len());
        let body: Vec<Vec<Cell>> =
            lines.iter().skip(2).map_while(|line| cells(line.text)).collect();

        let mut widths = vec![0; columns];
        for row in std::iter::once(&header).chain(&body) {
            for (i, cell) in row.iter().enumerate() {
                if let Some(slot) = widths.get_mut(i) {
                    *slot = (*slot).max(cell.text.width());
                }
            }
        }
        // Two spaces of padding per column, plus one rule per boundary.
        let drawn: usize = widths.iter().map(|w| w + 2).sum::<usize>() + columns + 1;
        if drawn > width {
            return None;
        }

        let mut rows = Vec::with_capacity(body.len() + 2);
        let mut spans = Vec::with_capacity(body.len() + 2);
        let (text, starts) = row_text(&header, &widths, &aligns);
        rows.push(text);
        spans.push(moved(lines.first()?.spans, &header, &starts));
        // The delimiter row is all rule: it renders as one and carries no emphasis.
        rows.push(rule(&widths, '├', '┼', '┤'));
        spans.push(Vec::new());

        for (index, cells) in body.iter().enumerate() {
            let (text, starts) = row_text(cells, &widths, &aligns);
            let source = lines.get(index + 2).map_or(&[][..], |line| line.spans);
            rows.push(text);
            spans.push(moved(source, cells, &starts));
        }

        Some(Self {
            rows,
            spans,
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
                // A delimiter row is all rule; every other row keeps the emphasis its source
                // carried, moved into the padded cell it now sits in.
                let line = if index == 1 {
                    LineStyle::new(vec![Run::new(0, dim)])
                } else {
                    let mut runs = rules.clone();
                    // Pushed last, so an emphasis run wins a rule at the same offset.
                    let spans = self.spans.get(index).map_or(&[][..], Vec::as_slice);
                    runs.extend(markdown::runs_of(spans, Style::new(), p));
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

/// A row's cells, or `None` when the line is not a pipe-table row. Each cell remembers the
/// character it started at, so the row's emphasis can follow it into the grid.
fn cells(line: &str) -> Option<Vec<Cell>> {
    let chars: Vec<char> = line.chars().collect();
    let start = chars.iter().position(|c| !c.is_whitespace())?;
    if chars.get(start) != Some(&'|') {
        return None;
    }
    let end = chars.iter().rposition(|c| !c.is_whitespace()).map_or(start, |at| at + 1);

    let bars: Vec<usize> = (start + 1..end).filter(|&at| chars.get(at) == Some(&'|')).collect();
    let mut out = Vec::with_capacity(bars.len() + 1);
    let mut from = start + 1;
    for &at in &bars {
        out.push(cell(&chars, from, at));
        from = at + 1;
    }
    if from < end {
        out.push(cell(&chars, from, end));
    }

    Some(out)
}

/// One cell: the characters `from..to` with their surrounding spaces dropped.
fn cell(chars: &[char], from: usize, to: usize) -> Cell {
    let lead = (from..to).take_while(|&at| chars.get(at) == Some(&' ')).count();
    let trail = (from + lead..to).rev().take_while(|&at| chars.get(at) == Some(&' ')).count();

    Cell {
        at: from + lead,
        text: chars.get(from + lead..to - trail).unwrap_or_default().iter().collect(),
    }
}

/// A row's spans, moved from source offsets into the rendered row's padded cells.
fn moved(spans: &[Span], cells: &[Cell], starts: &[usize]) -> Vec<Span> {
    cells
        .iter()
        .zip(starts)
        .flat_map(|(cell, &at)| {
            let end = cell.at + cell.text.chars().count();
            spans.iter().filter_map(move |span| span.shift(cell.at, end, at))
        })
        .collect()
}

/// Per-column alignment from a `|:--|:-:|--:|` row, or `None` when it is not one.
fn alignments(line: &str) -> Option<Vec<Align>> {
    let cells = cells(line)?;
    let scaffolding = line.chars().all(|c| matches!(c, '|' | '-' | ':' | ' '));
    if cells.is_empty() || !scaffolding {
        return None;
    }

    cells.iter().map(|cell| align_of(&cell.text)).collect()
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

/// One grid row: `│ cell │ cell │`, each cell padded to its column and aligned. Also hands
/// back where each cell's text starts, which is what carries its emphasis across.
fn row_text(cells: &[Cell], widths: &[usize], aligns: &[Align]) -> (String, Vec<usize>) {
    let mut out = String::from("│");
    let mut starts = Vec::with_capacity(widths.len());
    for (index, &width) in widths.iter().enumerate() {
        let cell = cells.get(index).map_or("", |cell| cell.text.as_str());
        let pad = width.saturating_sub(cell.width());
        let align = aligns.get(index).copied().unwrap_or(Align::Left);
        let (left, right) = match align {
            Align::Left => (0, pad),
            Align::Right => (pad, 0),
            Align::Center => (pad / 2, pad - pad / 2),
        };
        out.push(' ');
        out.push_str(&" ".repeat(left));
        starts.push(out.chars().count());
        out.push_str(cell);
        out.push_str(&" ".repeat(right));
        out.push(' ');
        out.push('│');
    }
    (out, starts)
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
    use markup::{Block, Emphasis, Span, Tone};

    use super::{Align, Table, alignments, cells};
    use crate::{theme, view::Markup};

    /// The fixture as the transcript hands it over: markers already stripped, and `code`
    /// carrying the emphasis its backticks used to say.
    const SRC: [&str; 4] =
        ["| slot | hue |", "|---|:-:|", "| code | orange |", "| strong | yellow |"];

    const CODE: [Span; 1] = [Span {
        from: 2,
        to: 6,
        emphasis: Emphasis {
            code: true,
            strong: false,
            italic: false,
            struck: false,
            link: false,
            dim: false,
        },
    }];

    fn rows() -> Vec<Markup<'static>> {
        SRC.iter()
            .enumerate()
            .map(|(index, text)| Markup {
                text,
                tone: Tone::Agent,
                block: Block::TableRow,
                spans: if index == 2 { &CODE } else { &[] },
            })
            .collect()
    }

    #[test]
    fn a_row_splits_into_trimmed_cells() {
        let split = cells("| a | bb |").unwrap_or_default();
        assert_eq!(
            split.iter().map(|cell| (cell.at, cell.text.as_str())).collect::<Vec<_>>(),
            [(2, "a"), (6, "bb")]
        );
        assert!(cells("no pipes").is_none());
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
        Table::parse(&rows(), 80)
            .map(|t| t.styles(&theme::default_theme().palette))
            .unwrap_or_default()
    }

    #[test]
    fn a_table_draws_a_grid_the_width_of_its_widest_cell() {
        assert_eq!(Table::parse(&rows(), 80).map(|t| t.len()), Some(SRC.len()));
        let styles = drawn_styles();
        let drawn: Vec<&str> = styles.iter().filter_map(|s| s.display()).collect();
        assert_eq!(drawn.first(), Some(&"│ slot   │  hue   │"));
        assert_eq!(drawn.get(1), Some(&"├────────┼────────┤"));
        assert_eq!(drawn.get(2), Some(&"│ code   │ orange │"));
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
        // "│ code   │ orange │" — the source cell's emphasis follows it into the padding.
        let at = |col: usize| styles.get(2).map(|row| row.at(col).fg);
        assert_eq!(at(0), Some(Some(p.overlay0))); // the grid rule
        assert_eq!(at(2), Some(Some(p.code))); // the cell that was `code`
        assert_eq!(at(6), Some(None)); // the padding after it
        assert_eq!(at(11), Some(None)); // an unemphasized cell
    }

    #[test]
    fn a_grid_too_wide_to_draw_stays_raw_markdown() {
        assert!(Table::parse(&rows(), 12).is_none());
    }

    #[test]
    fn a_row_without_a_delimiter_under_it_is_not_a_table() {
        let loose: Vec<Markup<'_>> = ["| a | b |", "just prose"]
            .iter()
            .map(|text| Markup { text, tone: Tone::Agent, block: Block::TableRow, spans: &[] })
            .collect();
        assert!(Table::parse(&loose, 80).is_none());
    }
}
