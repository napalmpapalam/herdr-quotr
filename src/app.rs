//! Picker state and the actions the keys drive — terminal-free, so it stays unit-testable.

use herdr::{AGENT_PANE_ENV, PaneId, focus, send_text};

/// The block slice 1 sends, standing in for a real selection + question. It is the exact
/// hand-written shape quotr must reproduce: quote, blank line, question.
const HARDCODED_BLOCK: &str = "> quoted line one\n> quoted line two\n\nthe question";

#[derive(Debug)]
pub struct App {
    /// Pane of the agent that opened us; `None` when launched standalone.
    pub agent_pane: Option<PaneId>,
    pub status: String,
    pub should_quit: bool,
}

impl App {
    pub fn new(agent_pane: Option<PaneId>) -> Self {
        let status = agent_pane.as_ref().map_or_else(
            || format!("no {AGENT_PANE_ENV} set — S will do nothing"),
            |pane| format!("agent pane {pane}"),
        );
        Self { agent_pane, status, should_quit: false }
    }

    /// Insert the block into the agent's input and hand focus back.
    ///
    /// A focus failure is non-fatal: the text landed, which is the part that matters.
    pub fn send(&mut self) {
        let Some(pane) = self.agent_pane.clone() else {
            self.status = format!("no {AGENT_PANE_ENV} — nowhere to send");
            return;
        };
        if let Err(e) = send_text(&pane, HARDCODED_BLOCK) {
            self.status = format!("send failed: {e}");
            return;
        }
        let _ = focus(&pane);
        self.should_quit = true;
    }
}
