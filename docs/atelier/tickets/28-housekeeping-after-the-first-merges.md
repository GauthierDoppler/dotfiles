# 28: Housekeeping after the first merges

**What to build:** Loose ends reported by the agents of tickets 03–19: machines that ran the old install keep dangling links, picker tests never run in CI, and small duplications crept in across modules.

**Blocked by:** 04 (Left block of the bar rendered by atelier), 08 (Task placements: split, popup and detach), 20 (Services on launchd and systemd)

**Status:** done

- [x] `atelier setup` removes symlinks in the places it manages that point into the repo at a path that no longer exists (e.g. `~/.local/bin/tmux-pick`, `tmux-status-right`, `local-diff`, `claude-settings-sync`, `md-preview`, `tmux-sessions`, `tmux-tasks`), and reports them; `--dry-run` lists them
- [x] CI installs an fzf recent enough for the pickers (≥ 0.45) on Linux and macOS, so the fzf-driven tests run instead of skipping
- [x] fzf invocation flags and the "atelier exe + `--socket`" callback prefix live in one shared module used by every picker (`pick`, `sessions`, `tasks/picker`, `preview/picker`); `shell::quote` is already shared
- [x] `atelier hook claude` and other tmux callers still find `tmux` when `PATH` lacks the Homebrew prefix
- [x] `atelier preview <file>` opens the URL with the OS opener on Linux when one exists, instead of only printing it
- [x] `jq` stays in the Brewfile only if something still needs it

## Comments

- **Pruning scope.** The folders scanned are the parents of every `LINKS`
  destination (whatever the profile) plus the `.pi/agent/agents` folder, one
  level deep, never recursively. A link is removed only if it is dangling and its
  target, resolved lexically against the link's folder, starts with the repo path
  as given or as canonicalised. A scanned folder that itself resolves inside the
  repo is skipped. A dangling link that is also a `LINKS`/stub destination is left
  to the existing `unlink` + `linked` actions, so it is not reported twice.
- **CI fzf.** Linux installs the pinned `fzf 0.65.2` release tarball into
  `/usr/local/bin` (Ubuntu's package is 0.44, too old for `transform`); macOS
  uses Homebrew's. `tests/common::fzf_available` checks the version is ≥ 0.45,
  skips locally without it, and panics when `CI` is set, so a missing fzf in CI
  fails instead of silently skipping. The workflow itself was not run from here.
- **Shared fzf module (`src/fzf.rs`).** Deviation: unifying the flags also
  unified the look. The session and task pickers now get the same
  `--layout=reverse`, `--info=inline`, `--height=100%` and Catppuccin colours as
  the token and markdown pickers, and those two gain `--cycle`, `--header-first`
  and the `▸` pointer. The `esc` bind's actions are now shell-quoted, which fixes
  the task picker's echo breaking on an exe or socket path with spaces. The
  session picker keeps its own `sessions escape` callback (it is tested and needs
  the scope).
- **tmux lookup.** `Tmux` resolves the binary once: `PATH`, then
  `/opt/homebrew/bin`, `/usr/local/bin`, `/home/linuxbrew/.linuxbrew/bin`,
  `/usr/bin`, `/bin`; `Tmux::command()` is used by the task runner's popup too.
  Not changed: `src/daemon/server.rs` still spawns `tmux` by name (ticket 12 owns
  the daemon); it is started by tmux, so it inherits tmux's own `PATH`.
- **Opener.** `src/opener.rs` is shared by `pick` and the Linux preview. It now
  spawns the opener detached (own process group) instead of waiting for it, since
  `xdg-open` can block for as long as the browser it starts runs, which would
  freeze a picker's `execute-silent`. A missing opener is still an error for
  `pick`; the preview ignores it and prints the URL either way. Tests put a fake
  `xdg-open`/`open` first on `PATH`, so no test launches a real browser.
- **jq** stays: `dot_claude/statusline-custom.sh` uses it on every refresh.
- Unverified on macOS: the Homebrew fallback path for tmux and the macOS CI fzf
  step.
