//! One line's color: its block shape, then the emphasis runs the transcript read inside it.

use ratatui::style::{Modifier, Style};

use markup::{Block, Emphasis, Span};

use crate::{
    style::{LineStyle, Run},
    theme::Palette,
    view::Markup,
};

/// Color one line from what the transcript already read off it.
pub(super) fn style(line: &Markup<'_>, p: &Palette) -> LineStyle {
    let outer = match line.block {
        Block::Heading(level) => heading(level, p),
        _ => Style::new(),
    };

    let mut runs = prefix(line, outer, p);
    runs.extend(runs_of(line.spans, outer, p));
    // Stable, so a span wins a block run starting at the same offset.
    runs.sort_by_key(Run::start);

    let style = LineStyle::new(runs);
    match line.block {
        Block::Quote(len) => style.showing(rule(line.text, len)),
        _ => style,
    }
}

/// Heading color by level: the top two carry the accent, deeper ones step back.
fn heading(level: usize, p: &Palette) -> Style {
    let fg = match level {
        1 | 2 => p.heading,
        3 => p.heading_deep,
        _ => p.subtext0,
    };

    Style::new().fg(fg).add_modifier(Modifier::BOLD)
}

/// The style a run of emphasis paints in. Code wins the color inside bold, and keeps the
/// weight, which is what a nested span should read as.
fn emphasis(e: Emphasis, p: &Palette) -> Style {
    let mut style = Style::new();
    if e.strong {
        style = style.fg(p.strong).add_modifier(Modifier::BOLD);
    }
    if e.link {
        style = style.fg(p.link).add_modifier(Modifier::UNDERLINED);
    }
    if e.code {
        style = style.fg(p.code);
    }
    if e.dim {
        style = style.fg(p.overlay0);
    }
    if e.italic {
        style = style.add_modifier(Modifier::ITALIC);
    }
    if e.struck {
        style = style.add_modifier(Modifier::CROSSED_OUT);
    }

    style
}

/// Runs for the markers the line still carries. A heading has none left, so its accent simply
/// opens the line.
fn prefix(line: &Markup<'_>, outer: Style, p: &Palette) -> Vec<Run> {
    let dim = Style::new().fg(p.overlay0);
    match line.block {
        Block::Heading(_) => vec![Run::new(0, outer)],
        Block::Rule => vec![Run::new(0, dim)],
        Block::Quote(len) => vec![Run::new(0, dim), Run::new(len, outer)],
        Block::Bullet { at, len } => {
            vec![Run::new(at, Style::new().fg(p.code)), Run::new(at + len, outer)]
        }
        Block::TableRow => pipes(line.text, outer, dim),
        _ => Vec::new(),
    }
}

/// A raw table row's pipes, dimmed so the cells read as columns.
fn pipes(text: &str, outer: Style, dim: Style) -> Vec<Run> {
    text.chars()
        .enumerate()
        .filter(|&(_, c)| c == '|')
        .flat_map(|(at, _)| [Run::new(at, dim), Run::new(at + 1, outer)])
        .collect()
}

/// A run in and a run back out for each span, laid over the line's own style.
pub(crate) fn runs_of(spans: &[Span], outer: Style, p: &Palette) -> Vec<Run> {
    spans
        .iter()
        .flat_map(|span| {
            [Run::new(span.from, outer.patch(emphasis(span.emphasis, p))), Run::new(span.to, outer)]
        })
        .collect()
}

/// A blockquote's `>` markers painted as `│`. One character for one of the same width, so
/// offsets and hit testing are untouched — and the quote comes from the transcript, so a
/// quoted blockquote still goes out holding `>`.
fn rule(text: &str, len: usize) -> String {
    text.chars().enumerate().map(|(i, c)| if i < len && c == '>' { '│' } else { c }).collect()
}
