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
    measure: Option<u16>,
}

/// What the config file settles for a run.
#[derive(Debug)]
pub(crate) struct Config {
    pub(crate) theme: Theme,
    /// Reading measure, in columns.
    pub(crate) measure: u16,
}

impl Default for Config {
    fn default() -> Self {
        Self { theme: ui::theme::default_theme(), measure: ui::DEFAULT_MEASURE }
    }
}

/// The config file's settings, or the defaults.
///
/// A missing directory, a missing file, and an omitted key all mean "use the default". A file
/// that fails to read or parse, or that carries a value quotr cannot honor, is an error —
/// silently painting the wrong thing would be worse.
pub(crate) fn load() -> Result<Config> {
    let file = read()?;
    Ok(Config { theme: theme(file.theme.as_deref())?, measure: measure(file.measure)? })
}

fn theme(name: Option<&str>) -> Result<Theme> {
    let Some(name) = name else {
        return Ok(ui::theme::default_theme());
    };

    ui::theme::resolve(name)
        .ok_or_else(|| anyhow!("unknown theme {name:?}. known: {}", ui::THEMES.join(", ")))
}

fn measure(columns: Option<u16>) -> Result<u16> {
    let Some(columns) = columns else {
        return Ok(ui::DEFAULT_MEASURE);
    };

    if (ui::MIN_MEASURE..=ui::MAX_MEASURE).contains(&columns) {
        return Ok(columns);
    }
    Err(anyhow!("measure {columns} is out of range ({}-{})", ui::MIN_MEASURE, ui::MAX_MEASURE))
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
    use super::{ConfigFile, measure};

    fn parse(text: &str) -> ConfigFile {
        toml::from_str(text).unwrap_or_default()
    }

    #[test]
    fn an_empty_file_leaves_every_key_unset() {
        let file = parse("");
        assert!(file.theme.is_none());
        assert!(file.measure.is_none());
    }

    #[test]
    fn an_unknown_key_is_ignored() {
        assert_eq!(parse("theme = \"nord\"\nfuture_key = 3").theme.as_deref(), Some("nord"));
    }

    #[test]
    fn an_omitted_measure_is_the_default() {
        assert_eq!(measure(None).ok(), Some(ui::DEFAULT_MEASURE));
    }

    #[test]
    fn a_measure_outside_the_range_is_an_error() {
        assert_eq!(measure(Some(ui::MIN_MEASURE)).ok(), Some(ui::MIN_MEASURE));
        assert_eq!(measure(Some(ui::MAX_MEASURE)).ok(), Some(ui::MAX_MEASURE));
        assert!(measure(Some(ui::MIN_MEASURE - 1)).is_err());
        assert!(measure(Some(ui::MAX_MEASURE + 1)).is_err());
    }
}
