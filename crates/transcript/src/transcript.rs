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
    pos::Pos,
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

    /// The characters `from..=to` covers, as quotable lines.
    ///
    /// The first and last lines are cut at their columns; whole lines in between come as
    /// they are, minus the blank rows that only separate turns.
    pub fn slice(&self, from: Pos, to: Pos) -> Vec<&str> {
        let (from, to) = (self.clamp(from), self.clamp(to));
        let Some(first) = self.lines.get(from.line) else { return Vec::new() };
        if from.line == to.line {
            return non_empty(first.slice(from.col, to.col)).into_iter().collect();
        }
        let middle = self
            .lines
            .get(from.line + 1..to.line)
            .unwrap_or_default()
            .iter()
            .filter(|line| line.kind != LineKind::Gap)
            .map(|line| line.text.as_str());
        let tail = self.lines.get(to.line).and_then(|last| non_empty(last.slice(0, to.col)));
        non_empty(first.slice(from.col, first.len()))
            .into_iter()
            .chain(middle)
            .chain(tail)
            .collect()
    }

    /// Pull a position back onto a line that exists, and onto a column that line has.
    pub fn clamp(&self, pos: Pos) -> Pos {
        let line = pos.line.min(self.lines.len().saturating_sub(1));
        let col = pos.col.min(self.lines.get(line).map_or(0, SourceLine::len));
        Pos { line, col }
    }

    /// End of `line` — where a linewise selection stops.
    pub fn line_end(&self, line: usize) -> Pos {
        Pos { line, col: self.lines.get(line).map_or(0, SourceLine::len) }
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

/// Drop a cut end that came out empty, so a drag starting at end of line adds no blank row.
fn non_empty(text: &str) -> Option<&str> {
    (!text.is_empty()).then_some(text)
}

#[cfg(test)]
mod tests {
    use super::{LineKind, Pos, Transcript};

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
        assert_eq!(t.prev_turn(5), Some(2));
    }

    #[test]
    fn slices_within_one_line() {
        assert_eq!(sample().slice(Pos::new(2, 1), Pos::new(2, 3)), ["ne"]);
    }

    #[test]
    fn cuts_the_ends_and_keeps_the_middle_whole() {
        // "first prompt" / gap / "one" / "two" / "" / "three" — the blank is a paragraph
        // break inside the merged answer, so it stays; only a turn gap is dropped.
        let t = sample();
        assert_eq!(t.slice(Pos::new(2, 1), Pos::new(5, 3)), ["ne", "two", "", "thr"]);
    }

    #[test]
    fn drops_the_blank_row_between_turns() {
        let t = sample();
        assert_eq!(t.slice(Pos::new(0, 0), Pos::new(2, 3)), ["first prompt", "one"]);
    }

    #[test]
    fn drops_an_end_that_cuts_to_nothing() {
        let t = sample();
        assert_eq!(t.slice(Pos::new(2, 3), Pos::new(3, 3)), ["two"]);
        assert_eq!(t.slice(Pos::new(2, 0), Pos::new(3, 0)), ["one"]);
    }

    #[test]
    fn clamps_a_position_past_the_end_of_the_transcript() {
        let t = sample();
        assert_eq!(t.clamp(Pos::new(99, 99)), Pos::new(5, 5));
        assert_eq!(t.line_end(2), Pos::new(2, 3));
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
