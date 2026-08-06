//! The JSONL shapes quotr cares about. Everything else in the transcript is skipped.

use serde::Deserialize;
use serde_json::Value;

use markup::Tone;

use crate::command;

/// One transcript line; `Other` swallows the kinds quotr ignores.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub(crate) enum Entry {
    Assistant(Turn),
    User(Turn),
    #[serde(other)]
    Other,
}

impl Entry {
    /// The turn's visible text, or `None` if this entry isn't one.
    pub(crate) fn into_turn(self) -> Option<(Tone, String)> {
        match self {
            Self::Assistant(turn) => turn.into_text(Tone::Agent),
            Self::User(turn) => turn.into_text(Tone::User),
            Self::Other => None,
        }
    }
}

/// Optional because Claude Code usually omits these, and writes `isMeta` as a bare `null`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Turn {
    message: Message,

    /// Set on a subagent's turns.
    #[serde(default)]
    is_sidechain: Option<bool>,

    /// Set on entries Claude injects rather than a person writing them.
    #[serde(default)]
    is_meta: Option<bool>,

    /// Present when the entry carries a tool's output.
    #[serde(default)]
    tool_use_result: Option<Value>,

    /// Present when a tool, not the user, produced the entry.
    #[serde(default, rename = "sourceToolUseID")]
    source_tool_use_id: Option<String>,
}

impl Turn {
    fn into_text(self, kind: Tone) -> Option<(Tone, String)> {
        if self.is_noise() {
            return None;
        }
        let text = self.message.content.into_text();
        let text = command::unwrap(&text).unwrap_or(text);
        (!text.trim().is_empty()).then(|| (kind, text.trim_end().to_owned()))
    }

    /// Subagent work, meta entries, and tool traffic — none of it is a turn a user saw.
    fn is_noise(&self) -> bool {
        self.is_sidechain.unwrap_or_default()
            || self.is_meta.unwrap_or_default()
            || self.tool_use_result.is_some()
            || self.source_tool_use_id.is_some()
    }
}

#[derive(Debug, Deserialize)]
struct Message {
    content: Content,
}

/// User prompts arrive as a bare string, assistant turns as a block list.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum Content {
    Text(String),
    Parts(Vec<Part>),
}

impl Content {
    fn into_text(self) -> String {
        match self {
            Self::Text(text) => text,
            Self::Parts(parts) => {
                parts.into_iter().filter_map(Part::into_text).collect::<Vec<_>>().join("\n")
            }
        }
    }
}

/// One piece of a message's content. `tool_use`, `tool_result`, and `thinking` all fall
/// through to `Other`. Named for the JSONL, not for [`crate::Block`], which is unrelated.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Part {
    Text {
        text: String,
    },
    #[serde(other)]
    Other,
}

impl Part {
    fn into_text(self) -> Option<String> {
        let Self::Text { text } = self else { return None };
        Some(text)
    }
}
