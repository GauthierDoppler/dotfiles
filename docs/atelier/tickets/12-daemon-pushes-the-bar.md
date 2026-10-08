# 12: Daemon pushes the bar

**What to build:** The bar stops running scripts. The daemon renders both blocks and sets them as session options on session switch, client resize and marker changes; the tmux config shows them and falls back to the `#()` call when they are unset.

**Blocked by:** 04 (Left block of the bar rendered by atelier), 11 (Daemon tracer bullet: atelier knows the live tmux state)

**Status:** done

- [x] Switching session updates both blocks without any `#()` call
- [x] Resizing the client re-renders at the new width tier
- [x] Killing the daemon leaves a working bar through the fallback
- [x] The control-mode client is never counted as someone looking at a window

## Comments

- No bash is replaced: the `#()` calls stay as the fallback, so nothing was
  deleted.
- "Marker changes" are not a trigger. `@task_status`/`@claude_status` are drawn
  by the window formats, not by `@bar_left`/`@bar_right`, so tmux redraws them
  itself; no subscription was added.
- The key-table slot stays a tmux format. Control mode has no key-table
  notification and a `refresh-client -B` subscription is evaluated against the
  daemon's own client (it reports `root`), so the daemon cannot observe it.
  The date also moved to tmux: the daemon writes `%a %d %b` and the config shows
  `#{T:@bar_right}`, so no midnight timer is needed. The clock is `%H:%M` after
  it, as planned.
- Resize signal: tmux 3.4 sends `%layout-change` only for windows of the
  control client's own session, and has no client-resize notification. The
  daemon installs a `client-resized[73]` hook that `display-message`s its own
  control client, which arrives as `%message`; fixture
  `tests/fixtures/control/client-resized.txt` (`record.sh client_resized`).
- Fallback: the config shows the pushed blocks only while the global
  `@bar_daemon` is set; a `client-detached[73]` hook installed by the daemon
  unsets it when the daemon's client goes away, which is what a killed daemon
  does. Stale per-session values are left in place and ignored.
- Width: the most recently active (`#{client_activity}`) non-control client on
  the session. Sessions nobody is looking at are not rendered; they get their
  blocks when a client switches to them.
- Added beyond the list: the battery is re-read once a minute and the bar
  re-pushed only if the reading changed, since nothing else would refresh it
  now that `#()` no longer runs every `status-interval`.
- Control-client fix outside the daemon: the session picker's `· attached`
  label counted `#{session_attached}`, which includes the daemon. It now counts
  non-control clients; the existing test used a control client as the viewer
  and now uses a terminal client (`TmuxServer::attach_terminal_client`, via
  `script`). `session_last_attached` is still bumped once by the daemon's own
  attach.
- `session::resolve` was split into `session::identify` (takes a field reader)
  so the daemon resolves from control-mode data without forking tmux.
- `tests/hook_claude.rs` and `tests/tasks.rs` each still carry a private
  terminal-client helper that duplicates the new one in `tests/common`; left for
  ticket 28 to avoid touching files other tickets are changing.
- Unverified on macOS: battery change detection over IOKit, `script -q` in the
  new common helper (same invocation `tests/tasks.rs` already uses there).
