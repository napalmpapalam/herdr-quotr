//! The three cells at the left of the reading column: bank marks and turn markers.

use markup::Tone;
use ratatui::{style::Style, text::Span};

use crate::{column::GUTTER, theme::Palette, view::View};

/// Marks the first line of an agent turn, and of a user prompt.
const AGENT_TURN: char = '●';
const USER_TURN: char = '❯';

/// A gutter cell and how bright it reads: a turn marker takes its own turn's color, so an
/// answer opens with a full-strength dot and a prompt stays as quiet as its text.
pub(super) struct Mark {
    text: String,
    tone: Option<Tone>,
}

pub(super) fn span(mark: Mark, p: &Palette) -> Span<'static> {
    let fg = match mark.tone {
        Some(Tone::Agent | Tone::Gap) => p.text,
        _ => p.overlay0,
    };

    Span::styled(mark.text, Style::new().fg(fg))
}

/// The gutter cell for a row: a banked pair's number where its range opens and a bar down the
/// rest of it, a turn marker where a turn opens, blank elsewhere.
///
/// `numbered` is false on a continuation row and on a card.
pub(super) fn mark(view: &View, line: usize, numbered: bool) -> Mark {
    let width = usize::from(GUTTER) - 1;
    let dim = |text: String| Mark { text, tone: None };

    // Banking wins the cell — it is the rarer signal, and a turn boundary is still readable
    // from the text.
    if let Some(pair) = view.banked.iter().find(|pair| (pair.from..=pair.to).contains(&line)) {
        let number =
            if numbered && line == pair.from { pair.number.to_string() } else { String::new() };
        return dim(format!("{number:>width$}│"));
    }

    if !numbered || !view.turns.contains(&line) {
        return dim(" ".repeat(usize::from(GUTTER)));
    }

    let tone = view.lines.get(line).map_or(Tone::Agent, |l| l.tone);
    let glyph = if tone == Tone::User { USER_TURN } else { AGENT_TURN };

    Mark { text: format!("{glyph:>width$} "), tone: Some(tone) }
}

#[cfg(test)]
mod tests {
    use super::mark;

    use markup::{Pos, Tone};

    use crate::{
        theme,
        view::{Banked, Scroll, SourceLine, View},
    };

    fn text(view: &View, line: usize, numbered: bool) -> String {
        mark(view, line, numbered).text
    }

    const BANKED: [Banked<'static>; 2] = [
        Banked { number: 1, from: 2, to: 4, question: "why?", quote: "answer" },
        Banked { number: 2, from: 9, to: 9, question: "", quote: "more" },
    ];

    const LINES: [SourceLine<'static>; 3] = [
        SourceLine { text: "prompt", tone: Tone::User },
        SourceLine { text: "answer", tone: Tone::Agent },
        SourceLine { text: "more", tone: Tone::Agent },
    ];

    fn view<'a>(banked: &'a [Banked<'a>], turns: &'a [usize]) -> View<'a> {
        View {
            lines: &LINES,
            styles: &[],
            palette: theme::default_theme().palette,
            turns,
            cursor: Pos::default(),
            selection: None,
            banked,
            scroll: Scroll::Bottom,
            question: None,
            status: "",
        }
    }

    #[test]
    fn the_number_prints_once_where_the_range_opens() {
        let view = view(&BANKED, &[]);
        assert_eq!(text(&view, 2, true), " 1│");
        assert_eq!(text(&view, 2, false), "  │"); // a wrapped continuation of the same line
        assert_eq!(text(&view, 3, true), "  │");
        assert_eq!(text(&view, 4, true), "  │");
    }

    #[test]
    fn an_unbanked_unmarked_line_gets_blank_cells_the_same_width() {
        assert_eq!(text(&view(&BANKED, &[]), 5, true), "   ");
        assert_eq!(text(&view(&[], &[]), 0, true), "   ");
    }

    #[test]
    fn a_one_line_pair_is_marked_too() {
        assert_eq!(text(&view(&BANKED, &[]), 9, true), " 2│");
    }

    #[test]
    fn a_turn_opens_with_its_own_glyph() {
        let view = view(&[], &[0, 1]);
        assert_eq!(text(&view, 0, true), " ❯ "); // a user prompt
        assert_eq!(text(&view, 1, true), " ● "); // an agent answer
        assert_eq!(text(&view, 1, false), "   "); // not on a wrapped continuation
        assert_eq!(text(&view, 2, true), "   "); // mid-turn
    }

    #[test]
    fn banking_wins_the_gutter_cell_from_a_turn_marker() {
        assert_eq!(text(&view(&BANKED, &[2]), 2, true), " 1│");
    }
}
