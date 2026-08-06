//! Picker state — terminal-free, so it stays testable. Actions live in `nav` and `send`.

use std::{env, path::Path};

use anyhow::{Context, Result};
use herdr::{AGENT_PANE_ENV, PaneId};
use markup::Pos;
use transcript::{SessionId, Transcript};
use ui::{LineStyle, Painted, Theme};

use crate::{bank::Bank, config, stash};

/// Read a transcript straight from a file instead of resolving one through herdr — how the
/// picker is driven outside a pane, for the demo recording and for working on the UI.
const TRANSCRIPT_ENV: &str = "QUOTR_TRANSCRIPT";

/// The session a [`TRANSCRIPT_ENV`] run reports. A stash is session-scoped, so a demo run
/// can never restore into a real session's picker.
const DEMO_SESSION: &str = "00000000-0000-0000-0000-000000000000";

/// Whether the question box is up.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    #[default]
    Browse,
    /// Typing a question; `editing` is the bank index when reopening a banked pair.
    Ask { editing: Option<usize> },
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
    /// Colors this run paints in, from the config file.
    pub theme: Theme,
    /// Reading measure in columns, from the config file.
    pub measure: u16,
    /// Markdown styling for each transcript line. Built once — `syntect` is far too slow to
    /// run per frame, and the transcript does not change under a run.
    pub styles: Vec<LineStyle>,
    /// Character the caret sits on.
    pub cursor: Pos,
    /// First source line painted. Meaningless until [`App::settle`] runs.
    pub scroll: usize,
    /// True until the first frame has resolved where the picker opens.
    pub opening: bool,
    /// Where the live selection started; `None` when nothing is selected.
    pub anchor: Option<Pos>,
    pub grain: Grain,
    /// Pairs finished and waiting for `S`.
    pub bank: Bank,
    pub mode: Mode,
    pub question: String,
    /// What the last frame put on screen — drives scrolling and mouse hit testing.
    pub painted: Painted,
    pub status: String,
    pub should_quit: bool,
}

impl App {
    pub fn new(agent_pane: Option<PaneId>) -> Self {
        let (session, transcript, mut status) = match load(agent_pane.as_ref()) {
            Ok((session, t)) => {
                let status = format!("{} lines", t.lines().len());
                (Some(session), t, status)
            }
            Err(e) => (None, Transcript::default(), format!("{e:#}")),
        };
        let config = config::load().unwrap_or_else(|e| {
            status = format!("{e:#}");
            config::Config::default()
        });
        let styles = analyze(&transcript, &config.theme, config.measure);
        let last = transcript.lines().len().saturating_sub(1);
        let mut app = Self {
            agent_pane,
            session,
            transcript,
            theme: config.theme,
            measure: config.measure,
            styles,
            cursor: Pos::line_start(last),
            scroll: 0,
            opening: true,
            anchor: None,
            grain: Grain::Char,
            bank: Bank::default(),
            mode: Mode::Browse,
            question: String::new(),
            painted: Painted::default(),
            status,
            should_quit: false,
        };
        app.restore();
        app
    }

    /// Adopt the scroll the first frame worked out. Where the newest line sits depends on how
    /// every line above it wraps, so only the paint layer can say.
    pub fn settle(&mut self) {
        if !self.opening {
            return;
        }
        self.opening = false;
        self.scroll = self.painted.top();
    }

    /// The range that would be quoted right now, in reading order.
    ///
    /// A linewise range covers whole lines however the caret and anchor sit inside them. A
    /// range touching a rendered table is always linewise, whichever key or click made it:
    /// the grid pads its cells, so a column there names no source character.
    pub fn range(&self) -> (Pos, Pos) {
        let anchor = self.anchor.unwrap_or(self.cursor);
        let (from, to) = (anchor.min(self.cursor), anchor.max(self.cursor));
        if self.grain == Grain::Char && !self.spans_rendered(from.line, to.line) {
            return (from, to);
        }
        (Pos::line_start(from.line), self.transcript.line_end(to.line))
    }

    /// Whether either end of a range sits on a line the paint layer rendered rather than
    /// styled.
    fn spans_rendered(&self, from: usize, to: usize) -> bool {
        [from, to].iter().any(|line| self.styles.get(*line).is_some_and(LineStyle::is_linewise))
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

    pub fn asking(&self) -> bool {
        matches!(self.mode, Mode::Ask { .. })
    }

    /// Open the question box over the live selection. Without one there is nothing to quote —
    /// `e` is what reopens a banked pair.
    pub fn ask(&mut self) {
        let (from, to) = self.range();
        if self.anchor.is_none() || self.transcript.slice(from, to).is_empty() {
            "nothing selected".clone_into(&mut self.status);
            return;
        }
        self.mode = Mode::Ask { editing: None };
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

    /// Pick up a batch a previous run parked because the agent was blocked.
    fn restore(&mut self) {
        let Some(pending) = self.session.as_ref().and_then(stash::take) else { return };
        // Any pair that no longer lines up drops the whole batch: mis-quoting one of them is
        // worse than restoring none, and the user still has the transcript in front of them.
        if pending.pairs.is_empty()
            || pending.pairs.iter().any(|pair| self.transcript.clamp(pair.to) != pair.to)
        {
            return;
        }
        let count = pending.pairs.len();
        if let Some(first) = pending.pairs.first() {
            self.cursor = first.from;
            self.scroll = first.from.line;
            self.opening = false; // the batch's own position beats opening at the newest line
        }
        self.bank = Bank::from(pending.pairs);
        self.status = format!("restored {count} parked pair(s) — S sends");
    }
}

fn load(pane: Option<&PaneId>) -> Result<(SessionId, Transcript)> {
    if let Some(path) = env::var_os(TRANSCRIPT_ENV) {
        let session = DEMO_SESSION.parse().context("the demo session id")?;
        return Ok((session, Transcript::load(Path::new(&path))?));
    }
    let pane = pane.with_context(|| format!("{AGENT_PANE_ENV} not set"))?;
    let session = herdr::agent_session(pane)?;
    let transcript = Transcript::load(&transcript::find(&session)?)?;
    Ok((session, transcript))
}

/// Color the whole transcript once, in the shape the paint layer takes.
fn analyze(transcript: &Transcript, theme: &Theme, measure: u16) -> Vec<LineStyle> {
    let markup: Vec<ui::Markup<'_>> = transcript
        .lines()
        .iter()
        .map(|line| ui::Markup {
            text: &line.text,
            tone: line.tone,
            block: line.block,
            spans: &line.spans,
        })
        .collect();

    ui::analyze(&markup, theme, measure)
}
