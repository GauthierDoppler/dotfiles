# 09: Claude Code marker through atelier

**What to build:** Claude Code's hooks call `atelier hook claude`, which sets the window's waiting or done marker and clears it on prompt submit. Replaces the shell hook.

**Blocked by:** 02 (Atelier tracer bullet: the bar's project name comes from atelier)

**Status:** done

- [x] Event taken from the hook payload, one command for all three hook entries
- [x] Marker not set when its window is the active one of an attached, non-control-mode client
- [x] Marker cleared when the window is selected
- [x] Tested through the harness by feeding hook payloads
- [x] Claude settings point at the new command, old hook removed

## Comments

- `atelier hook claude` reads the payload on stdin and the window from
  `$TMUX_PANE`. It exits 0 on every path (no `$TMUX_PANE`, unknown pane, bad
  JSON, tmux error). Only `UserPromptSubmit` clears; any other event is a no-op,
  where `notify.sh` cleared on it — no such event is wired.
- The settings entries are `"$HOME/.local/bin/atelier" hook claude 2>/dev/null
  || true`: Claude Code treats exit 2 as blocking (it would discard the prompt
  on `UserPromptSubmit`), and clap exits 2 on an unknown subcommand, so a stale
  binary must not leak its status. A missing binary means no marker, nothing
  else. Recorded in AGENTS.md.
- "Watched" is now: some client with `client_control_mode` 0 has this window as
  its current window (`list-clients`), replacing `window_active &&
  session_attached`, which counted control-mode clients.
- `after-select-window` in `dot_tmux.conf` is kept unchanged; the test sources
  that exact line from the real config. Pre-existing gap, not addressed:
  `switch-client` onto a session whose current window is marked does not fire
  `after-select-window`, so that marker stays until the next window selection.
- Adds `serde_json` to `Cargo.toml`.
- `dot_claude/hooks/` stays linked: it still holds `ClaudeCodeNotifier.app`.
- The old script prepended `/opt/homebrew/bin` to `PATH`; atelier relies on the
  `PATH` Claude Code inherits to find `tmux`. Not verified on macOS (container
  is Linux, tmux 3.4) nor against a live Claude Code session.
