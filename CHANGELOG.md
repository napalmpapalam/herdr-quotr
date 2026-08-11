# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project follows
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] - 2026-08-11

### Added

- **The question box has a caret.** `←`/`→` move through the question a character at a time,
  `Home`/`End` jump to its ends, and `Delete` cuts the character under the caret as `Backspace`
  cuts the one before it — a typo in the middle of a long question no longer means retyping the
  tail. Typing inserts where the caret sits, and a box taller than the pane scrolls to the row
  the caret is on.

### Fixed

- **herdr's own keybindings keep working while the picker is up.** The picker opened as a
  popup, which herdr treats as session-modal: it swallowed the prefix chain, so switching
  workspace or tab meant quitting the picker first. It now opens as an overlay, which covers
  the tab just the same and leaves those keys alone. Opening the picker a second time in a
  workspace focuses the one already open instead of stacking another over it.

## [0.1.1] - 2026-08-07

### Fixed

- **A long question is no longer typed blind.** The question box wraps at the reading measure
  and grows a row at a time instead of clipping everything past its right border; once it
  reaches the height of the pane it stops growing and follows the caret.

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

[Unreleased]: https://github.com/napalmpapalam/herdr-quotr/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/napalmpapalam/herdr-quotr/compare/v0.1.1...v0.2.0
[0.1.1]: https://github.com/napalmpapalam/herdr-quotr/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/napalmpapalam/herdr-quotr/releases/tag/v0.1.0
