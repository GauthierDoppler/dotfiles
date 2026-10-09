# 11: Daemon tracer bullet: atelier knows the live tmux state

**What to build:** tmux starts `atelier daemon --ensure`; the daemon holds one control-mode connection, builds its view of sessions and windows, keeps it current from events, and exits with tmux. `atelier status` prints that view.

**Blocked by:** 02 (Atelier tracer bullet: the bar's project name comes from atelier)

**Status:** done

- [x] Starting tmux starts exactly one daemon per tmux socket, even when the config is reloaded
- [x] Creating, renaming and killing sessions and windows is reflected in `atelier status` without polling
- [x] The daemon exits when the tmux server exits
- [x] CLI commands still work when the daemon is not running
- [x] Control-mode parsing covered by recorded transcripts and by the harness

## Comments

- No bash is replaced by this slice, so nothing was deleted.
- The daemon is started both by a `run-shell -b` at config load and by a
  `session-created` hook: at load the first session may not exist yet, and tmux
  3.4 has no session-less control client.
- "Exits on `%exit`" is softened: after `%exit` the daemon reattaches when the
  server still has a session (its own session was killed under
  `detach-on-destroy on`) and exits otherwise. Covered by
  `the_daemon_outlives_the_session_it_watches_through`.
- `atelier status` prints `daemon: running (pid N)` or `daemon: not running`
  first, then sessions sorted by name with `(attached)` when a non-control
  client is on them, then `index name` per window.
- `tests/common/mod.rs` now gives each test server its own `XDG_RUNTIME_DIR`
  and exposes `atelier_command`.
- Unverified on macOS: the `$TMPDIR/atelier-<uid>` fallback (no
  `XDG_RUNTIME_DIR` there), `File::try_lock` (flock), and `process_group(0)`
  detaching from the `run-shell` job. The real `dot_tmux.conf` was checked on
  Linux with a fake `$HOME`: one daemon after start and two reloads, none after
  `kill-server`, and no error when the binary is absent.
