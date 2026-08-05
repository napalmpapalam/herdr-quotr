//! Picker state — terminal-free, so it stays testable. Actions live in [`nav`] and [`send`].
//!
//! [`nav`]: crate::nav
//! [`send`]: crate::send

use anyhow::{Context, Result};
use herdr::{AGENT_PANE_ENV, PaneId};
use transcript::{Pos, SessionId, Transcript};
use ui::Painted;

use crate::stash;

/// Whether the question box is up.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    #[default]
    Browse,
    Ask,
}

/// How much of a line a selection takes: exactly the characters crossed, or all of them.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Grain {
    #[default]
    Char,
    Line,
}

#[derive(Debug)]
pub struct App {
    /// Pane of the agent that opened us; `None` when launched standalone.
    pub agent_pane: Option<PaneId>,
    /// Session the buffer came from; a stash only restores into the same one.
    pub session: Option<SessionId>,
    pub transcript: Transcript,
    /// Character the caret sits on.
    pub cursor: Pos,
    /// First source line painted.
    pub scroll: usize,
    /// Where the live selection started; `None` when nothing is selected.
    pub anchor: Option<Pos>,
    pub grain: Grain,
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
        let line = transcript.last_answer().unwrap_or(0);
        let mut app = Self {
            agent_pane,
            session,
            transcript,
            cursor: Pos::line_start(line),
            scroll: line,
            anchor: None,
            grain: Grain::Char,
            mode: Mode::Browse,
            question: String::new(),
            painted: Painted::default(),
            status,
            should_quit: false,
        };
        app.restore();
        app
    }

    /// The range that would be quoted right now, in reading order.
    ///
    /// A linewise range covers whole lines however the caret and anchor sit inside them.
    pub fn range(&self) -> (Pos, Pos) {
        let anchor = self.anchor.unwrap_or(self.cursor);
        let (from, to) = (anchor.min(self.cursor), anchor.max(self.cursor));
        match self.grain {
            Grain::Char => (from, to),
            Grain::Line => (Pos::line_start(from.line), self.transcript.line_end(to.line)),
        }
    }

    pub fn selection(&self) -> Option<(Pos, Pos)> {
        self.anchor.map(|_| self.range())
    }

    /// Anchor a selection at the caret, or drop the one that is live.
    ///
    /// Asking for the other grain keeps the range and re-cuts it, rather than starting over.
    pub fn toggle_range(&mut self, grain: Grain) {
        if self.anchor.is_none() || self.grain == grain {
            self.anchor = self.anchor.is_none().then_some(self.cursor);
        }
        self.grain = grain;
    }

    pub fn clear_range(&mut self) {
        self.anchor = None;
        self.grain = Grain::Char;
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
        if self.transcript.clamp(pending.to) != pending.to {
            return; // the transcript no longer lines up; better to drop it than mis-quote
        }
        self.anchor = Some(pending.from);
        self.cursor = pending.to;
        self.scroll = pending.from.line;
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
