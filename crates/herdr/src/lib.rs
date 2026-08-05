//! herdr host integration: write into the origin agent's input and hand focus back.
//!
//! The origin pane id arrives as `QUOTR_AGENT_PANE`, set by `herdr/open.sh` when it
//! opens the picker — so nothing here has to resolve which agent we came from.

use std::{env, fmt, process::Command};

use anyhow::{Context, Result, bail};

/// Env var carrying the pane id of the agent that opened the picker.
pub const AGENT_PANE_ENV: &str = "QUOTR_AGENT_PANE";

/// A herdr pane id, e.g. `w2N:p8`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaneId(String);

impl PaneId {
    fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PaneId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

fn herdr(args: &[&str]) -> Result<String> {
    let bin = env::var("HERDR_BIN_PATH").unwrap_or_else(|_| "herdr".to_string());
    let out =
        Command::new(bin).args(args).output().with_context(|| format!("running herdr {args:?}"))?;
    if !out.status.success() {
        bail!("herdr {args:?} failed: {}", String::from_utf8_lossy(&out.stderr).trim());
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// The origin agent pane, or `None` when the picker was launched outside the action.
pub fn agent_pane() -> Option<PaneId> {
    env::var(AGENT_PANE_ENV).ok().filter(|p| !p.is_empty()).map(PaneId)
}

/// Write literal text into the agent pane's input, without submitting.
///
/// `pane send-text` honors the pane's live bracketed-paste mode, which is what keeps a
/// multi-line block from executing at the first newline. Claude Code enables it.
pub fn send_text(pane: &PaneId, text: &str) -> Result<()> {
    herdr(&["pane", "send-text", pane.as_str(), text])?;
    Ok(())
}

/// Focus the agent pane so the user can edit and submit.
pub fn focus(pane: &PaneId) -> Result<()> {
    herdr(&["agent", "focus", pane.as_str()])?;
    Ok(())
}
