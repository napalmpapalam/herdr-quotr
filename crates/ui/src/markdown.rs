//! Markdown styling of the raw source: dim the markers, color what they mark.
//!
//! No rendering — a display character always is a source character, which is what lets a
//! selection map back to bytes the transcript holds. Tables are the one exception
//! ([`crate::table`]).

mod block;
mod fence;
mod inline;

use ratatui::style::Style;

use crate::{
    column::{GUTTER, MAX_WIDTH},
    highlight::Highlighter,
    markdown::fence::Fence,
    style::{LineStyle, Run},
    table::Table,
    theme::{Palette, Theme},
    view::{SourceLine, Tone},
};

pub(crate) use crate::markdown::inline::runs as inline_runs;

/// Style every source line. Run once per transcript — `syntect` is far too slow per frame.
///
/// User turns are left unstyled: they read dim as a whole, which is the separator's job.
pub fn analyze(lines: &[SourceLine<'_>], theme: &Theme) -> Vec<LineStyle> {
    Styler {
        highlighter: Highlighter::new(theme.syntax),
        palette: &theme.palette,
        // Tables are laid out for the full measure; a narrower pane drops back to raw markdown
        // at paint time, the only place the real width is known.
        width: usize::from(MAX_WIDTH - GUTTER),
        out: vec![LineStyle::default(); lines.len()],
    }
    .run(lines)
}

/// Walks the transcript once, holding the state a line's styling depends on.
struct Styler<'a> {
    highlighter: Highlighter,
    palette: &'a Palette,
    width: usize,
    out: Vec<LineStyle>,
}

impl Styler<'_> {
    fn run(mut self, lines: &[SourceLine<'_>]) -> Vec<LineStyle> {
        let mut open: Option<Fence> = None;
        // Lines a rendered table already claimed.
        let mut skip_to = 0;

        for (index, line) in lines.iter().enumerate() {
            if index < skip_to {
                continue;
            }
            // A fence never spans a turn boundary, so an unclosed one ends with its turn.
            if line.tone != Tone::Agent {
                self.close(open.take());
                continue;
            }
            match open.take() {
                Some(fence) => open = self.in_fence(fence, index, line.text),
                None => skip_to = self.opening(lines, index, &mut open),
            }
        }

        self.close(open);
        self.out
    }

    /// A line inside a fence: the closing rail, or one more body line.
    fn in_fence(&mut self, mut fence: Fence, index: usize, text: &str) -> Option<Fence> {
        if !fence.closed_by(text) {
            fence.push(text);
            return Some(fence);
        }

        self.set(index, self.marker_line());
        self.close(Some(fence));
        None
    }

    /// A line outside a fence: it opens one, opens a table, or is prose. Returns the line to
    /// resume at, which a table moves past its own rows.
    fn opening(
        &mut self,
        lines: &[SourceLine<'_>],
        index: usize,
        open: &mut Option<Fence>,
    ) -> usize {
        let Some(text) = lines.get(index).map(|line| line.text) else { return 0 };

        if let Some(fence) = Fence::opened_by(text, index + 1) {
            self.set(index, self.marker_line());
            *open = Some(fence);
            return 0;
        }

        if let Some(table) = self.table_at(lines, index) {
            let end = index + table.len();
            for (offset, style) in table.styles(self.palette).into_iter().enumerate() {
                self.set(index + offset, style);
            }
            return end;
        }

        self.set(index, inline::line(text, self.palette));
        0
    }

    /// The table opening at `index`. Only agent lines take part, so a table never runs across
    /// a turn boundary.
    fn table_at(&self, lines: &[SourceLine<'_>], index: usize) -> Option<Table> {
        let rows: Vec<&str> = lines
            .iter()
            .skip(index)
            .take_while(|line| line.tone == Tone::Agent)
            .map(|line| line.text)
            .collect();

        Table::parse(&rows, self.width)
    }

    /// Highlight a finished fence's body into the lines it covered.
    fn close(&mut self, fence: Option<Fence>) {
        let Some(fence) = fence else { return };

        for (index, style) in fence.highlight(&self.highlighter) {
            self.set(index, style);
        }
    }

    /// A line that is all marker — a fence rail.
    fn marker_line(&self) -> LineStyle {
        LineStyle::new(vec![Run::new(0, Style::new().fg(self.palette.overlay0))])
    }

    fn set(&mut self, index: usize, style: LineStyle) {
        if let Some(slot) = self.out.get_mut(index) {
            *slot = style;
        }
    }
}

#[cfg(test)]
mod tests {
    use ratatui::style::Style;

    use super::analyze;
    use crate::{
        style::LineStyle,
        theme,
        view::{SourceLine, Tone},
    };

    fn agent(text: &str) -> SourceLine<'_> {
        SourceLine { text, tone: Tone::Agent }
    }

    fn styled(lines: &[SourceLine<'_>]) -> Vec<LineStyle> {
        analyze(lines, &theme::default_theme())
    }

    #[test]
    fn a_fenced_block_is_syntax_highlighted_between_dim_rails() {
        let p = theme::default_theme().palette;
        let styled = styled(&[agent("```rust"), agent("let x = 1;"), agent("```"), agent("after")]);
        assert_eq!(styled.first().map(|s| s.at(0).fg), Some(Some(p.overlay0)));
        assert_eq!(styled.get(2).map(|s| s.at(0).fg), Some(Some(p.overlay0)));

        // The body is colored by syntect, so `let` differs from the text that follows it.
        let body = styled.get(1).map(|s| (s.at(0).fg, s.at(4).fg));
        assert!(matches!(body, Some((Some(_), Some(_)))));
        assert_ne!(body.map(|(a, b)| a == b), Some(true));

        // Markdown styling resumes after the closing rail.
        assert_eq!(styled.get(3).map(|s| s.at(0)), Some(Style::new()));
    }

    #[test]
    fn markdown_inside_a_fence_stays_literal() {
        let styled = styled(&[agent("```"), agent("# not a heading"), agent("```")]);
        assert_eq!(styled.get(1).map(|s| s.at(0)), Some(Style::new()));
    }

    #[test]
    fn an_unclosed_fence_still_highlights_what_it_has() {
        // The transcript is being appended to, so the last fence is often still open.
        let styled = styled(&[agent("```rust"), agent("let x = 1;")]);
        assert!(styled.get(1).is_some_and(|s| s.at(0) != Style::new()));
    }

    #[test]
    fn a_user_turn_is_left_unstyled_and_ends_any_open_fence() {
        let user = SourceLine { text: "# my prompt", tone: Tone::User };
        let styled = styled(&[agent("```rust"), agent("let x = 1;"), user]);
        assert_eq!(styled.get(2).map(|s| s.at(0)), Some(Style::new()));
    }

    #[test]
    fn a_table_claims_its_own_rows_and_prose_resumes_after_them() {
        let styled = styled(&[
            agent("| a | b |"),
            agent("|---|---|"),
            agent("| 1 | 2 |"),
            agent("**after**"),
        ]);
        assert!(styled.first().is_some_and(|s| s.display().is_some()));
        assert!(styled.get(2).is_some_and(|s| s.display().is_some()));
        assert!(styled.get(3).is_some_and(|s| s.display().is_none()));
    }
}
