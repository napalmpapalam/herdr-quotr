#!/usr/bin/env bash
# Open the quotr picker over the focused agent pane.
#
# Invoked by herdr with the plugin runtime env set (HERDR_BIN_PATH, HERDR_PANE_ID,
# HERDR_PLUGIN_*, HERDR_PLUGIN_CONTEXT_JSON). The origin pane id is handed to the picker
# process as QUOTR_AGENT_PANE — the picker is transient and owns no state file, unlike an
# ambient sidebar.
#
# No `set -e`: a transient jq/herdr hiccup must not abort the open half-done; each step is
# checked explicitly.
set -uo pipefail

# herdr runs plugin commands with a minimal PATH; ensure jq resolves on common installs.
export PATH="/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:${PATH:-}"

H="${HERDR_BIN_PATH:-herdr}"

ctx="${HERDR_PLUGIN_CONTEXT_JSON:-}"
pane=""
cwd=""
ws=""
if [ -n "$ctx" ]; then
  pane=$(printf '%s' "$ctx" | jq -r '.focused_pane_id // empty' 2>/dev/null)
  cwd=$(printf '%s' "$ctx" | jq -r '.focused_pane_cwd // empty' 2>/dev/null)
  ws=$(printf '%s' "$ctx" | jq -r '.workspace_id // empty' 2>/dev/null)
fi
[ -n "$pane" ] || pane="${HERDR_PANE_ID:-}"
[ -n "$ws" ] || ws="${HERDR_ACTIVE_WORKSPACE_ID:-}"

# Without an origin pane there is nothing to quote and nowhere to send; do nothing.
[ -n "$pane" ] || exit 0

# The picker is one per workspace, and this is the only thing that enforces it: an overlay is
# an ordinary pane, so a second press would stack a second full-screen picker over the first
# and `q` would reveal it. herdr labels a plugin pane from the manifest title, and only this
# action ever opens one, so the label is enough to find it. Focus rather than close, because
# the open picker may already hold a batch of banked pairs — it keeps pointing at the agent it
# was opened from, which is the price of not throwing that batch away. A focus that fails (the
# plugin-pane registry forgets a pane across a herdr restart) falls through to a fresh open,
# since a dead keypress is worse than a duplicate.
if [ -n "$ws" ]; then
  open=$("$H" pane list --workspace "$ws" 2>/dev/null |
    jq -r 'first(.result.panes[] | select(.label == "quotr") | .pane_id) // empty' 2>/dev/null)
  if [ -n "$open" ] && "$H" plugin pane focus "$open" >/dev/null 2>&1; then
    exit 0
  fi
fi

# No `--placement`: the manifest's `overlay` is what covers the tab. It was a `popup` until the
# session-modal placement turned out to swallow herdr's own keybindings while the picker was up.
set -- --plugin "${HERDR_PLUGIN_ID:-napalmpapalam.quotr}" --entrypoint picker \
  --env "QUOTR_AGENT_PANE=$pane" --focus
[ -n "$cwd" ] && set -- "$@" --cwd "$cwd"

# stdout is dropped, stderr is not: herdr captures a plugin command's stderr, so a failed
# open shows up in `herdr plugin log --plugin napalmpapalam.quotr` instead of vanishing.
"$H" plugin pane open "$@" >/dev/null
