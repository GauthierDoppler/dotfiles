#!/bin/bash
# Claude Code hook: publishes this session's state on its tmux window, so the
# window list says which Claude wants you without a notification to click.
#
# Wired to Notification / Stop / UserPromptSubmit in dot_claude/settings.json.
# The event is read from the payload rather than passed as an argument, so all
# three entries are the same command.

set -uo pipefail
export PATH="/opt/homebrew/bin:$PATH"

input="$(cat)"
command -v jq >/dev/null 2>&1 || exit 0
event="$(printf '%s' "$input" | jq -r '.hook_event_name // ""')"

case "$event" in
  Notification) state=waiting ;;
  Stop)         state=done ;;
  *)            state="" ;;
esac

[ -n "${TMUX_PANE:-}" ] || exit 0
win="$(tmux display-message -p -t "$TMUX_PANE" '#{window_id}' 2>/dev/null)"
[ -n "$win" ] || exit 0

if [ -z "$state" ]; then
  tmux set-option -uw -t "$win" @claude_status 2>/dev/null
  exit 0
fi

# Skipped when this window is the active one of an attached client: marking a
# window you are already looking at is noise, and after-select-window has
# already fired, so nothing would clear it.
[ "$(tmux display-message -p -t "$win" \
      '#{&&:#{window_active},#{session_attached}}' 2>/dev/null)" = 1 ] && exit 0

tmux set-option -w -t "$win" @claude_status "$state" 2>/dev/null

exit 0
