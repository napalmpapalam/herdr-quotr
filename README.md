# herdr-quotr

Quote an agent's own response back at it, from a [herdr](https://herdr.dev) popup.

One key opens a full-screen picker over the focused Claude Code pane. You select line
ranges from its transcript, attach a question to each, and `S` writes the composed
markdown into the agent's chatbox **without submitting** — you hit Enter yourself.
It replaces the manual copy-paste-and-wrap-in-`>` ritual.

```
> quoted line one
> quoted line two

the question
```

Claude Code only: the picker reads the session's transcript JSONL, so the quoted text is
the exact markdown the agent wrote — no box-drawing or soft-wrap artifacts.

## Status

Early. Slice 1 is in: the plugin scaffold, the popup, the origin-pane hand-off, and the
send round-trip — with a **hardcoded** quote block. Transcript parsing and selection land
next.

## Install (local)

```sh
cargo build --release
install -D target/release/herdr-quotr bin/herdr-quotr
herdr plugin link .
```

Then bind a key in `~/.config/herdr/config.toml`:

```toml
[[keys.command]]
key = "prefix+'"
type = "plugin_action"
command = "napalmpapalam.quotr.open"
description = "Quote agent answer"
```

If your terminal owns `Cmd`, give it the matching mapping — in Ghostty:

```
keybind = cmd+apostrophe=text:\x02'
```

## Keys

| key | does |
| --- | --- |
| `S` | send the block to the agent's input, then focus it |
| `q` / `esc` | quit |

## License

MIT
