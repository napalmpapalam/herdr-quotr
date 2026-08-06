//! Color for lines whose markers the transcript already removed: emphasis, code, tables.
//!
//! Nothing here rewrites text. What the picker shows is what a quote sends, with two
//! exceptions that carry their own reason: a blockquote's rule and a rendered table.

mod line;

use ratatui::style::Style;

use markup::{Block, Tone};

use crate::{
    column::GUTTER,
    highlight::Highlighter,
    style::{LineStyle, Run},
    table::Table,
    theme::{Palette, Theme},
    view::Markup,
};

pub(crate) use crate::markdown::line::runs_of;

/// Color every line. Run once per transcript — `syntect` is far too slow per frame.
///
/// User turns are left unstyled: they read dim as a whole, which is the separator's job.
pub fn analyze(lines: &[Markup<'_>], theme: &Theme, measure: u16) -> Vec<LineStyle> {
    Styler {
        highlighter: Highlighter::new(theme.syntax),
        palette: &theme.palette,
        // Tables are laid out for the full measure; a narrower pane drops back to raw markdown
        // at paint time, the only place the real width is known.
        width: usize::from(measure.saturating_sub(GUTTER)),
        out: vec![LineStyle::default(); lines.len()],
    }
    .run(lines)
}

/// Walks the transcript once, holding what a multi-line construct needs.
struct Styler<'a> {
    highlighter: Highlighter,
    palette: &'a Palette,
    width: usize,
    out: Vec<LineStyle>,
}

