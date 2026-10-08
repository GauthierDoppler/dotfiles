# 28: Housekeeping after the first merges

**What to build:** Loose ends reported by the agents of tickets 03–19: machines that ran the old install keep dangling links, picker tests never run in CI, and small duplications crept in across modules.

**Blocked by:** 04 (Left block of the bar rendered by atelier), 08 (Task placements: split, popup and detach), 20 (Services on launchd and systemd)

**Status:** ready-for-agent

- [ ] `atelier setup` removes symlinks in the places it manages that point into the repo at a path that no longer exists (e.g. `~/.local/bin/tmux-pick`, `tmux-status-right`, `local-diff`, `claude-settings-sync`, `md-preview`, `tmux-sessions`, `tmux-tasks`), and reports them; `--dry-run` lists them
- [ ] CI installs an fzf recent enough for the pickers (≥ 0.45) on Linux and macOS, so the fzf-driven tests run instead of skipping
- [ ] fzf invocation flags and `shell_quote` live in one shared module used by every picker
- [ ] `atelier hook claude` and other tmux callers still find `tmux` when `PATH` lacks the Homebrew prefix
- [ ] `atelier preview <file>` opens the URL with the OS opener on Linux when one exists, instead of only printing it
- [ ] `jq` stays in the Brewfile only if something still needs it
