//! herdr-quotr — quote an agent's own response back at it.
//!
//! A short-lived picker: herdr opens it in a popup over the agent pane, it composes a
//! markdown quote block, writes it into the agent's input without submitting, and exits.
//!
//! The binary owns the picker: state ([`app`]), terminal lifecycle, and the event loop.
//! The reusable layers are crates beside it — `ui` (paint) and `herdr` (host).

pub mod app;

use anyhow::Result;
use ratatui::{
    DefaultTerminal,
    crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
};

use crate::app::App;

/// Entry point: set up the terminal, run the loop, restore.
pub fn run() -> Result<()> {
    let mut app = App::new(herdr::agent_pane());
    let mut terminal = ratatui::init();
    let result = event_loop(&mut terminal, &mut app);
    ratatui::restore();
    result
}

fn event_loop(terminal: &mut DefaultTerminal, app: &mut App) -> Result<()> {
    while !app.should_quit {
        terminal.draw(|f| ui::render(f, &app.status))?;
        if let Event::Key(k) = event::read()?
            && k.kind == KeyEventKind::Press
        {
            handle_key(app, k);
        }
    }
    Ok(())
}

fn handle_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true,
        KeyCode::Char('S' | 's') => app.send(),
        _ => {}
    }
}
