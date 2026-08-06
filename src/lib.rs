//! herdr-quotr — quote an agent's own response back at it.

pub mod app;
mod bank;
mod config;
mod nav;
mod send;
mod stash;

use std::io;

use anyhow::{Context, Result};
use ratatui::{
    DefaultTerminal,
    crossterm::{
        cursor::SetCursorStyle,
        event::{
            self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind,
            KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
        },
        execute,
    },
};
use ui::Painted;

use crate::app::{App, Grain};

/// Source lines a wheel notch moves the viewport. One, not the conventional three: the
/// picker is read at reading speed, and a three-line jump overshoots the line you came for.
const WHEEL: isize = 1;

/// Entry point: set up the terminal, run the loop, restore.
pub fn run() -> Result<()> {
    let mut app = App::new(herdr::agent_pane());
    let mut terminal = ratatui::init();
    // Mouse capture buys drag-to-select and costs the terminal's own text selection. The caret
    // is a reading position, not an insertion point, so it holds steady rather than blinking.
    let result = execute!(io::stdout(), EnableMouseCapture, SetCursorStyle::SteadyBlock)
        .context("enabling mouse capture")
        .and_then(|()| event_loop(&mut terminal, &mut app));
    let _ = execute!(io::stdout(), DisableMouseCapture, SetCursorStyle::DefaultUserShape);
    ratatui::restore();
    result
}

fn event_loop(terminal: &mut DefaultTerminal, app: &mut App) -> Result<()> {
    // One measuring frame first: where the picker opens depends on how many lines fit.
    app.painted = draw(terminal, app)?;
    app.settle();
    while !app.should_quit {
        app.painted = draw(terminal, app)?;
        match event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => handle_key(app, key),
            Event::Mouse(mouse) => handle_mouse(app, mouse),
            _ => {}
        }
    }
    Ok(())
}

fn draw(terminal: &mut DefaultTerminal, app: &App) -> Result<Painted> {
    let lines: Vec<ui::SourceLine<'_>> = app
        .transcript
        .lines()
        .iter()
        .map(|line| ui::SourceLine { text: &line.text, tone: line.tone })
        .collect();
    let banked: Vec<ui::Banked<'_>> = app
        .bank
        .iter()
        .enumerate()
        .map(|(index, pair)| ui::Banked {
            number: index + 1,
            from: pair.from.line,
            to: pair.to.line,
            question: pair.question.trim(),
            quote: app.transcript.slice(pair.from, pair.to).first().copied().unwrap_or_default(),
        })
        .collect();
    let view = ui::View {
        lines: &lines,
        styles: &app.styles,
        palette: app.theme.palette,
        measure: app.measure,
        turns: app.transcript.turn_starts(),
        cursor: app.cursor,
        selection: app.selection(),
        banked: &banked,
        scroll: if app.opening { ui::Scroll::Bottom } else { ui::Scroll::From(app.scroll) },
        question: app.asking().then_some(app.question.as_str()),
        status: &app.status,
    };
    let mut painted = Painted::default();
    terminal.draw(|f| painted = ui::render(f, &view))?;
    Ok(painted)
}

fn handle_key(app: &mut App, key: KeyEvent) {
    if app.asking() {
        handle_ask_key(app, key);
        return;
    }
    match key.code {
        KeyCode::Esc if app.selection().is_some() => app.clear_range(),
        KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true,
        KeyCode::Char('j') | KeyCode::Down => app.move_by_line(1),
        KeyCode::Char('k') | KeyCode::Up => app.move_by_line(-1),
        KeyCode::Char('l') | KeyCode::Right => app.move_by_char(1),
        KeyCode::Char('h') | KeyCode::Left => app.move_by_char(-1),
        KeyCode::Char('g') | KeyCode::Home => app.move_to_line(0),
        KeyCode::Char('G') | KeyCode::End => app.move_to_line(usize::MAX),
        KeyCode::PageDown => app.page_by(1),
        KeyCode::PageUp => app.page_by(-1),
        KeyCode::Char(']') => app.next_turn(),
        KeyCode::Char('[') => app.prev_turn(),
        KeyCode::Char('v') => app.toggle_range(Grain::Char),
        KeyCode::Char('V') => app.toggle_range(Grain::Line),
        KeyCode::Char('C' | 'c') => app.ask(),
        KeyCode::Char('E' | 'e') => app.edit_pair(),
        KeyCode::Char('D' | 'd') => app.delete_pair(),
        KeyCode::Char('S' | 's') => app.send(),
        _ => {}
    }
}

fn handle_ask_key(app: &mut App, key: KeyEvent) {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    match key.code {
        KeyCode::Esc => app.cancel_ask(),
        KeyCode::Enter => app.bank_pair(),
        KeyCode::Backspace => app.backspace(),
        KeyCode::Char('u') if ctrl => app.question.clear(),
        KeyCode::Char(c) if !ctrl => app.type_char(c),
        _ => {}
    }
}

fn handle_mouse(app: &mut App, mouse: MouseEvent) {
    if app.asking() {
        return;
    }
    let cell = app.painted.hit(mouse.column, mouse.row);
    match mouse.kind {
        MouseEventKind::ScrollDown => app.scroll_by(WHEEL),
        MouseEventKind::ScrollUp => app.scroll_by(-WHEEL),
        MouseEventKind::Down(MouseButton::Left) => app.press(cell),
        MouseEventKind::Drag(MouseButton::Left) => app.drag(cell),
        _ => {}
    }
}
