//! `herdr agent list` — what the origin pane's agent is and what it is doing.

use anyhow::{Context, Result, anyhow};
use serde::Deserialize;
use transcript::SessionId;

use crate::{PaneId, herdr};

/// What an agent is doing. `Blocked` means a permission prompt is up, so the input box is
/// not accepting text.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Idle,
    Working,
    Blocked,
    Done,
    #[default]
    #[serde(other)]
    Unknown,
}

/// The Claude session id of the agent running in `pane`.
pub fn session(pane: &PaneId) -> Result<SessionId> {
    find(pane)?
        .agent_session
        .ok_or_else(|| anyhow!("agent in pane {pane} has no session — quotr needs Claude"))?
        .value
        .parse()
}

/// What the agent in `pane` is doing right now.
pub fn status(pane: &PaneId) -> Result<Status> {
    Ok(find(pane)?.agent_status)
}

fn find(pane: &PaneId) -> Result<Entry> {
    let output = herdr(&["agent", "list"])?;
    let list: List = serde_json::from_str(&output).context("parsing herdr agent list output")?;
    list.result
        .agents
        .into_iter()
        .find(|agent| agent.pane_id == pane.as_str())
        .ok_or_else(|| anyhow!("no agent in pane {pane}"))
}

#[derive(Debug, Deserialize)]
struct List {
    result: ListResult,
}

#[derive(Debug, Deserialize)]
struct ListResult {
    agents: Vec<Entry>,
}

#[derive(Debug, Deserialize)]
struct Entry {
    pane_id: String,
    /// Absent for agent kinds that carry no session, which quotr cannot read.
    #[serde(default)]
    agent_session: Option<Session>,
    #[serde(default)]
    agent_status: Status,
}

#[derive(Debug, Deserialize)]
struct Session {
    value: String,
}

#[cfg(test)]
mod tests {
    use super::{Entry, Status};

    fn status_of(json: &str) -> Option<Status> {
        serde_json::from_str::<Entry>(json).ok().map(|entry| entry.agent_status)
    }

    #[test]
    fn reads_the_status_and_tolerates_a_state_it_does_not_know() {
        let entry = |status| format!(r#"{{"pane_id":"w1:p1","agent_status":"{status}"}}"#);
        assert_eq!(status_of(&entry("blocked")), Some(Status::Blocked));
        assert_eq!(status_of(&entry("idle")), Some(Status::Idle));
        assert_eq!(status_of(&entry("something-new")), Some(Status::Unknown));
    }

    #[test]
    fn a_missing_status_is_unknown() {
        assert_eq!(status_of(r#"{"pane_id":"w1:p1"}"#), Some(Status::Unknown));
    }
}
