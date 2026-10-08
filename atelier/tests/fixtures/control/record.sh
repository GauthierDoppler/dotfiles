#!/usr/bin/env bash
set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)
socket=atelier-record-$$

t() { tmux -L "$socket" "$@"; }

lifecycle() {
  sleep 0.5
  t rename-window -t work:0 '%end 1 1 1'
  sleep 0.3
  echo "list-sessions -F '#{session_id} #{session_name}'"
  sleep 0.3
  echo "list-windows -a -F '#{session_id} #{window_id} #{window_index} #{window_name}'"
  sleep 0.3
  echo "kill-session -t nosuch"
  sleep 0.3
  t new-session -d -s api
  sleep 0.3
  sleep 1 | t -C attach-session -t work >/dev/null &
  sleep 0.3
  t switch-client -c "$(t list-clients -F '#{client_name}' | tail -1)" -t api 2>/dev/null || true
  wait
  sleep 0.3
  t new-window -t work -n logs
  sleep 0.3
  t new-window -t api -n build
  sleep 0.3
  t rename-window -t work:logs tail
  sleep 0.3
  t rename-window -t api:build make
  sleep 0.3
  t rename-session -t api backend
  sleep 0.3
  t kill-window -t work:tail
  sleep 0.3
  t kill-session -t backend
  sleep 0.3
  t new-session -d -s viewer
  t switch-client -c "$(t list-clients -F '#{client_name}')" -t viewer 2>/dev/null || true
  sleep 0.3
  t kill-server
  sleep 0.5
}

output() {
  sleep 0.5
  t send-keys -t work 'echo "%end 1 2 1"' Enter
  sleep 0.5
  t kill-server
  sleep 0.5
}

session_killed() {
  sleep 0.5
  t new-session -d -s other
  sleep 0.3
  t kill-session -t work
  sleep 0.5
  t kill-server
  sleep 0.3
}

record() {
  local scenario=$1 flags=$2
  t -f /dev/null new-session -d -s work -x 80 -y 24 -c / "env PS1='$ ' sh"
  "$scenario" | tmux -L "$socket" -C attach-session -t work $flags >"$here/${scenario//_/-}.txt"
}

record lifecycle "-f no-output,ignore-size"
record output ""
record session_killed "-f no-output,ignore-size"
