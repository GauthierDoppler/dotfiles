# 21: atelier doctor

**What to build:** `atelier doctor` checks what `AGENTS.md` currently asks me to remember, and prints the fix for each failure.

**Blocked by:** 17 (Setup: links and stubs), 20 (Services on launchd and systemd)

**Status:** done

- [x] Terminfo exists for the current `$TERM`, with the command to copy it from another machine
- [x] Nerd Font glyphs render, tmux is recent enough, extended keys are on
- [x] Services are running; on Linux, linger is enabled
- [x] Exit code is non-zero when a check fails

## Comments

- **Nerd Font is a manual check.** No terminal reports which font drew a glyph,
  so `doctor` prints U+E0B6/U+E0B4 on a `look` line and leaves the verdict to
  the reader; `look` never fails the run.
- **tmux minimum is 3.3**, not 3.2: `display-popup`, `extended-keys`,
  `terminal-features` and control-mode `attach -f no-output,ignore-size` are all
  3.2, but `dot_tmux.conf` sets `allow-passthrough` and `pane-border-indicators`,
  both added in 3.3 (tmux CHANGES). atelier does not use control-mode
  subscriptions (`refresh-client -B`) yet. Development builds (`next-3.6`)
  parse as their version; an unparsable `-V` is `look`.
- **Checks added from AGENTS.md gotchas:** duplicated `terminal-features`
  (appending reloads), `~/.local/bin/atelier` present (tmux calls it by that
  path), pending `atelier setup` changes (stubs replaced by symlinks, missing
  links), a root-owned `~/Library/LaunchAgents` (launchd only).
- Terminfo is checked for `$TERM` plus the TERM of every attached non-control
  tmux client, since the tmux server needs the client's entry on this machine.
- Checks needing a tmux server (`extended keys`, `terminal features`, `daemon`)
  are `skip` when none answers, so running `doctor` outside tmux does not fail.
- `service list` now goes through the same per-system status function the
  doctor uses; its output is unchanged.
- **Unverified on macOS:** the launchd path (`launchctl print`, LaunchAgents
  probe) is driven only through a fake `launchctl` on Linux.
