//! The parsed session: turns flattened into the lines the picker scrolls.

use std::{
    fs,
    io::{BufRead, BufReader},
    path::Path,
};

use anyhow::{Context, Result};

use crate::{
    entry::Entry,
    line::{LineKind, SourceLine},
};

#[derive(Debug, Default)]
pub struct Transcript {
    lines: Vec<SourceLine>,
    /// Index of the first line of each turn — what `[` and `]` jump between.
    starts: Vec<usize>,
}

impl Transcript {
    pub fn load(path: &Path) -> Result<Self> {
        let file = fs::File::open(path).with_context(|| format!("opening {}", path.display()))?;
        Ok(Self::read(BufReader::new(file)))
    }

    pub fn lines(&self) -> &[SourceLine] {
        &self.lines
    }

    /// First line of the newest agent turn — where the picker opens.
    pub fn last_answer(&self) -> Option<usize> {
        self.starts
            .iter()
            .rev()
            .copied()
            .find(|&i| self.lines.get(i).is_some_and(|l| l.kind == LineKind::Agent))
    }

    pub fn next_turn(&self, from: usize) -> Option<usize> {
        self.starts.iter().copied().find(|&start| start > from)
    }

    pub fn prev_turn(&self, from: usize) -> Option<usize> {
        self.starts.iter().rev().copied().find(|&start| start < from)
    }

    /// Skips malformed lines: a live transcript can hand us a half-written last one.
    fn read(reader: impl BufRead) -> Self {
        let mut turns: Vec<(LineKind, String)> = Vec::new();
        let entries = reader
            .lines()
            .map_while(Result::ok)
            .filter_map(|line| serde_json::from_str::<Entry>(&line).ok())
            .filter_map(Entry::into_turn);
        for (kind, text) in entries {
            // Tool calls split one answer across entries; for quoting it is still one turn.
            match turns.last_mut() {
                Some((last, body)) if *last == kind => {
                    body.push_str("\n\n");
                    body.push_str(&text);
                }
                _ => turns.push((kind, text)),
            }
        }
        Self::from_turns(&turns)
    }

    fn from_turns(turns: &[(LineKind, String)]) -> Self {
        let mut lines: Vec<SourceLine> = Vec::new();
        let mut starts = Vec::with_capacity(turns.len());
        for (kind, body) in turns {
            if !lines.is_empty() {
                lines.push(SourceLine { text: String::new(), kind: LineKind::Gap });
            }
            starts.push(lines.len());
            lines
                .extend(body.lines().map(|text| SourceLine { text: text.to_owned(), kind: *kind }));
        }
        Self { lines, starts }
    }
}

#[cfg(test)]
mod tests {
    use super::{LineKind, Transcript};

    const SAMPLE: &str = concat!(
        r#"{"type":"user","message":{"content":"first prompt"}}"#,
        "\n",
        r#"{"type":"system","subtype":"noise"}"#,
        "\n",
        r#"{"type":"assistant","message":{"content":[{"type":"thinking","thinking":"hm"}]}}"#,
        "\n",
        r#"{"type":"assistant","message":{"content":[{"type":"text","text":"one\ntwo"}]}}"#,
        "\n",
        r#"{"type":"assistant","message":{"content":[{"type":"tool_use","id":"t1"}]}}"#,
        "\n",
        r#"{"type":"user","message":{"content":[{"type":"tool_result"}]},"toolUseResult":{}}"#,
        "\n",
        r#"{"type":"assistant","message":{"content":[{"type":"text","text":"three"}]}}"#,
        "\n",
        r#"{"type":"assistant","isSidechain":true,"message":{"content":[{"type":"text","text":"sub"}]}}"#,
        "\n",
        "{ not json",
        "\n",
    );

    fn sample() -> Transcript {
        Transcript::read(SAMPLE.as_bytes())
    }

    #[test]
    fn keeps_only_the_turns_a_user_saw() {
        let t = sample();
        let text: Vec<_> = t.lines().iter().map(|l| l.text.as_str()).collect();
        assert_eq!(text, ["first prompt", "", "one", "two", "", "three"]);
    }

    #[test]
    fn merges_an_answer_split_by_tool_calls() {
        let t = sample();
        // One user turn, one agent turn — the tool round trip between them is not a turn.
        assert_eq!(t.next_turn(0), Some(2));
        assert_eq!(t.next_turn(2), None);
        assert_eq!(t.last_answer(), Some(2));
    }

    #[test]
    fn marks_the_blank_row_between_turns_as_a_gap() {
        let t = sample();
        let kinds: Vec<_> = t.lines().iter().map(|l| l.kind).collect();
        assert_eq!(
            kinds,
            [
                LineKind::User,
                LineKind::Gap,
                LineKind::Agent,
                LineKind::Agent,
                LineKind::Agent,
                LineKind::Agent
            ]
        );
    }
}
