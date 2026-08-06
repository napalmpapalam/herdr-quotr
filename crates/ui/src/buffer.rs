//! The scrolled transcript, painted into the reading column.

mod gutter;
mod row;

use ratatui::{Frame as TerminalFrame, layout::Rect, text::Line, widgets::Paragraph};

use crate::{
    card,
    column::{content_column, text_area},
    style::LineStyle,
    theme::Palette,
    view::{Banked, Painted, PaintedRow, Scroll, SourceLine, View},
    wrap::wrap,
};

/// Blank rows left under the last line, so it never sits flush against the footer.
const TAIL: usize = 2;

/// The reading area. No border of its own — herdr's popup already frames the pane.
pub(crate) fn render(f: &mut TerminalFrame, area: Rect, view: &View) -> Painted {
    let column = content_column(area, view.measure);
    let text = text_area(column);
    let height = usize::from(column.height);
    let width = usize::from(text.width);

    let top = match view.scroll {
        Scroll::From(line) => line,
        Scroll::Bottom => bottom_start(view, width, height.saturating_sub(TAIL).max(1)),
    };
    let mut frame = Frame { drawn: Vec::with_capacity(height), painted: Vec::new(), lines: 0 };

    for (index, source) in view.lines.iter().enumerate().skip(top) {
        if !frame.push_line(view, index, source, width, height) {
            break;
        }
    }

    frame.drawn.truncate(height);
    frame.painted.truncate(height);
    f.render_widget(Paragraph::new(frame.drawn), column);

    let painted = Painted::new(frame.painted, frame.lines, top, text);
    if let Some(caret) = painted.caret(view.cursor) {
        f.set_cursor_position(caret);
    }
    painted
}

/// The rows built so far, and what each one came from.
struct Frame {
    drawn: Vec<Line<'static>>,
    painted: Vec<Option<PaintedRow>>,
    /// Source lines that fit whole.
    lines: usize,
}

impl Frame {
    /// Add one source line's rows. Returns false when the frame is full.
    fn push_line(
        &mut self,
        view: &View,
        index: usize,
        source: &SourceLine<'_>,
        width: usize,
        height: usize,
    ) -> bool {
        let wrapped = wrap(row::shown(view, index, source.text, width), width);
        let cards = cards_under(view.banked, index, width, &view.palette);
        let style = row::style(view, index, width);
        let chrome = chrome_rows(style);

        // A half-fitting line waits for the next scroll — except the first, which must show.
        if !self.drawn.is_empty()
            && self.drawn.len() + wrapped.len() + cards.len() + chrome > height
        {
            return false;
        }

        if let Some((text, border)) = style.and_then(LineStyle::above_row) {
            self.chrome(row::border(text, border, view, index, width));
        }

        let base = row::base(source.tone, &view.palette);
        let linewise = style.is_some_and(LineStyle::is_linewise);
        for wrapped_row in wrapped {
            self.drawn.push(row::paint(&wrapped_row, index, view, base, width));
            self.painted.push(Some(PaintedRow {
                line: index,
                start: wrapped_row.start,
                text: wrapped_row.text,
                linewise,
            }));
        }

        if let Some((text, border)) = style.and_then(LineStyle::below_row) {
            self.chrome(row::border(text, border, view, index, width));
        }
        for mut card in cards {
            card.spans.insert(0, gutter::span(gutter::mark(view, index, false), &view.palette));
            self.chrome(card);
        }

        self.lines += 1;
        self.drawn.len() < height
    }

    /// A row that takes height but carries nothing to select — a border or a card.
    fn chrome(&mut self, line: Line<'static>) {
        self.drawn.push(line);
        self.painted.push(None);
    }
}

/// The topmost source line that still leaves the last one [`TAIL`] rows off the bottom. A line
/// that would only half fit is left above the viewport rather than clipped.
fn bottom_start(view: &View, width: usize, height: usize) -> usize {
    let mut rows = 0;

    for (index, source) in view.lines.iter().enumerate().rev() {
        rows += wrap(row::shown(view, index, source.text, width), width).len()
            + cards_under(view.banked, index, width, &view.palette).len()
            + chrome_rows(row::style(view, index, width));
        if rows >= height {
            return index + usize::from(rows > height);
        }
    }

    0
}

/// Border rows a rendered table hangs above and below a line.
fn chrome_rows(style: Option<&LineStyle>) -> usize {
    usize::from(style.is_some_and(|s| s.above_row().is_some()))
        + usize::from(style.is_some_and(|s| s.below_row().is_some()))
}

/// Card rows for every pair whose range ends on `line` — reviewr's inline comment card.
fn cards_under(banked: &[Banked], line: usize, width: usize, p: &Palette) -> Vec<Line<'static>> {
    banked
        .iter()
        .filter(|pair| pair.to == line)
        .flat_map(|pair| card::lines(pair.number, pair.question, pair.quote, width, p))
        .collect()
}
