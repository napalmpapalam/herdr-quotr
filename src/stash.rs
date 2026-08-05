//! Hand a composed batch to the next run, for when the agent can't take it now.

use std::{env, fs, path::PathBuf};

use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use transcript::SessionId;

use crate::bank::Pair;

/// Where herdr lets a plugin keep state between runs.
const STATE_DIR_ENV: &str = "HERDR_PLUGIN_STATE_DIR";
const FILE: &str = "pending.json";

/// A whole batch, parked until the agent can be written to.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Pending {
    /// Only restored into the session it was taken from.
    pub(crate) session: String,
    pub(crate) pairs: Vec<Pair>,
}

pub(crate) fn save(pending: &Pending) -> Result<()> {
    let path = path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    }
    let json = serde_json::to_string(pending).context("serializing the pending batch")?;
    fs::write(&path, json).with_context(|| format!("writing {}", path.display()))
}

/// Take the pending batch for `session`, removing it — a one-shot handoff, so a stash the
/// user abandons can't come back a third time.
pub(crate) fn take(session: &SessionId) -> Option<Pending> {
    let path = path().ok()?;
    let text = fs::read_to_string(&path).ok()?;
    let _ = fs::remove_file(&path);
    let pending: Pending = serde_json::from_str(&text).ok()?;
    (pending.session == session.as_str()).then_some(pending)
}

fn path() -> Result<PathBuf> {
    let dir = env::var_os(STATE_DIR_ENV).ok_or_else(|| anyhow!("{STATE_DIR_ENV} is not set"))?;
    Ok(PathBuf::from(dir).join(FILE))
}
