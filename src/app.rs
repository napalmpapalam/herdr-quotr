//! Picker state — terminal-free, so it stays testable. Actions live in [`nav`] and [`send`].
//!
//! [`nav`]: crate::nav
//! [`send`]: crate::send

use anyhow::{Context, Result};
use herdr::{AGENT_PANE_ENV, PaneId};
use transcript::{SessionId, Transcript};
use ui::Painted;

use crate::stash;

/// Whether the question box is up.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    #[default]
    Browse,
    Ask,
}

#[derive(Debug)]
pub struct App {
    /// Pane of the agent that opened us; `None` when launched standalone.
    pub agent_pane: Option<PaneId>,
    /// Session the buffer came from; a stash only restores into the same one.
    pub session: Option<SessionId>,
    pub transcript: Transcript,
    /// Source line the cursor is on.
    pub cursor: usize,
    /// First source line painted.
    pub scroll: usize,
    /// Where the live selection started; `None` when nothing is selected.
    pub anchor: Option<usize>,
    pub mode: Mode,
    pub question: String,
    /// What the last frame put on screen — drives scrolling and mouse hit testing.
    pub painted: Painted,
    pub status: String,
    pub should_quit: bool,
}

impl App {
    pub fn new(agent_pane: Option<PaneId>) -> Self {
        let (session, transcript, status) = match load(agent_pane.as_ref()) {
            Ok((session, t)) => {
                let status = format!("{} lines", t.lines().len());
                (Some(session), t, status)
            }
            Err(e) => (None, Transcript::default(), format!("{e:#}")),
        };
        let cursor = transcript.last_answer().unwrap_or(0);
        let mut app = Self {
            agent_pane,
            session,
            transcript,
            cursor,
            scroll: cursor,
            anchor: None,
            mode: Mode::Browse,
            question: String::new(),
            painted: Painted::default(),
            status,
            should_quit: false,
        };
        app.restore();
        app
    }

    /// Inclusive source-line range that would be quoted right now.
    pub fn range(&self) -> (usize, usize) {
        let anchor = self.anchor.unwrap_or(self.cursor);
        (anchor.min(self.cursor), anchor.max(self.cursor))
    }

    pub fn selection(&self) -> Option<(usize, usize)> {
        self.anchor.map(|_| self.range())
    }

    pub fn ask(&mut self) {
        self.mode = Mode::Ask;
        self.question.clear();
    }

    pub fn cancel_ask(&mut self) {
        self.mode = Mode::Browse;
    }

    pub fn type_char(&mut self, c: char) {
        self.question.push(c);
    }

    pub fn backspace(&mut self) {
        self.question.pop();
    }

    /// Pick up a selection a previous run parked because the agent was blocked.
    fn restore(&mut self) {
        let Some(pending) = self.session.as_ref().and_then(stash::take) else { return };
        if pending.to >= self.transcript.lines().len() {
            return; // the transcript no longer lines up; better to drop it than mis-quote
        }
        self.anchor = Some(pending.from);
        self.cursor = pending.to;
        self.scroll = pending.from;
        self.question = pending.question;
        self.mode = Mode::Ask;
        "restored your parked quote — enter sends it".clone_into(&mut self.status);
    }
}

fn load(pane: Option<&PaneId>) -> Result<(SessionId, Transcript)> {
    let pane = pane.with_context(|| format!("{AGENT_PANE_ENV} not set"))?;
    let session = herdr::agent_session(pane)?;
    let transcript = Transcript::load(&transcript::find(&session)?)?;
    Ok((session, transcript))
}
