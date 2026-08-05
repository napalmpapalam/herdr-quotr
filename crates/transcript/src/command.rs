//! Slash commands, unwrapped from the XML Claude Code stores them as.

/// The command a wrapped prompt stands for, or `None` when `text` is an ordinary prompt.
/// Everything outside the tags is dropped with them — the caveat block, the empty stdout.
pub(crate) fn unwrap(text: &str) -> Option<String> {
    let name = tag(text, "command-name")?;
    let args = tag(text, "command-args").unwrap_or_default();
    let line = format!("{} {}", name.trim(), args.trim());
    Some(line.trim_end().to_owned())
}

/// The contents of the first `<name>…</name>` element in `text`.
fn tag<'a>(text: &'a str, name: &str) -> Option<&'a str> {
    let open = format!("<{name}>");
    let close = format!("</{name}>");
    let rest = text.split_once(&open)?.1;
    rest.split_once(&close).map(|(inner, _)| inner)
}

#[cfg(test)]
mod tests {
    use super::unwrap;

    #[test]
    fn a_wrapped_command_becomes_the_line_the_user_typed() {
        let text = "<command-message>dd:flow:go</command-message>\n\
                    <command-name>/dd:flow:go</command-name>\n\
                    <command-args>~/.context/quotr/quotr.md — build slice 5</command-args>";
        assert_eq!(
            unwrap(text).as_deref(),
            Some("/dd:flow:go ~/.context/quotr/quotr.md — build slice 5")
        );
    }

    #[test]
    fn a_command_with_no_arguments_keeps_no_trailing_space() {
        let text = "<command-name>/clear</command-name>\n\
                    <command-message>clear</command-message>\n\
                    <command-args></command-args>";
        assert_eq!(unwrap(text).as_deref(), Some("/clear"));
    }

    #[test]
    fn the_caveat_and_the_empty_stdout_block_go_with_the_tags() {
        let text = "<local-command-caveat>Caveat: ignore this.</local-command-caveat>\n\
                    <command-name>/clear</command-name>\n\
                    <command-args></command-args>\n\
                    <local-command-stdout></local-command-stdout>";
        assert_eq!(unwrap(text).as_deref(), Some("/clear"));
    }

    #[test]
    fn an_ordinary_prompt_is_left_alone() {
        assert_eq!(unwrap("please split edit and delete"), None);
        assert_eq!(unwrap("use <command-name> in a sentence"), None);
    }
}
