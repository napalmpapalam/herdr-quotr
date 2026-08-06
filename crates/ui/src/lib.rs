//! Rendering. Takes plain data, never the app state, so the binary owns its own types.

mod buffer;
mod card;
mod chrome;
mod column;
mod highlight;
mod markdown;
mod style;
mod table;
pub mod theme;
mod view;
mod wrap;

pub use crate::{
    card::rows as card_rows,
    column::{DEFAULT_MEASURE, MAX_MEASURE, MIN_MEASURE, content_column},
    markdown::analyze,
    style::LineStyle,
    theme::{NAMES as THEMES, Palette, Theme},
    view::{Banked, Hit, Markup, Painted, Scroll, SourceLine, View},
};

use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::Style,
    widgets::Block,
};

/// Paint the whole picker: framed buffer above a one-row footer, question box on top.
pub fn render(f: &mut Frame, view: &View) -> Painted {
    let [body, footer] =
        Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).areas(f.area());
    // The theme's own ground, under everything: the popup is full-screen, so it cannot borrow
    // the terminal's and stay legible across light and dark themes.
    f.render_widget(Block::new().style(Style::new().bg(view.palette.base)), f.area());
    let painted = buffer::render(f, body, view);
    chrome::render_footer(f, footer, view);
    if let Some(question) = view.question {
        chrome::render_question(f, body, view, &painted, question);
    }
    painted
}
