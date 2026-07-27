#!/bin/bash
input=$(cat)

# Single jq pass: statusline runs on every refresh tick.
# \x1f (unit separator) rather than \t: tab is IFS whitespace, so bash collapses
# consecutive tabs and an empty field (absent effort) would shift every field after it.
IFS=$'\x1f' read -r pwd model effort context_remaining cost duration_ms < <(
    echo "$input" | jq -r '[
        .workspace.current_dir,
        (.model.display_name // "?"),
        (.effort.level // ""),
        (.context_window.remaining_percentage // 0 | round),
        (.cost.total_cost_usd // 0),
        (.cost.total_duration_ms // 0)
    ] | map(tostring) | join("\u001f")'
)

dir_name=$(basename "$pwd")

git_branch=""
if git -C "$pwd" rev-parse --git-dir > /dev/null 2>&1; then
    git_branch=$(git -C "$pwd" --no-optional-locks rev-parse --abbrev-ref HEAD 2>/dev/null)
    if [ -n "$git_branch" ]; then
        git_branch=" \033[31m⌥ $git_branch\033[0m"
    fi
fi

# effort is absent when the model does not support the reasoning parameter
# braces required: bash 3.2 folds a following multibyte char into the parameter name
effort_display=""
[ -n "$effort" ] && effort_display=" \033[34m${effort}\033[0m"

seconds=$((duration_ms / 1000))
if [ "$seconds" -ge 3600 ]; then
    duration=$(printf "%dh%02dm" $((seconds / 3600)) $((seconds % 3600 / 60)))
elif [ "$seconds" -ge 60 ]; then
    duration=$(printf "%dm" $((seconds / 60)))
else
    duration=$(printf "%ds" "$seconds")
fi

printf "\033[36m%s\033[0m%b \033[35m%s\033[0m%b \033[33m%s%%\033[0m \033[32m\$%.2f\033[0m \033[90m%s\033[0m" \
    "$dir_name" "$git_branch" "$model" "$effort_display" "$context_remaining" "$cost" "$duration"
