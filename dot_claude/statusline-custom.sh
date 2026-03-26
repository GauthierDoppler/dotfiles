#!/bin/bash
input=$(cat)
pwd=$(echo "$input" | jq -r '.workspace.current_dir')
dir_name=$(basename "$pwd")
git_branch=""
if git -C "$pwd" rev-parse --git-dir > /dev/null 2>&1; then
    git_branch=$(git -C "$pwd" --no-optional-locks rev-parse --abbrev-ref HEAD 2>/dev/null)
    if [ -n "$git_branch" ]; then
        git_branch=" \033[31m⌥ $git_branch\033[0m"
    fi
fi
context_remaining=$(echo "$input" | jq -r '.context_window.remaining_percentage // 0' | awk '{printf "%.0f", $1}')
context_display=" ${context_remaining}%"
total_input=$(echo "$input" | jq -r '.context_window.total_input_tokens // 0')
total_output=$(echo "$input" | jq -r '.context_window.total_output_tokens // 0')
model_id=$(echo "$input" | jq -r '.model.id')
if [[ "$model_id" == *"opus"* ]]; then
    cost=$(awk "BEGIN {printf \"%.2f\", ($total_input * 15 + $total_output * 75) / 1000000}")
elif [[ "$model_id" == *"sonnet"* ]]; then
    cost=$(awk "BEGIN {printf \"%.2f\", ($total_input * 3 + $total_output * 15) / 1000000}")
else
    cost="0.00"
fi
printf "\033[36m%s\033[0m%b\033[33m%s\033[0m \033[32m\$%s\033[0m" "$dir_name" "$git_branch" "$context_display" "$cost"
