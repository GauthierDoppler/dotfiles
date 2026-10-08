# 10: Token picker

**What to build:** `Prefix + u` picks a URL or file path from the current pane through atelier: open, copy through OSC 52, or hand to the OS opener. The bash picker is deleted.

**Blocked by:** 02 (Atelier tracer bullet: the bar's project name comes from atelier)

**Status:** done

- [x] Candidates extracted in one pass over the capture
- [x] Path candidates that do not exist are dropped
- [x] Opening a file targets the nvim of this session, or a new nvim window at the session root
- [x] Copy works over SSH
- [x] Bash script removed, `AGENTS.md` updated

## Comments

- Built as `atelier pick list|open|system|copy|popup` (`atelier/src/pick.rs`);
  `Prefix + u` runs `atelier pick popup -t '#{pane_id}'`. `scripts/tmux-pick`,
  its `install.sh` link and its README line are gone. An existing machine keeps
  a dangling `~/.local/bin/tmux-pick` symlink until it is removed by hand.
- Deviations from the bash: a URL goes to the OS opener (default browser)
  instead of Google Chrome by name; copy is `set-buffer -w` only, no `pbcopy`,
  so the local clipboard depends on the terminal accepting OSC 52 (tmux has
  `set-clipboard on`); an empty pane shows a placeholder row instead of a
  message and an early exit, as the other pickers do; `~/` paths are now
  expanded for the existence check (the bash quoted them, so they never
  matched).
- Tests (`atelier/tests/pick.rs`): extraction against two captured fixtures, a
  wrapped URL, a 2000-line scrollback, every action through a private tmux
  server with fake `nvim`/`open`/`xdg-open`, OSC 52 observed by a second tmux
  server acting as the client's terminal, and Enter / Ctrl-y driven through
  real fzf. The fzf tests skip when fzf is not installed; CI installs none, so
  they only run locally. The `esc` binding needs a recent fzf (`transform`,
  `$FZF_INPUT_STATE`; Debian's 0.44 is too old), as the bash did; verified with fzf 0.65.2.
- Unverified on macOS: `open` as the opener, ghostty receiving OSC 52 through
  tmux, and `pane_current_command` reading `nvim` there.
