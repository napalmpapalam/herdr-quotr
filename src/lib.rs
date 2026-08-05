//! herdr-quotr — quote an agent's own response back at it.

pub mod app;
mod nav;
mod send;
mod stash;

use std::io;

use anyhow::{Context, Result};
use ratatui::{
    DefaultTerminal,
    crossterm::{
        event::{
            self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind,
            KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
        },
        execute,
    },
};
use transcript::LineKind;
use ui::{Painted, Tone};

use crate::app::{App, Mode};

/// Source lines a wheel notch moves the viewport — the conventional step.
const WHEEL: isize = 3;

/// Entry point: set up the terminal, run the loop, restore.
pub fn run() -> Result<()> {
    let mut app = App::new(herdr::agent_pane());
    let mut terminal = ratatui::init();
    // Mouse capture buys drag-to-select and costs the terminal's own text selection.
    let result = execute!(io::stdout(), EnableMouseCapture)
        .context("enabling mouse capture")
        .and_then(|()| event_loop(&mut terminal, &mut app));
    let _ = execute!(io::stdout(), DisableMouseCapture);
    ratatui::restore();
    result
}

fn event_loop(terminal: &mut DefaultTerminal, app: &mut App) -> Result<()> {
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
        .map(|line| ui::SourceLine { text: &line.text, tone: tone(line.kind) })
        .collect();
    let view = ui::View {
        lines: &lines,
        cursor: app.cursor,
        selection: app.selection(),
        scroll: app.scroll,
        question: (app.mode == Mode::Ask).then_some(app.question.as_str()),
        status: &app.status,
    };
    let mut painted = Painted::default();
    terminal.draw(|f| painted = ui::render(f, &view))?;
    Ok(painted)
}

fn tone(kind: LineKind) -> Tone {
    match kind {
        LineKind::Agent => Tone::Agent,
        LineKind::User => Tone::User,
        LineKind::Gap => Tone::Gap,
    }
}

fn handle_key(app: &mut App, key: KeyEvent) {
    if app.mode == Mode::Ask {
        handle_ask_key(app, key);
        return;
    }
    match key.code {
        KeyCode::Esc if app.selection().is_some() => app.clear_range(),
        KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true,
        KeyCode::Char('j') | KeyCode::Down => app.move_by(1),
        KeyCode::Char('k') | KeyCode::Up => app.move_by(-1),
        KeyCode::Char('g') | KeyCode::Home => app.move_to(0),
        KeyCode::Char('G') | KeyCode::End => app.move_to(usize::MAX),
        KeyCode::PageDown => app.page_by(1),
        KeyCode::PageUp => app.page_by(-1),
        KeyCode::Char(']') => app.next_turn(),
        KeyCode::Char('[') => app.prev_turn(),
        KeyCode::Char('V') => app.toggle_range(),
        KeyCode::Char('C' | 'c') => app.ask(),
        KeyCode::Char('S' | 's') => app.send(),
        _ => {}
    }
}

fn handle_ask_key(app: &mut App, key: KeyEvent) {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    match key.code {
        KeyCode::Esc => app.cancel_ask(),
        KeyCode::Enter => app.send(),
        KeyCode::Backspace => app.backspace(),
        KeyCode::Char('u') if ctrl => app.question.clear(),
        KeyCode::Char(c) if !ctrl => app.type_char(c),
        _ => {}
    }
}

fn handle_mouse(app: &mut App, mouse: MouseEvent) {
    if app.mode == Mode::Ask {
        return;
    }
    match mouse.kind {
        MouseEventKind::ScrollDown => app.scroll_by(WHEEL),
        MouseEventKind::ScrollUp => app.scroll_by(-WHEEL),
        MouseEventKind::Down(MouseButton::Left) => app.press(mouse.row),
        MouseEventKind::Drag(MouseButton::Left) => app.drag(mouse.row),
        _ => {}
    }
}
