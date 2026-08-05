//! Compose the markdown quotr writes into the agent's input.

/// Build the block for one selection: `"quote"`, blank line, question.
///
/// One quote pair wraps the whole range, so fenced code inside it survives byte-for-byte.
pub fn block(lines: &[&str], question: Option<&str>) -> String {
    let quote = format!("\"{}\"", lines.join("\n"));
    question.map(|q| format!("{quote}\n\n{q}")).unwrap_or(quote)
}

#[cfg(test)]
mod tests {
    use super::block;

    #[test]
    fn wraps_the_whole_range_in_one_quote_pair() {
        assert_eq!(block(&["one", "two"], None), "\"one\ntwo\"");
    }

    #[test]
    fn puts_the_question_a_blank_line_below() {
        assert_eq!(block(&["one"], Some("why?")), "\"one\"\n\nwhy?");
    }

    #[test]
    fn leaves_markdown_inside_the_quote_untouched() {
        let lines = ["```rust", "let x = 1;", "```"];
        assert_eq!(block(&lines, None), "\"```rust\nlet x = 1;\n```\"");
    }
}
