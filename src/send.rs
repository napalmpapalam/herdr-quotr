//! Getting the composed quote into the agent's input — or parking it when it can't go.

use anyhow::anyhow;
use herdr::{AGENT_PANE_ENV, Status, focus, send_text};
use transcript::LineKind;

use crate::{app::App, stash};

/// Shown when a blocked agent forces the quote to wait for the next run.
const PARKED: &str = "agent is blocked — quote parked. q to close, answer the prompt, then reopen";

impl App {
    /// Insert the composed block into the agent's input and hand focus back.
    ///
    /// A focus failure is non-fatal: the text landed, which is the part that matters.
    pub fn send(&mut self) {
        let Some(pane) = self.agent_pane.clone() else {
            self.status = format!("no {AGENT_PANE_ENV} — nowhere to send");
            return;
        };
        let quoted = self.quoted_lines();
        if quoted.is_empty() {
            "nothing selected".clone_into(&mut self.status);
            return;
        }
        // A permission prompt eats the text while `send_text` still reports success, so
        // sending now would drop the selection and the question without a trace. Only a
        // confirmed `Blocked` stops us — if the status can't be read, the send is the
        // better bet.
        if herdr::agent_status(&pane).is_ok_and(|status| status == Status::Blocked) {
            self.park();
            return;
        }
        let question = self.question.trim();
        let block = export::block(&quoted, (!question.is_empty()).then_some(question));
        if let Err(e) = send_text(&pane, &block) {
            self.status = format!("send failed: {e}");
            return;
        }
        let _ = focus(&pane);
        self.should_quit = true;
    }

    /// Park the selection for the next run, since the popup covers the prompt the user has
    /// to answer before anything can be sent.
    ///
    /// Deliberately does not quit: the picker has to stay up long enough to be read, and a
    /// park that failed to write must not close over the user's work.
    fn park(&mut self) {
        let (from, to) = self.range();
        let saved = self.session.as_ref().map_or_else(
            || Err(anyhow!("no session to park under")),
            |session| {
                stash::save(&stash::Pending {
                    session: session.as_str().to_owned(),
                    from,
                    to,
                    question: self.question.clone(),
                })
            },
        );
        self.status = saved.map_or_else(
            |e| format!("agent is blocked and the quote could not be parked: {e:#}"),
            |()| PARKED.to_owned(),
        );
    }

    /// Selected lines, minus the blank rows that only separate turns.
    fn quoted_lines(&self) -> Vec<&str> {
        let (from, to) = self.range();
        self.transcript
            .lines()
            .get(from..=to)
            .unwrap_or_default()
            .iter()
            .filter(|line| line.kind != LineKind::Gap)
            .map(|line| line.text.as_str())
            .collect()
    }
}
