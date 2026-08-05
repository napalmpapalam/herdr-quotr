//! Getting the composed batch into the agent's input — or parking it when it can't go.

use anyhow::anyhow;
use herdr::{AGENT_PANE_ENV, Status, focus, send_text};

use crate::{app::App, stash};

/// Shown when a blocked agent forces the batch to wait for the next run.
const PARKED: &str = "agent is blocked — batch parked. q to close, answer the prompt, then reopen";

impl App {
    /// Insert the whole bank into the agent's input and hand focus back.
    ///
    /// A live selection is banked first, so `S` sends what is on screen without the question
    /// box. A focus failure is non-fatal: the text landed, which is the part that matters.
    pub fn send(&mut self) {
        let Some(pane) = self.agent_pane.clone() else {
            self.status = format!("no {AGENT_PANE_ENV} — nowhere to send");
            return;
        };
        self.bank_selection();
        if self.bank.is_empty() {
            "nothing to send".clone_into(&mut self.status);
            return;
        }
        // A permission prompt eats the text while `send_text` still reports success, so
        // sending now would drop the whole batch without a trace. Only a confirmed `Blocked`
        // stops us — if the status can't be read, the send is the better bet.
        if herdr::agent_status(&pane).is_ok_and(|status| status == Status::Blocked) {
            self.park();
            return;
        }
        if let Err(e) = send_text(&pane, &self.compose()) {
            self.status = format!("send failed: {e}");
            return;
        }
        let _ = focus(&pane);
        self.should_quit = true;
    }

    /// Park the batch for the next run, since the popup covers the prompt the user has to
    /// answer before anything can be sent.
    ///
    /// Deliberately does not quit: the picker has to stay up long enough to be read, and a
    /// park that failed to write must not close over the user's work.
    fn park(&mut self) {
        let pairs: Vec<_> = self.bank.iter().cloned().collect();
        let saved = self.session.as_ref().map_or_else(
            || Err(anyhow!("no session to park under")),
            |session| stash::save(&stash::Pending { session: session.as_str().to_owned(), pairs }),
        );
        self.status = saved.map_or_else(
            |e| format!("agent is blocked and the batch could not be parked: {e:#}"),
            |()| PARKED.to_owned(),
        );
    }
}
