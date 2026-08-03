#!/bin/bash
# Claude Code notification hook — sends macOS notification via terminal-notifier
export PATH="/opt/homebrew/bin:$PATH"
input=$(cat)
cwd=$(echo "$input" | jq -r '.cwd // ""')
dir_name=$(basename "$cwd")

terminal-notifier \
  -title "Claude Code" \
  -subtitle "$dir_name" \
  -message "Waiting for input" \
  -sender com.claude-code-notifier \
  -group "claude-code" \
  > /dev/null 2>&1