impl Styler<'_> {
    fn run(mut self, lines: &[Markup<'_>]) -> Vec<LineStyle> {
        let mut index = 0;
        while let Some(line) = lines.get(index) {
            index += self.one(lines, index, line);
        }

        self.out
    }

    /// Style what starts at `index`, returning the lines it claimed — a fenced block and a
    /// rendered table each take several.
    fn one(&mut self, lines: &[Markup<'_>], index: usize, line: &Markup<'_>) -> usize {
        if line.tone != Tone::Agent {
            return 1;
        }
        match line.block {
            Block::Rail => {
                self.set(index, LineStyle::new(vec![Run::new(0, self.dim())]));
                1
            }
            Block::Code => self.fence(lines, index),
            Block::TableRow => match self.table(lines, index) {
                Some(claimed) => claimed,
                None => self.prose(index, line),
            },
            _ => self.prose(index, line),
        }
    }

    fn prose(&mut self, index: usize, line: &Markup<'_>) -> usize {
        let style = line::style(line, self.palette);
        self.set(index, style);
        1
    }

    /// A fenced block's body, highlighted in one pass — `syntect` wants the whole thing, and
    /// the language comes off the rail above it. An unclosed fence ends with its turn, so the
    /// newest one in a live transcript still highlights what it has.
    fn fence(&mut self, lines: &[Markup<'_>], index: usize) -> usize {
        let body: Vec<&str> = lines
            .iter()
            .skip(index)
            .take_while(|line| line.block == Block::Code && line.tone == Tone::Agent)
            .map(|line| line.text)
            .collect();
        let language = index
            .checked_sub(1)
            .and_then(|at| lines.get(at))
            .filter(|rail| rail.block == Block::Rail)
            .map_or("", |rail| language(rail.text));

        let mut source = body.join("\n");
        source.push('\n');
        for (offset, runs) in self.highlighter.runs(&source, language).into_iter().enumerate() {
            self.set(index + offset, LineStyle::new(runs));
        }

        body.len().max(1)
    }

    /// The table opening at `index`, if it is one and its grid fits. Only agent lines take
    /// part, so a table never runs across a turn boundary.
    fn table(&mut self, lines: &[Markup<'_>], index: usize) -> Option<usize> {
        let rows = lines.iter().skip(index).take_while(|line| line.tone == Tone::Agent).count();
        let table = Table::parse(lines.get(index..index + rows)?, self.width)?;

        for (offset, style) in table.styles(self.palette).into_iter().enumerate() {
            self.set(index + offset, style);
        }

        Some(table.len())
    }

    fn dim(&self) -> Style {
        Style::new().fg(self.palette.overlay0)
    }

    fn set(&mut self, index: usize, style: LineStyle) {
        if let Some(slot) = self.out.get_mut(index) {
            *slot = style;
        }
    }
}

/// A fence rail's info string, matched as a language by the highlighter.
fn language(rail: &str) -> &str {
    rail.trim_start_matches(' ')
        .trim_start_matches(['`', '~'])
        .split_whitespace()
        .next()
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use ratatui::style::{Modifier, Style};

    use markup::{Block, Emphasis, Span, Tone};

    use super::analyze;
    use crate::{column::DEFAULT_MEASURE, style::LineStyle, theme, view::Markup};

    fn agent(text: &str) -> Markup<'_> {
        Markup { text, tone: Tone::Agent, block: Block::Prose, spans: &[] }
    }

    fn styled(lines: &[Markup<'_>]) -> Vec<LineStyle> {
        analyze(lines, &theme::default_theme(), DEFAULT_MEASURE)
    }

    /// The color in force at `col` of line `index`.
    fn fg(styled: &[LineStyle], index: usize, col: usize) -> Option<ratatui::style::Color> {
        styled.get(index).and_then(|line| line.at(col).fg)
    }

    #[test]
    fn a_fenced_block_is_syntax_highlighted_between_dim_rails() {
        let p = theme::default_theme().palette;
        let rail = |text| Markup { block: Block::Rail, ..agent(text) };
        let code = |text| Markup { block: Block::Code, ..agent(text) };
        let styled = styled(&[rail("```rust"), code("let x = 1;"), rail("```"), agent("after")]);

        assert_eq!(fg(&styled, 0, 0), Some(p.overlay0));
        assert_eq!(fg(&styled, 2, 0), Some(p.overlay0));
        // The body is colored by syntect, so `let` differs from the text that follows it.
        assert!(fg(&styled, 1, 0).is_some());
        assert_ne!(fg(&styled, 1, 0), fg(&styled, 1, 4));
        assert_eq!(styled.get(3).map(|s| s.at(0)), Some(Style::new()));
    }

    #[test]
    fn an_untagged_fence_leaves_its_body_in_the_base_style() {
        let rail = |text| Markup { block: Block::Rail, ..agent(text) };
        let styled = styled(&[rail("```"), Markup { block: Block::Code, ..agent("# literal") }]);
        assert_eq!(styled.get(1).map(|s| s.at(0)), Some(Style::new()));
    }

    #[test]
    fn a_user_turn_is_left_unstyled() {
        let user = Markup { tone: Tone::User, block: Block::Heading(1), ..agent("my prompt") };
        assert_eq!(styled(&[user]).first().map(|s| s.at(0)), Some(Style::new()));
    }

    #[test]
    fn a_heading_opens_in_its_accent_and_a_span_inside_it_still_reads() {
        let p = theme::default_theme().palette;
        let strong =
            [Span { from: 6, to: 10, emphasis: Emphasis { strong: true, ..<_>::default() } }];
        let line = Markup { block: Block::Heading(1), spans: &strong, ..agent("Title bold") };
        let styled = styled(&[line]);

        assert_eq!(fg(&styled, 0, 0), Some(p.heading));
        assert_eq!(fg(&styled, 0, 6), Some(p.strong));
        assert_eq!(fg(&styled, 0, 10), Some(p.heading));
        assert!(styled.first().is_some_and(|s| s.at(6).add_modifier.contains(Modifier::BOLD)));
    }

    #[test]
    fn a_blockquote_paints_a_rule_and_carries_its_prose_on() {
        let p = theme::default_theme().palette;
        let line = Markup { block: Block::Quote(2), ..agent("> quoted") };
        let styled = styled(&[line]);

        assert_eq!(styled.first().and_then(LineStyle::display), Some("│ quoted"));
        assert_eq!(fg(&styled, 0, 0), Some(p.overlay0));
        assert_eq!(fg(&styled, 0, 2), None);
    }

    #[test]
    fn a_bullet_is_accented_and_a_rule_is_all_marker() {
        let p = theme::default_theme().palette;
        let bullet = Markup { block: Block::Bullet { at: 0, len: 1 }, ..agent("- item") };
        let styled = styled(&[bullet, Markup { block: Block::Rule, ..agent("---") }]);

        assert_eq!(fg(&styled, 0, 0), Some(p.code));
        assert_eq!(fg(&styled, 0, 2), None);
        assert_eq!(fg(&styled, 1, 2), Some(p.overlay0));
    }

    #[test]
    fn a_table_claims_its_own_rows_and_prose_resumes_after_them() {
        let row = |text| Markup { block: Block::TableRow, ..agent(text) };
        let styled = styled(&[
            row("| a | b |"),
            Markup { block: Block::Rule, ..agent("|---|---|") },
            row("| 1 | 2 |"),
            agent("after"),
        ]);

        assert!(styled.first().is_some_and(|s| s.display().is_some()));
        assert!(styled.get(2).is_some_and(|s| s.display().is_some()));
        assert!(styled.get(3).is_some_and(|s| s.display().is_none()));
    }

    #[test]
    fn a_table_too_wide_to_draw_falls_back_to_dim_pipes() {
        let p = theme::default_theme().palette;
        let wide = "| ".to_owned() + &"x".repeat(200) + " |";
        let styled = styled(&[
            Markup { block: Block::TableRow, ..agent(&wide) },
            Markup { block: Block::Rule, ..agent("|---|") },
        ]);

        assert!(styled.first().is_some_and(|s| s.display().is_none()));
        assert_eq!(fg(&styled, 0, 0), Some(p.overlay0));
        assert_eq!(fg(&styled, 0, 2), None);
    }
}
