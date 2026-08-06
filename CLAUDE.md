# herdr-quotr

A herdr plugin: a full-screen popup that reads the focused Claude Code session's transcript,
lets you select ranges and attach questions, and writes the batch into the agent's input box
without submitting.

## Build and run

The picker only runs inside a herdr popup, so there is no `cargo run` path. It is launched by
absolute path from the plugin root, which means **a code change is invisible until you
reinstall the binary**:

```sh
cargo build --release && mkdir -p bin && install -m 0755 target/release/herdr-quotr bin/herdr-quotr
```

`bin/` is gitignored. `herdr plugin link .` links the checkout; `herdr plugin install` would
download a release binary instead (`herdr/install.sh`).

Checks, all of which CI runs:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features --locked
cargo test --workspace --all-features --locked
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features --locked
shellcheck herdr/*.sh
```

`cargo run --example themes` prints every theme through the real paint stack; add `html` for a
page comparing them all.

`QUOTR_TRANSCRIPT=<path>` runs the picker against a transcript file directly, with no herdr
server and no live session — how the demo is recorded and the fastest way to iterate on the UI:

```sh
QUOTR_TRANSCRIPT=assets/demo.jsonl target/release/herdr-quotr
vhs assets/demo.tape   # re-records assets/demo.gif
```

Sending needs a real pane, so a `QUOTR_TRANSCRIPT` run can browse and bank but not deliver.

## Layout

The app is the root binary; every reusable layer is a crate under `crates/*`.

- `src/` — `main.rs`, `lib.rs` (terminal lifecycle + event loop), `app.rs` (state), `nav.rs`,
  `bank.rs`, `send.rs`, `stash.rs`, `config.rs`.
- `markup` — the shared vocabulary: `Block`, `Span`, `Emphasis`, `Pos`, `Tone`. A leaf.
- `transcript` — JSONL → `SourceLine`s, markers already stripped. No terminal, no CLI.
- `export` — composes the block that gets sent. No dependencies.
- `herdr` — the host CLI: `PaneId`, `agent_session`, `send_text`, `focus`.
- `ui` — ratatui paint; owns the reading column.

**The dependency rule:** a crate never depends on the app, and `transcript` and `ui` never
depend on each other — they meet in `markup`, which knows about neither. `ui::render` takes a
`ui::View` of plain data, never `&App`, so the arrow cannot reverse.

## Invariants worth knowing before editing

- **Screen text is sent text.** `SourceLine::text` is the single copy: markers are stripped in
  the parser, and selection, slicing, and the quote all work on that string. A change that
  makes the painter rewrite text breaks the product.
  - The two deliberate exceptions paint one character in place of another of the same width
    (a blockquote's `>` as `│`) or select linewise (a rendered table), so offsets still hold.
- **Positions are `(source line, char offset)`**, never display rows. Scroll offset is a
  source-line index, so no wrap state is stored.
- **Only the paint layer knows how text wrapped**, so `ui::render` returns `Painted` — that is
  what resolves a mouse cell to a character, places the caret, and finds where the buffer opens.
- **Never submit.** The send path is `pane send-text` + `agent focus`. A `blocked` agent parks
  the batch to the state dir instead of sending, because a permission prompt eats the text
  while the call still exits 0.
- **Structure detection lives in `transcript` alone**; `ui` keeps color, syntect, and table
  layout.
- **No log file.** Errors surface in the picker's status line; `open.sh` leaves stderr
  unredirected, so herdr captures it (`herdr plugin log --plugin napalmpapalam.quotr`).

## Conventions

Rust follows `dd:rust:core` and `dd:rust:linting`. Specific to this repo:

- Lints are stricter than usual: `unwrap_used`, `expect_used`, `panic`, and `indexing_slicing`
  are **denied**, plus `unsafe_code = "forbid"`. Reach for `get()`, `?`, and combinators.
- One file per concern, none over ~350 lines. Module roots are `foo.rs` beside `foo/`, never
  `mod.rs`.
- Commits are conventional (`feat(picker): …`).
- Every user-facing behavior change gets a `CHANGELOG.md` entry under `## [Unreleased]` — the
  release workflow reads the tag's section as the release body and fails without one.

## Release

Push a `v*` tag matching the `version` in `herdr-plugin.toml`. `.github/workflows/release.yml`
drafts the release from the CHANGELOG section, builds four target triples, attaches signed
archives, and publishes only once every target lands. `herdr/install.sh` downloads the asset
whose tag matches the manifest version, so the two must stay in step.
