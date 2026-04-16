#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

TEST_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/nvim-config-test.XXXXXX")"
trap 'rm -rf "$TEST_ROOT"' EXIT

mkdir -p "$TEST_ROOT/cache" "$TEST_ROOT/data" "$TEST_ROOT/state" "$TEST_ROOT/tmp"

export XDG_CACHE_HOME="$TEST_ROOT/cache"
export XDG_DATA_HOME="$TEST_ROOT/data"
export XDG_STATE_HOME="$TEST_ROOT/state"
export TMPDIR="$TEST_ROOT/tmp"
export NVIM_LOG_FILE="$TEST_ROOT/nvim.log"

nvim --headless -u init.lua \
  "+lua local ok, err = pcall(dofile, 'tests/smoke.lua'); if not ok then print(err); vim.cmd('cq 1') end" \
  "+qa"

git restore --worktree --staged lazy-lock.json >/dev/null 2>&1 || true
