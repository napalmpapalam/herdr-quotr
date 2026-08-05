//! Syntax highlighting for fenced code, themed by the active theme's paired syntax theme.

use std::{fmt, io::Cursor, sync::OnceLock};

use ratatui::style::{Color, Style};
use syntect::{
    easy::HighlightLines,
    highlighting::{Style as SyntectStyle, Theme, ThemeSet},
    parsing::SyntaxSet,
    util::LinesWithEndings,
};

use crate::{style::Run, theme::SyntaxChoice};

/// The broad bat/two-face syntax set, built once per process — it is expensive to
/// deserialize — and shared by every [`Highlighter`].
fn syntaxes() -> &'static SyntaxSet {
    static SYNTAXES: OnceLock<SyntaxSet> = OnceLock::new();
    SYNTAXES.get_or_init(two_face::syntax::extra_newlines)
}

/// The two-face embedded theme set, deserialized once and shared, like [`syntaxes`].
fn embedded_themes() -> &'static two_face::theme::EmbeddedLazyThemeSet {
    static THEMES: OnceLock<two_face::theme::EmbeddedLazyThemeSet> = OnceLock::new();
    THEMES.get_or_init(two_face::theme::extra)
}

/// Holds the active syntax theme — `None` when it failed to parse, which degrades a fenced
/// block to plain text rather than crashing.
pub(crate) struct Highlighter {
    theme: Option<Theme>,
}

impl fmt::Debug for Highlighter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Highlighter").finish_non_exhaustive()
    }
}

impl Highlighter {
    pub(crate) fn new(syntax: SyntaxChoice) -> Self {
        let theme = match syntax {
            SyntaxChoice::Bundled(bytes) => {
                ThemeSet::load_from_reader(&mut Cursor::new(bytes)).ok()
            }
            SyntaxChoice::Embedded(name) => Some(embedded_themes().get(name).clone()),
        };
        Self { theme }
    }

    /// Style runs for each line of `content`, or an empty run list per line when the language
    /// is unknown — the caller then paints the block in its own base style.
    ///
    /// `language` matches as a file extension first, then as a fence tag like `rust` or `sh`.
    pub(crate) fn runs(&self, content: &str, language: &str) -> Vec<Vec<Run>> {
        let syntaxes = syntaxes();
        let plain = || content.lines().map(|_| Vec::new()).collect();
        // An untagged fence stays base text rather than picking up the plain-text syntax.
        if language.is_empty() {
            return plain();
        }
        let syntax = syntaxes
            .find_syntax_by_extension(language)
            .or_else(|| syntaxes.find_syntax_by_token(language));
        let (Some(syntax), Some(theme)) = (syntax, self.theme.as_ref()) else {
            return plain();
        };
        let mut lines = HighlightLines::new(syntax, theme);
        LinesWithEndings::from(content)
            .map(|line| lines.highlight_line(line, syntaxes).map_or_else(|_| Vec::new(), runs))
            .collect()
    }
}

/// One line's highlighted regions as style runs, each starting where the last one ended.
/// A grammar error yields no regions, which degrades that line to plain text.
fn runs(regions: Vec<(SyntectStyle, &str)>) -> Vec<Run> {
    let mut at = 0;
    let mut out = Vec::with_capacity(regions.len());

    for (style, text) in regions {
        let fg = style.foreground;
        out.push(Run::new(at, Style::new().fg(Color::Rgb(fg.r, fg.g, fg.b))));
        at += text.trim_end_matches('\n').chars().count();
    }

    out
}

#[cfg(test)]
mod tests {
    use super::Highlighter;
    use crate::theme;

    fn highlighter(name: &str) -> Highlighter {
        Highlighter::new(
            theme::resolve(name).map_or_else(|| theme::default_theme().syntax, |t| t.syntax),
        )
    }

    #[test]
    fn rust_tokenizes_into_several_colored_runs() {
        let runs = highlighter("vs-dark-plus").runs("let x = 1;\n", "rust");
        assert_eq!(runs.len(), 1);
        assert!(runs.first().is_some_and(|line| line.len() > 1), "expected several runs");
    }

    #[test]
    fn an_unknown_language_leaves_every_line_unstyled() {
        let runs = highlighter("vs-dark-plus").runs("alpha\nbeta\n", "wingdings");
        assert_eq!(runs, vec![Vec::new(), Vec::new()]);
    }

    #[test]
    fn every_bundled_syntax_theme_parses() {
        // A `.tmTheme` that fails to load leaves the highlighter theme-less, which silently
        // degrades every fenced block to plain text.
        for name in ["catppuccin", "vs-dark-plus", "tokyo-night", "rose-pine-dawn"] {
            let runs = highlighter(name).runs("let x = 1;\n", "rust");
            assert!(
                runs.first().is_some_and(|line| line.len() > 1),
                "{name}: bundled syntax theme failed to load",
            );
        }
    }
}
