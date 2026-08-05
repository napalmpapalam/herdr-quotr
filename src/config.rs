//! The plugin config file — `config.toml` under `$HERDR_PLUGIN_CONFIG_DIR`.

use std::{
    env, fs,
    io::ErrorKind,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, anyhow};
use ui::Theme;

/// Where herdr tells a plugin its config lives. The picker only ever runs from herdr, which
/// always sets this, so there is no `plugin config-dir` fallback to shell out to.
const CONFIG_DIR_ENV: &str = "HERDR_PLUGIN_CONFIG_DIR";

/// The keys quotr reads. Unknown keys are ignored, so a config written for a later version
/// still applies.
#[derive(Debug, Default, serde::Deserialize)]
struct ConfigFile {
    theme: Option<String>,
}

/// The theme named in the config file, or the default.
///
/// A missing directory, a missing file, and an omitted key all mean "use the default". A file
/// that fails to read or parse, or that names a theme that does not exist, is an error —
/// silently painting the wrong colors would be worse.
pub(crate) fn theme() -> Result<Theme> {
    let Some(name) = read()?.theme else {
        return Ok(ui::theme::default_theme());
    };

    ui::theme::resolve(&name)
        .ok_or_else(|| anyhow!("unknown theme {name:?}. known: {}", ui::THEMES.join(", ")))
}

/// The parsed config file, or its defaults when there is nothing to read.
fn read() -> Result<ConfigFile> {
    let Some(path) = path() else {
        return Ok(ConfigFile::default());
    };

    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(ConfigFile::default()),
        Err(e) => return Err(e).with_context(|| format!("reading {}", path.display())),
    };

    toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))
}

fn path() -> Option<PathBuf> {
    env::var_os(CONFIG_DIR_ENV).map(|dir| Path::new(&dir).join("config.toml"))
}

#[cfg(test)]
mod tests {
    use super::ConfigFile;

    fn parse(text: &str) -> ConfigFile {
        toml::from_str(text).unwrap_or_default()
    }

    #[test]
    fn an_empty_file_leaves_every_key_unset() {
        assert!(parse("").theme.is_none());
    }

    #[test]
    fn an_unknown_key_is_ignored() {
        assert_eq!(parse("theme = \"nord\"\nfuture_key = 3").theme.as_deref(), Some("nord"));
    }
}
