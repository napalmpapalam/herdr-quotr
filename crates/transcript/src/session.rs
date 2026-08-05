//! The Claude Code session uuid.

use std::{fmt, str::FromStr};

use anyhow::{Result, bail};

/// The five hex groups of a uuid: `673dfc91-ee56-4884-a4ab-01fc543dabc8`.
const GROUP_LENGTHS: [usize; 5] = [8, 4, 4, 4, 12];

/// A Claude Code session uuid, as reported by `herdr agent list`.
///
/// Parsed, not merely wrapped: the value is joined into a filesystem path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionId(String);

impl SessionId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for SessionId {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        if !is_uuid(value) {
            bail!("not a session uuid: {value:?}");
        }
        Ok(Self(value.to_owned()))
    }
}

impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

fn is_uuid(value: &str) -> bool {
    let mut groups = value.split('-');
    let shaped = GROUP_LENGTHS
        .iter()
        .all(|&length| groups.next().is_some_and(|group| group.len() == length && is_hex(group)));
    shaped && groups.next().is_none()
}

fn is_hex(group: &str) -> bool {
    group.chars().all(|c| c.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::SessionId;

    fn parses(value: &str) -> bool {
        value.parse::<SessionId>().is_ok()
    }

    #[test]
    fn accepts_a_uuid() {
        assert!(parses("673dfc91-ee56-4884-a4ab-01fc543dabc8"));
    }

    #[test]
    fn rejects_anything_that_could_escape_the_transcript_dir() {
        assert!(!parses("../../../etc/passwd"));
        assert!(!parses("673dfc91-ee56-4884-a4ab-01fc543dabc8/../evil"));
    }

    #[test]
    fn rejects_wrong_grouping_and_non_hex() {
        assert!(!parses("673dfc91ee564884a4ab01fc543dabc8"), "no dashes");
        assert!(!parses("673dfc91-ee56-4884-a4ab-01fc543dabc8-extra"), "sixth group");
        assert!(!parses("673dfc91-ee56-4884-a4ab-01fc543dabcz"), "z is not hex");
        assert!(!parses("673dfc9-1ee56-4884-a4ab-01fc543dabc8"), "dash misplaced");
    }
}
