# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project follows
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-08-06

First release.

### Added

- **The picker.** `Cmd+'` opens a full-screen popup over the focused Claude Code pane,
  holding that session's whole transcript, opened with the newest line on the bottom row.
- **Mouse-first selection.** The wheel scrolls one line at a time, a left drag selects
  exactly the characters it crosses, and a click places the caret. Keyboard nav is a full
  fallback: `h/j/k/l`, `g`/`G`, `PgUp`/`PgDn`, `[`/`]` by turn, `v`/`V` to select.
- **A batch of quote+question pairs.** `c` opens a question box under the quote, Enter banks
  the pair, `e` and `d` edit and drop the pair under the caret, and `s` sends the lot into
  the agent's input box **without submitting**.
- **Markdown, rendered.** Inline markers are stripped, so what's on screen is what gets
  sent; pipe tables are drawn as a grid; fenced blocks are syntax-highlighted with
  [syntect](https://github.com/trishume/syntect).
- **19 themes**, dark and light, named to match herdr's own. Set `theme` in the plugin's
  `config.toml`. The picker paints its theme's own background, so a light theme stays readable
  on a dark terminal.
- **A configurable reading measure.** `measure` in `config.toml` caps the centered text
  column; the default is 100 columns.
- **A parked quote survives a permission prompt.** Sending into a `blocked` agent would be
  swallowed silently, so the batch is written to the plugin's state dir and restored on the
  next open instead.

[Unreleased]: https://github.com/napalmpapalam/herdr-quotr/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/napalmpapalam/herdr-quotr/releases/tag/v0.1.0
