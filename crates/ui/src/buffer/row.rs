//! One display row: its text, its styling, and the selection laid over both.

use ratatui::{
    style::Style,
    text::{Line, Span},
};
use unicode_width::UnicodeWidthStr;

use crate::{
    buffer::gutter,
    column::padding,
    style::LineStyle,
    theme::Palette,
    view::{Pos, Tone, View},
    wrap::{Row, cut},
};

/// This line's style, dropped when what it renders to is wider than the column — a table
/// laid out for the full measure has to fall back to raw markdown on a narrower pane.
pub(super) fn style<'a>(view: &'a View, line: usize, width: usize) -> Option<&'a LineStyle> {
    let style = view.styles.get(line)?;
    style.display().is_none_or(|text| text.width() <= width).then_some(style)
}

/// What a line paints as: its rendered form when it has one, else the source itself.
pub(super) fn shown<'a>(view: &'a View, line: usize, source: &'a str, width: usize) -> &'a str {
    style(view, line, width).and_then(LineStyle::display).unwrap_or(source)
}

/// A line's starting style: agent prose in the text color, a user turn stepped back so the
/// prompts read as separators between answers.
pub(super) fn base(tone: Tone, p: &Palette) -> Style {
    match tone {
        Tone::User => Style::new().fg(p.overlay0),
        Tone::Agent | Tone::Gap => Style::new().fg(p.text),
    }
}

/// A table border: chrome, so it carries the gutter but selects nothing.
pub(super) fn border(
    text: &str,
    style: Style,
    view: &View,
    line: usize,
    width: usize,
) -> Line<'static> {
    Line::from(vec![
        gutter::span(gutter::mark(view, line, false), &view.palette),
        Span::styled(text.to_owned(), style),
        Span::raw(padding(text, width)),
    ])
}

/// One display row, cut wherever its markdown styling or the selection changes. The padding
/// out to the measure joins the highlight only when the range carries on past this row.
pub(super) fn paint(
    row: &Row,
    line: usize,
    view: &View,
    base: Style,
    width: usize,
) -> Line<'static> {
    let end = row.start + row.text.chars().count();
    let style = style(view, line, width);
    let (start, stop) = selected(view.selection, line).unwrap_or((end, end));

    let mut spans = vec![gutter::span(gutter::mark(view, line, row.start == 0), &view.palette)];
    spans.extend(cuts(row, end, style, (start, stop)).windows(2).filter_map(|pair| {
        let &[from, to] = pair else { return None };
        let styled = base.patch(style.map_or_else(Style::new, |style| style.at(from)));
        let text = cut(&row.text, from - row.start, to - row.start).to_owned();
        Some(Span::styled(text, fill(styled, (start..stop).contains(&from), &view.palette)))
    }));

    // Only a range that runs on past this row keeps its highlight across the padding.
    let past = stop > end && start <= end;
    spans.push(Span::styled(padding(&row.text, width), fill(base, past, &view.palette)));
    Line::from(spans)
}

/// Character offsets this row has to be cut at: its own ends, the selection's, and every
/// point the markdown styling changes.
fn cuts(row: &Row, end: usize, style: Option<&LineStyle>, selection: (usize, usize)) -> Vec<usize> {
    let inside = row.start..end;
    let mut cuts = vec![row.start, end];

    cuts.extend([selection.0, selection.1].into_iter().filter(|col| inside.contains(col)));
    cuts.extend(style.into_iter().flat_map(|style| style.breaks(row.start, end)));
    cuts.sort_unstable();
    cuts.dedup();
    cuts
}

/// Lay a style over the selection fill, lifting a color that would vanish on it.
fn fill(style: Style, selected: bool, p: &Palette) -> Style {
    if !selected {
        return style;
    }

    let lifted = style.fg.map_or(p.subtext0, |fg| p.on_fill(fg));
    style.fg(lifted).bg(p.select_bg)
}

/// The columns of `line` a selection covers, unbounded at an end the range passes through.
fn selected(selection: Option<(Pos, Pos)>, line: usize) -> Option<(usize, usize)> {
    let (from, to) = selection.filter(|(from, to)| (from.line..=to.line).contains(&line))?;
    let start = if line == from.line { from.col } else { 0 };
    let end = if line == to.line { to.col } else { usize::MAX };

    Some((start, end))
}
