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
if [ -n "$ctx" ]; then
  pane=$(printf '%s' "$ctx" | jq -r '.focused_pane_id // empty' 2>/dev/null)
  cwd=$(printf '%s' "$ctx" | jq -r '.focused_pane_cwd // empty' 2>/dev/null)
fi
[ -n "$pane" ] || pane="${HERDR_PANE_ID:-}"

# Without an origin pane there is nothing to quote and nowhere to send; do nothing.
[ -n "$pane" ] || exit 0

# No `--placement`: the CLI flag has no `popup` value, so the entrypoint's manifest placement
# is what puts the picker in a full-screen popup (verified on herdr 0.8.0).
set -- --plugin "${HERDR_PLUGIN_ID:-napalmpapalam.quotr}" --entrypoint picker \
  --env "QUOTR_AGENT_PANE=$pane" --focus
[ -n "$cwd" ] && set -- "$@" --cwd "$cwd"

# stdout is dropped, stderr is not: herdr captures a plugin command's stderr, so a failed
# open shows up in `herdr plugin log --plugin napalmpapalam.quotr` instead of vanishing.
"$H" plugin pane open "$@" >/dev/null
