//! Finding the transcript file for a session.

use std::{
    env, fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, anyhow};

use crate::SessionId;

/// The transcript for `session`. Scanned, not computed: the dir name is slugified from the
/// session's *original* cwd, which can differ from the pane's.
pub fn find(session: &SessionId) -> Result<PathBuf> {
    let root = projects_dir()?;
    let name = format!("{session}.jsonl");
    fs::read_dir(&root)
        .with_context(|| format!("reading {}", root.display()))?
        .filter_map(Result::ok)
        .map(|dir| dir.path().join(&name))
        .find(|path| path.is_file())
        .ok_or_else(|| anyhow!("no transcript for session {session} under {}", root.display()))
}

fn projects_dir() -> Result<PathBuf> {
    let home = env::var_os("HOME").ok_or_else(|| anyhow!("HOME is not set"))?;
    Ok(Path::new(&home).join(".claude").join("projects"))
}
